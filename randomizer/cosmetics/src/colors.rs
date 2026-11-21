use std::collections::BTreeMap as BM;
use std::fs;
use std::path::{Path as P, PathBuf as PB};

use crate::bfres::{self, Mat};
use crate::error::{Error as E, Res};
use crate::rng::Rng;
use crate::romfs::Romfs;
use crate::rsizetable::{self, Rst};
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
    /// The game's own colours: part picks are ignored.
    Off,
    /// Only the parts picked by hand change.
    Custom,
    /// Every part draws its own random colour, except the parts given their own pick.
    Random,
    /// One random hue, each part in its own shade of it, except the parts given their own pick.
    Shared,
}

impl Colors {
    pub fn parse(s: &str) -> Option<Colors> {
        match s {
            "off" => Some(Colors::Off),
            "custom" => Some(Colors::Custom),
            "random" => Some(Colors::Random),
            "shared" => Some(Colors::Shared),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Colors::Off => "off",
            Colors::Custom => "custom",
            Colors::Random => "random",
            Colors::Shared => "shared",
        }
    }
}

/// What one part is painted with in the custom mode. `Random` draws a colour for the part
/// alone: the same one the random mode would give it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Original,
    Random,
    Rgb([u8; 3]),
}

pub type Picks = BM<String, Pick>;

impl Pick {
    pub fn parse(s: &str) -> Option<Pick> {
        match s.trim() {
            "" | "original" | "follow" => Some(Pick::Original),
            "random" => Some(Pick::Random),
            h => {
                let h: &str = h.strip_prefix('#')?;
                if h.len() != 6 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return None;
                }
                let v = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
                Some(Pick::Rgb([v(0)?, v(2)?, v(4)?]))
            }
        }
    }

    pub fn id(self) -> String {
        match self {
            Pick::Original => "original".to_string(),
            Pick::Random => "random".to_string(),
            Pick::Rgb(c) => hex(c).to_ascii_lowercase(),
        }
    }
}

/// Reads `id=value` part picks, as the command line and the build request give them.
pub fn parse_picks(pairs: &[(String, String)]) -> Res<Picks> {
    let mut out: Picks = Picks::new();
    for (k, v) in pairs {
        if !ITEMS.iter().any(|i| i.id == k) {
            return Err(E::Msg(format!("colours: there is no part named {k}")));
        }
        let p: Pick = Pick::parse(v).ok_or_else(|| E::Msg(format!("colours: {k} takes original, random or a #rrggbb colour, not {v}")))?;
        out.insert(k.clone(), p);
    }
    Ok(out)
}

/// How a part's pixels are chosen: by hue band and the default chroma ramp, by hue band
/// with a soft ramp (cloth keeps its weave), or every pixel (undershirts that are almost grey).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    Plain,
    Cloth,
    Whole,
}

pub struct Item {
    pub id: &'static str,
    pub label: &'static str,
    pub group: &'static str,
    pub tex: &'static [&'static str],
    pub band: Option<(f32, f32)>,
    pub look: Look,
    pub vals: &'static [(&'static str, &'static str, &'static str)],
}

const C0: &str = "ns_param_color0";
const C1: &str = "ns_param_color1";
const C2: &str = "ns_param_color2";
const C3: &str = "ns_param_color3";
const SOFT: (f32, f32) = (3.0, 12.0);
const ALL: (f32, f32) = (-1.0, 0.0);

pub const KOKIRI: &str = "Kokiri tunic";
pub const GORON: &str = "Goron tunic";
pub const ZORA: &str = "Zora tunic";
pub const GEAR: &str = "Equipment";
pub const OTHER: &str = "Other";
const GREEN: Option<(f32, f32)> = Some((100.0, 45.0));
const LEATHER: Option<(f32, f32)> = Some((25.0, 12.0));
const RED: Option<(f32, f32)> = Some((0.0, 8.0));

/// The tiled cloth of the Kokiri tunic. Mido and the Fairy Ocarina holder use it too, so
/// Link's models are pointed at a recoloured copy with a name of the same length, as the
/// song harps are.
pub const TILE: &str = "Angin_Tile_A_Alb";
pub const LINK_TILE: &str = "Angin_Tile_L_Alb";
pub const LINK_MODELS: [&str; 6] = [
    "Player_Adult.Player_Adult",
    "Player_Child.Player_Child",
    "Player_Child_Op.Player_Child_Op",
    "Player_Adult_Dark.Player_Adult_Dark",
    "Obj_Armor_Adult_Normal.Obj_Armor_Adult_Normal",
    "Obj_Armor_Child_Normal.Obj_Armor_Child_Normal",
];

const ADULT_BELT: &str = "Mt_Leater_Upper_Player_Adult_Alb";
const CHILD_LEATHER: &str = "Mt_Leather_Player_Child_Alb";
const BOOTS: &str = "Mt_Boots_Player_Adult_Alb";
const GLOVE: &str = "Mt_Glove_Player_Adult_Alb";
const GAUNTLET_GLOVE: &str = "Mt_Glove_Armor_Adult_Gauntlet_Silver_Alb";

