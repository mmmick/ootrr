use super::{COLOR_LEVELS, Ends, Kind, Pair, block_mode, endpoints, ise, ise_bits, level, quints, rd, trits, unq_color, unq_weight};

const TRIT_BITS: [u32; 5] = [2, 2, 1, 2, 1];
const QUINT_BITS: [u32; 3] = [3, 2, 2];
const PASSES: usize = 4;
const REACH: usize = 2;

pub struct Quant {
    vals: Vec<(i32, Pair)>,
    near: Vec<usize>,
}

pub struct Tables {
    q: Vec<Quant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Done {
    Skipped,
    Same,
    Changed,
}

fn digits(k: Kind) -> u32 {
    match k {
        Kind::Bits => 1,
        Kind::Trit => 3,
        Kind::Quint => 5,
    }
}

fn quant(lv: u32) -> Quant {
    let (b, k): (u32, Kind) = level(lv);
    let mut vals: Vec<(i32, Pair)> = Vec::new();
    for d in 0..digits(k) {
        for m in 0..(1u32 << b) {
            vals.push((unq_color(d, m, lv) as i32, (d, m)));
        }
    }
    vals.sort_unstable();
    let mut near: Vec<usize> = Vec::with_capacity(256);
    for want in 0..256i32 {
        let mut best: usize = 0;
        for (i, v) in vals.iter().enumerate() {
            if (v.0 - want).abs() < (vals[best].0 - want).abs() {
                best = i;
            }
        }
        near.push(best);
    }
    Quant { vals, near }
}

impl Tables {
    pub fn new() -> Tables {
        Tables { q: COLOR_LEVELS.iter().map(|lv| quant(*lv)).collect() }
    }

    fn of(&self, lv: u32) -> Option<&Quant> {
        COLOR_LEVELS.iter().position(|l| *l == lv).map(|i| &self.q[i])
    }
}

impl Default for Tables {
    fn default() -> Tables {
        Tables::new()
    }
}

impl Quant {
    fn find(&self, p: Pair) -> usize {
        self.vals.iter().position(|v| v.1 == p).unwrap_or(0)
    }

