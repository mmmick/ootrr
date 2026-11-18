use std::collections::{BTreeMap as BM, BTreeSet as BS};

use crate::byml::{Byml, Node};
use crate::error::Res;
use crate::placement::{banc_files, hash_of};
use crate::romfs::Romfs;
use crate::settings::seed::SeedFile;
use crate::settings::Settings;
use crate::slice::data::{Data, Ev, Item};

pub const SETTING: &str = "chests_match_contents";

/// The plain wooden chests, small model first. Other chests (the shield chest, invisible
/// chests, the minigame chests, A_03 and A_04) keep their model.
pub const PAIRS: [(&str, &str); 2] = [("Obj_TBox_A_01", "Obj_TBox_A_02"), ("Obj_TBox_A_01_NoBake", "Obj_TBox_A_02_NoBake")];

pub const NOTE: &str = "Chest size matches contents: chests holding progression items, songs, stones, medallions, boss keys, keys away from their own dungeon and shields or capacity upgrades you do not start with are big, the rest are small. Boss hearts that fall back to a chest and summoned chests (Jabu Boomerang, Iron Boots, Ice Arrows, Silver Gauntlets, Dampe race) follow the same rule. Story-scene chests, invisible chests and other special models keep their size. Experimental, untested in the game.";

pub const AREAS: [&str; 2] = ["DropKeepAwayArea_TBox_A_01", "DropKeepAwayArea_TBox_A_02"];

pub const ROOM: f32 = 1.5;

pub const TILT: f32 = 0.06;

pub const MARKS: [&str; 46] = [
    "anchor", "area", "tag", "startpos", "mapicon", "locator", "eventrequest", "eventproxy", "logic", "spawner", "navi", "sensor", "trigger", "point", "camera", "sound",
    "bgm", "se_", "light", "effect", "emitter", "fog", "env", "spot", "guide", "hint", "link", "rail", "path", "region", "water", "kill", "respawn", "check", "wind",
    "dropkeepaway", "mgr", "manager", "director", "controller", "counter", "timer", "flag", "switch_hidden", "dummy", "debug",
];

pub const WALLS: [&str; 6] = ["land_", "dgn", "field", "fldobj_wall", "fldobj_airwall", "pushairwall"];

