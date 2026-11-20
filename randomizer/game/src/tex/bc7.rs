//! BC7 blocks: decoding, and recolouring by rewriting the colour endpoints in place.
//!
//! A recolour keeps every index, partition, rotation and p-bit of a block and moves only its
//! endpoints, so the block's pattern stays and only its colours change, as BC1 recolouring
//! does. Link's adult hair albedo is BC7.

struct Mode {
    subsets: usize,
    part_bits: u32,
    rot_bits: u32,
    sel_bits: u32,
    color_bits: u32,
    alpha_bits: u32,
    /// One p-bit per endpoint.
    end_pbit: bool,
    /// One p-bit per subset, shared by its two endpoints.
    shared_pbit: bool,
    index_bits: u32,
    index2_bits: u32,
}

const fn mode(subsets: usize, part_bits: u32, rot_bits: u32, sel_bits: u32, color_bits: u32, alpha_bits: u32, end_pbit: bool, shared_pbit: bool, index_bits: u32, index2_bits: u32) -> Mode {
    Mode { subsets, part_bits, rot_bits, sel_bits, color_bits, alpha_bits, end_pbit, shared_pbit, index_bits, index2_bits }
}

const MODES: [Mode; 8] = [
    mode(3, 4, 0, 0, 4, 0, true, false, 3, 0),
    mode(2, 6, 0, 0, 6, 0, false, true, 3, 0),
    mode(3, 6, 0, 0, 5, 0, false, false, 2, 0),
    mode(2, 6, 0, 0, 7, 0, true, false, 2, 0),
    mode(1, 0, 2, 1, 5, 6, false, false, 2, 3),
    mode(1, 0, 2, 0, 7, 8, false, false, 2, 2),
    mode(1, 0, 0, 0, 7, 7, true, false, 4, 0),
    mode(2, 6, 0, 0, 5, 5, true, false, 2, 0),
];

/// Two subset partitions: bit `i` set puts texel `i` in subset 1.
const PART2: [u16; 64] = [
    0xcccc, 0x8888, 0xeeee, 0xecc8, 0xc880, 0xfeec, 0xfec8, 0xec80, 0xc800, 0xffec, 0xfe80, 0xe800, 0xffe8, 0xff00, 0xfff0, 0xf000, 0xf710, 0x008e, 0x7100, 0x08ce, 0x008c, 0x7310,
    0x3100, 0x8cce, 0x088c, 0x3110, 0x6666, 0x366c, 0x17e8, 0x0ff0, 0x718e, 0x399c, 0xaaaa, 0xf0f0, 0x5a5a, 0x33cc, 0x3c3c, 0x55aa, 0x9696, 0xa55a, 0x73ce, 0x13c8, 0x324c, 0x3bdc,
    0x6996, 0xc33c, 0x9966, 0x0660, 0x0272, 0x04e4, 0x4e40, 0x2720, 0xc936, 0x936c, 0x39c6, 0x639c, 0x9336, 0x9cc6, 0x817e, 0xe718, 0xccf0, 0x0fcc, 0x7744, 0xee22,
];

/// The texel of subset 1 whose index has one bit fewer, per two subset partition.
const ANCHOR2: [usize; 64] = [
    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 2, 8, 2, 2, 8, 8, 15, 2, 8, 2, 2, 8, 8, 2, 2, 15, 15, 6, 8, 2, 8, 15, 15, 2, 8, 2, 2, 2, 15, 15, 6, 6, 2, 6, 8, 15, 15, 2, 2,
    15, 15, 15, 15, 15, 2, 2, 15,
];

const W2: [u32; 4] = [0, 21, 43, 64];
const W3: [u32; 8] = [0, 9, 18, 27, 37, 46, 55, 64];
const W4: [u32; 16] = [0, 4, 9, 13, 17, 21, 26, 30, 34, 38, 43, 47, 51, 55, 60, 64];

fn rd(x: u128, pos: u32, n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    ((x >> pos) & ((1u128 << n) - 1)) as u32
}

fn wr(x: &mut u128, pos: u32, n: u32, v: u32) {
    if n == 0 {
        return;
    }
    let m: u128 = ((1u128 << n) - 1) << pos;
    *x = (*x & !m) | (((v as u128) << pos) & m);
}

fn weight(bits: u32, i: u32) -> u32 {
    match bits {
        2 => W2[i as usize],
        3 => W3[i as usize],
        _ => W4[i as usize],
    }
}

/// An endpoint value of `bits` bits, with its p-bit if it has one, widened to 8 bits.
fn widen(v: u32, bits: u32, p: Option<u32>) -> u32 {
    let (x, n): (u32, u32) = match p {
        Some(p) => ((v << 1) | p, bits + 1),
        None => (v, bits),
    };
    if n >= 8 { x } else { (x << (8 - n)) | (x >> (2 * n - 8)) }
}