    fn pick(&self, want: i32) -> usize {
        self.near[want.clamp(0, 255) as usize]
    }
}

fn put(out: &mut u128, pos: &mut u32, v: u32, n: u32) {
    if n > 0 && *pos < 128 {
        *out |= ((v as u128) & ((1u128 << n) - 1)) << *pos;
    }
    *pos += n;
}

fn pack(ps: &[Pair], lv: u32) -> Option<u128> {
    let (b, k): (u32, Kind) = level(lv);
    let mut out: u128 = 0;
    let mut pos: u32 = 0;
    match k {
        Kind::Bits => {
            for p in ps {
                put(&mut out, &mut pos, p.1, b);
            }
        }
        Kind::Trit => {
            for g in ps.chunks(5) {
                let mut want: [u32; 5] = [0; 5];
                for (i, p) in g.iter().enumerate() {
                    want[i] = p.0;
                }
                let t: u32 = (0..256u32).find(|t| trits(*t) == want)?;
                let mut tb: u32 = 0;
                for (j, n) in TRIT_BITS.iter().enumerate() {
                    put(&mut out, &mut pos, g.get(j).map(|p| p.1).unwrap_or(0), b);
                    put(&mut out, &mut pos, t >> tb, *n);
                    tb += *n;
                }
            }
        }
        Kind::Quint => {
            for g in ps.chunks(3) {
                let mut want: [u32; 3] = [0; 3];
                for (i, p) in g.iter().enumerate() {
                    want[i] = p.0;
                }
                let q: u32 = (0..128u32).find(|q| quints(*q) == want)?;
                let mut qb: u32 = 0;
                for (j, n) in QUINT_BITS.iter().enumerate() {
                    put(&mut out, &mut pos, g.get(j).map(|p| p.1).unwrap_or(0), b);
                    put(&mut out, &mut pos, q >> qb, *n);
                    qb += *n;
                }
            }
        }
    }
    let n: u32 = ise_bits(ps.len() as u32, lv);
    if n < 128 && out >> n != 0 {
        return None;
    }
    Some(out)
}

fn mirror(p: Pair, lv: u32) -> Option<Pair> {
    let (b, k): (u32, Kind) = level(lv);
    let want: u32 = 64 - unq_weight(p.0, p.1, lv);
    for d in 0..digits(k) {
        for m in 0..(1u32 << b) {
            if unq_weight(d, m, lv) == want {
                return Some((d, m));
            }
        }
    }
    None
}

fn grey(c: &[i32; 4]) -> i32 {
    (c[0] + c[1] + c[2] + 1) / 3
}

fn unblue(c: &[i32; 4]) -> [i32; 4] {
    [(2 * c[0] - c[2]).clamp(0, 255), (2 * c[1] - c[2]).clamp(0, 255), c[2], c[3]]
}

fn offset(base: i32, to: i32) -> (i32, i32) {
    let off: i32 = (to - base).clamp(-32, 31);
    (((base << 1) & 0xff) | 1, (base & 0x80) | ((off & 0x3f) << 1))
}

fn guess(cem: u32, w: &Ends) -> ([[i32; 8]; 2], usize) {
    let (a, b): (&[i32; 4], &[i32; 4]) = (&w.0, &w.1);
    let mut g: [[i32; 8]; 2] = [[0; 8]; 2];
    let mut n: usize = 1;
    match cem {
        0 | 4 => g[0] = [grey(a), grey(b), a[3], b[3], 0, 0, 0, 0],
        1 => {
            let l: i32 = grey(a);
            g[0][0] = ((l & 0x3f) << 2) | 2;
            g[0][1] = (l & 0xc0) | (grey(b) - l).clamp(0, 63);
        }
        5 => {
            (g[0][0], g[0][1]) = offset(grey(a), grey(b));
            (g[0][2], g[0][3]) = offset(a[3], b[3]);
        }
        6 | 10 => {
            let dot: i32 = b[0] * b[0] + b[1] * b[1] + b[2] * b[2];
            let s: i32 = if dot > 0 { ((256 * (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]) + dot / 2) / dot).clamp(0, 255) } else { 0 };
            g[0] = [b[0], b[1], b[2], s, a[3], b[3], 0, 0];
        }
        8 | 12 => {
            let (ua, ub): ([i32; 4], [i32; 4]) = (unblue(a), unblue(b));
            g[0] = [a[0], b[0], a[1], b[1], a[2], b[2], a[3], b[3]];
            g[1] = [ub[0], ua[0], ub[1], ua[1], ub[2], ua[2], b[3], a[3]];
            n = 2;
        }
        _ => {
            let (ua, ub): ([i32; 4], [i32; 4]) = (unblue(a), unblue(b));
            for c in 0..3 {
                (g[0][2 * c], g[0][2 * c + 1]) = offset(a[c], b[c]);
                (g[1][2 * c], g[1][2 * c + 1]) = offset(ub[c], ua[c]);
            }
            (g[0][6], g[0][7]) = offset(a[3], b[3]);
            (g[1][6], g[1][7]) = offset(b[3], a[3]);
            n = 2;
        }
    }
    (g, n)
}

fn wide(cem: u32, i: usize) -> bool {
    matches!(cem, 1 | 5 | 9 | 13) && i % 2 == 1
}

fn miss(cem: u32, q: &Quant, idx: &[usize], w: &Ends) -> u32 {
    let mut v: [i32; 8] = [0; 8];
    for (i, x) in idx.iter().enumerate() {
        v[i] = q.vals[*x].0;
    }
    let Some((a, b)) = endpoints(cem, &v[..idx.len()]) else {
        return u32::MAX;
    };
    let mut e: u32 = 0;
    for c in 0..4 {
        e += ((a[c] - w.0[c]) * (a[c] - w.0[c]) + (b[c] - w.1[c]) * (b[c] - w.1[c])) as u32;
    }
    e
}

fn refine(cem: u32, q: &Quant, idx: &mut [usize], w: &Ends) -> u32 {
    let mut best: u32 = miss(cem, q, idx, w);
    for _ in 0..PASSES {
        let before: u32 = best;
        for i in 0..idx.len() {
            let (lo, hi): (usize, usize) = if wide(cem, i) { (0, q.vals.len() - 1) } else { (idx[i].saturating_sub(REACH), (idx[i] + REACH).min(q.vals.len() - 1)) };
            let keep: usize = idx[i];
            let mut pick: usize = keep;
            for x in lo..=hi {
                idx[i] = x;
                let e: u32 = miss(cem, q, idx, w);
                if e < best {
                    best = e;
                    pick = x;
                }
            }
            idx[i] = pick;
        }
        if best == before || best == 0 {
            break;
        }
    }
    best
}