#[derive(Clone, Debug, PartialEq)]
pub struct Seen {
    pub hash: u64,
    pub gyaml: String,
    pub at: [f32; 3],
    pub rot: [f32; 3],
    pub story: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Moves {
    pub chests: BS<u64>,
    pub areas: BM<(String, [u32; 3]), &'static str>,
    pub tight: BS<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Want {
    pub big: bool,
    pub strip: bool,
}

pub fn mode(s: &Settings, bad: &mut Vec<String>) -> bool {
    s.flag(SETTING).unwrap_or_else(|e| {
        bad.push(e.to_string());
        false
    })
}

/// Gear that is not progression but still worth a big chest, unless the seed starts with it.
pub const GEAR: [&str; 5] = ["Deku Shield", "Hylian Shield", "Double Defense", "Deku Nut Capacity", "Deku Stick Capacity"];

/// Small keys, key rings and silver rupees: big only when they sit outside their own dungeon.
pub const KEYS: [&str; 7] = ["SmallKey", "SmallKeyRing", "HideoutSmallKey", "HideoutSmallKeyRing", "TCGSmallKey", "TCGSmallKeyRing", "SilverRupee"];

/// What the build knows about the player when the chest is placed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ctx {
    /// Bombchus are in logic.
    pub chus: bool,
    /// Logic names of the starting items.
    pub owned: Vec<&'static str>,
}

/// Major items get big chests: progression items, songs, stones and medallions, boss keys,
/// small keys / key rings / silver rupees found outside their own dungeon, and the gear in
/// GEAR the player does not start with. Hearts, Bombchus (unless in logic), maps, compasses,
/// tokens and the rest are minor. `at` is "<location name>|<location area>", see [at].
pub fn major(i: &Item, at: &str, ctx: &Ctx) -> bool {
    match i.kind.as_str() {
        "BossKey" | "GanonBossKey" | "Song" | "DungeonReward" => true,
        k if KEYS.contains(&k) => !home(&i.name, at),
        "Item" if GEAR.contains(&i.name.as_str()) => !ctx.owned.contains(&i.name.as_str()),
        "Item" => i.adv && i.name != "Heart Container" && !i.name.starts_with("Piece of Heart") && (ctx.chus || !i.name.starts_with("Bombchus")),
        _ => false,
    }
}

fn norm(s: &str) -> String {
    s.replace('\'', "").replace("Ganons Tower", "Ganons Castle")
}

/// Whether a dungeon item (named "Small Key (Forest Temple)", "Silver Rupee (Dodongos Cavern
/// Staircase)", ...) sits in its own dungeon. An item that names no dungeon counts as home.
pub fn home(item: &str, at: &str) -> bool {
    let Some(d) = item.split_once('(').and_then(|(_, r)| r.strip_suffix(')')) else {
        return true;
    };
    let d: String = norm(d);
    let (name, area) = at.split_once('|').unwrap_or((at, ""));
    let (name, area) = (norm(name), norm(area));
    name.contains(&d) ||(!area.is_empty() && (d == area || d.starts_with(&format!("{area} "))))
}

pub fn at(l: &crate::catalog::Loc) -> String {
    format!("{}|{}", l.name, l.area)
}

/// The size each placed chest should have, by actor hash. Chests whose story event stays,
/// and the special chests that always run the big chest flow, are left out.
pub fn wanted(d: &Data, seed: &SeedFile, ctx: &Ctx, maps: bool) -> BM<u64, Want> {
    let mut out: BM<u64, Want> = BM::new();
    for (loc, item) in &seed.placements {
        let (Some(l), Some(it)) = (d.cat.by_name(loc), d.item(item)) else {
            continue;
        };
        if l.kind == "bossheart" {
            continue;
        }
        let big: bool = major(it, &at(l), ctx);
        if !big && !shrinks(it, maps) {
            continue;
        }
        let moved: bool = d.vanilla_of(loc) != Some(item.as_str());
        for r in &l.refs {
            let ev: Option<Ev> = d.event_mode(r.hash);
            let summoned: bool = crate::special::sized(r.hash).is_some();
            if ev == Some(Ev::Keep) && !summoned {
                continue;
            }
            out.insert(r.hash, Want { big, strip: summoned || (moved && ev == Some(Ev::Strip)) });
        }
    }
    pair(&mut out);
    out
}

/// The Dampe race rupee chests share one open transition: they change size only together.
/// A chest that is not listed keeps its vanilla small model.
pub fn pair(w: &mut BM<u64, Want>) {
    let [a, b] = crate::special::DAMPE_RUPEES;
    let big = |h: u64| w.get(&h).is_some_and(|x| x.big);
    if big(a) != big(b) {
        w.remove(&a);
        w.remove(&b);
    }
}

/// The summoned chests that changed size, with their new size, for special::stage.
pub fn retargets(w: &BM<u64, Want>, m: &Moves) -> Vec<(u64, crate::progressive::Size)> {
    use crate::progressive::Size;
    let mut out: Vec<(u64, Size)> = m
        .chests
        .iter()
        .filter(|h| crate::special::sized(**h).is_some())
        .filter_map(|h| w.get(h).map(|x| (*h, if x.big { Size::Big } else { Size::Small })))
        .collect();
    let [a, b] = crate::special::DAMPE_RUPEES;
    if out.iter().any(|x| x.0 == a) != out.iter().any(|x| x.0 == b) {
        out.retain(|x| x.0 != a && x.0 != b);
    }
    out
}

/// Boss heart spots whose new item is minor. Their heart container turns into a small chest
/// instead of the big one; the heart itself (kept vanilla) stays a heart.
pub fn small_hearts(d: &Data, seed: &SeedFile, ctx: &Ctx, maps: bool) -> BS<u64> {
    let mut out: BS<u64> = BS::new();
    for (loc, item) in &seed.placements {
        let (Some(l), Some(it)) = (d.cat.by_name(loc), d.item(item)) else {
            continue;
        };
        if l.kind == "bossheart" && !major(it, &at(l), ctx) && shrinks(it, maps) {
            out.extend(l.refs.iter().map(|r| r.hash));
        }
    }
    out
}

pub fn shrinks(i: &Item, maps: bool) -> bool {
    maps || !matches!(i.kind.as_str(), "Map" | "Compass")
}

pub fn area(big: bool) -> &'static str {
    AREAS[usize::from(big)]
}

pub fn stem(file: &str) -> String {
    for s in ["_Static.bcett.byml.zs", "_Dynamic.bcett.byml.zs"] {
        if let Some(x) = file.strip_suffix(s) {
            return x.to_string();
        }
    }
    file.to_string()
}

fn triple(n: Option<&Node>) -> Option<[f32; 3]> {
    match n?.as_arr()? {
        [Node::F32(x), Node::F32(y), Node::F32(z)] => Some([*x, *y, *z]),
        _ => None,
    }
}

fn bits(p: &[f32; 3]) -> [u32; 3] {
    [p[0].to_bits(), p[1].to_bits(), p[2].to_bits()]
}

pub fn seen(y: &Byml) -> Vec<Seen> {
    let Some(list) = y.root.get("Actors").and_then(|a| a.as_arr()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|a| {
            Some(Seen {
                hash: hash_of(a).unwrap_or(0),
                gyaml: a.get("Gyaml").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                at: triple(a.get("Translate"))?,
                rot: triple(a.get("Rotate")).unwrap_or([0.0; 3]),
                story: a.get("Dynamic").and_then(|d| d.get("GetEventName")).is_some(),
            })
        })
        .collect()
}

