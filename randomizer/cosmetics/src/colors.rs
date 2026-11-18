use std::collections::BTreeMap as BM;
use std::fs;
use std::path::{Path as P, PathBuf as PB};

use crate::bfres::{self, Mat};
use crate::error::{Error as E, Res};
use crate::rng::Rng;
use crate::romfs::Romfs;
use crate::tex::bntx::{self, Tex};
use crate::tex::image::Img;
use crate::tex::png;
use crate::tex::repaint::{self, Count};
use crate::tex::shift::{Tint, luma};
use crate::tex::surface;
use crate::zs;

pub const NAMES: &str = "Shader/ExternalBinaryString.bfres.zs";
const SIDE: usize = 384;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Colors {
    Off,
    Random,
    Shared,
}

impl Colors {
    pub fn parse(s: &str) -> Option<Colors> {
        match s {
            "off" => Some(Colors::Off),
            "random" => Some(Colors::Random),
            "shared" => Some(Colors::Shared),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Colors::Off => "off",
            Colors::Random => "random",
            Colors::Shared => "shared",
        }
    }
}

pub struct Item {
    pub id: &'static str,
    pub label: &'static str,
    pub tex: &'static [&'static str],
    pub band: Option<(f32, f32)>,
    pub cloth: bool,
    pub vals: &'static [(&'static str, &'static str, &'static str)],
}

const C0: &str = "ns_param_color0";
const C1: &str = "ns_param_color1";
const C2: &str = "ns_param_color2";
const C3: &str = "ns_param_color3";
const SOFT: (f32, f32) = (3.0, 12.0);