fn fit(cem: u32, q: &Quant, cur: &[usize], w: &Ends, keep: bool) -> (u32, [usize; 8]) {
    let n: usize = cur.len();
    let mut out: [usize; 8] = [0; 8];
    let mut best: u32 = u32::MAX;
    if keep {
        out[..n].copy_from_slice(cur);
        best = refine(cem, q, &mut out[..n], w);
    }
    let (gs, m): ([[i32; 8]; 2], usize) = guess(cem, w);
    for g in gs.iter().take(m) {
        let mut idx: [usize; 8] = [0; 8];
        for i in 0..n {
            idx[i] = q.pick(g[i]);
        }
        let e: u32 = refine(cem, q, &mut idx[..n], w);
        if e < best {
            best = e;
            out = idx;
        }
    }
    (best, out)
}

fn void(data: &mut [u8; 16], f: &dyn Fn([u8; 3]) -> [u8; 3]) -> Done {
    let old: [u8; 3] = [data[9], data[11], data[13]];
    let new: [u8; 3] = f(old);
    if new == old {
        return Done::Same;
    }
    for c in 0..3 {
        data[8 + 2 * c] = new[c];
        data[9 + 2 * c] = new[c];
    }
    Done::Changed
}

fn shade(c: &[i32; 4], f: &dyn Fn([u8; 3]) -> [u8; 3]) -> [i32; 4] {
    let n: [u8; 3] = f([c[0] as u8, c[1] as u8, c[2] as u8]);
    [n[0] as i32, n[1] as i32, n[2] as i32, c[3]]
}

/// How far from grey a wanted endpoint may be before a luminance block is given colour.
const LUM_CHROMA: i32 = 6;

fn chroma(c: &[i32; 4]) -> i32 {
    c[..3].iter().max().unwrap_or(&0) - c[..3].iter().min().unwrap_or(&0)
}

/// Rewrites a block whose endpoints are luminance only (modes 0, 1, 4, 5) in the matching
/// direct colour mode (8, or 12 with alpha), so a grey block can take a colour. The weights
/// and partitions stay; the endpoints are refit at the finest level the freed bits allow.
fn promote(x: u128, to: u32, parts: usize, start: u32, below: u32, want: &[Ends; 4], tb: &Tables) -> Option<u128> {
    let n: usize = 2 * ((to >> 2) as usize + 1);
    let nvals: u32 = (n * parts) as u32;
    if nvals > 18 || below < start {
        return None;
    }
    let clv: u32 = COLOR_LEVELS.iter().copied().find(|lv| ise_bits(nvals, *lv) <= below - start)?;
    let q: &Quant = tb.of(clv)?;
    let mut ps: [Pair; 18] = [(0, 0); 18];
    for i in 0..parts {
        let (_, got): (u32, [usize; 8]) = fit(to, q, &[0; 8][..n], &want[i], false);
        for j in 0..n {
            ps[i * n + j] = q.vals[got[j]].1;
        }
    }
    let bits: u128 = pack(&ps[..nvals as usize], clv)?;
    let span: u128 = ((1u128 << (below - start)) - 1) << start;
    let mut y: u128 = (x & !span) | (bits << start);
    if parts == 1 {
        y = (y & !(0xfu128 << 13)) | ((to as u128) << 13);
    } else {
        y = (y & !(0xfu128 << 25)) | ((to as u128) << 25);
    }
    Some(y)
}

fn near_weight(lv: u32, want: u32) -> (Pair, u32) {
    let (b, k): (u32, Kind) = level(lv);
    let mut best: (Pair, u32) = ((0, 0), unq_weight(0, 0, lv));
    for d in 0..digits(k) {
        for m in 0..(1u32 << b) {
            let v: u32 = unq_weight(d, m, lv);
            if v.abs_diff(want) < best.1.abs_diff(want) {
                best = ((d, m), v);
            }
        }
    }
    best
}

fn sample(tw: &[u32], bw: usize, bh: usize, s: f32, t: f32) -> u32 {
    let (x0, y0): (usize, usize) = (s.floor() as usize, t.floor() as usize);
    let (x1, y1): (usize, usize) = ((x0 + 1).min(bw - 1), (y0 + 1).min(bh - 1));
    let (fx, fy): (f32, f32) = (s - x0 as f32, t - y0 as f32);
    let at = |x: usize, y: usize| tw[y * bw + x] as f32;
    let v: f32 = at(x0, y0) * (1.0 - fx) * (1.0 - fy) + at(x1, y0) * fx * (1.0 - fy) + at(x0, y1) * (1.0 - fx) * fy + at(x1, y1) * fx * fy;
    v.round() as u32
}