/// The stored value whose widened value is closest to `want`, for a fixed p-bit.
fn narrow(want: u32, bits: u32, p: Option<u32>) -> u32 {
    (0..1u32 << bits).min_by_key(|v| widen(*v, bits, p).abs_diff(want)).unwrap_or(0)
}

struct Layout {
    mode: &'static Mode,
    part: usize,
    rot: u32,
    sel: u32,
    color: u32,
    alpha: u32,
    pbits: u32,
    index: u32,
}

fn layout(x: u128) -> Option<Layout> {
    let low: u32 = (x & 0xff) as u32;
    if low == 0 {
        return None;
    }
    let m: usize = low.trailing_zeros() as usize;
    let mode: &Mode = &MODES[m];
    let mut pos: u32 = m as u32 + 1;
    let part: usize = rd(x, pos, mode.part_bits) as usize;
    pos += mode.part_bits;
    let rot: u32 = rd(x, pos, mode.rot_bits);
    pos += mode.rot_bits;
    let sel: u32 = rd(x, pos, mode.sel_bits);
    pos += mode.sel_bits;
    let ends: u32 = 2 * mode.subsets as u32;
    let color: u32 = pos;
    let alpha: u32 = color + 3 * ends * mode.color_bits;
    let pbits: u32 = alpha + ends * mode.alpha_bits;
    let pcount: u32 = if mode.end_pbit {
        ends
    } else if mode.shared_pbit {
        mode.subsets as u32
    } else {
        0
    };
    Some(Layout { mode, part, rot, sel, color, alpha, pbits, index: pbits + pcount })
}

impl Layout {
    fn pbit(&self, x: u128, end: usize) -> Option<u32> {
        if self.mode.end_pbit {
            Some(rd(x, self.pbits + end as u32, 1))
        } else if self.mode.shared_pbit {
            Some(rd(x, self.pbits + (end / 2) as u32, 1))
        } else {
            None
        }
    }

    fn color_at(&self, c: usize, end: usize) -> u32 {
        self.color + ((c * 2 * self.mode.subsets + end) as u32) * self.mode.color_bits
    }

    fn alpha_at(&self, end: usize) -> u32 {
        self.alpha + end as u32 * self.mode.alpha_bits
    }

    /// Endpoint `end` as stored: RGB and alpha, widened, before any rotation.
    fn endpoint(&self, x: u128, end: usize) -> [u32; 4] {
        let p: Option<u32> = self.pbit(x, end);
        let mut e: [u32; 4] = [255; 4];
        for (c, v) in e.iter_mut().enumerate().take(3) {
            *v = widen(rd(x, self.color_at(c, end), self.mode.color_bits), self.mode.color_bits, p);
        }
        if self.mode.alpha_bits > 0 {
            let ap: Option<u32> = if self.mode.end_pbit { p } else { None };
            e[3] = widen(rd(x, self.alpha_at(end), self.mode.alpha_bits), self.mode.alpha_bits, ap);
        }
        e
    }

    fn subset(&self, texel: usize) -> Option<usize> {
        match self.mode.subsets {
            1 => Some(0),
            2 => Some(((PART2[self.part] >> texel) & 1) as usize),
            _ => None,
        }
    }

    fn anchor(&self, texel: usize) -> bool {
        texel == 0 || (self.mode.subsets == 2 && texel == ANCHOR2[self.part])
    }
}