pub const ITEMS: [Item; 14] = [
    Item {
        id: "kokiri",
        label: "Kokiri tunic",
        band: Some((100.0, 45.0)),
        cloth: true,
        tex: &[
            "Angin_Tile_A_Alb",
            "Angin_Tile_A_Dark_Alb",
            "Mt_Vest_Player_Adult_Alb",
            "Mt_Vest_Player_Child_Alb",
            "Mt_Angin_Acs_Player_Adult_Alb",
            "Mt_Cloth_Parts_Player_Child_Alb",
            "Mt_PantsParts_Player_Child_Alb",
        ],
        vals: &[],
    },
    Item {
        id: "goron",
        label: "Goron tunic",
        band: Some((354.0, 20.0)),
        cloth: true,
        tex: &["Mt_Angin_Armor_Adult_Goron_Alb", "Mt_Inner_Armor_Adult_Goron_Alb", "Mt_Vest_Armor_Adult_Goron_Alb"],
        vals: &[],
    },
    Item {
        id: "zora",
        label: "Zora tunic",
        band: Some((222.0, 25.0)),
        cloth: true,
        tex: &["Mt_Angin_Armor_Adult_Zora_Alb", "Mt_Angin_Acs_Armor_Adult_Zora_Alb", "Mt_Vest_Armor_Adult_Zora_Alb"],
        vals: &[("Armor_Adult_Zora.Armor_Adult_Zora", "Mt_Angin", C0)],
    },
    Item { id: "kokiri_sword", label: "Kokiri Sword", band: Some((350.0, 18.0)), cloth: false, tex: &["Mt_Weapon_Sword_Kokiri_Alb"], vals: &[] },
    Item {
        id: "master_sword",
        label: "Master Sword",
        band: None,
        cloth: false,
        tex: &["Mt_Weapon_Sword_Master_Alb"],
        vals: &[
            ("Weapon_Sword_Master.Weapon_Sword_Master", "Mt_Weapon_Sword_Master", C0),
            ("Weapon_Sword_Master.Weapon_Sword_Master", "Mt_Weapon_Sword_Master", C1),
            ("Weapon_Sword_Master.Land_HrlCity_MasterSword", "Mt_Weapon_Sword_Master", C0),
            ("Weapon_Sword_Master.Land_HrlCity_MasterSword", "Mt_Weapon_Sword_Master", C1),
        ],
    },
    Item { id: "biggoron_sword", label: "Biggoron Sword", band: None, cloth: false, tex: &["Mt_Weapon_Sword_Biggoron_Alb.00", "Mt_Weapon_Sword_Biggoron_Alb.01"], vals: &[] },
    Item { id: "deku_shield", label: "Deku Shield", band: Some((2.0, 14.0)), cloth: false, tex: &["Mt_Body_Weapon_Shield_Deku_Alb"], vals: &[] },
    Item { id: "hylian_shield", label: "Hylian Shield", band: Some((231.0, 35.0)), cloth: false, tex: &["Mt_Weapon_Shield_Hylian_Alb"], vals: &[] },
    Item { id: "mirror_shield", label: "Mirror Shield", band: Some((2.0, 14.0)), cloth: false, tex: &["Mt_Weapon_Shield_Mirror_Alb"], vals: &[] },
    Item {
        id: "navi",
        label: "Navi",
        band: None,
        cloth: false,
        tex: &["Npc_Navi_Body_Alb.00", "Npc_Navi_Body_Alb.01", "Npc_Navi_Wings_Alb.00", "Npc_Navi_Wings_Emc.00", "Npc_Navi_Wings_Emc.01"],
        vals: &[("Npc_Navi.Npc_Navi", "Mt_Body", C1), ("Npc_Navi.Npc_Navi", "Mt_Body_AlphaMask", C1)],
    },
    Item {
        id: "boomerang",
        label: "Boomerang gem",
        band: None,
        cloth: false,
        tex: &[],
        vals: &[("Item_Boomerang.Item_Boomerang", "Mt_Item_Boomerang_Gem", C0), ("Item_Boomerang.Item_Boomerang", "Mt_Item_Boomerang_Gem", C1), ("Item_Boomerang.Item_Boomerang", "Mt_Item_Boomerang_Gem", C2)],
    },
    Item {
        id: "gauntlets",
        label: "Gauntlet jewels",
        band: None,
        cloth: false,
        tex: &[],
        vals: &[
            ("Armor_Adult_Gauntlet_Silver.Armor_Adult_Gauntlet_Silver", "Mt_Jewelry", C0),
            ("Armor_Adult_Gauntlet_Silver.Armor_Adult_Gauntlet_Silver", "Mt_Jewelry", C1),
            ("Armor_Adult_Gauntlet_Silver.Armor_Adult_Gauntlet_Silver", "Mt_Jewelry", C2),
            ("Armor_Adult_Gauntlet_Gold.Armor_Adult_Gauntlet_Gold", "Mt_Jewelry", C0),
            ("Armor_Adult_Gauntlet_Gold.Armor_Adult_Gauntlet_Gold", "Mt_Jewelry", C1),
            ("Armor_Adult_Gauntlet_Gold.Armor_Adult_Gauntlet_Gold", "Mt_Jewelry", C2),
        ],
    },
    Item {
        id: "hair",
        label: "Hair",
        band: None,
        cloth: false,
        tex: &[],
        vals: &[
            ("Player_Adult.Player_Adult", "Mt_Hairs", C0),
            ("Player_Adult.Player_Adult", "Mt_Hairs", C1),
            ("Player_Adult.Player_Adult", "Mt_Hairs", C3),
            ("Player_Child.Player_Child", "Mt_Hairs", C0),
            ("Player_Child.Player_Child", "Mt_Hairs", C1),
            ("Player_Child.Player_Child", "Mt_Hairs", C3),
        ],
    },
    Item { id: "hearts", label: "Recovery hearts", band: None, cloth: false, tex: &["Mt_Obj_Heart_Alb", "Mt_Heart_Container_Alb"], vals: &[] },
];