/// Rewrites a single partition luminance block whose weights leave no room for colour
/// endpoints. The texel weights are resampled onto the coarser weight grid that best keeps
/// the block's shading while freeing enough bits for direct colour endpoints.
fn rebuild(x: u128, to: u32, bw: usize, bh: usize, old: (usize, usize, u32), want: &Ends, tb: &Tables) -> Option<u128> {
    let (xw, yw, wlv): (usize, usize, u32) = old;
    let texels: usize = bw * bh;
    if bw < 2 || bh < 2 || texels > super::MAX_TEXELS || xw * yw > 64 {
        return None;
    }
    let wbits: u32 = ise_bits((xw * yw) as u32, wlv);
    let mut raw: [Pair; 64] = [(0, 0); 64];
    ise(x.reverse_bits() & ((1u128 << wbits) - 1), xw * yw, wlv, &mut raw);
    let wts: Vec<u32> = raw[..xw * yw].iter().map(|p| unq_weight(p.0, p.1, wlv)).collect();
    let mut tw: [u32; super::MAX_TEXELS] = [0; super::MAX_TEXELS];
    super::infill(&wts, 0, 1, xw, yw, bw, bh, &mut tw);
    let n: u32 = 2 * ((to >> 2) + 1);
    let span: f32 = (0..3).map(|c| (want.1[c] - want.0[c]).abs()).max().unwrap_or(0) as f32;
    let mut seen: std::collections::BTreeSet<(usize, usize, u32)> = std::collections::BTreeSet::new();
    let mut best: Option<(f32, u32, u32, u32, Vec<Pair>)> = None;
    for m in 0..2048u32 {
        if m & 0x1ff == 0x1fc {
            continue;
        }
        let Some((nx, ny, dual, lv)) = block_mode(m) else {
            continue;
        };
        if dual || nx > bw || ny > bh || nx * ny > 64 || !seen.insert((nx, ny, lv)) {
            continue;
        }
        let nb: u32 = ise_bits((nx * ny) as u32, lv);
        if !(24..=96).contains(&nb) || 128 - nb < 17 {
            continue;
        }
        let Some(clv) = COLOR_LEVELS.iter().copied().find(|l| ise_bits(n, *l) <= 128 - nb - 17) else {
            continue;
        };
        if clv < 32 {
            continue;
        }
        let mut g: Vec<Pair> = Vec::with_capacity(nx * ny);
        let mut gv: Vec<u32> = Vec::with_capacity(nx * ny);
        for j in 0..ny {
            for i in 0..nx {
                let s: f32 = i as f32 * (bw - 1) as f32 / (nx - 1) as f32;
                let t: f32 = j as f32 * (bh - 1) as f32 / (ny - 1) as f32;
                let (p, v): (Pair, u32) = near_weight(lv, sample(&tw, bw, bh, s, t));
                g.push(p);
                gv.push(v);
            }
        }
        let mut out: [u32; super::MAX_TEXELS] = [0; super::MAX_TEXELS];
        super::infill(&gv, 0, 1, nx, ny, bw, bh, &mut out);
        let werr: f32 = (0..texels).map(|i| ((out[i] as f32 - tw[i] as f32) * span / 64.0).powi(2)).sum();
        let step: f32 = 256.0 / clv as f32;
        let score: f32 = werr + texels as f32 * step * step / 4.0;
        if best.as_ref().is_none_or(|b| score < b.0) {
            best = Some((score, m, clv, lv, g));
        }
    }
    let (_, m, clv, lv, g): (f32, u32, u32, u32, Vec<Pair>) = best?;
    let q: &Quant = tb.of(clv)?;
    let (_, got): (u32, [usize; 8]) = fit(to, q, &[0; 8][..n as usize], want, false);
    let ps: Vec<Pair> = got[..n as usize].iter().map(|i| q.vals[*i].1).collect();
    let cbits: u128 = pack(&ps, clv)?;
    let wb: u32 = ise_bits(g.len() as u32, lv);
    let w: u128 = pack(&g, lv)?;
    let mut y: u128 = (m as u128) | ((to as u128) << 13) | (cbits << 17);
    y = (y & ((1u128 << (128 - wb)) - 1)) | w.reverse_bits();
    Some(y)
}

pub fn recolor_block(data: &mut [u8; 16], tb: &Tables, f: &dyn Fn([u8; 3]) -> [u8; 3]) -> Done {
    recolor_sized(data, 0, 0, tb, f)
}