/// Decodes one BC7 block into 16 texels. Blocks of the three subset modes, and invalid
/// blocks, are not read.
pub fn decode(b: &[u8], out: &mut [[u8; 4]; 16]) -> bool {
    let Ok(arr) = <[u8; 16]>::try_from(b) else {
        return false;
    };
    let x: u128 = u128::from_le_bytes(arr);
    let Some(l) = layout(x) else {
        return false;
    };
    if l.mode.subsets == 3 {
        return false;
    }
    let mut ends: [[u32; 4]; 4] = [[0; 4]; 4];
    for (i, e) in ends.iter_mut().enumerate().take(2 * l.mode.subsets) {
        *e = l.endpoint(x, i);
    }
    let mut prim: [u32; 16] = [0; 16];
    let mut pos: u32 = l.index;
    for (i, v) in prim.iter_mut().enumerate() {
        let n: u32 = l.mode.index_bits - l.anchor(i) as u32;
        *v = rd(x, pos, n);
        pos += n;
    }
    let mut sec: [u32; 16] = [0; 16];
    if l.mode.index2_bits > 0 {
        for (i, v) in sec.iter_mut().enumerate() {
            let n: u32 = l.mode.index2_bits - (i == 0) as u32;
            *v = rd(x, pos, n);
            pos += n;
        }
    }
    for (i, o) in out.iter_mut().enumerate() {
        let Some(s) = l.subset(i) else {
            return false;
        };
        let (e0, e1): ([u32; 4], [u32; 4]) = (ends[2 * s], ends[2 * s + 1]);
        let (ci, cb, ai, ab): (u32, u32, u32, u32) = if l.mode.index2_bits == 0 {
            (prim[i], l.mode.index_bits, prim[i], l.mode.index_bits)
        } else if l.sel == 1 {
            (sec[i], l.mode.index2_bits, prim[i], l.mode.index_bits)
        } else {
            (prim[i], l.mode.index_bits, sec[i], l.mode.index2_bits)
        };
        let (wc, wa): (u32, u32) = (weight(cb, ci), weight(ab, ai));
        let mix = |a: u32, b: u32, w: u32| (((64 - w) * a + w * b + 32) >> 6) as u8;
        let mut px: [u8; 4] = [mix(e0[0], e1[0], wc), mix(e0[1], e1[1], wc), mix(e0[2], e1[2], wc), mix(e0[3], e1[3], wa)];
        if l.rot > 0 {
            px.swap(3, l.rot as usize - 1);
        }
        *o = px;
    }
    true
}

