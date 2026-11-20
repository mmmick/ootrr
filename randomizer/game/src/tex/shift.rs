use crate::tex::image::Img;

const ROOT3: f32 = 1.732_050_8;
const LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];

fn curve(v: f32) -> f32 {
    if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
}

fn uncurve(v: f32) -> f32 {
    if v <= 0.040_45 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

pub fn lin(c: [u8; 3]) -> [f32; 3] {
    c.map(|v| uncurve(v as f32 / 255.0))
}

pub fn srgb(l: [f32; 3]) -> [u8; 3] {
    l.map(|v| (curve(v.clamp(0.0, 1.0)) * 255.0).round() as u8)
}

pub fn luma(l: [f32; 3]) -> f32 {
    LUMA[0] * l[0] + LUMA[1] * l[1] + LUMA[2] * l[2]
}

pub fn hue(c: [f32; 3]) -> (f32, f32) {
    let a: f32 = c[0] - (c[1] + c[2]) / 2.0;
    let b: f32 = ROOT3 / 2.0 * (c[1] - c[2]);
    (b.atan2(a).to_degrees().rem_euclid(360.0), a.hypot(b))
}

/// Linear RGB to OKLab: lightness, then the green-red and blue-yellow axes.
pub fn oklab(c: [f32; 3]) -> [f32; 3] {
    let l: f32 = (0.412_221_47 * c[0] + 0.536_332_55 * c[1] + 0.051_445_995 * c[2]).max(0.0).cbrt();
    let m: f32 = (0.211_903_5 * c[0] + 0.680_699_5 * c[1] + 0.107_396_96 * c[2]).max(0.0).cbrt();
    let s: f32 = (0.088_302_46 * c[0] + 0.281_718_85 * c[1] + 0.629_978_7 * c[2]).max(0.0).cbrt();
    [
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    ]
}

/// OKLab back to linear RGB (not clamped).
pub fn from_oklab(c: [f32; 3]) -> [f32; 3] {
    let l: f32 = (c[0] + 0.396_337_78 * c[1] + 0.215_803_76 * c[2]).powi(3);
    let m: f32 = (c[0] - 0.105_561_346 * c[1] - 0.063_854_17 * c[2]).powi(3);
    let s: f32 = (c[0] - 0.089_484_18 * c[1] - 1.291_485_5 * c[2]).powi(3);
    [
        4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s,
        -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s,
        -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s,
    ]
}

/// Below this mean OKLab chroma a texture counts as grey: it has no hue of its own to turn,
/// so it is painted the target colour outright. Above `REL_FULL` its own hues are kept.
const REL_FLOOR: f32 = 0.02;
const REL_FULL: f32 = 0.05;
const CHROMA_CAP: f32 = 0.37;
const CHROMA_BINS: usize = 370;
const TOP_SHARE: f64 = 0.97;
/// Pixels at least this share of the part's chroma form its colourful core.
pub const CORE: f32 = 0.8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tint {
    pub to: [u8; 3],
    pub mid: f32,
    pub span: f32,
    pub floor: f32,
    pub full: f32,
    pub from: f32,
    /// The OKLab chroma of the most colourful pixels the tint takes, and their mean hue (radians).
    pub chroma: f32,
    pub angle: f32,
}

impl Tint {
    pub fn new(to: [u8; 3]) -> Tint {
        Tint { to, mid: 0.0, span: 360.0, floor: 10.0, full: 40.0, from: 0.0, chroma: 0.0, angle: 0.0 }
    }

    pub fn band(mut self, mid: f32, span: f32) -> Tint {
        self.mid = mid;
        self.span = span;
        self
    }

    pub fn chroma(mut self, floor: f32, full: f32) -> Tint {
        self.floor = floor;
        self.full = full;
        self
    }

    pub fn weight(&self, c: [u8; 3]) -> f32 {
        let (h, chroma): (f32, f32) = hue([c[0] as f32, c[1] as f32, c[2] as f32]);
        let by_chroma: f32 = ((chroma - self.floor) / (self.full - self.floor)).clamp(0.0, 1.0);
        if self.span >= 180.0 {
            return by_chroma;
        }
        let far: f32 = ((h - self.mid + 540.0).rem_euclid(360.0) - 180.0).abs();
        let by_hue: f32 = ((self.span * 1.5 - far) / (self.span * 0.5)).clamp(0.0, 1.0);
        by_hue * by_chroma
    }

    pub fn fit(mut self, img: &Img) -> Option<Tint> {
        let (mut sum, mut wsum): (f64, f64) = (0.0, 0.0);
        let (mut a, mut b): (f64, f64) = (0.0, 0.0);
        let mut bins: [f64; CHROMA_BINS] = [0.0; CHROMA_BINS];
        let mut sums: [f64; CHROMA_BINS] = [0.0; CHROMA_BINS];
        for p in img.px.chunks_exact(4) {
            let c: [u8; 3] = [p[0], p[1], p[2]];
            let w: f64 = self.weight(c) as f64 * p[3].max(1) as f64;
            if w <= 0.0 {
                continue;
            }
            let l: [f32; 3] = lin(c);
            sum += w * luma(l) as f64;
            wsum += w;
            let lab: [f32; 3] = oklab(l);
            a += w * lab[1] as f64;
            b += w * lab[2] as f64;
            let c: f32 = lab[1].hypot(lab[2]);
            let k: usize = ((c / CHROMA_CAP * CHROMA_BINS as f32) as usize).min(CHROMA_BINS - 1);
            bins[k] += w;
            sums[k] += w * c as f64;
        }
        if wsum <= 0.0 || sum <= 0.0 {
            return None;
        }
        self.from = (sum / wsum) as f32;
        // The part's chroma is that of its most colourful pixels (the 97th percentile), so a
        // part that is mostly near-grey stone or weave around some cloth keeps those greys
        // muted after the turn while the cloth takes the target's full colour.
        let mut seen: f64 = 0.0;
        let mut at: usize = CHROMA_BINS - 1;
        for (i, w) in bins.iter().enumerate() {
            seen += w;
            if seen >= wsum * TOP_SHARE {
                at = i;
                break;
            }
        }
        self.chroma = if bins[at] > 0.0 { (sums[at] / bins[at]) as f32 } else { (at as f32 + 0.5) * CHROMA_CAP / CHROMA_BINS as f32 };
        self.angle = b.atan2(a) as f32;
        // Lightness is matched on the same colourful core, so the cloth takes the target's
        // lightness; a grey part (no core to speak of) keeps the mean of all its pixels.
        let keep: f32 = self.keep();
        if keep > 0.0 {
            let (mut cl, mut cw): (f64, f64) = (0.0, 0.0);
            for p in img.px.chunks_exact(4) {
                let c: [u8; 3] = [p[0], p[1], p[2]];
                let w: f64 = self.weight(c) as f64 * p[3].max(1) as f64;
                let l: [f32; 3] = lin(c);
                let lab: [f32; 3] = oklab(l);
                if w > 0.0 && lab[1].hypot(lab[2]) >= self.chroma * CORE {
                    cl += w * luma(l) as f64;
                    cw += w;
                }
            }
            if cw > 0.0 && cl > 0.0 {
                self.from += keep * ((cl / cw) as f32 - self.from);
            }
        }
        Some(self)
    }

    /// How much of the part's own hues and chroma a tint keeps: none for a grey part, all
    /// for a colourful one.
    pub fn keep(&self) -> f32 {
        ((self.chroma - REL_FLOOR) / (REL_FULL - REL_FLOOR)).clamp(0.0, 1.0)
    }

    pub fn rgb(&self, c: [u8; 3]) -> [u8; 3] {
        let w: f32 = self.weight(c);
        if w <= 0.0 || self.from <= 0.0 {
            return c;
        }
        let l: [f32; 3] = lin(c);
        let k: f32 = luma(l) / self.from;
        let t: [f32; 3] = lin(self.to);
        let flat: [f32; 3] = t.map(|v| v * k);
        let keep: f32 = self.keep();
        let to: [f32; 3] = if keep > 0.0 { self.turned(l, flat, keep) } else { flat };
        srgb([0, 1, 2].map(|i| l[i] + w * (to[i] - l[i])))
    }

    /// The pixel moved as the whole part moves, keeping its own place in the part: its hue
    /// turns by the angle from the part's mean hue to the target's, its chroma scales by the
    /// target's over the part's mean, and its lightness follows the flat tint's as far as the
    /// pixel is colourful. Two shades that differed in the game still differ after.
    fn turned(&self, l: [f32; 3], flat: [f32; 3], keep: f32) -> [f32; 3] {
        let px: [f32; 3] = oklab(l);
        let goal: [f32; 3] = oklab(lin(self.to));
        let (c, h): (f32, f32) = (px[1].hypot(px[2]), px[2].atan2(px[1]));
        // A pixel takes the new lightness as far as it is as colourful as the part: cloth
        // moves fully, near-neutral stone, weave and stitching keep their own lightness.
        let own: f32 = (c / self.chroma).clamp(0.0, 1.0);
        let light: f32 = px[0] + own * (oklab(flat)[0] - px[0]);
        let (ct, ht): (f32, f32) = (goal[1].hypot(goal[2]), goal[2].atan2(goal[1]));
        let c2: f32 = (c * ct / self.chroma).min(CHROMA_CAP);
        let h2: f32 = h + (ht - self.angle);
        let rel: [f32; 3] = from_oklab([light, c2 * h2.cos(), c2 * h2.sin()]).map(|v| v.max(0.0));
        [0, 1, 2].map(|i| flat[i] + keep * (rel[i] - flat[i]))
    }

    pub fn param(&self, v: [f32; 3], from: f32) -> [f32; 3] {
        if from <= 0.0 {
            return v;
        }
        let k: f32 = luma(v.map(|x| x.max(0.0))) / from;
        lin(self.to).map(|t| t * k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(c: [u8; 3], n: usize) -> Img {
        let mut a: Img = Img::new(n, 1);
        for x in 0..n {
            a.put(x, 0, [c[0], c[1], c[2], 255]);
        }
        a
    }

    #[test]
    fn a_fitted_tint_moves_the_mean_colour_onto_the_target() {
        for to in [[255, 0, 0], [12, 200, 90], [250, 250, 250], [5, 5, 5], [128, 64, 200]] {
            let t: Tint = Tint::new(to).chroma(3.0, 12.0).fit(&flat([64, 82, 42], 4)).unwrap();
            let got: [u8; 3] = t.rgb([64, 82, 42]);
            for i in 0..3 {
                assert!((got[i] as i32 - to[i] as i32).abs() <= 2, "{to:?} {got:?}");
            }
        }
    }

    #[test]
    fn shading_is_kept_as_brightness() {
        let mut img: Img = flat([60, 90, 40], 2);
        img.put(1, 0, [30, 45, 20, 255]);
        let t: Tint = Tint::new([200, 40, 40]).chroma(3.0, 12.0).fit(&img).unwrap();
        let (a, b): ([u8; 3], [u8; 3]) = (t.rgb([60, 90, 40]), t.rgb([30, 45, 20]));
        assert!(luma(lin(a)) > luma(lin(b)) * 3.0, "{a:?} {b:?}");
        assert!(a[0] > a[1] && b[0] > b[1]);
    }

    #[test]
    fn greys_and_colours_outside_the_band_are_left() {
        let t: Tint = Tint::new([255, 0, 255]).band(90.0, 40.0).fit(&flat([64, 82, 42], 2)).unwrap();
        assert_eq!(t.rgb([120, 120, 120]), [120, 120, 120]);
        assert_eq!(t.rgb([170, 60, 40]), [170, 60, 40]);
        assert_ne!(t.rgb([64, 82, 42]), [64, 82, 42]);
    }

    #[test]
    fn a_low_chroma_floor_reaches_dull_cloth() {
        let dull: [u8; 3] = [53, 63, 46];
        let hard: Tint = Tint::new([255, 0, 0]).band(100.0, 45.0).fit(&flat([90, 140, 40], 1)).unwrap();
        let soft: Tint = Tint::new([255, 0, 0]).band(100.0, 45.0).chroma(3.0, 12.0).fit(&flat(dull, 1)).unwrap();
        assert!(hard.weight(dull) < 0.3);
        assert_eq!(soft.weight(dull), 1.0);
        let got: [u8; 3] = soft.rgb(dull);
        assert!(got[0] > 200 && got[1] < 10 && got[2] < 10, "{got:?}");
    }

    #[test]
    fn oklab_round_trips() {
        for c in [[0u8, 0, 0], [255, 255, 255], [200, 30, 40], [12, 200, 90], [70, 60, 220]] {
            let back: [u8; 3] = srgb(from_oklab(oklab(lin(c))));
            for i in 0..3 {
                assert!((back[i] as i32 - c[i] as i32).abs() <= 1, "{c:?} {back:?}");
            }
        }
    }

    #[test]
    fn shades_that_differed_keep_their_difference_around_the_new_colour() {
        let (red, orange): ([u8; 3], [u8; 3]) = ([170, 30, 40], [190, 80, 30]);
        let mut img: Img = flat(red, 2);
        img.put(1, 0, [orange[0], orange[1], orange[2], 255]);
        let t: Tint = Tint::new([40, 70, 220]).fit(&img).unwrap();
        let (a, b): ([u8; 3], [u8; 3]) = (t.rgb(red), t.rgb(orange));
        let hue_of = |c: [u8; 3]| {
            let p: [f32; 3] = oklab(lin(c));
            p[2].atan2(p[1]).to_degrees()
        };
        let was: f32 = hue_of(orange) - hue_of(red);
        let now: f32 = hue_of(b) - hue_of(a);
        assert!((now - was).abs() < 6.0, "hue gap was {was}, now {now}: {a:?} {b:?}");
        assert!(a[2] > a[0] && b[2] > b[0] || b[1] > b[0], "both blue-ish: {a:?} {b:?}");
        assert_ne!(a, b);
    }

    #[test]
    fn a_grey_part_takes_the_target_outright() {
        let grey: Img = flat([128, 126, 130], 2);
        let t: Tint = Tint::new([30, 200, 60]).chroma(-1.0, 0.0).fit(&grey).unwrap();
        assert!(t.chroma < REL_FLOOR);
        let got: [u8; 3] = t.rgb([128, 126, 130]);
        assert!(got[1] > 150 && got[0] < 90, "{got:?}");
    }

    #[test]
    fn nothing_to_fit_gives_no_tint() {
        assert!(Tint::new([1, 2, 3]).fit(&flat([128, 128, 128], 3)).is_none());
    }

    #[test]
    fn a_parameter_takes_the_target_at_its_own_brightness() {
        let t: Tint = Tint::new([255, 128, 0]);
        let first: [f32; 3] = [0.46, 0.31, 0.86];
        let got: [f32; 3] = t.param(first, luma(first));
        let want: [f32; 3] = lin([255, 128, 0]);
        for i in 0..3 {
            assert!((got[i] - want[i]).abs() < 1e-4);
        }
        let half: [f32; 3] = t.param([0.23, 0.155, 0.43], luma(first));
        assert!((half[0] - want[0] / 2.0).abs() < 1e-3);
    }
}