/// Recolours one block of a texture whose blocks are `bw` by `bh` texels. Knowing the size
/// lets a grey block with no room for colour endpoints be rebuilt on a coarser weight grid.
pub fn recolor_sized(data: &mut [u8; 16], bw: usize, bh: usize, tb: &Tables, f: &dyn Fn([u8; 3]) -> [u8; 3]) -> Done {
    let x: u128 = u128::from_le_bytes(*data);
    let m: u32 = rd(x, 0, 11);
    if m & 0x1ff == 0x1fc {
        if (m >> 9) & 1 != 0 {
            return Done::Skipped;
        }
        return void(data, f);
    }
    let Some((xw, yw, dual, wlv)) = block_mode(m) else {
        return Done::Skipped;
    };
    let parts: usize = rd(x, 11, 2) as usize + 1;
    let planes: usize = if dual { 2 } else { 1 };
    let nw: usize = xw * yw * planes;
    if (dual && parts == 4) || nw > 64 {
        return Done::Skipped;
    }
    let wbits: u32 = ise_bits(nw as u32, wlv);
    if !(24..=96).contains(&wbits) {
        return Done::Skipped;
    }
    let mut below: u32 = 128 - wbits;
    if dual {
        below -= 2;
    }
    let mut cems: [u32; 4] = [0; 4];
    let start: u32;
    if parts == 1 {
        cems[0] = rd(x, 13, 4);
        start = 17;
    } else {
        start = 29;
        let mut enc: u32 = rd(x, 23, 6);
        let base: u32 = enc & 3;
        if base == 0 {
            cems[..parts].fill((enc >> 2) & 0xf);
        } else {
            let extra: u32 = 3 * parts as u32 - 4;
            if below < extra {
                return Done::Skipped;
            }
            below -= extra;
            enc |= rd(x, below, extra) << 6;
            for (i, c) in cems.iter_mut().enumerate().take(parts) {
                let cs: u32 = (enc >> (2 + i as u32)) & 1;
                let mm: u32 = (enc >> (2 + parts as u32 + 2 * i as u32)) & 3;
                *c = ((base - 1 + cs) << 2) | mm;
            }
        }
    }
    let nvals: u32 = cems[..parts].iter().map(|c| 2 * ((c >> 2) + 1)).sum();
    if nvals > 18 || below < start {
        return Done::Skipped;
    }
    let Some(clv) = COLOR_LEVELS.iter().copied().find(|lv| ise_bits(nvals, *lv) <= below - start) else {
        return Done::Skipped;
    };
    let Some(q) = tb.of(clv) else {
        return Done::Skipped;
    };
    let cbits: u32 = ise_bits(nvals, clv);
    let mut craw: [Pair; 64] = [(0, 0); 64];
    ise(x >> start & ((1u128 << cbits) - 1), nvals as usize, clv, &mut craw);
    let mut cur: [usize; 18] = [0; 18];
    let mut vals: [i32; 18] = [0; 18];
    for i in 0..nvals as usize {
        cur[i] = q.find(craw[i]);
        vals[i] = q.vals[cur[i]].0;
    }
    let mut want: [Ends; 4] = [([0; 4], [0; 4]); 4];
    let mut same: bool = true;
    let mut pos: usize = 0;
    for i in 0..parts {
        let n: usize = 2 * ((cems[i] >> 2) as usize + 1);
        let Some((a, b)) = endpoints(cems[i], &vals[pos..pos + n]) else {
            return Done::Skipped;
        };
        want[i] = (shade(&a, f), shade(&b, f));
        same &= want[i] == (a, b);
        pos += n;
    }
    if same {
        return Done::Same;
    }
    let shared: bool = parts == 1 || rd(x, 23, 2) == 0;
    let tinted: bool = want[..parts].iter().any(|(a, b)| chroma(a) > LUM_CHROMA || chroma(b) > LUM_CHROMA);
    if tinted && shared && !dual && cems[..parts].iter().all(|c| matches!(c, 0 | 1 | 4 | 5)) {
        let to: u32 = if cems[0] & 4 == 0 { 8 } else { 12 };
        if let Some(y) = promote(x, to, parts, start, below, &want, tb) {
            *data = y.to_le_bytes();
            return Done::Changed;
        }
        if parts == 1
            && let Some(y) = rebuild(x, to, bw, bh, (xw, yw, wlv), &want[0], tb)
        {
            *data = y.to_le_bytes();
            return Done::Changed;
        }
    }
    let step: u32 = 256 / clv;
    let tol: u32 = parts as u32 * 6 * (step / 2 + 2) * (step / 2 + 2);
    let mut best: (u32, [usize; 18], bool) = (u32::MAX, [0; 18], false);
    for flip in [false, true] {
        let mut total: u32 = 0;
        let mut idx: [usize; 18] = [0; 18];
        let mut pos: usize = 0;
        for i in 0..parts {
            let n: usize = 2 * ((cems[i] >> 2) as usize + 1);
            let w: Ends = if flip { (want[i].1, want[i].0) } else { want[i] };
            let (e, got): (u32, [usize; 8]) = fit(cems[i], q, &cur[pos..pos + n], &w, !flip);
            total = total.saturating_add(e);
            idx[pos..pos + n].copy_from_slice(&got[..n]);
            pos += n;
        }
        if total < best.0 {
            best = (total, idx, flip);
        }
        if best.0 <= tol {
            break;
        }
    }
    let mut ps: [Pair; 18] = [(0, 0); 18];
    for i in 0..nvals as usize {
        ps[i] = q.vals[best.1[i]].1;
    }
    let Some(bits) = pack(&ps[..nvals as usize], clv) else {
        return Done::Skipped;
    };
    let mut y: u128 = x & !(((1u128 << cbits) - 1) << start) | (bits << start);
    if best.2 {
        let mut raw: [Pair; 64] = [(0, 0); 64];
        ise(x.reverse_bits() & ((1u128 << wbits) - 1), nw, wlv, &mut raw);
        for p in raw.iter_mut().take(nw) {
            let Some(n) = mirror(*p, wlv) else {
                return Done::Skipped;
            };
            *p = n;
        }
        let Some(w) = pack(&raw[..nw], wlv) else {
            return Done::Skipped;
        };
        y = (y & ((1u128 << (128 - wbits)) - 1)) | w.reverse_bits();
    }
    if y == x {
        return Done::Same;
    }
    *data = y.to_le_bytes();
    Done::Changed
}