pub fn tilted(rot: &[f32; 3]) -> bool {
    [rot[0], rot[2]].iter().any(|x| {
        let m: f32 = x.abs() % std::f32::consts::PI;
        m >= TILT && std::f32::consts::PI - m >= TILT
    })
}

/// Half extents (local X, local Z) of a chest's body, from the game's
/// Obj_TBox_A_01__Body_0 / Obj_TBox_A_02__Body_0 shapes. (The DropKeepAwayArea boxes are
/// padded and would call Mido's chests, 1.5 m apart, too tight.)
pub const SMALL_HALF: [f32; 2] = [0.364, 0.283];
pub const BIG_HALF: [f32; 2] = [0.64, 0.5065];

/// Clearance kept between two chest footprints.
pub const GAP: f32 = 0.05;

pub fn half(big: bool) -> [f32; 2] {
    if big { BIG_HALF } else { SMALL_HALF }
}

/// The chest's local X and Z axes on the ground plane (rotation applied as Rz * Ry * Rx; a
/// chest that is not tilted only has X and Z at 0 or pi, where every Euler order agrees).
pub fn axes(rot: &[f32; 3]) -> [[f32; 2]; 2] {
    let (sx, cx) = rot[0].sin_cos();
    let (sy, cy) = rot[1].sin_cos();
    let (sz, cz) = rot[2].sin_cos();
    // Columns of Rz * Ry * Rx for the local X and Z axes, keeping the world X and Z parts.
    let x: [f32; 2] = [cz * cy, -sy];
    let z: [f32; 2] = [cz * sy * cx + sz * sx, cy * cx];
    [x, z]
}

/// Whether two rotated footprints overlap on the ground plane (separating axis test).
pub fn overlap(a: &Seen, ah: [f32; 2], b: &Seen, bh: [f32; 2], gap: f32) -> bool {
    let (aa, ba) = (axes(&a.rot), axes(&b.rot));
    let d: [f32; 2] = [b.at[0] - a.at[0], b.at[2] - a.at[2]];
    let dot = |u: [f32; 2], v: [f32; 2]| u[0] * v[0] + u[1] * v[1];
    let reach = |ax: &[[f32; 2]; 2], h: [f32; 2], n: [f32; 2]| h[0] * dot(ax[0], n).abs() + h[1] * dot(ax[1], n).abs();
    for n in aa.iter().chain(ba.iter()) {
        let len: f32 = dot(*n, *n).sqrt();
        if len < 1e-6 {
            continue;
        }
        let n: [f32; 2] = [n[0] / len, n[1] / len];
        if dot(d, n).abs() >= reach(&aa, ah, n) + reach(&ba, bh, n) + gap {
            return false;
        }
    }
    true
}

/// Whether `o` stands where `chest` would grow. Another chest blocks only when the two
/// footprints overlap (`o_big`: the size that chest ends up with); anything else that is not
/// a marker blocks inside ROOM.
pub fn blocks(chest: &Seen, o: &Seen, o_big: bool) -> bool {
    if o.hash == chest.hash {
        return false;
    }
    let d: f32 = chest.at.iter().zip(o.at.iter()).map(|(a, b)| (a - b) * (a - b)).sum::<f32>().sqrt();
    let g: String = o.gyaml.to_ascii_lowercase();
    if g.starts_with("obj_tbox") {
        return !g.contains("pedestal") && d > 0.05 && (chest.at[1] - o.at[1]).abs() < ROOM && overlap(chest, BIG_HALF, o, half(o_big), GAP);
    }
    if d >= ROOM {
        return false;
    }
    !(MARKS.iter().any(|m| g.contains(m)) || g.ends_with("pos") || WALLS.iter().any(|w| g.starts_with(w)))
}

/// The size a neighbouring chest ends up with: what the build wants, else its model.
pub fn ends_big(o: &Seen, w: &BM<u64, Want>) -> bool {
    w.get(&o.hash).map(|x| x.big).unwrap_or_else(|| crate::progressive::Size::of_gyaml(&o.gyaml) == crate::progressive::Size::Big)
}

pub fn room(chest: &Seen, near: &[Seen], w: &BM<u64, Want>) -> bool {
    !tilted(&chest.rot) && !near.iter().any(|o| blocks(chest, o, ends_big(o, w)))
}