impl Item {
    pub fn tint(&self, to: [u8; 3]) -> Tint {
        let t: Tint = self.band.map(|b| Tint::new(to).band(b.0, b.1)).unwrap_or(Tint::new(to));
        if self.cloth { t.chroma(SOFT.0, SOFT.1) } else { t }
    }
}

pub fn roll(rng: &mut Rng) -> [u8; 3] {
    [rng.below(256) as u8, rng.below(256) as u8, rng.below(256) as u8]
}

pub fn plan(c: Colors, seed: &str) -> Vec<(&'static Item, [u8; 3])> {
    if c == Colors::Off {
        return Vec::new();
    }
    let mut rng: Rng = Rng::from_text(&format!("{seed} colors {}", c.id()));
    let one: [u8; 3] = roll(&mut rng);
    ITEMS.iter().map(|it| (it, if c == Colors::Shared { one } else { roll(&mut rng) })).collect()
}

fn fitted(raw: &[u8], t: Tint) -> Res<Option<Tint>> {
    let mut all: Img = Img::new(0, 0);
    for x in bntx::parse(raw)? {
        let img: Img = surface::decode(raw, &x)?;
        all.px.extend_from_slice(&img.px);
    }
    Ok(t.fit(&all))
}

/// Every game file the colour shuffle may read. A missing one is skipped, not reported,
/// so a partial copy of the game (an unpacked .nup or .nx) must hold all of them.
pub fn files() -> Vec<String> {
    let mut out: Vec<String> = vec![NAMES.to_string()];
    for it in &ITEMS {
        out.extend(it.tex.iter().map(|t| tex_path(t)));
        out.extend(it.vals.iter().map(|v| model_path(v.0)));
    }
    out
}

fn tex_path(name: &str) -> String {
    format!("Tex/{name}.bntx.zs")
}

fn model_path(name: &str) -> String {
    format!("Model/{name}.bfres.zs")
}

fn put(root: &P, r: &str, raw: &[u8], z: &[u8]) -> Res<()> {
    let out: PB = root.join("romfs").join(r);
    if let Some(d) = out.parent() {
        fs::create_dir_all(d)?;
    }
    fs::write(&out, zs::compress_raw(raw, zs::dict_of(z))?)?;
    Ok(())
}

pub fn tint_model(b: &mut [u8], keys: &BM<String, u64>, edits: &[(&str, &str)], t: &Tint) -> Res<usize> {
    let ms: Vec<Mat> = bfres::mats(b)?;
    let at: Vec<usize> = edits.iter().filter_map(|(mat, param)| keys.get(*param).and_then(|k| bfres::find(&ms, mat, *k)).map(|p| p.at)).collect();
    let Some(first) = at.first() else {
        return Ok(0);
    };
    let v: [f32; 4] = bfres::read4(b, *first)?;
    let from: f32 = luma([v[0].max(0.0), v[1].max(0.0), v[2].max(0.0)]);
    for p in &at {
        let v: [f32; 4] = bfres::read4(b, *p)?;
        let l: [f32; 3] = t.param([v[0], v[1], v[2]], from);
        bfres::write4(b, *p, [l[0], l[1], l[2], v[3]])?;
    }
    Ok(at.len())
}

type Edits<'a> = (Tint, Vec<(&'a str, &'a str)>);