#[cfg(test)]
mod tests {
    use super::super::decode_block;
    use super::*;

    fn set(x: &mut u128, pos: u32, n: u32, v: u32) {
        *x |= ((v as u128) & ((1u128 << n) - 1)) << pos;
    }

    fn rgb(vals: [u32; 6], weights: &[(usize, u32)]) -> [u8; 16] {
        let mut x: u128 = 0;
        set(&mut x, 0, 11, 0x42);
        set(&mut x, 13, 4, 8);
        for (i, v) in vals.iter().enumerate() {
            set(&mut x, 17 + 8 * i as u32, 8, *v);
        }
        for (i, w) in weights {
            set(&mut x, 127 - 2 * *i as u32, 1, w & 1);
            set(&mut x, 126 - 2 * *i as u32, 1, (w >> 1) & 1);
        }
        x.to_le_bytes()
    }

    fn px(b: &[u8; 16]) -> Vec<[u8; 4]> {
        let mut out: [[u8; 4]; 16] = [[0; 4]; 16];
        assert!(decode_block(b, 4, 4, false, &mut out));
        out.to_vec()
    }

    fn swap(c: [u8; 3]) -> [u8; 3] {
        [c[1], c[2], c[0]]
    }

    fn close(a: &[[u8; 4]], b: &[[u8; 4]], by: i32) -> bool {
        a.iter().zip(b).all(|(p, q)| (0..4).all(|c| (p[c] as i32 - q[c] as i32).abs() <= by))
    }

    #[test]
    fn an_unchanged_colour_map_leaves_the_bytes_alone() {
        let tb: Tables = Tables::new();
        let mut b: [u8; 16] = rgb([10, 200, 20, 210, 30, 220], &[(1, 3), (4, 1), (9, 2)]);
        let old: [u8; 16] = b;
        assert_eq!(recolor_block(&mut b, &tb, &|c| c), Done::Same);
        assert_eq!(b, old);
    }

    #[test]
    fn a_channel_swap_moves_every_texel_of_a_direct_block() {
        let tb: Tables = Tables::new();
        let mut b: [u8; 16] = rgb([10, 200, 20, 210, 30, 220], &[(1, 3), (4, 1), (9, 2), (15, 3)]);
        let old: Vec<[u8; 4]> = px(&b);
        assert_eq!(recolor_block(&mut b, &tb, &swap), Done::Changed);
        let want: Vec<[u8; 4]> = old.iter().map(|p| [p[1], p[2], p[0], p[3]]).collect();
        assert_eq!(px(&b), want);
    }