/// Moves every colour endpoint of one BC7 block through `f`. A rotated block keeps one
/// colour channel in its alpha endpoints, so that channel is read and written there, and the
/// real alpha, kept in the colour channel, is left alone. Returns whether the block changed.
pub fn recolor(b: &mut [u8], f: &dyn Fn([u8; 3]) -> [u8; 3]) -> bool {
    let Ok(arr) = <[u8; 16]>::try_from(&*b) else {
        return false;
    };
    let x: u128 = u128::from_le_bytes(arr);
    let Some(l) = layout(x) else {
        return false;
    };
    let rot: Option<usize> = (l.rot > 0).then(|| l.rot as usize - 1);
    let mut y: u128 = x;
    for end in 0..2 * l.mode.subsets {
        let e: [u32; 4] = l.endpoint(x, end);
        let mut rgb: [u32; 3] = [e[0], e[1], e[2]];
        if let Some(c) = rot {
            rgb[c] = e[3];
        }
        let now: [u8; 3] = f(rgb.map(|v| v as u8));
        let p: Option<u32> = l.pbit(x, end);
        for c in 0..3 {
            if rot == Some(c) {
                wr(&mut y, l.alpha_at(end), l.mode.alpha_bits, narrow(now[c] as u32, l.mode.alpha_bits, None));
            } else {
                wr(&mut y, l.color_at(c, end), l.mode.color_bits, narrow(now[c] as u32, l.mode.color_bits, p));
            }
        }
    }
    if y == x {
        return false;
    }
    b.copy_from_slice(&y.to_le_bytes());
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(m: usize, fill: &dyn Fn(&mut u128, &Layout)) -> [u8; 16] {
        let mut x: u128 = 1u128 << m;
        let l: Layout = layout(x).unwrap();
        fill(&mut x, &l);
        x.to_le_bytes()
    }

    fn px(b: &[u8; 16]) -> [[u8; 4]; 16] {
        let mut o: [[u8; 4]; 16] = [[0; 4]; 16];
        assert!(decode(b, &mut o));
        o
    }

    #[test]
    fn every_anchor_sits_in_its_own_subset() {
        for (p, a) in ANCHOR2.iter().enumerate() {
            assert_eq!((PART2[p] >> a) & 1, 1, "partition {p}");
            assert_eq!(PART2[p] & 1, 0, "partition {p} starts in subset 0");
        }
    }

    #[test]
    fn widening_spreads_the_top_bits_and_narrowing_undoes_it() {
        assert_eq!(widen(0x7f, 7, None), 0xff);
        assert_eq!(widen(0, 7, Some(1)), 1);
        assert_eq!(widen(0x1f, 5, None), 0xff);
        for bits in [4, 5, 6, 7] {
            for p in [None, Some(0), Some(1)] {
                for v in 0..1u32 << bits {
                    assert_eq!(narrow(widen(v, bits, p), bits, p), v, "{bits} {p:?} {v}");
                }
            }
        }
    }

    #[test]
    fn a_mode_six_block_decodes_its_endpoints_and_weights() {
        let b: [u8; 16] = block(6, &|x, l| {
            for (c, (a, z)) in [(10u32, 100u32), (60, 20), (127, 0)].iter().enumerate() {
                wr(x, l.color_at(c, 0), 7, *a);
                wr(x, l.color_at(c, 1), 7, *z);
            }
            wr(x, l.alpha_at(0), 7, 127);
            wr(x, l.alpha_at(1), 7, 127);
            wr(x, l.pbits, 1, 1);
            wr(x, l.pbits + 1, 1, 1);
            wr(x, l.index + 3, 4, 15);
        });
        let o: [[u8; 4]; 16] = px(&b);
        assert_eq!(o[0], [21, 121, 255, 255]);
        assert_eq!(o[1], [201, 41, 1, 255]);
    }

    #[test]
    fn an_unchanged_map_leaves_every_mode_alone() {
        for m in 0..8 {
            let mut b: [u8; 16] = block(m, &|x, l| {
                for end in 0..2 * l.mode.subsets {
                    for c in 0..3 {
                        let top: u32 = (1 << l.mode.color_bits) - 1;
                        wr(x, l.color_at(c, end), l.mode.color_bits, (end as u32 * 7 + c as u32 * 5) % top);
                    }
                }
            });
            let old: [u8; 16] = b;
            assert!(!recolor(&mut b, &|c| c), "mode {m}");
            assert_eq!(b, old);
        }
    }

    #[test]
    fn a_rotated_block_recolours_the_channel_kept_in_alpha() {
        let mut b: [u8; 16] = block(5, &|x, l| {
            wr(x, l.color.saturating_sub(2), 2, 1);
            for (c, (a, z)) in [(30u32, 40u32), (20, 120), (10, 100)].iter().enumerate() {
                wr(x, l.color_at(c, 0), 7, *a);
                wr(x, l.color_at(c, 1), 7, *z);
            }
            wr(x, l.alpha_at(0), 8, 40);
            wr(x, l.alpha_at(1), 8, 220);
            wr(x, l.index + 1, 2, 3);
            wr(x, l.index + 31 + 1, 2, 3);
        });
        let l: Layout = layout(u128::from_le_bytes(b)).unwrap();
        assert_eq!(l.rot, 1);
        let old: [[u8; 4]; 16] = px(&b);
        let swap = |c: [u8; 3]| [c[1], c[2], c[0]];
        assert!(recolor(&mut b, &swap));
        let new: [[u8; 4]; 16] = px(&b);
        for (o, n) in old.iter().zip(new.iter()) {
            let want: [u8; 3] = swap([o[0], o[1], o[2]]);
            for c in 0..3 {
                assert!((want[c] as i32 - n[c] as i32).abs() <= 3, "{o:?} -> {n:?}");
            }
            assert_eq!(o[3], n[3], "alpha stays");
        }
    }

    #[test]
    fn a_two_subset_block_recolours_both_subsets_and_keeps_its_pattern() {
        let mut b: [u8; 16] = block(3, &|x, l| {
            wr(x, 4, 6, 13);
            for end in 0..4 {
                for c in 0..3 {
                    wr(x, l.color_at(c, end), 7, [[10, 90, 30], [40, 120, 60], [100, 10, 10], [127, 40, 40]][end][c]);
                }
            }
        });
        let old: [[u8; 4]; 16] = px(&b);
        let darker = |c: [u8; 3]| c.map(|v| v / 2);
        assert!(recolor(&mut b, &darker));
        let new: [[u8; 4]; 16] = px(&b);
        for (o, n) in old.iter().zip(new.iter()) {
            for c in 0..3 {
                assert!((o[c] as i32 / 2 - n[c] as i32).abs() <= 2, "{o:?} -> {n:?}");
            }
        }
    }

    #[test]
    fn the_real_hair_decodes_and_an_unchanged_map_keeps_it_byte_for_byte() {
        let Some(root) = std::env::var("OOTRR_ROMFS").map(std::path::PathBuf::from).ok().filter(|p| p.join("Pack").join("ZsDic.pack.zs").is_file()) else {
            return;
        };
        let mut rf: crate::romfs::Romfs = crate::romfs::Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read_zs("Tex/Mt_Hairs_Player_Adult_Alb.bntx.zs").unwrap();
        let t: crate::tex::bntx::Tex = crate::tex::bntx::parse(&raw).unwrap().remove(0);
        let (mut good, mut all): (usize, usize) = (0, 0);
        let mut o: [[u8; 4]; 16] = [[0; 4]; 16];
        for blk in raw[t.off..t.off + t.size].chunks_exact(16) {
            all += 1;
            good += decode(blk, &mut o) as usize;
            let mut same: Vec<u8> = blk.to_vec();
            assert!(!recolor(&mut same, &|c| c));
            assert_eq!(same, blk);
        }
        assert!(good * 100 >= all * 99, "{good} of {all}");
    }
}