const fn part(id: &'static str, label: &'static str, group: &'static str, tex: &'static [&'static str], band: Option<(f32, f32)>, look: Look) -> Item {
    Item { id, label, group, tex, band, look, vals: &[] }
}

pub const ITEMS: [Item; 27] = [
    part(
        "kokiri",
        "Tunic",
        KOKIRI,
        &[LINK_TILE, "Mt_Vest_Player_Adult_Alb", "Mt_Vest_Player_Child_Alb", "Mt_Angin_Acs_Player_Adult_Alb", "Mt_Cloth_Parts_Player_Child_Alb"],
        GREEN,
        Look::Cloth,
    ),
    part("kokiri_pants", "Child pants", KOKIRI, &["Angin_Tile_A_Dark_Alb", "Mt_PantsParts_Player_Child_Alb"], GREEN, Look::Cloth),
    part("kokiri_shirt", "Undershirt", KOKIRI, &["Mt_Inner_Player_Adult_Alb"], None, Look::Whole),
    part("kokiri_belt", "Belts (and child boots)", KOKIRI, &[ADULT_BELT, CHILD_LEATHER], LEATHER, Look::Cloth),
    part("feathers", "Feathers", KOKIRI, &["Mt_Feather_Player_Adult_Alb", "Mt_Feather_Player_Child_Alb"], None, Look::Plain),
    part("goron", "Tunic", GORON, &["Mt_Angin_Armor_Adult_Goron_Alb", "Mt_Inner_Armor_Adult_Goron_Alb", "Mt_Vest_Armor_Adult_Goron_Alb"], Some((354.0, 20.0)), Look::Cloth),
    part("goron_trim", "Gold trim", GORON, &["Mt_Angin_Armor_Adult_Goron_Alb", "Mt_HoodAcs_Armor_Adult_Goron_Alb"], Some((38.0, 10.0)), Look::Cloth),
    part("goron_belt", "Belt", GORON, &["Mt_Leater_Upper_Armor_Adult_Goron_Alb"], LEATHER, Look::Cloth),
    part("goron_beads", "Beads", GORON, &["Mt_Beads_Armor_Adult_Goron_Alb", "Mt_Beads_B_Armor_Adult_Goron_Alb"], None, Look::Plain),
    Item {
        id: "zora",
        label: "Tunic",
        group: ZORA,
        tex: &["Mt_Angin_Armor_Adult_Zora_Alb", "Mt_Angin_Acs_Armor_Adult_Zora_Alb", "Mt_Vest_Armor_Adult_Zora_Alb"],
        band: Some((222.0, 25.0)),
        look: Look::Cloth,
        vals: &[("Armor_Adult_Zora.Armor_Adult_Zora", "Mt_Angin", C0)],
    },
    part("zora_belt", "Belt sash", ZORA, &["Mt_Leater_Upper_Armor_Adult_Zora_Alb"], Some((305.0, 18.0)), Look::Cloth),
    part("zora_shirt", "Undershirt", ZORA, &["Mt_Inner_Armor_Adult_Zora_Alb"], None, Look::Whole),
    part("zora_scales", "Scale trim", ZORA, &["Mt_Parts_Armor_Adult_Zora_Alb"], Some((200.0, 15.0)), Look::Cloth),
    part("boots", "Boots", GEAR, &[BOOTS], LEATHER, Look::Cloth),
    part("gloves", "Gloves", GEAR, &[GLOVE, GAUNTLET_GLOVE], LEATHER, Look::Cloth),
    part("leather_trim", "Red leather trim", GEAR, &[ADULT_BELT, CHILD_LEATHER, BOOTS, GLOVE, GAUNTLET_GLOVE], RED, Look::Cloth),
    Item {
        id: "hair",
        label: "Hair",
        group: OTHER,
        // Adult hair takes its colour from its BC7 albedo: its Mt_Hairs colour values change
        // nothing that shows (tried in game). Child hair has no albedo file, so its values stay.
        tex: &["Mt_Hairs_Player_Adult_Alb"],
        band: None,
        look: Look::Plain,
        vals: &[
            ("Player_Child.Player_Child", "Mt_Hairs", C0),
            ("Player_Child.Player_Child", "Mt_Hairs", C1),
            ("Player_Child.Player_Child", "Mt_Hairs", C3),
        ],
    },
    part("kokiri_sword", "Kokiri Sword", GEAR, &["Mt_Weapon_Sword_Kokiri_Alb"], Some((350.0, 18.0)), Look::Plain),
    Item {
        id: "master_sword",
        label: "Master Sword",
        group: GEAR,
        tex: &["Mt_Weapon_Sword_Master_Alb"],
        band: None,
        look: Look::Plain,
        // color1 is the grip, color0 the wrap stripes (seen in game). The first value of a
        // model is the one that takes the picked colour exactly, the others keep their
        // brightness relative to it, so the grip leads and the stripes stay a darker shade.
        vals: &[
            ("Weapon_Sword_Master.Weapon_Sword_Master", "Mt_Weapon_Sword_Master", C1),
            ("Weapon_Sword_Master.Weapon_Sword_Master", "Mt_Weapon_Sword_Master", C0),
            ("Weapon_Sword_Master.Land_HrlCity_MasterSword", "Mt_Weapon_Sword_Master", C1),
            ("Weapon_Sword_Master.Land_HrlCity_MasterSword", "Mt_Weapon_Sword_Master", C0),
        ],
    },
    part("biggoron_sword", "Biggoron Sword", GEAR, &["Mt_Weapon_Sword_Biggoron_Alb.00", "Mt_Weapon_Sword_Biggoron_Alb.01"], None, Look::Plain),
    part("deku_shield", "Deku Shield", GEAR, &["Mt_Body_Weapon_Shield_Deku_Alb"], Some((2.0, 14.0)), Look::Plain),
    part("hylian_shield", "Hylian Shield", GEAR, &["Mt_Weapon_Shield_Hylian_Alb"], Some((231.0, 35.0)), Look::Plain),
    part("mirror_shield", "Mirror Shield", GEAR, &["Mt_Weapon_Shield_Mirror_Alb"], Some((2.0, 14.0)), Look::Plain),
    Item {
        id: "boomerang",
        label: "Boomerang gem",
        group: GEAR,
        tex: &[],
        band: None,
        look: Look::Plain,
        vals: &[("Item_Boomerang.Item_Boomerang", "Mt_Item_Boomerang_Gem", C0), ("Item_Boomerang.Item_Boomerang", "Mt_Item_Boomerang_Gem", C1), ("Item_Boomerang.Item_Boomerang", "Mt_Item_Boomerang_Gem", C2)],
    },
    Item {
        id: "gauntlets",
        label: "Gauntlet jewels",
        group: GEAR,
        tex: &[],
        band: None,
        look: Look::Plain,
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
        id: "navi",
        label: "Navi",
        group: OTHER,
        tex: &["Npc_Navi_Body_Alb.00", "Npc_Navi_Body_Alb.01", "Npc_Navi_Wings_Alb.00", "Npc_Navi_Wings_Emc.00", "Npc_Navi_Wings_Emc.01"],
        band: None,
        look: Look::Plain,
        vals: &[("Npc_Navi.Npc_Navi", "Mt_Body", C1), ("Npc_Navi.Npc_Navi", "Mt_Body_AlphaMask", C1)],
    },
    part("hearts", "Recovery hearts", OTHER, &["Mt_Obj_Heart_Alb", "Mt_Heart_Container_Alb"], None, Look::Plain),
];

impl Item {
    pub fn tint(&self, to: [u8; 3]) -> Tint {
        let t: Tint = self.band.map(|b| Tint::new(to).band(b.0, b.1)).unwrap_or(Tint::new(to));
        match self.look {
            Look::Plain => t,
            Look::Cloth => t.chroma(SOFT.0, SOFT.1),
            Look::Whole => t.chroma(ALL.0, ALL.1),
        }
    }
}

/// A part's name as the notes and the page show it.
pub fn name_of(it: &Item) -> String {
    if it.label == "Tunic" {
        it.group.to_string()
    } else if it.group == GORON || it.group == ZORA || it.group == KOKIRI {
        format!("{} {}", it.group, it.label.to_ascii_lowercase())
    } else {
        it.label.to_string()
    }
}

/// The game texture a part's texture is painted from.
pub fn source_of(name: &str) -> &str {
    if name == LINK_TILE { TILE } else { name }
}

fn unit(rng: &mut Rng) -> f32 {
    (rng.next_u64() >> 40) as f32 / (1u64 << 24) as f32
}

/// Shade families a random colour is drawn from: weight, saturation range, value range.
/// Vivid, deep, pastel, soft, dark, muted, and greys from near black to near white.
pub const SHADES: [(u64, (f32, f32), (f32, f32)); 7] = [
    (5, (0.80, 1.00), (0.80, 1.00)),
    (3, (0.65, 1.00), (0.35, 0.60)),
    (2, (0.25, 0.45), (0.85, 1.00)),
    (2, (0.45, 0.70), (0.65, 0.90)),
    (2, (0.50, 0.90), (0.12, 0.30)),
    (2, (0.12, 0.35), (0.40, 0.70)),
    (1, (0.00, 0.04), (0.08, 0.97)),
];

pub fn hsv(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h: f32 = h.rem_euclid(360.0);
    let c: f32 = v * s;
    let x: f32 = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let (r, g, b): (f32, f32, f32) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m: f32 = v - c;
    [r, g, b].map(|q| ((q + m) * 255.0).round().clamp(0.0, 255.0) as u8)
}

/// A random colour: any hue, in one of the shade families.
pub fn roll(rng: &mut Rng) -> [u8; 3] {
    let h: f32 = unit(rng) * 360.0;
    let total: u64 = SHADES.iter().map(|s| s.0).sum();
    let mut k: u64 = rng.below(total);
    let mut f: (u64, (f32, f32), (f32, f32)) = SHADES[0];
    for s in SHADES {
        if k < s.0 {
            f = s;
            break;
        }
        k -= s.0;
    }
    let s: f32 = f.1.0 + unit(rng) * (f.1.1 - f.1.0);
    let v: f32 = f.2.0 + unit(rng) * (f.2.1 - f.2.0);
    hsv(h, s, v)
}

/// The hue, saturation and value of a colour, the inverse of `hsv`.
pub fn hsv_of(c: [u8; 3]) -> (f32, f32, f32) {
    let [r, g, b]: [f32; 3] = c.map(|v| v as f32 / 255.0);
    let (hi, lo): (f32, f32) = (r.max(g).max(b), r.min(g).min(b));
    let d: f32 = hi - lo;
    let h: f32 = if d <= 0.0 {
        0.0
    } else if hi == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if hi == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, if hi <= 0.0 { 0.0 } else { d / hi }, hi)
}