    fn lum(vals: [u32; 2], weights: &[(usize, u32)]) -> [u8; 16] {
        let mut x: u128 = 0;
        set(&mut x, 0, 11, 0x42);
        set(&mut x, 13, 4, 0);
        for (i, v) in vals.iter().enumerate() {
            set(&mut x, 17 + 8 * i as u32, 8, *v);
        }
        for (i, w) in weights {
            set(&mut x, 127 - 2 * *i as u32, 1, w & 1);
            set(&mut x, 126 - 2 * *i as u32, 1, (w >> 1) & 1);
        }
        x.to_le_bytes()
    }

    #[test]
    fn a_grey_luminance_block_is_rewritten_in_colour_with_its_weights() {
        let tb: Tables = Tables::new();
        let mut b: [u8; 16] = lum([40, 200], &[(1, 3), (4, 1), (9, 2), (15, 3)]);
        let old: Vec<[u8; 4]> = px(&b);
        assert!(old.iter().all(|p| p[0] == p[1] && p[1] == p[2]));
        let green = |c: [u8; 3]| [c[0] / 4, c[1], c[2] / 4];
        assert_eq!(recolor_block(&mut b, &tb, &green), Done::Changed);
        assert_eq!(rd(u128::from_le_bytes(b), 13, 4), 8);
        let new: Vec<[u8; 4]> = px(&b);
        let want: Vec<[u8; 4]> = old.iter().map(|p| [p[0] / 4, p[1], p[2] / 4, p[3]]).collect();
        assert!(close(&new, &want, 6), "{new:?} {want:?}");
        let mut g: [u8; 16] = lum([40, 200], &[(1, 3)]);
        assert_eq!(recolor_block(&mut g, &tb, &|c| [c[0] / 2, c[1] / 2, c[2] / 2]), Done::Changed);
        assert_eq!(rd(u128::from_le_bytes(g), 13, 4), 0, "a grey target keeps the luminance mode");
    }

    #[test]
    fn a_grey_block_full_of_weights_is_rebuilt_on_a_coarser_grid_in_colour() {
        let tb: Tables = Tables::new();
        let (bw, bh): (usize, usize) = (8, 8);
        let m: u32 = (0..2048u32)
            .filter(|m| m & 0x1ff != 0x1fc)
            .find(|m| block_mode(*m).is_some_and(|(x, y, d, lv)| !d && x == 8 && y == 6 && ise_bits(48, lv) > 80))
            .unwrap();
        let (xw, yw, _, wlv): (usize, usize, bool, u32) = block_mode(m).unwrap();
        let wb: u32 = ise_bits((xw * yw) as u32, wlv);
        let clv: u32 = COLOR_LEVELS.iter().copied().find(|l| ise_bits(2, *l) <= 128 - wb - 17).unwrap();
        let q: &Quant = tb.of(clv).unwrap();
        let ends: Vec<Pair> = [q.vals[q.pick(30)].1, q.vals[q.pick(220)].1].to_vec();
        let (b, k): (u32, Kind) = level(wlv);
        let top: u32 = digits(k) * (1 << b);
        let grid: Vec<Pair> = (0..xw * yw)
            .map(|i| {
                let want: u32 = (i % xw) as u32 * 64 / (xw as u32 - 1);
                (0..top).map(|j| (j / (1 << b), j % (1 << b))).min_by_key(|p| unq_weight(p.0, p.1, wlv).abs_diff(want)).unwrap()
            })
            .collect();
        let mut x: u128 = (m as u128) | (pack(&ends, clv).unwrap() << 17);
        x = (x & ((1u128 << (128 - wb)) - 1)) | pack(&grid, wlv).unwrap().reverse_bits();
        let mut blk: [u8; 16] = x.to_le_bytes();
        let mut old: [[u8; 4]; 64] = [[0; 4]; 64];
        assert!(decode_block(&blk, bw, bh, false, &mut old));
        assert!(old.iter().all(|p| p[0] == p[1] && p[1] == p[2]));
        let green = |c: [u8; 3]| [c[0] / 4, c[1], c[2] / 4];
        assert_eq!(recolor_block(&mut { blk }, &tb, &green), Done::Changed, "without the size the block keeps its luminance mode");
        assert_eq!(recolor_sized(&mut blk, bw, bh, &tb, &green), Done::Changed);
        assert_eq!(rd(u128::from_le_bytes(blk), 13, 4), 8);
        let mut new: [[u8; 4]; 64] = [[0; 4]; 64];
        assert!(decode_block(&blk, bw, bh, false, &mut new));
        for (p, n) in old.iter().zip(new.iter()) {
            let w: [u8; 3] = green([p[0], p[1], p[2]]);
            for c in 0..3 {
                assert!((w[c] as i32 - n[c] as i32).abs() <= 14, "{p:?} -> {n:?}, want {w:?}");
            }
        }
    }