pub fn moves(scenes: &BM<String, Vec<Seen>>, w: &BM<u64, Want>) -> Moves {
    let mut m: Moves = Moves::default();
    let mut spots: Vec<(u64, String, [u32; 3], bool)> = Vec::new();
    for (st, list) in scenes {
        for s in list {
            let Some(want) = w.get(&s.hash) else {
                continue;
            };
            let Some(t) = target(&s.gyaml, want.big) else {
                continue;
            };
            if s.gyaml == t || (s.story && !want.strip) {
                continue;
            }
            if want.big && !room(s, list, w) {
                m.tight.insert(s.hash);
                continue;
            }
            spots.push((s.hash, st.clone(), bits(&s.at), want.big));
        }
    }
    let [ra, rb] = crate::special::DAMPE_RUPEES;
    let has = |h: u64| spots.iter().any(|s| s.0 == h);
    if has(ra) != has(rb) {
        spots.retain(|s| s.0 != ra && s.0 != rb);
    }
    for (h, st, at, big) in spots {
        if m.tight.contains(&h) {
            continue;
        }
        m.chests.insert(h);
        m.areas.insert((st, at), area(big));
    }
    m
}

pub fn scan(rf: &mut Romfs, w: &BM<u64, Want>) -> Res<BM<String, Vec<Seen>>> {
    let mut all: BM<String, Vec<Seen>> = BM::new();
    let mut keep: BS<String> = BS::new();
    for f in banc_files(rf)? {
        let y: Byml = Byml::parse(&rf.read_zs(&f)?)?;
        let st: String = stem(&f);
        let v: Vec<Seen> = seen(&y);
        if v.iter().any(|s| w.contains_key(&s.hash)) {
            keep.insert(st.clone());
        }
        all.entry(st).or_default().extend(v);
    }
    all.retain(|k, _| keep.contains(k));
    Ok(all)
}

pub fn target(gyaml: &str, big: bool) -> Option<&'static str> {
    PAIRS.iter().find(|(s, b)| gyaml == *s || gyaml == *b).map(|(s, b)| if big { *b } else { *s })
}

pub fn apply(y: &mut Byml, st: &str, w: &BM<u64, Want>, m: &Moves) -> (usize, usize) {
    let Some(list) = y.root.get_mut("Actors").and_then(|a| a.as_arr_mut()) else {
        return (0, 0);
    };
    let mut n: usize = 0;
    let mut k: usize = 0;
    for a in list.iter_mut() {
        let g: &str = a.get("Gyaml").and_then(|v| v.as_str()).unwrap_or("");
        if AREAS.contains(&g) {
            let Some(t) = triple(a.get("Translate")).and_then(|p| m.areas.get(&(st.to_string(), bits(&p)))) else {
                continue;
            };
            if g != *t {
                a.set("Gyaml", Node::Str(t.to_string()));
                k += 1;
            }
            continue;
        }
        let Some(want) = hash_of(a).filter(|h| m.chests.contains(h)).and_then(|h| w.get(&h)) else {
            continue;
        };
        let Some(t) = target(g, want.big) else {
            continue;
        };
        if g == t {
            continue;
        }
        let story: bool = a.get("Dynamic").and_then(|d| d.get("GetEventName")).is_some();
        if story && !want.strip {
            continue;
        }
        a.set("Gyaml", Node::Str(t.to_string()));
        n += 1;
    }
    (n, k)
}

pub fn resize(raw: &[u8], file: &str, w: &BM<u64, Want>, m: &Moves) -> Res<Option<(Vec<u8>, usize)>> {
    if w.is_empty() || m.chests.is_empty() {
        return Ok(None);
    }
    let mut y: Byml = Byml::parse(raw)?;
    let (n, k): (usize, usize) = apply(&mut y, &stem(file), w, m);
    if n + k == 0 {
        return Ok(None);
    }
    Ok(Some((y.write()?, n)))
}