/// A shade of `base` for one part: nearly the same hue, its own saturation and lightness.
pub fn shade_of(base: [u8; 3], rng: &mut Rng) -> [u8; 3] {
    let (h, s, v): (f32, f32, f32) = hsv_of(base);
    let h: f32 = h + (unit(rng) - 0.5) * 16.0;
    let s: f32 = (s * (0.55 + unit(rng) * 0.65)).clamp(0.0, 1.0);
    let v: f32 = (v * (0.5 + unit(rng) * 0.8)).clamp(0.08, 1.0);
    hsv(h, s, v)
}

fn part_roll(seed: &str, id: &str) -> [u8; 3] {
    roll(&mut Rng::from_text(&format!("{seed} colors part {id}")))
}

pub fn plan(c: Colors, seed: &str, picks: &Picks) -> Vec<(&'static Item, [u8; 3])> {
    let base: [u8; 3] = roll(&mut Rng::from_text(&format!("{seed} colors shared")));
    let mut out: Vec<(&'static Item, [u8; 3])> = Vec::new();
    for it in &ITEMS {
        let to: Option<[u8; 3]> = match (c, picks.get(it.id).copied()) {
            (Colors::Off, _) | (_, Some(Pick::Original)) | (Colors::Custom, None) => None,
            (_, Some(Pick::Rgb(x))) => Some(x),
            (Colors::Shared, None) => Some(shade_of(base, &mut Rng::from_text(&format!("{seed} colors shade {}", it.id)))),
            (Colors::Random, _) | (_, Some(Pick::Random)) => Some(part_roll(seed, it.id)),
        };
        if let Some(to) = to {
            out.push((it, to));
        }
    }
    out
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
    let mut out: Vec<String> = vec![NAMES.to_string(), rsizetable::TABLE_PATH.to_string()];
    for it in &ITEMS {
        out.extend(it.tex.iter().map(|t| tex_path(source_of(t))));
        out.extend(it.vals.iter().map(|v| model_path(v.0)));
    }
    out.extend(LINK_MODELS.iter().map(|m| model_path(m)));
    out.sort();
    out.dedup();
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

/// Swaps every stored name `old` for `new` (the same length). Only whole names count: the
/// two bytes before hold the name's length and a NUL follows, as bfres and bntx store them.
pub fn rename(b: &mut [u8], old: &str, new: &str) -> Res<usize> {
    let (o, n): (&[u8], &[u8]) = (old.as_bytes(), new.as_bytes());
    if o.len() != n.len() || o.len() > u16::MAX as usize {
        return Err(E::Format("colours: a texture name must keep its length"));
    }
    let len: [u8; 2] = (o.len() as u16).to_le_bytes();
    let mut hits: usize = 0;
    let mut i: usize = 2;
    while i + o.len() < b.len() {
        if &b[i..i + o.len()] == o && b[i - 2..i] == len && b[i + o.len()] == 0 {
            b[i..i + o.len()].copy_from_slice(n);
            hits += 1;
            i += o.len();
        } else {
            i += 1;
        }
    }
    Ok(hits)
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

/// One colour map for every part painted on one texture. Each part's weight is read from
/// the game's own pixel, so a part painted first cannot pull a pixel into another's band.
pub fn combined(fits: &[Tint], c: [u8; 3]) -> [u8; 3] {
    let mut d: [i32; 3] = [0; 3];
    for s in fits {
        let r: [u8; 3] = s.rgb(c);
        for i in 0..3 {
            d[i] += r[i] as i32 - c[i] as i32;
        }
    }
    [0, 1, 2].map(|i| (c[i] as i32 + d[i]).clamp(0, 255) as u8)
}

type Planned = (&'static Item, [u8; 3]);

/// The parts painted on each texture, in part order.
fn by_texture(todo: &[Planned]) -> BM<&'static str, Vec<Planned>> {
    let mut out: BM<&'static str, Vec<Planned>> = BM::new();
    for (it, to) in todo {
        for t in it.tex {
            out.entry(*t).or_default().push((*it, *to));
        }
    }
    out
}

/// Gives each new file of the mod the size table entry of the game file it was made from.
fn size_new(mod_root: &P, rf: &mut Romfs, made: &[(String, String)]) -> Res<()> {
    if made.is_empty() {
        return Ok(());
    }
    let at: PB = mod_root.join("romfs").join(rsizetable::TABLE_PATH);
    let mut t: Rst = match fs::read(&at) {
        Ok(z) if !z.is_empty() => Rst::parse(&rf.zs.decompress(&z)?)?,
        _ => rsizetable::load(rf)?,
    };
    for (new, from) in made {
        let n: u32 = t.get(from).ok_or_else(|| E::Msg(format!("colours: the size table has no entry for {from}")))?;
        t.set_size(new, n)?;
    }
    if let Some(d) = at.parent() {
        fs::create_dir_all(d)?;
    }
    fs::write(&at, t.to_zs()?)?;
    Ok(())
}

pub fn apply(mod_root: &P, romfs: &P, c: Colors, picks: &Picks, seed: &str) -> Res<Vec<String>> {
    let mut notes: Vec<String> = Vec::new();
    let todo: Vec<Planned> = plan(c, seed, picks);
    if todo.is_empty() {
        return Ok(notes);
    }
    let mut rf: Romfs = Romfs::open(romfs)?;
    let keys: BM<String, u64> = bfres::keys(&rf.read_zs(NAMES)?)?;
    let mut hit: Vec<&str> = Vec::new();
    let mut made: Vec<(String, String)> = Vec::new();
    let (mut files, mut bytes): (usize, u64) = (0, 0);
    for (name, parts) in by_texture(&todo) {
        let src: &str = source_of(name);
        let Ok(z) = rf.read(&tex_path(src)) else {
            continue;
        };
        let mut raw: Vec<u8> = rf.zs.decompress(&z)?;
        let mut fits: Vec<Tint> = Vec::new();
        for (it, to) in &parts {
            if let Some(s) = fitted(&raw, it.tint(*to))? {
                fits.push(s);
                hit.push(it.id);
            }
        }
        if fits.is_empty() {
            continue;
        }
        let n: Count = repaint::repaint(&mut raw, &|px| combined(&fits, px))?;
        if n.changed == 0 {
            continue;
        }
        if name != src {
            if rename(&mut raw, src, name)? == 0 {
                return Err(E::Format("colours: a texture file does not carry its own name, the game files differ"));
            }
            made.push((tex_path(name), tex_path(src)));
        }
        put(mod_root, &tex_path(name), &raw, &z)?;
        files += 1;
        bytes += raw.len() as u64;
    }
    let mut by_model: BM<&str, Vec<Edits>> = BM::new();
    for (it, to) in &todo {
        let mut groups: BM<&str, Vec<(&str, &str)>> = BM::new();
        for (m, mat, param) in it.vals {
            groups.entry(m).or_default().push((mat, param));
            hit.push(it.id);
        }
        for (m, g) in groups {
            by_model.entry(m).or_default().push((it.tint(*to), g));
        }
    }
    let renamed: bool = made.iter().any(|m| m.0 == tex_path(LINK_TILE));
    let mut models: Vec<&str> = by_model.keys().copied().collect();
    if renamed {
        models.extend(LINK_MODELS);
        models.sort_unstable();
        models.dedup();
    }
    for m in models {
        let r: String = model_path(m);
        let Ok(z) = rf.read(&r) else {
            continue;
        };
        let mut raw: Vec<u8> = rf.zs.decompress(&z)?;
        let mut n: usize = 0;
        for (t, g) in by_model.get(m).map(|v| v.as_slice()).unwrap_or(&[]) {
            n += tint_model(&mut raw, &keys, g, t)?;
        }
        if renamed && LINK_MODELS.contains(&m) {
            n += rename(&mut raw, TILE, LINK_TILE)?;
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
    size_new(mod_root, &mut rf, &made)?;
    let done: Vec<String> = todo.iter().filter(|(it, _)| hit.contains(&it.id)).map(|(it, to)| format!("{} {}", name_of(it), hex(*to))).collect();
    let how: &str = match c {
        Colors::Off => "not at all",
        Colors::Custom => "for the parts picked by hand",
        Colors::Random => "with a random colour for each part",
        Colors::Shared => "in shades of one random colour",
    };
    notes.push(format!(
        "Colours are changed {how}. {} files rewritten, {} MB added to the mod. Changed: {}. Particle glows, trails and the menu keep their own colours.",
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

pub fn preview(romfs: &P, out: &P, c: Colors, picks: &Picks, seed: &str) -> Res<Vec<String>> {
    let mut lines: Vec<String> = Vec::new();
    let mut rf: Romfs = Romfs::open(romfs)?;
    fs::create_dir_all(out)?;
    let todo: Vec<Planned> = plan(c, seed, picks);
    for (t, parts) in by_texture(&todo) {
        let Ok(old) = rf.read_zs(&tex_path(source_of(t))) else {
            lines.push(format!("{t}: missing"));
            continue;
        };
        let mut fits: Vec<Tint> = Vec::new();
        for (it, to) in &parts {
            match fitted(&old, it.tint(*to))? {
                Some(s) => fits.push(s),
                None => lines.push(format!("{t} {}: no pixel in the colour band", it.id)),
            }
        }
        if fits.is_empty() {
            continue;
        }
        let mut new: Vec<u8> = old.clone();
        let n: Count = repaint::repaint(&mut new, &|px| combined(&fits, px))?;
        let (a, b): (Img, Img) = (shot(&old)?, shot(&new)?);
        let (mut sum, mut worst): (u64, u32) = (0, 0);
        for (p, q) in a.px.chunks_exact(4).zip(b.px.chunks_exact(4)) {
            let w: [u8; 3] = combined(&fits, [p[0], p[1], p[2]]);
            for i in 0..3 {
                let d: u32 = (w[i] as i32 - q[i] as i32).unsigned_abs();
                sum += d as u64;
                worst = worst.max(d);
            }
        }
        fs::write(out.join(format!("{t}_before.png")), png::encode(&a))?;
        fs::write(out.join(format!("{t}_after.png")), png::encode(&b))?;
        let to: Vec<String> = parts.iter().map(|p| format!("{} {}", p.0.id, hex(p.1))).collect();
        lines.push(format!("{t}: {}, {} of {} blocks changed, {} skipped, mean error {:.2}, worst {}", to.join(", "), n.changed, n.blocks, n.skipped, sum as f64 / (a.px.len() as f64 * 0.75), worst));
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfres::fixture;

    fn none() -> Picks {
        Picks::new()
    }

    fn item(id: &str) -> &'static Item {
        ITEMS.iter().find(|i| i.id == id).unwrap()
    }

    fn real() -> Option<PB> {
        std::env::var("OOTRR_ROMFS").map(PB::from).ok().filter(|p| p.join("Pack").join("ZsDic.pack.zs").is_file())
    }

    #[test]
    fn mode_names_round_trip() {
        for m in [Colors::Off, Colors::Custom, Colors::Random, Colors::Shared] {
            assert_eq!(Colors::parse(m.id()), Some(m));
        }
        assert_eq!(Colors::parse("loud"), None);
    }

    #[test]
    fn picks_parse_and_round_trip() {
        for p in [Pick::Original, Pick::Random, Pick::Rgb([0x12, 0xab, 0xff])] {
            assert_eq!(Pick::parse(&p.id()), Some(p));
        }
        assert_eq!(Pick::parse(""), Some(Pick::Original));
        assert_eq!(Pick::parse("follow"), Some(Pick::Original));
        assert_eq!(Pick::parse("#12ABff"), Some(Pick::Rgb([0x12, 0xab, 0xff])));
        for bad in ["12abff", "#12abf", "#12abfg", "#12abff0", "blue", "#ééé"] {
            assert_eq!(Pick::parse(bad), None, "{bad}");
        }
        let ok: Picks = parse_picks(&[("kokiri".into(), "#00ff00".into()), ("boots".into(), "original".into())]).unwrap();
        assert_eq!(ok.len(), 2);
        assert_eq!(ok.get("boots"), Some(&Pick::Original));
        assert!(parse_picks(&[("cape".into(), "random".into())]).is_err());
        assert!(parse_picks(&[("kokiri".into(), "green".into())]).is_err());
    }

    #[test]
    fn the_plan_is_fixed_by_the_seed_and_off_plans_nothing() {
        let a: Vec<[u8; 3]> = plan(Colors::Random, "s", &none()).into_iter().map(|x| x.1).collect();
        let b: Vec<[u8; 3]> = plan(Colors::Random, "s", &none()).into_iter().map(|x| x.1).collect();
        let c: Vec<[u8; 3]> = plan(Colors::Random, "t", &none()).into_iter().map(|x| x.1).collect();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), ITEMS.len());
        assert!(a.windows(2).any(|w| w[0] != w[1]));
        assert!(plan(Colors::Off, "s", &none()).is_empty());
    }

    #[test]
    fn custom_paints_only_the_picked_parts_and_random_modes_keep_the_picks() {
        let picks: Picks = [("kokiri_belt".to_string(), Pick::Rgb([1, 2, 3])), ("kokiri".to_string(), Pick::Original), ("boots".to_string(), Pick::Random)].into_iter().collect();
        let mine: Vec<(&Item, [u8; 3])> = plan(Colors::Custom, "s", &picks);
        assert_eq!(mine.iter().map(|x| x.0.id).collect::<Vec<_>>(), ["kokiri_belt", "boots"]);
        assert_eq!(mine[0].1, [1, 2, 3]);
        assert!(plan(Colors::Custom, "s", &none()).is_empty());
        assert!(plan(Colors::Off, "s", &picks).is_empty());
        let of = |p: &[(&Item, [u8; 3])], id: &str| p.iter().find(|x| x.0.id == id).map(|x| x.1);
        let free: Vec<(&Item, [u8; 3])> = plan(Colors::Random, "s", &none());
        for c in [Colors::Random, Colors::Shared] {
            let all: Vec<(&Item, [u8; 3])> = plan(c, "s", &picks);
            assert_eq!(all.len(), ITEMS.len() - 1);
            assert_eq!(of(&all, "kokiri_belt"), Some([1, 2, 3]));
            assert_eq!(of(&all, "kokiri"), None);
            assert_eq!(of(&all, "boots"), of(&mine, "boots"));
        }
        assert_eq!(of(&plan(Colors::Random, "s", &picks), "gloves"), of(&free, "gloves"));
        assert_ne!(of(&mine, "boots"), of(&plan(Colors::Custom, "t", &picks), "boots"));
    }

    #[test]
    fn every_random_part_draws_its_own_colour() {
        let p: Vec<(&Item, [u8; 3])> = plan(Colors::Random, "s", &none());
        let mut seen: Vec<[u8; 3]> = p.iter().map(|x| x.1).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), ITEMS.len());
    }

    #[test]
    fn hsv_of_undoes_hsv() {
        for (h, s, v) in [(0.0, 1.0, 1.0), (95.0, 0.4, 0.6), (222.0, 0.8, 0.3), (300.0, 0.0, 0.5)] {
            let (a, b, c): (f32, f32, f32) = hsv_of(hsv(h, s, v));
            assert!((b - s).abs() < 0.02 && (c - v).abs() < 0.01, "{h} {s} {v} -> {a} {b} {c}");
            if s > 0.0 {
                assert!((a - h).abs() < 1.5, "{h} -> {a}");
            }
        }
    }

    #[test]
    fn shared_mode_gives_every_part_a_shade_of_one_hue() {
        for seed in ["s", "t", "u", "v", "w", "x"] {
            let base: [u8; 3] = roll(&mut Rng::from_text(&format!("{seed} colors shared")));
            let a: Vec<[u8; 3]> = plan(Colors::Shared, seed, &none()).into_iter().map(|x| x.1).collect();
            assert_eq!(a.len(), ITEMS.len());
            let mut d: Vec<[u8; 3]> = a.clone();
            d.sort_unstable();
            d.dedup();
            assert!(d.len() > ITEMS.len() / 2, "{seed}: shades should differ");
            let (bh, bs, _): (f32, f32, f32) = hsv_of(base);
            for c in &a {
                let (h, s, _): (f32, f32, f32) = hsv_of(*c);
                if bs > 0.15 && s > 0.1 {
                    let far: f32 = ((h - bh + 540.0).rem_euclid(360.0) - 180.0).abs();
                    assert!(far <= 9.0, "{seed}: {} far from {}", hex(*c), hex(base));
                }
                if bs < 0.05 {
                    assert!(s < 0.06, "{seed}: a grey base keeps greys, got {}", hex(*c));
                }
            }
        }
        assert_ne!(plan(Colors::Shared, "s", &none())[0].1, plan(Colors::Shared, "t", &none())[0].1);
    }

    #[test]
    fn hsv_hits_the_primaries_and_greys() {
        assert_eq!(hsv(0.0, 1.0, 1.0), [255, 0, 0]);
        assert_eq!(hsv(120.0, 1.0, 1.0), [0, 255, 0]);
        assert_eq!(hsv(240.0, 1.0, 1.0), [0, 0, 255]);
        assert_eq!(hsv(300.0, 0.0, 0.5), [128, 128, 128]);
        assert_eq!(hsv(359.9, 1.0, 1.0)[0], 255);
    }

    #[test]
    fn random_colours_cover_every_hue_and_every_shade() {
        let runs: u32 = 4000;
        let mut hues: [u32; 12] = [0; 12];
        let (mut dark, mut pastel, mut vivid, mut grey, mut white): (u32, u32, u32, u32, u32) = (0, 0, 0, 0, 0);
        let mut seen: std::collections::BTreeSet<[u8; 3]> = std::collections::BTreeSet::new();
        for i in 0..runs {
            let c: [u8; 3] = roll(&mut Rng::from_text(&format!("seed {i}")));
            seen.insert(c);
            let (hi, lo): (u8, u8) = (*c.iter().max().unwrap(), *c.iter().min().unwrap());
            let (h, chroma): (f32, f32) = crate::tex::shift::hue(c.map(|v| v as f32));
            if hi - lo < 16 {
                grey += 1;
                if lo > 220 {
                    white += 1;
                }
                continue;
            }
            hues[(h / 30.0) as usize % 12] += 1;
            if hi < 80 {
                dark += 1;
            }
            if lo > 150 && chroma > 20.0 {
                pastel += 1;
            }
            if hi > 200 && lo < 50 {
                vivid += 1;
            }
        }
        let want: f64 = hues.iter().sum::<u32>() as f64 / 12.0;
        assert!(hues.iter().all(|n| (*n as f64) > want * 0.6 && (*n as f64) < want * 1.6), "{hues:?}");
        assert!(dark > runs / 12 && pastel > runs / 20 && vivid > runs / 5 && grey > runs / 40 && white > 0, "{dark} {pastel} {vivid} {grey} {white}");
        assert!(seen.len() as u32 > runs * 9 / 10);
    }

    #[test]
    fn every_tunic_covers_its_body_texture_and_link_gets_his_own_cloth() {
        assert!(item("kokiri").tex.contains(&"Mt_Vest_Player_Adult_Alb") && item("kokiri").tex.contains(&"Mt_Vest_Player_Child_Alb"));
        assert!(item("kokiri").tex.contains(&LINK_TILE) && !ITEMS.iter().any(|i| i.tex.contains(&TILE)));
        assert!(item("goron").tex.contains(&"Mt_Vest_Armor_Adult_Goron_Alb"));
        assert!(item("zora").tex.contains(&"Mt_Vest_Armor_Adult_Zora_Alb"));
        assert!(ITEMS.iter().filter(|i| i.look == Look::Cloth).all(|i| i.band.is_some()));
        assert_eq!(LINK_TILE.len(), TILE.len());
        assert_eq!(source_of(LINK_TILE), TILE);
        assert_eq!(source_of("Mt_Vest_Player_Adult_Alb"), "Mt_Vest_Player_Adult_Alb");
        assert!(files().contains(&tex_path(TILE)) && !files().contains(&tex_path(LINK_TILE)));
        assert!(LINK_MODELS.iter().all(|m| files().contains(&model_path(m))));
    }

    #[test]
    fn item_ids_are_unique_and_every_item_changes_something() {
        let mut ids: Vec<&str> = ITEMS.iter().map(|i| i.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), ITEMS.len());
        assert!(ITEMS.iter().all(|i| !i.tex.is_empty() || !i.vals.is_empty()));
        assert!(ITEMS.iter().all(|i| !i.label.is_empty() && !i.group.is_empty()));
    }

    #[test]
    fn the_master_sword_grip_takes_the_picked_colour_and_the_stripes_a_darker_shade() {
        let Some(root) = real() else {
            return;
        };
        let out: PB = std::env::temp_dir().join(format!("ootrr_colors_sword_{}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        let picks: Picks = [("master_sword".to_string(), Pick::Rgb([40, 200, 60]))].into_iter().collect();
        apply(&out, &root, Colors::Custom, &picks, "s").unwrap();
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let keys: BM<String, u64> = bfres::keys(&rf.read_zs(NAMES).unwrap()).unwrap();
        let r: String = model_path("Weapon_Sword_Master.Weapon_Sword_Master");
        let b: Vec<u8> = rf.zs.decompress(&fs::read(out.join("romfs").join(&r)).unwrap()).unwrap();
        let ms: Vec<Mat> = bfres::mats(&b).unwrap();
        let at = |c: &str| bfres::read4(&b, bfres::find(&ms, "Mt_Weapon_Sword_Master", keys[c]).unwrap().at).unwrap();
        let want: [f32; 3] = crate::tex::shift::lin([40, 200, 60]);
        let grip: [f32; 4] = at(C1);
        assert!((0..3).all(|i| (grip[i] - want[i]).abs() < 1e-3), "{grip:?} {want:?}");
        let stripes: [f32; 4] = at(C0);
        assert!(luma([stripes[0], stripes[1], stripes[2]]) < luma(want) * 0.7, "{stripes:?}");
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn a_rename_only_swaps_whole_stored_names() {
        let stored = |s: &str| {
            let mut v: Vec<u8> = (s.len() as u16).to_le_bytes().to_vec();
            v.extend_from_slice(s.as_bytes());
            v.push(0);
            v
        };
        let mut b: Vec<u8> = [stored(TILE), stored("Angin_Tile_A_Alb_X"), stored(TILE), b"xxAngin_Tile_A_Alb\0".to_vec()].concat();
        let old: Vec<u8> = b.clone();
        assert_eq!(rename(&mut b, TILE, LINK_TILE).unwrap(), 2);
        assert_eq!(b.len(), old.len());
        let s: String = String::from_utf8_lossy(&b).into_owned();
        assert_eq!(s.matches(LINK_TILE).count(), 2);
        assert_eq!(s.matches("Angin_Tile_A_Alb_X").count(), 1);
        assert!(rename(&mut b, TILE, "short").is_err());
    }

    #[test]
    fn parts_on_one_texture_read_their_weight_from_the_game_pixel() {
        let green: Tint = Tint::new([200, 0, 0]).band(100.0, 45.0).fit(&{
            let mut i: Img = Img::new(1, 1);
            i.put(0, 0, [60, 120, 40, 255]);
            i
        })
        .unwrap();
        let red: Tint = Tint::new([0, 200, 0]).band(0.0, 8.0).fit(&{
            let mut i: Img = Img::new(1, 1);
            i.put(0, 0, [150, 30, 30, 255]);
            i
        })
        .unwrap();
        let fits: [Tint; 2] = [green, red];
        let g: [u8; 3] = combined(&fits, [60, 120, 40]);
        assert!(g[0] > 150 && g[1] < 40, "{g:?}");
        let r: [u8; 3] = combined(&fits, [150, 30, 30]);
        assert!(r[1] > 150 && r[0] < 40, "{r:?}");
        assert_eq!(combined(&fits, [128, 128, 128]), [128, 128, 128]);
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
        let Some(root) = real() else {
            return;
        };
        let out: PB = std::env::temp_dir().join(format!("ootrr_colors_{}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        let notes: Vec<String> = apply(&out, &root, Colors::Random, &none(), "colour seed").unwrap();
        assert!(notes[0].contains("Kokiri tunic #") && notes[0].contains("Master Sword") && notes[0].contains("Zora tunic scale trim"), "{notes:?}");
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut seen: usize = 0;
        let mut paths: Vec<String> = Vec::new();
        for it in &ITEMS {
            paths.extend(it.tex.iter().map(|t| tex_path(t)));
            paths.extend(it.vals.iter().map(|v| model_path(v.0)));
        }
        paths.extend(LINK_MODELS.iter().map(|m| model_path(m)));
        paths.sort();
        paths.dedup();
        for r in paths {
            let p: PB = out.join("romfs").join(&r);
            if !p.is_file() {
                continue;
            }
            let from: String = if r == tex_path(LINK_TILE) { tex_path(TILE) } else { r.clone() };
            let old: Vec<u8> = rf.read_zs(&from).unwrap();
            let new: Vec<u8> = rf.zs.decompress(&fs::read(&p).unwrap()).unwrap();
            assert_eq!(old.len(), new.len(), "{r}");
            assert_ne!(old, new, "{r}");
            seen += 1;
        }
        assert!(seen >= 45, "{seen}");
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn the_real_kokiri_tunic_leaves_mido_alone() {
        let Some(root) = real() else {
            return;
        };
        let out: PB = std::env::temp_dir().join(format!("ootrr_colors_tile_{}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        let picks: Picks = [("kokiri".to_string(), Pick::Rgb([200, 40, 40]))].into_iter().collect();
        apply(&out, &root, Colors::Custom, &picks, "s").unwrap();
        let rom: PB = out.join("romfs");
        assert!(!rom.join(tex_path(TILE)).exists());
        assert!(!rom.join(model_path("Npc_Kokiri_MC_Mido.Npc_Kokiri_MC_Mido")).exists());
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let t: Vec<u8> = rf.zs.decompress(&fs::read(rom.join(tex_path(LINK_TILE))).unwrap()).unwrap();
        assert_eq!(bntx::parse(&t).unwrap()[0].name, LINK_TILE);
        for m in LINK_MODELS {
            let b: Vec<u8> = rf.zs.decompress(&fs::read(rom.join(model_path(m))).unwrap()).unwrap();
            let s: String = String::from_utf8_lossy(&b).into_owned();
            assert!(s.contains(LINK_TILE) && !s.contains(TILE), "{m}");
            assert_eq!(b.len(), rf.read_zs(&model_path(m)).unwrap().len(), "{m}");
        }
        let table: Rst = Rst::parse(&rf.zs.decompress(&fs::read(rom.join(rsizetable::TABLE_PATH)).unwrap()).unwrap()).unwrap();
        let game: Rst = rsizetable::load(&mut rf).unwrap();
        assert_eq!(table.get(&tex_path(LINK_TILE)), game.get(&tex_path(TILE)));
        assert!(game.get(&tex_path(LINK_TILE)).is_none());
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn an_unchanged_repaint_keeps_every_real_texture_byte_for_byte() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        for it in &ITEMS {
            for t in it.tex {
                let old: Vec<u8> = rf.read_zs(&tex_path(source_of(t))).unwrap();
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
                let lab: [f32; 3] = crate::tex::shift::oklab(crate::tex::shift::lin([p[0], p[1], p[2]]));
                let core: bool = t.keep() < 1.0 || lab[1].hypot(lab[2]) >= t.chroma * crate::tex::shift::CORE;
                let w: f64 = if core { t.weight([p[0], p[1], p[2]]) as f64 } else { 0.0 };
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
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut k: usize = 0;
        for seed in ["first seed", "second seed"] {
            for (it, to) in plan(Colors::Random, seed, &none()).into_iter().filter(|x| x.0.look != Look::Plain) {
                for t in it.tex {
                    k += 1;
                    if k % PARTS != part {
                        continue;
                    }
                    let old: Vec<u8> = rf.read_zs(&tex_path(source_of(t))).unwrap();
                    let s: Tint = fitted(&old, it.tint(to)).unwrap().unwrap();
                    let mut new: Vec<u8> = old.clone();
                    let n: Count = repaint::repaint(&mut new, &|c| s.rgb(c)).unwrap();
                    assert!(n.changed > 0, "{t} {n:?}");
                    let got: [u8; 3] = weighted_mean(&new, &s, &old);
                    // The part keeps its own shades: its colourful core (pixels about as colourful
                    // as the part's most colourful) takes the target's hue, lightness and most of
                    // its chroma (the most vivid shades clip at the edge of the gamut), while its
                    // near-neutral stone, weave and stitching stay muted.
                    let lab = |c: [u8; 3]| crate::tex::shift::oklab(crate::tex::shift::lin(c));
                    let (g, w): ([f32; 3], [f32; 3]) = (lab(got), lab(to));
                    let (gc, wc): (f32, f32) = (g[1].hypot(g[2]), w[1].hypot(w[2]));
                    let turn: f32 = ((g[2].atan2(g[1]) - w[2].atan2(w[1])).to_degrees() + 540.0).rem_euclid(360.0) - 180.0;
                    println!("{seed} {} {t} target {} got {} light {:+.3} chroma {:.2} hue {turn:+.1} changed {} of {}", it.id, hex(to), hex(got), g[0] - w[0], gc / wc.max(1e-4), n.changed, n.blocks);
                    assert!((g[0] - w[0]).abs() <= 0.1, "{} {t} lightness: target {} got {}", it.id, hex(to), hex(got));
                    if wc > 0.04 {
                        // A muted target has little hue to hit: judge it more loosely.
                        let (tol, share): (f32, f32) = if wc < 0.08 { (20.0, 0.5) } else { (12.0, 0.6) };
                        assert!(turn.abs() <= tol && gc >= wc * share, "{} {t} colour: target {} got {}", it.id, hex(to), hex(got));
                    } else {
                        assert!(gc <= 0.06, "{} {t} a grey target stays grey: got {}", it.id, hex(got));
                    }
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
        let (a, b): (Vec<[u8; 3]>, Vec<[u8; 3]>) = (plan(Colors::Random, "first seed", &none()).iter().map(|x| x.1).collect(), plan(Colors::Random, "second seed", &none()).iter().map(|x| x.1).collect());
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