pub fn hex(c: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

pub fn apply(mod_root: &P, romfs: &P, c: Colors, seed: &str) -> Res<Vec<String>> {
    let mut notes: Vec<String> = Vec::new();
    let todo: Vec<(&Item, [u8; 3])> = plan(c, seed);
    if todo.is_empty() {
        return Ok(notes);
    }
    let mut rf: Romfs = Romfs::open(romfs)?;
    let keys: BM<String, u64> = bfres::keys(&rf.read_zs(NAMES)?)?;
    let mut by_model: BM<&str, Vec<Edits>> = BM::new();
    let mut done: Vec<String> = Vec::new();
    let (mut files, mut bytes): (usize, u64) = (0, 0);
    for (it, to) in &todo {
        let mut hit: bool = false;
        for t in it.tex {
            let r: String = tex_path(t);
            let Ok(z) = rf.read(&r) else {
                continue;
            };
            let mut raw: Vec<u8> = rf.zs.decompress(&z)?;
            let Some(s) = fitted(&raw, it.tint(*to))? else {
                continue;
            };
            let n: Count = repaint::repaint(&mut raw, &|c| s.rgb(c))?;
            if n.changed == 0 {
                continue;
            }
            put(mod_root, &r, &raw, &z)?;
            files += 1;
            bytes += raw.len() as u64;
            hit = true;
        }
        let mut groups: BM<&str, Vec<(&str, &str)>> = BM::new();
        for (m, mat, param) in it.vals {
            groups.entry(m).or_default().push((mat, param));
            hit = true;
        }
        for (m, g) in groups {
            by_model.entry(m).or_default().push((it.tint(*to), g));
        }
        if hit {
            done.push(format!("{} {}", it.label, hex(*to)));
        }
    }
    for (m, edits) in &by_model {
        let r: String = model_path(m);
        let Ok(z) = rf.read(&r) else {
            continue;
        };
        let mut raw: Vec<u8> = rf.zs.decompress(&z)?;
        let mut n: usize = 0;
        for (t, g) in edits {
            n += tint_model(&mut raw, &keys, g, t)?;
        }
        if n == 0 {
            continue;
        }
        put(mod_root, &r, &raw, &z)?;
        files += 1;
        bytes += raw.len() as u64;
    }
    if files == 0 {
        return Err(E::Format("no colour could be changed"));
    }
    notes.push(format!(
        "Colours are randomized {}. {} files rewritten, {} MB added to the mod. Changed: {}. Particle glows, trails and the menu keep their own colours.",
        if c == Colors::Shared { "with one shared colour" } else { "one by one" },
        files,
        bytes / 1_000_000,
        done.join(", ")
    ));
    Ok(notes)
}

fn shot(b: &[u8]) -> Res<Img> {
    let t: Tex = bntx::parse(b)?.into_iter().next().ok_or(E::Format("the texture file is empty"))?;
    Ok(surface::decode(b, &t)?.shrink(SIDE))
}

pub fn preview(romfs: &P, out: &P, c: Colors, seed: &str) -> Res<Vec<String>> {
    let mut lines: Vec<String> = Vec::new();
    let mut rf: Romfs = Romfs::open(romfs)?;
    fs::create_dir_all(out)?;
    for (it, to) in plan(c, seed) {
        for t in it.tex {
            let Ok(old) = rf.read_zs(&tex_path(t)) else {
                lines.push(format!("{t}: missing"));
                continue;
            };
            let Some(s) = fitted(&old, it.tint(to))? else {
                lines.push(format!("{t}: no pixel in the colour band"));
                continue;
            };
            let mut new: Vec<u8> = old.clone();
            let n: Count = repaint::repaint(&mut new, &|c| s.rgb(c))?;
            let (a, b): (Img, Img) = (shot(&old)?, shot(&new)?);
            let mut want: Img = a.clone();
            let (mut sum, mut worst): (u64, u32) = (0, 0);
            for (p, q) in want.px.chunks_exact_mut(4).zip(b.px.chunks_exact(4)) {
                let w: [u8; 3] = s.rgb([p[0], p[1], p[2]]);
                p[..3].copy_from_slice(&w);
                for i in 0..3 {
                    let d: u32 = (w[i] as i32 - q[i] as i32).unsigned_abs();
                    sum += d as u64;
                    worst = worst.max(d);
                }
            }
            fs::write(out.join(format!("{}_{t}_before.png", it.id)), png::encode(&a))?;
            fs::write(out.join(format!("{}_{t}_after.png", it.id)), png::encode(&b))?;
            lines.push(format!("{t}: target {}, {} of {} blocks changed, {} skipped, mean error {:.2}, worst {}", hex(to), n.changed, n.blocks, n.skipped, sum as f64 / (a.px.len() as f64 * 0.75), worst));
        }
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfres::fixture;

    #[test]
    fn mode_names_round_trip() {
        for m in [Colors::Off, Colors::Random, Colors::Shared] {
            assert_eq!(Colors::parse(m.id()), Some(m));
        }
        assert_eq!(Colors::parse("loud"), None);
    }

    #[test]
    fn the_plan_is_fixed_by_the_seed_and_off_plans_nothing() {
        let a: Vec<[u8; 3]> = plan(Colors::Random, "s").into_iter().map(|x| x.1).collect();
        let b: Vec<[u8; 3]> = plan(Colors::Random, "s").into_iter().map(|x| x.1).collect();
        let c: Vec<[u8; 3]> = plan(Colors::Random, "t").into_iter().map(|x| x.1).collect();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), ITEMS.len());
        assert!(a.windows(2).any(|w| w[0] != w[1]));
        assert!(plan(Colors::Off, "s").is_empty());
    }

    #[test]
    fn shared_mode_gives_every_item_the_same_colour() {
        let a: Vec<[u8; 3]> = plan(Colors::Shared, "s").into_iter().map(|x| x.1).collect();
        assert!(a.windows(2).all(|w| w[0] == w[1]));
        assert_ne!(plan(Colors::Shared, "s")[0].1, plan(Colors::Shared, "t")[0].1);
    }

    #[test]
    fn colours_spread_over_the_whole_rgb_cube() {
        let mut bins: [[u32; 8]; 3] = [[0; 8]; 3];
        let mut octants: [u32; 8] = [0; 8];
        let mut seen: std::collections::BTreeSet<[u8; 3]> = std::collections::BTreeSet::new();
        let (mut lo, mut hi): ([u8; 3], [u8; 3]) = ([255; 3], [0; 3]);
        let runs: u32 = 2000;
        for i in 0..runs {
            let to: [u8; 3] = plan(Colors::Random, &format!("seed {i}"))[0].1;
            seen.insert(to);
            for c in 0..3 {
                bins[c][to[c] as usize / 32] += 1;
                lo[c] = lo[c].min(to[c]);
                hi[c] = hi[c].max(to[c]);
            }
            octants[(to[0] as usize / 128) * 4 + (to[1] as usize / 128) * 2 + to[2] as usize / 128] += 1;
        }
        let want: f64 = runs as f64 / 8.0;
        for c in 0..3 {
            let chi: f64 = bins[c].iter().map(|n| (*n as f64 - want).powi(2) / want).sum();
            assert!(chi < 24.3, "channel {c} chi square {chi} {:?}", bins[c]);
            assert!(lo[c] < 4 && hi[c] > 251, "{lo:?} {hi:?}");
        }
        assert!(octants.iter().all(|n| *n > 180), "{octants:?}");
        assert!(seen.len() > 1990);
    }

    #[test]
    fn every_tunic_covers_its_body_texture() {
        let tex = |id: &str| ITEMS.iter().find(|i| i.id == id).unwrap().tex;
        assert!(tex("kokiri").contains(&"Mt_Vest_Player_Adult_Alb") && tex("kokiri").contains(&"Mt_Vest_Player_Child_Alb"));
        assert!(tex("goron").contains(&"Mt_Vest_Armor_Adult_Goron_Alb"));
        assert!(tex("zora").contains(&"Mt_Vest_Armor_Adult_Zora_Alb"));
        assert!(ITEMS.iter().filter(|i| i.cloth).all(|i| i.band.is_some()));
    }

    #[test]
    fn item_ids_are_unique_and_every_item_changes_something() {
        let mut ids: Vec<&str> = ITEMS.iter().map(|i| i.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), ITEMS.len());
        assert!(ITEMS.iter().all(|i| !i.tex.is_empty() || !i.vals.is_empty()));
    }

    #[test]
    fn a_model_value_takes_the_target_and_others_are_left() {
        let mut b: Vec<u8> = fixture::model(&[("Mt_A", &[(7, [0.8, 0.1, 0.05, 0.5]), (9, [0.4, 0.05, 0.025, 1.0])]), ("Mt_B", &[(7, [0.5, 0.6, 0.7, 0.8])])]);
        let old: Vec<u8> = b.clone();
        let keys: BM<String, u64> = [("c0".to_string(), 7u64), ("c1".to_string(), 9u64)].into_iter().collect();
        let t: Tint = Tint::new([0, 0, 255]);
        let n: usize = tint_model(&mut b, &keys, &[("Mt_A", "c0"), ("Mt_A", "c1"), ("Mt_C", "c0"), ("Mt_A", "c9")], &t).unwrap();
        assert_eq!((n, b.len()), (2, old.len()));
        let ms: Vec<Mat> = bfres::mats(&b).unwrap();
        let v: [f32; 4] = bfres::read4(&b, bfres::find(&ms, "Mt_A", 7).unwrap().at).unwrap();
        assert!(v[0].abs() < 1e-4 && v[1].abs() < 1e-4 && (v[2] - 1.0).abs() < 1e-4 && v[3] == 0.5, "{v:?}");
        let g: [f32; 4] = bfres::read4(&b, bfres::find(&ms, "Mt_A", 9).unwrap().at).unwrap();
        assert!(g[0].abs() < 1e-4 && (g[2] - 0.5).abs() < 1e-3 && g[3] == 1.0, "{g:?}");
        assert_eq!(bfres::read4(&b, bfres::find(&ms, "Mt_B", 7).unwrap().at).unwrap(), [0.5, 0.6, 0.7, 0.8]);
    }

    #[test]
    fn the_real_files_keep_their_size() {
        let Some(root) = std::env::var("OOTRR_ROMFS").map(PB::from).ok().filter(|p| p.join("Pack").join("ZsDic.pack.zs").is_file()) else {
            return;
        };
        let out: PB = std::env::temp_dir().join(format!("ootrr_colors_{}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        let notes: Vec<String> = apply(&out, &root, Colors::Random, "colour seed").unwrap();
        assert!(notes[0].contains("Kokiri tunic") && notes[0].contains("Master Sword") && notes[0].contains("Navi"), "{notes:?}");
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut seen: usize = 0;
        for it in &ITEMS {
            for r in it.tex.iter().map(|t| tex_path(t)).chain(it.vals.iter().map(|v| model_path(v.0))) {
                let p: PB = out.join("romfs").join(&r);
                if !p.is_file() {
                    continue;
                }
                let old: Vec<u8> = rf.read_zs(&r).unwrap();
                let new: Vec<u8> = rf.zs.decompress(&fs::read(&p).unwrap()).unwrap();
                assert_eq!(old.len(), new.len(), "{r}");
                assert_ne!(old, new, "{r}");
                seen += 1;
            }
        }
        assert!(seen >= 25, "{seen}");
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn an_unchanged_repaint_keeps_every_real_texture_byte_for_byte() {
        let Some(root) = std::env::var("OOTRR_ROMFS").map(PB::from).ok().filter(|p| p.join("Pack").join("ZsDic.pack.zs").is_file()) else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        for it in &ITEMS {
            for t in it.tex {
                let old: Vec<u8> = rf.read_zs(&tex_path(t)).unwrap();
                let mut same: Vec<u8> = old.clone();
                assert_eq!(repaint::repaint(&mut same, &|c| c).unwrap().changed, 0, "{t}");
                assert_eq!(same, old, "{t}");
            }
        }
    }

    fn weighted_mean(raw: &[u8], t: &Tint, by: &[u8]) -> [u8; 3] {
        let (mut sum, mut wsum): ([f64; 3], f64) = ([0.0; 3], 0.0);
        let (a, b): (Vec<bntx::Tex>, Vec<bntx::Tex>) = (bntx::parse(by).unwrap(), bntx::parse(raw).unwrap());
        for (x, y) in a.iter().zip(b.iter()) {
            let (old, new): (Img, Img) = (surface::decode(by, x).unwrap(), surface::decode(raw, y).unwrap());
            for (p, q) in old.px.chunks_exact(4).zip(new.px.chunks_exact(4)) {
                let w: f64 = t.weight([p[0], p[1], p[2]]) as f64;
                let l: [f32; 3] = crate::tex::shift::lin([q[0], q[1], q[2]]);
                for i in 0..3 {
                    sum[i] += w * l[i] as f64;
                }
                wsum += w;
            }
        }
        crate::tex::shift::srgb(sum.map(|v| (v / wsum) as f32))
    }

    const PARTS: usize = 4;

    fn tunics_take_the_seed_colour(part: usize) {
        let Some(root) = std::env::var("OOTRR_ROMFS").map(PB::from).ok().filter(|p| p.join("Pack").join("ZsDic.pack.zs").is_file()) else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut k: usize = 0;
        for seed in ["first seed", "second seed"] {
            for (it, to) in plan(Colors::Random, seed).into_iter().filter(|x| x.0.cloth) {
                for t in it.tex {
                    k += 1;
                    if k % PARTS != part {
                        continue;
                    }
                    let old: Vec<u8> = rf.read_zs(&tex_path(t)).unwrap();
                    let s: Tint = fitted(&old, it.tint(to)).unwrap().unwrap();
                    let mut new: Vec<u8> = old.clone();
                    let n: Count = repaint::repaint(&mut new, &|c| s.rgb(c)).unwrap();
                    assert!(n.changed * 10 > n.blocks * 3, "{t} {n:?}");
                    let got: [u8; 3] = weighted_mean(&new, &s, &old);
                    let far: i32 = (0..3).map(|i| (got[i] as i32 - to[i] as i32).abs()).max().unwrap();
                    println!("{seed} {} {t} target {} got {} off {far} changed {} of {}", it.id, hex(to), hex(got), n.changed, n.blocks);
                    assert!(far <= 40, "{t} target {} got {}", hex(to), hex(got));
                }
            }
        }
    }

    parts!(
        tunics_take_the_seed_colour,
        the_real_tunics_take_the_seed_colour_part_1 = 0,
        the_real_tunics_take_the_seed_colour_part_2 = 1,
        the_real_tunics_take_the_seed_colour_part_3 = 2,
        the_real_tunics_take_the_seed_colour_part_4 = 3,
    );

    #[test]
    fn two_seeds_give_every_tunic_another_colour() {
        let (a, b): (Vec<[u8; 3]>, Vec<[u8; 3]>) = (plan(Colors::Random, "first seed").iter().map(|x| x.1).collect(), plan(Colors::Random, "second seed").iter().map(|x| x.1).collect());
        assert!(a.iter().zip(&b).take(3).all(|(x, y)| x != y));
    }

    #[test]
    fn the_dictionary_id_is_read_from_the_frame_header() {
        assert_eq!(zs::dict_of(&[0x28, 0xb5, 0x2f, 0xfd, 0xa1, 0x01, 0, 0]), Some(1));
        assert_eq!(zs::dict_of(&[0x28, 0xb5, 0x2f, 0xfd, 0xa0, 0x70, 0, 0]), None);
        assert_eq!(zs::dict_of(&[0x28, 0xb5, 0x2f, 0xfd, 0x81, 0x68, 0x01, 0]), Some(1));
        assert_eq!(zs::dict_of(&[0x28, 0xb5, 0x2f, 0xfd, 0x22, 0x02, 0x01, 0]), Some(258));
    }
}