pub fn report(n: usize, tight: usize) -> String {
    format!("{n} chests changed size to match their contents. {tight} chests stayed small because a big chest would not fit at their spot.")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str, kind: &str, adv: bool) -> Item {
        Item {
            name: name.to_string(),
            kind: kind.to_string(),
            adv,
            actor: None,
            conf: String::new(),
            dungeon: None,
            tiers: Vec::new(),
            world: Vec::new(),
        }
    }

    #[test]
    fn majors_are_progression_rewards_boss_keys_and_unowned_gear() {
        let c: Ctx = Ctx::default();
        let x: &str = "KF Midos Top Left Chest|Kokiri Forest";
        assert!(major(&item("Progressive Hookshot", "Item", true), x, &c));
        assert!(major(&item("Boss Key (Forest Temple)", "BossKey", false), "Forest Temple Map Chest|Forest Temple", &c));
        assert!(major(&item("Zeldas Lullaby", "Song", true), x, &c));
        assert!(major(&item("Forest Medallion", "DungeonReward", true), x, &c));
        assert!(major(&item("Mirror Shield", "Item", true), x, &c));
        assert!(!major(&item("Piece of Heart", "Item", true), x, &c));
        assert!(!major(&item("Heart Container", "Item", true), x, &c));
        assert!(!major(&item("Bombchus (10)", "Item", true), x, &c));
        assert!(major(&item("Bombchus (10)", "Item", true), x, &Ctx { chus: true, ..Ctx::default() }));
        assert!(!major(&item("Rupees (20)", "Item", false), x, &c));
        assert!(!major(&item("Map (Forest Temple)", "Map", false), x, &c));
        assert!(major(&item("Deku Shield", "Item", false), x, &c));
        assert!(!major(&item("Deku Shield", "Item", false), x, &Ctx { owned: vec!["Deku Shield"], ..Ctx::default() }));
        assert!(major(&item("Hylian Shield", "Item", false), x, &Ctx { owned: vec!["Deku Shield"], ..Ctx::default() }));
    }

    #[test]
    fn keys_are_big_only_away_from_their_dungeon() {
        let c: Ctx = Ctx::default();
        let key: Item = item("Small Key (Forest Temple)", "SmallKey", true);
        assert!(!major(&key, "Forest Temple First Room Chest|Forest Temple", &c));
        assert!(major(&key, "Fire Temple Map Chest|Fire Temple", &c));
        assert!(major(&key, "KF Midos Top Left Chest|Kokiri Forest", &c));
        let ring: Item = item("Small Key Ring (Ganons Castle)", "SmallKeyRing", true);
        assert!(!major(&ring, "Ganons Tower Boss Key Chest|Ganon's Tower", &c));
        assert!(!major(&ring, "Ganons Castle Light Trial Lullaby Room Invisible Chest|Ganon's Castle", &c));
        let silver: Item = item("Silver Rupee (Dodongos Cavern Staircase)", "SilverRupee", true);
        assert!(!major(&silver, "Dodongos Cavern Map Chest|Dodongo's Cavern", &c));
        assert!(major(&silver, "Deku Tree Map Chest|Deku Tree", &c));
        assert!(!major(&item("Small Key (Treasure Chest Game)", "TCGSmallKey", true), "Market Treasure Chest Game Room 1 Top|Market", &c));
        assert!(!major(&item("Small Key (Thieves Hideout)", "HideoutSmallKey", true), "Hideout 1 Torch Jail Gerudo Key|Thieves' Hideout", &c));
        assert!(major(&item("Boss Key (Fire Temple)", "BossKey", true), "Fire Temple Map Chest|Fire Temple", &c));
    }

    fn chest(hash: u64, gyaml: &str, story: bool) -> Node {
        let mut dy: Vec<(String, Node)> = vec![("Drop__DropActor".to_string(), Node::Str("X".to_string()))];
        if story {
            dy.push(("GetEventName".to_string(), Node::Str("Ev".to_string())));
        }
        Node::Map(vec![
            ("Dynamic".to_string(), Node::Map(dy)),
            ("Gyaml".to_string(), Node::Str(gyaml.to_string())),
            ("Hash".to_string(), Node::U64(hash)),
        ])
    }

    fn gyamls(y: &Byml) -> Vec<String> {
        y.root.get("Actors").unwrap().as_arr().unwrap().iter().map(|a| a.get("Gyaml").unwrap().as_str().unwrap().to_string()).collect()
    }

    #[test]
    fn plain_chests_take_the_wanted_size_and_others_stay() {
        let mut y: Byml = Byml {
            root: Node::Map(vec![(
                "Actors".to_string(),
                Node::Arr(vec![
                    chest(1, "Obj_TBox_A_01", false),
                    chest(2, "Obj_TBox_A_02_NoBake", false),
                    chest(3, "Obj_TBox_A_06", false),
                    chest(4, "Obj_TBox_A_01", true),
                    chest(5, "Obj_TBox_A_01", true),
                    chest(6, "Obj_TBox_A_02", false),
                    chest(7, "Obj_TBox_A_01", false),
                ]),
            )]),
            version: 7,
            big: false,
        };
        let big = Want { big: true, strip: false };
        let w: BM<u64, Want> = BM::from([
            (1, big),
            (2, Want { big: false, strip: false }),
            (3, big),
            (4, big),
            (5, Want { big: true, strip: true }),
            (6, big),
        ]);
        let all: Moves = Moves { chests: w.keys().copied().collect(), ..Moves::default() };
        assert_eq!(apply(&mut y, "s", &w, &all), (3, 0));
        assert_eq!(
            gyamls(&y),
            vec!["Obj_TBox_A_02", "Obj_TBox_A_01_NoBake", "Obj_TBox_A_06", "Obj_TBox_A_01", "Obj_TBox_A_02", "Obj_TBox_A_02", "Obj_TBox_A_01"]
        );
        let raw: Vec<u8> = y.write().unwrap();
        assert!(resize(&raw, "s", &w, &all).unwrap().is_none());
        assert!(resize(&raw, "s", &BM::new(), &all).unwrap().is_none());
        let one: BM<u64, Want> = BM::from([(1, Want { big: false, strip: false })]);
        let (back, n): (Vec<u8>, usize) = resize(&raw, "s", &one, &Moves { chests: BS::from([1]), ..Moves::default() }).unwrap().unwrap();
        assert_eq!(n, 1);
        assert_eq!(gyamls(&Byml::parse(&back).unwrap())[0], "Obj_TBox_A_01");
        assert!(resize(&raw, "s", &one, &Moves::default()).unwrap().is_none());
    }

    #[test]
    fn only_the_plain_pairs_change_and_each_size_has_its_area() {
        assert_eq!(target("Obj_TBox_A_01", true), Some("Obj_TBox_A_02"));
        assert_eq!(target("Obj_TBox_A_02_NoBake", false), Some("Obj_TBox_A_01_NoBake"));
        assert_eq!(target("Obj_TBox_A_01_NoBake", true), Some("Obj_TBox_A_02_NoBake"));
        for g in ["Obj_TBox_A_03", "Obj_TBox_A_03_NoBake", "Obj_TBox_A_04", "Obj_TBox_A_06", "Obj_TBox_A_06_NoBake", "Obj_TBox_GlassTruth_A_01", "Obj_TBox_A_01_memo11", "Obj_HrlCity_Shop_TBox_A_02"] {
            assert_eq!(target(g, true), None, "{g}");
            assert_eq!(target(g, false), None, "{g}");
        }
        for (s, b) in PAIRS {
            assert_eq!(s.len(), b.len());
        }
        assert_eq!((area(false), area(true)), ("DropKeepAwayArea_TBox_A_01", "DropKeepAwayArea_TBox_A_02"));
        assert_eq!(AREAS[0].len(), AREAS[1].len());
    }

    #[test]
    fn maps_and_compasses_only_shrink_when_the_build_credits_them() {
        let m: Item = item("Map (Deku Tree)", "Map", false);
        let c: Item = item("Compass (Deku Tree)", "Compass", false);
        let r: Item = item("Rupees (20)", "Item", false);
        assert!(!shrinks(&m, false) && !shrinks(&c, false));
        assert!(shrinks(&m, true) && shrinks(&c, true));
        assert!(shrinks(&r, false));
    }

    #[test]
    fn scene_stems_pair_the_static_and_dynamic_files() {
        assert_eq!(stem("Banc/GerudoValley/GerudoValley_Adult_Static.bcett.byml.zs"), "Banc/GerudoValley/GerudoValley_Adult");
        assert_eq!(stem("Banc/GerudoValley/GerudoValley_Adult_Dynamic.bcett.byml.zs"), "Banc/GerudoValley/GerudoValley_Adult");
        assert_ne!(stem("Banc/GerudoValley/GerudoValley_Child_Dynamic.bcett.byml.zs"), "Banc/GerudoValley/GerudoValley_Adult");
        assert_eq!(stem("Banc/X/Other.byml"), "Banc/X/Other.byml");
    }

    fn at(hash: u64, gyaml: &str, x: f32) -> Seen {
        Seen { hash, gyaml: gyaml.to_string(), at: [x, 10.0, 0.0], rot: [0.0; 3], story: false }
    }

    #[test]
    fn footprints_overlap_by_their_rotated_boxes() {
        let a: Seen = at(1, "Obj_TBox_A_02", 0.0);
        let mut b: Seen = at(2, "Obj_TBox_A_02", 1.5);
        // Side by side along X: 0.64 + 0.64 + gap fits in 1.5.
        assert!(!overlap(&a, BIG_HALF, &b, BIG_HALF, GAP));
        // Turned a quarter, the neighbour's depth faces X: still fits.
        b.rot = [0.0, std::f32::consts::FRAC_PI_2, 0.0];
        assert!(!overlap(&a, BIG_HALF, &b, BIG_HALF, GAP));
        // Turned 45 degrees, the corners reach 0.81 m along X each: they touch.
        let mut c: Seen = at(3, "Obj_TBox_A_02", 1.4);
        c.rot = [0.0, std::f32::consts::FRAC_PI_4, 0.0];
        let mut d: Seen = a.clone();
        d.rot = c.rot;
        assert!(overlap(&d, BIG_HALF, &c, BIG_HALF, GAP));
        // Flipped X and Z read as a turn about Y, not as a tilt.
        let mut e: Seen = a.clone();
        e.rot = [std::f32::consts::PI, 0.0, std::f32::consts::PI];
        let ax = axes(&e.rot);
        assert!((ax[0][0].abs() - 1.0).abs() < 1e-5 && (ax[1][1].abs() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn the_clearance_rule_skips_crowded_and_tilted_spots() {
        let none: BM<u64, Want> = BM::new();
        let c: Seen = at(1, "Obj_TBox_A_01", 0.0);
        assert!(room(&c, &[c.clone()], &none));
        for (g, x) in [("Obj_Pot_A_01", 1.0), ("Enemy_Beamos", 0.0), ("Obj_TBox_A_01", 1.0), ("Obj_TBox_A_02", 1.3), ("Obj_FireWall_A_01", 0.3)] {
            let o: Seen = at(9, g, x);
            assert!(blocks(&c, &o, ends_big(&o, &none)), "{g}");
            assert!(!room(&c, &[o], &none), "{g}");
        }
        for (g, x) in [
            ("Obj_Pot_A_01", 1.6),
            ("Land_FireTemple_02_Wall_A_03", 0.4),
            ("DropKeepAwayArea_TBox_A_01", 0.0),
            ("DestinationAnchor", 0.0),
            ("Obj_TBoxPedestal_A_01", 0.0),
            ("PushAirWall_Box", 0.2),
            ("StartPos", 0.5),
            ("Obj_TBox_A_02", 0.0),
            ("Obj_TBox_A_01", 1.2),
        ] {
            let o: Seen = at(9, g, x);
            assert!(!blocks(&c, &o, ends_big(&o, &none)), "{g}");
        }
        // A neighbour that grows too needs the room of two big chests.
        let o: Seen = at(9, "Obj_TBox_A_01", 1.2);
        assert!(blocks(&c, &o, ends_big(&o, &BM::from([(9, Want { big: true, strip: false })]))));
        let mut t: Seen = c.clone();
        t.rot = [0.3, 0.0, 0.0];
        assert!(tilted(&t.rot) && !room(&t, &[], &none));
        t.rot = [std::f32::consts::PI, 1.2, -std::f32::consts::PI];
        assert!(!tilted(&t.rot) && room(&t, &[], &none));
        t.rot = [0.0, 2.0, 0.2];
        assert!(tilted(&t.rot));
    }

    #[test]
    fn moves_skip_tight_grows_keep_shrinks_and_pair_each_chest_with_its_area() {
        let grow = Want { big: true, strip: false };
        let shrink = Want { big: false, strip: false };
        let pot: Seen = at(50, "Obj_Pot_A_01", 20.8);
        let mut story: Seen = at(4, "Obj_TBox_A_01", 40.0);
        story.story = true;
        let scenes: BM<String, Vec<Seen>> = BM::from([
            ("a".to_string(), vec![at(1, "Obj_TBox_A_01", 0.0), at(2, "Obj_TBox_A_01", 20.0), pot.clone(), at(3, "Obj_TBox_A_02", 21.5), story, at(5, "Obj_TBox_A_01", 60.0)]),
            ("b".to_string(), vec![at(5, "Obj_TBox_A_01", 60.0), at(51, "Enemy_Beamos", 60.5), at(6, "Obj_TBox_A_06", 80.0)]),
        ]);
        let w: BM<u64, Want> = BM::from([(1, grow), (2, grow), (3, shrink), (4, grow), (5, grow), (6, grow)]);
        let m: Moves = moves(&scenes, &w);
        assert_eq!(m.chests, BS::from([1, 3]));
        assert_eq!(m.tight, BS::from([2, 5]));
        assert_eq!(m.areas.len(), 2);
        assert_eq!(m.areas.get(&("a".to_string(), bits(&[0.0, 10.0, 0.0]))), Some(&"DropKeepAwayArea_TBox_A_02"));
        assert_eq!(m.areas.get(&("a".to_string(), bits(&[21.5, 10.0, 0.0]))), Some(&"DropKeepAwayArea_TBox_A_01"));
        let strip: BM<u64, Want> = BM::from([(4, Want { big: true, strip: true })]);
        assert_eq!(moves(&scenes, &strip).chests, BS::from([4]));
    }

    fn placed(gyaml: &str, x: f32) -> Node {
        Node::Map(vec![
            ("Gyaml".to_string(), Node::Str(gyaml.to_string())),
            ("Hash".to_string(), Node::U64(1000 + x as u64)),
            ("Translate".to_string(), Node::Arr(vec![Node::F32(x), Node::F32(10.0), Node::F32(0.0)])),
        ])
    }

    fn file(nodes: Vec<Node>) -> Vec<u8> {
        Byml { root: Node::Map(vec![("Actors".to_string(), Node::Arr(nodes))]), version: 7, big: false }.write().unwrap()
    }

    #[test]
    fn the_area_follows_its_chest_into_the_other_file_of_the_scene() {
        let mut c: Node = chest(1, "Obj_TBox_A_01", false);
        c.set("Translate", Node::Arr(vec![Node::F32(0.0), Node::F32(10.0), Node::F32(0.0)]));
        let fs: &str = "Banc/S/S_Adult_Static.bcett.byml.zs";
        let fd: &str = "Banc/S/S_Adult_Dynamic.bcett.byml.zs";
        let fo: &str = "Banc/S/S_Child_Dynamic.bcett.byml.zs";
        let st: Vec<u8> = file(vec![c]);
        let dy: Vec<u8> = file(vec![placed("DropKeepAwayArea_TBox_A_01", 0.0), placed("DropKeepAwayArea_TBox_A_01", 5.0), placed("DropKeepAwayArea", 0.0)]);
        let scenes: BM<String, Vec<Seen>> = BM::from([(stem(fs), seen(&Byml::parse(&st).unwrap()).into_iter().chain(seen(&Byml::parse(&dy).unwrap())).collect())]);
        let w: BM<u64, Want> = BM::from([(1, Want { big: true, strip: false })]);
        let m: Moves = moves(&scenes, &w);
        assert_eq!(m.chests, BS::from([1]));
        let (b, n): (Vec<u8>, usize) = resize(&st, fs, &w, &m).unwrap().unwrap();
        assert_eq!((n, gyamls(&Byml::parse(&b).unwrap())), (1, vec!["Obj_TBox_A_02".to_string()]));
        let (b, n): (Vec<u8>, usize) = resize(&dy, fd, &w, &m).unwrap().unwrap();
        assert_eq!(n, 0);
        assert_eq!(gyamls(&Byml::parse(&b).unwrap()), vec!["DropKeepAwayArea_TBox_A_02", "DropKeepAwayArea_TBox_A_01", "DropKeepAwayArea"]);
        assert!(resize(&dy, fo, &w, &m).unwrap().is_none());
    }

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = ootrr_game::test_romfs::path();
        if p.join("Pack").join("ZsDic.pack.zs").is_file() { Some(p) } else { None }
    }

    #[test]
    fn every_summoned_chest_retargets_in_the_real_flows() {
        use crate::progressive::Size;
        let Some(p) = dump() else {
            eprintln!("skipped, no RomFS dump");
            return;
        };
        let mut rf: Romfs = Romfs::open(&p).unwrap();
        let all: Vec<&crate::special::Spot> = crate::special::SPOTS.iter().chain(crate::special::MORE.iter()).collect();
        let flip: Vec<(u64, Size)> = all.iter().map(|s| (s.hash, if crate::special::base_size(s) == Size::Big { Size::Small } else { Size::Big })).collect();
        let (outs, kids) = crate::special::stage(&mut rf, &[], &flip, &[]).unwrap();
        assert_eq!(outs.len(), 5);
        assert_eq!(kids.len(), 6, "the Dampe chests share one parent row per open event");
        let d: Data = Data::load().unwrap();
        for s in &all {
            assert_eq!(d.event_mode(s.hash), Some(Ev::Keep), "{}", s.hash);
        }
    }

    #[test]
    fn every_catalog_chest_finds_one_area_in_the_real_files() {
        let Some(p) = dump() else {
            eprintln!("skipped, no RomFS dump");
            return;
        };
        let d: Data = Data::load().unwrap();
        let mut rf: Romfs = Romfs::open(&p).unwrap();
        let mut w: BM<u64, Want> = BM::new();
        for l in d.cat.locs.iter().filter(|l| l.kind == "chest") {
            for r in &l.refs {
                if crate::special::of(r.hash).is_none() && d.event_mode(r.hash) != Some(Ev::Keep) {
                    w.insert(r.hash, Want { big: true, strip: true });
                }
            }
        }
        let scenes: BM<String, Vec<Seen>> = scan(&mut rf, &w).unwrap();
        let grow: Moves = moves(&scenes, &w);
        let flip: BM<u64, Want> = w.keys().map(|h| (*h, Want { big: false, strip: true })).collect();
        let shrink: Moves = moves(&scenes, &flip);
        let mut chests: usize = 0;
        let mut areas: usize = 0;
        for f in banc_files(&rf).unwrap() {
            let raw: Vec<u8> = rf.read_zs(&f).unwrap();
            for (ww, m) in [(&w, &grow), (&flip, &shrink)] {
                let mut y: Byml = Byml::parse(&raw).unwrap();
                let (n, k): (usize, usize) = apply(&mut y, &stem(&f), ww, m);
                chests += n;
                areas += k;
            }
        }
        let names = |s: &BS<u64>| -> BS<String> { d.cat.locs.iter().filter(|l| l.refs.iter().any(|r| s.contains(&r.hash))).map(|l| l.name.clone()).collect() };
        eprintln!("grown {} shrunk {} tight {}", grow.chests.len(), shrink.chests.len(), grow.tight.len());
        for n in names(&grow.tight) {
            eprintln!("tight: {n}");
        }
        // Mido's four chests stand 1.5 m apart: all four fit as big chests.
        assert!(!names(&grow.tight).iter().any(|n| n.starts_with("KF Midos")), "{:?}", names(&grow.tight));
        assert_eq!(chests, areas);
        assert_eq!(grow.areas.len() + shrink.areas.len(), areas);
        assert!(!grow.chests.is_empty() && !shrink.chests.is_empty() && !grow.tight.is_empty());
    }
}