    #[test]
    fn a_blue_contracted_block_is_rebuilt_with_mirrored_weights() {
        let tb: Tables = Tables::new();
        let mut b: [u8; 16] = rgb([250, 10, 120, 20, 60, 30], &[(0, 3), (5, 1), (10, 2)]);
        let old: Vec<[u8; 4]> = px(&b);
        let red = |c: [u8; 3]| [c[0].saturating_add(c[1] / 2), c[1] / 4, c[2] / 4];
        assert_eq!(recolor_block(&mut b, &tb, &red), Done::Changed);
        let want: Vec<[u8; 4]> = old.iter().map(|p| { let n: [u8; 3] = red([p[0], p[1], p[2]]); [n[0], n[1], n[2], p[3]] }).collect();
        assert!(close(&px(&b), &want, 3), "{:?} {:?}", px(&b), want);
    }

    #[test]
    fn a_void_block_takes_the_new_colour_and_keeps_its_alpha() {
        let tb: Tables = Tables::new();
        let mut x: u128 = 0;
        set(&mut x, 0, 9, 0x1fc);
        set(&mut x, 12, 32, 0xffff_ffff);
        set(&mut x, 44, 20, 0xfffff);
        for (i, v) in [0x1234u32, 0x8080, 0xfe00, 0x7fff].iter().enumerate() {
            set(&mut x, 64 + 16 * i as u32, 16, *v);
        }
        let mut b: [u8; 16] = x.to_le_bytes();
        assert_eq!(recolor_block(&mut b, &tb, &swap), Done::Changed);
        assert!(px(&b).iter().all(|p| *p == [0x80, 0xfe, 0x12, 0x7f]));
        let mut z: [u8; 16] = [0; 16];
        assert_eq!(recolor_block(&mut z, &tb, &swap), Done::Skipped);
    }

    #[test]
    fn packing_undoes_the_integer_sequence_reader_at_every_level() {
        for lv in COLOR_LEVELS {
            let (b, k): (u32, Kind) = level(lv);
            for n in (1..=18usize).filter(|n| ise_bits(*n as u32, lv) < 128) {
                let ps: Vec<Pair> = (0..n).map(|i| ((i as u32 * 7 + lv) % digits(k), (i as u32 * 13 + 5) % (1 << b))).collect();
                let bits: u128 = pack(&ps, lv).unwrap();
                assert!(bits >> ise_bits(n as u32, lv) == 0, "{lv} {n}");
                let mut back: [Pair; 64] = [(0, 0); 64];
                ise(bits, n, lv, &mut back);
                assert_eq!(&back[..n], &ps[..], "{lv} {n}");
            }
        }
    }

    #[test]
    fn every_weight_has_a_mirror_at_its_own_level() {
        for lv in super::super::WEIGHT_LEVELS {
            let (b, k): (u32, Kind) = level(lv);
            for d in 0..digits(k) {
                for m in 0..(1u32 << b) {
                    let p: Pair = mirror((d, m), lv).unwrap();
                    assert_eq!(unq_weight(p.0, p.1, lv), 64 - unq_weight(d, m, lv));
                }
            }
        }
    }

    #[test]
    fn offset_and_scale_modes_reach_a_rotated_target() {
        let tb: Tables = Tables::new();
        let q: &Quant = tb.of(256).unwrap();
        for cem in [6u32, 9, 13, 10, 12, 0, 1, 4, 5] {
            let n: usize = 2 * ((cem >> 2) as usize + 1);
            let seedv: [i32; 8] = [120, 140, 90, 200, 60, 70, 255, 255];
            let cur: Vec<usize> = (0..n).map(|i| q.pick(seedv[i])).collect();
            let (a, b): Ends = endpoints(cem, &seedv[..n]).unwrap();
            let w: Ends = ([a[1], a[2], a[0], a[3]], [b[1], b[2], b[0], b[3]]);
            let direct: u32 = fit(cem, q, &cur, &w, true).0;
            let turned: u32 = fit(cem, q, &cur, &(w.1, w.0), false).0;
            assert!(direct.min(turned) <= 12, "{cem} {direct} {turned}");
        }
    }
}
