use std::collections::{BTreeMap as BM, BTreeSet as BS, HashSet as HS};

use crate::bfevfl::flow::NONE;
use crate::bfevfl::{Bf, Case, Cont, Entry, Flow, Kind as Ev, Val};
use crate::byml::{Byml, Node};
use crate::dungeon_credit::{action_slot, idx, push, set_bool, sound, talk, DROP, HOLDER, SET, TALK, TALKER};
use crate::dungeon_grant::{merge, OURS, ROOT_ENTRY};
use crate::dungeon_text::{FILE, FILE_ID};
use crate::error::{Error as E, Res};
use crate::hints::languages;
use crate::logic_items::ItemId;
use crate::msbt::{self, Msbt};
use crate::placement::hash_of;
use crate::progressive::{add_rows, Size, ANALYZED_TABLE, BIG_FLOW, SETTING_TABLE, SMALL_FLOW};
use crate::romfs::Romfs;
use crate::sarc;
use crate::settings::seed::SeedFile;
use crate::settings::Settings;
use crate::slice::data::Data;
use crate::slice::edit::{Edited, Target};
use crate::slice::support::{Mode, Plan};
use crate::staging::Out;
use crate::world::{LocId, World};

pub const SETTING: &str = "shuffle_silver_rupees";
pub const TOKEN: &str = "Obj_RupeeToken_Silver";
pub const CONTENT: &str = "TBox_Rupee_Green";
pub const ASK: &str = "EventQueryIsGameDataBool";
pub const ENTRY: &str = "Success_Silver_";
pub const NOTE: &str = "Silver rupees are shuffled in the Forest and Light Trials of Ganon's Castle, the Ice Cavern push block room, the Bottom of the Well basement and the Gerudo Training Ground slopes room. Each room has five Silver Rupee items, found only in chests. Each one gives one rupee and counts on a spare saved flag, and the fifth sets the flag that opens the room. The spots where these silver rupees hung hold rupees. None of this is tested in the game yet.";
pub const NEEDS_CHESTS: &str = "shuffle_silver_rupees needs shuffle_chests on, because the 25 Silver Rupee items can only be found in chests and the other chests are already full of items that have nowhere else to go";
pub const LEFT: &str = "silver rupee spot is not in the logic world";

#[derive(Debug, PartialEq, Eq)]
pub struct Room {
    pub tag: &'static str,
    pub item: &'static str,
    pub text: &'static str,
    pub files: &'static [&'static str],
    pub spots: [(&'static str, &'static [u64]); 5],
    pub solved: &'static [&'static str],
    pub spare: [&'static str; 4],
}

pub const ROOMS: [Room; 5] = [
    Room {
        tag: "Forest",
        item: "Silver Rupee (Ganons Castle Forest Trial)",
        text: "You found a Silver Rupee for the Forest Trial in Ganon's Castle.",
        files: &["Banc/InsideGanonCastle/InsideGanonCastle_Static.bcett.byml.zs"],
        spots: [
            ("Ganons Castle Forest Trial Silver Rupee Front Right", &[13035701009514315398]),
            ("Ganons Castle Forest Trial Silver Rupee Front Left", &[7597206279643935976]),
            ("Ganons Castle Forest Trial Silver Rupee Back Right", &[11815666052437125601]),
            ("Ganons Castle Forest Trial Silver Rupee Back Center", &[10052148868361476941]),
            ("Ganons Castle Forest Trial Silver Rupee Center Left", &[8097387562381544300]),
        ],
        solved: &["InsideGanonCastle_Dungeon_Room03_SilverRupee"],
        spare: ["HavePlayedEvent.Dm_StaffCredit_0021", "HavePlayedEvent.Test_Npc_Hylia_063_D_N", "HavePlayedEvent.Test_Npc_Zora_Ruto_Adult_003_D_T", "HavePlayedEvent.Test_Npc_Zora_Ruto_Adult_003_D_N"],
    },
    Room {
        tag: "Light",
        item: "Silver Rupee (Ganons Castle Light Trial)",
        text: "You found a Silver Rupee for the Light Trial in Ganon's Castle.",
        files: &["Banc/InsideGanonCastle/InsideGanonCastle_Static.bcett.byml.zs"],
        spots: [
            ("Ganons Castle Light Trial Silver Rupee Right Alcove", &[12211581836782090940]),
            ("Ganons Castle Light Trial Silver Rupee Left Alcove", &[8437066945828664269]),
            ("Ganons Castle Light Trial Silver Rupee Center Top", &[12932474451250284286]),
            ("Ganons Castle Light Trial Silver Rupee Center Right", &[7766186285350213443]),
            ("Ganons Castle Light Trial Silver Rupee Center Left", &[11173621843222054502]),
        ],
        solved: &["InsideGanonCastle_Dungeon_Room14_SilverRupee"],
        spare: ["HavePlayedEvent.Test_Npc_Hylia_049_Adult_D_T", "HavePlayedEvent.Test_DofCheck", "HavePlayedEvent.Test_DofCheck2", "HavePlayedEvent.Test_GhostPicture"],
    },
    Room {
        tag: "IceBlock",
        item: "Silver Rupee (Ice Cavern Push Block)",
        text: "You found a Silver Rupee for the push block room in the Ice Cavern.",
        files: &["Banc/IceCavern/IceCavern_Static.bcett.byml.zs"],
        spots: [
            ("Ice Cavern Push Block Silver Rupee Back Left", &[7358957810395899450]),
            ("Ice Cavern Push Block Silver Rupee Back Center", &[7241953878866093533]),
            ("Ice Cavern Push Block Silver Rupee Front Center", &[10169213272312887158]),
            ("Ice Cavern Push Block Silver Rupee Red Ice", &[6554307454311215168]),
            ("Ice Cavern Push Block Silver Rupee Front Left", &[6874997892371023062]),
        ],
        solved: &["IceCavern_Room06_SilverRupeeGet", "IceCavern_Room06_Unlocked"],
        spare: ["HavePlayedEvent.Test_Weather", "HavePlayedEvent.Test_PlayOcarinaSequence", "HavePlayedEvent.Test_PhantomGanonEvent", "HavePlayedEvent.Test_Dm_FinalBattle_0000"],
    },
    Room {
        tag: "Well",
        item: "Silver Rupee (Bottom of the Well Basement)",
        text: "You found a Silver Rupee for the basement of the Bottom of the Well.",
        files: &["Banc/BottomOfTheWell/BottomOfTheWell_Static.bcett.byml.zs"],
        spots: [
            ("Bottom of the Well Basement Silver Rupee Wood Beam Front", &[7533299169670713716]),
            ("Bottom of the Well Basement Silver Rupee Wood Beam Back", &[14387941463161581636]),
            ("Bottom of the Well Basement Silver Rupee Ladders Bottom", &[15275262799417877871]),
            ("Bottom of the Well Basement Silver Rupee Ladders Middle", &[9102498605737057290]),
            ("Bottom of the Well Basement Silver Rupee Ladders Top", &[15745063167016692451]),
        ],
        solved: &["BottomOfTheWell_Level_Room_09_A_SilverRupee_GetAll", "BottomOfTheWell_Level_Room_09_A_Clear"],
        spare: ["HavePlayedEvent.Test_HidingHoleCoverDisappearStrom_Disappear", "HavePlayedEvent.CuccoTest", "HavePlayedEvent.EventAddTest", "HavePlayedEvent.EventWarpTest"],
    },
    Room {
        tag: "GtgSlopes",
        item: "Silver Rupee (Gerudo Training Ground Slopes)",
        text: "You found a Silver Rupee for the slopes room in the Gerudo Training Ground.",
        files: &["Banc/GerudoTrainingGround/GerudoTrainingGround_Static.bcett.byml.zs"],
        spots: [
            ("Gerudo Training Ground Boulder Room Silver Rupee Bottom Right", &[15349616442720873517, 5383014084010738718]),
            ("Gerudo Training Ground Boulder Room Silver Rupee Bottom Left", &[10908634349331605464, 2136711048273783826]),
            ("Gerudo Training Ground Boulder Room Silver Rupee Ledge", &[14408488134523816378, 1041487935715411607]),
            ("Gerudo Training Ground Boulder Room Silver Rupee Top Left", &[13198507528048466310, 8645220969166832948]),
            ("Gerudo Training Ground Boulder Room Silver Rupee Ceiling", &[9877205116342061532, 16184272864443992469]),
        ],
        solved: &["GerudoTrainingGround_Dungeon_Room02_SilverRupee"],
        spare: ["HavePlayedEvent.FlowchartTest", "HavePlayedEvent.MovieTest", "HavePlayedEvent.WellEventAdd", "HavePlayedEvent.Test_Npc_Hylia_063_D_T"],
    },
];

pub fn mode(s: &Settings, chests: Mode, bad: &mut Vec<String>) -> bool {
    let on: bool = s.flag(SETTING).unwrap_or_else(|e| {
        bad.push(e.to_string());
        false
    });
    if on && chests == Mode::Off {
        bad.push(NEEDS_CHESTS.to_string());
    }
    on
}

pub fn room(tag: &str) -> Option<&'static Room> {
    ROOMS.iter().find(|r| r.tag == tag)
}

pub fn room_of_item(item: &str) -> Option<&'static Room> {
    ROOMS.iter().find(|r| r.item == item)
}

pub fn spot(loc: &str) -> Option<(&'static Room, &'static [u64])> {
    ROOMS.iter().find_map(|r| r.spots.iter().find(|s| s.0 == loc).map(|s| (r, s.1)))
}

pub fn is_spot(loc: &str) -> bool {
    spot(loc).is_some()
}

pub fn is_item(item: &str) -> bool {
    room_of_item(item).is_some()
}

pub fn junk(item: &str) -> Option<&'static str> {
    crate::slice::pickups::world_actor(item).filter(|a| a.starts_with("Obj_Rupee_"))
}

pub fn chest_ok(d: &Data, name: &str) -> bool {
    d.cat.by_name(name).is_some_and(|c| c.kind == "chest" && !c.refs.is_empty() && c.refs.iter().all(|r| !d.events.keep.contains(&r.hash) && crate::special::of(r.hash).is_none()))
        && d.flow.by_name(name).is_none()
        && !crate::slice::pickups::is_held(name)
        && !crate::shops::is_loc(name)
        && crate::slice::dungeon_items::chest_ok(d, name)
}

pub fn at(d: &Data, w: &World, p: &Plan, it: ItemId, l: LocId) -> Option<bool> {
    if !p.silver {
        return None;
    }
    let name: &str = &w.locs[l].name;
    let item: &str = &w.items.names[it];
    if is_spot(name) {
        return Some(junk(item).is_some());
    }
    if is_item(item) {
        return Some(chest_ok(d, name));
    }
    None
}

pub fn select(w: &World, p: &Plan, free: &mut Vec<LocId>, left: &mut BM<String, usize>) -> usize {
    if !p.silver {
        return 0;
    }
    let mut n: usize = 0;
    for r in &ROOMS {
        for (name, _) in &r.spots {
            n += 1;
            match w.loc(name) {
                Some(x) => free.push(x),
                None => *left.entry(LEFT.to_string()).or_insert(0) += 1,
            }
        }
    }
    n
}

pub fn delta(seed: &SeedFile, item: &str) -> isize {
    -(seed.placements.iter().filter(|(l, i)| is_spot(l) && i.as_str() == item).count() as isize)
}

pub fn targets(loc: &str, item: &str) -> Res<Option<Vec<Target>>> {
    let Some((r, hashes)) = spot(loc) else {
        return Ok(None);
    };
    let new: &str = junk(item).ok_or_else(|| E::Msg(format!("{item} cannot lie at the silver rupee spot {loc}, only rupees can")))?;
    Ok(Some(
        hashes
            .iter()
            .map(|h| Target {
                name: loc.to_string(),
                hash: *h,
                files: r.files.iter().map(|f| f.to_string()).collect(),
                old_item: r.item.to_string(),
                new_item: item.to_string(),
                old_actor: TOKEN.to_string(),
                new_actor: new.to_string(),
                ev: None,
                family: None,
                key: None,
                heart_chest: false,
            })
            .collect(),
    ))
}

pub fn grants(d: &Data, seed: &SeedFile) -> Res<BM<u64, &'static str>> {
    let mut out: BM<u64, &'static str> = BM::new();
    for (loc, item) in &seed.placements {
        let Some(r) = room_of_item(item) else {
            continue;
        };
        if !chest_ok(d, loc) {
            return Err(E::Msg(format!("{item} at {loc} is not in a chest this build can route, a Silver Rupee can only be found in a chest")));
        }
        for x in &d.cat.by_name(loc).ok_or_else(|| E::Msg(format!("{loc} is not in the catalog")))?.refs {
            out.insert(x.hash, r.tag);
        }
    }
    Ok(out)
}

pub fn entry_name(tag: &str) -> String {
    format!("{ENTRY}{tag}")
}

pub fn event_name(tag: &str, size: Size) -> String {
    let s: &str = match size {
        Size::Small => "Small",
        Size::Big => "Big",
    };
    format!("{OURS}Silver_{tag}_{s}")
}

pub fn label(tag: &str) -> String {
    format!("{OURS}Silver_{tag}")
}

pub fn message_id(tag: &str) -> String {
    format!("{FILE_ID}:{}", label(tag))
}

fn text<'a>(n: &'a Node, k: &str) -> &'a str {
    n.get(k).and_then(|v| v.as_str()).unwrap_or("")
}

pub fn mark(y: &mut Byml, g: &BM<u64, &'static str>, used: &mut BS<&'static str>, hit: &mut BS<u64>) -> Res<usize> {
    let Some(list) = y.root.get_mut("Actors").and_then(|a| a.as_arr_mut()) else {
        return Ok(0);
    };
    let mut n: usize = 0;
    for a in list.iter_mut() {
        let Some((h, tag)) = hash_of(a).and_then(|h| g.get(&h).map(|t| (h, *t))) else {
            continue;
        };
        let size: Size = Size::of_gyaml(text(a, "Gyaml"));
        let Some(dy) = a.get_mut("Dynamic").filter(|d| d.get(DROP).is_some()) else {
            continue;
        };
        dy.set(DROP, Node::Str(CONTENT.to_string()));
        dy.set("GetEventName", Node::Str(event_name(tag, size)));
        used.insert(tag);
        hit.insert(h);
        n += 1;
    }
    Ok(n)
}

pub fn apply(file: &str, raw: &[u8], prev: Option<Edited>, g: &BM<u64, &'static str>, used: &mut BS<&'static str>, hit: &mut BS<u64>) -> Res<Option<Edited>> {
    if g.is_empty() {
        return Ok(prev);
    }
    let mut y: Byml = match &prev {
        Some(e) => Byml::parse(&e.bytes)?,
        None => Byml::parse(raw)?,
    };
    if mark(&mut y, g, used, hit)? == 0 {
        return Ok(prev);
    }
    let bytes: Vec<u8> = y.write()?;
    Ok(Some(match prev {
        Some(mut e) => {
            e.bytes = bytes;
            e
        }
        None => Edited { file: file.to_string(), old_len: raw.len(), bytes, hits: Vec::new(), used: Vec::new(), hearts: Vec::new(), wraps: Vec::new(), touched: Vec::new() },
    }))
}

pub fn all_found(g: &BM<u64, &'static str>, hit: &BS<u64>) -> Res<()> {
    match g.keys().find(|h| !hit.contains(h)) {
        Some(h) => Err(E::Msg(format!("chest {h} that holds a Silver Rupee was not found in the scenes, the game files differ"))),
        None => Ok(()),
    }
}

fn ask(name: &str) -> Cont {
    Cont { items: vec![("Index".to_string(), Val::Int(-1)), ("GameDataName".to_string(), Val::Str(name.to_string()))] }
}

fn query_slot(f: &mut Flow, actor: u16) -> Res<u16> {
    let a = f.actors.get_mut(actor as usize).ok_or(E::Format("silver: the flag actor vanished"))?;
    let j: usize = match a.queries.iter().position(|q| q == ASK) {
        Some(j) => j,
        None => {
            a.queries.push(ASK.to_string());
            a.queries.len() - 1
        }
    };
    idx(j)
}

pub fn credit(f: &mut Flow, r: &Room) -> Res<()> {
    let name: String = entry_name(r.tag);
    if f.entries.iter().any(|e| e.name == name) {
        return Ok(());
    }
    let pad: u16 = f.entries.iter().find(|e| e.name == ROOT_ENTRY).map(|e| e.pad).ok_or(E::Format("silver: the chest flow has no Success_ entry"))?;
    let (a, set): (u16, u16) = action_slot(f, HOLDER, SET)?;
    let q: u16 = query_slot(f, a)?;
    let (t, say): (u16, u16) = action_slot(f, TALKER, TALK)?;
    let base: u16 = idx(f.events.len())?;
    let give: u16 = base;
    let first: u16 = base + 2;
    let solved: u16 = first + 2 * r.spare.len() as u16;
    let c: Cont = Cont { items: vec![("ContentName".to_string(), Val::Str(CONTENT.to_string())), ("IsMiniGame".to_string(), Val::Bool(0))] };
    push(f, Ev::Sub { next: give + 1, params: Some(c), flow: String::new(), entry: ROOT_ENTRY.to_string() })?;
    push(f, Ev::Action { next: NONE, actor: t, act: say, params: Some(talk(message_id(r.tag))) })?;
    for (i, s) in r.spare.iter().enumerate() {
        let here: u16 = first + 2 * i as u16;
        let next: u16 = if i + 1 < r.spare.len() { here + 2 } else { solved };
        push(f, Ev::Switch { actor: a, query: q, cases: vec![Case { val: 0, ev: next }, Case { val: 1, ev: here + 1 }], params: Some(ask(s)) })?;
        push(f, Ev::Action { next: give, actor: a, act: set, params: Some(set_bool(s.to_string())) })?;
    }
    for (j, s) in r.solved.iter().enumerate() {
        let next: u16 = if j + 1 < r.solved.len() { solved + j as u16 + 1 } else { give };
        push(f, Ev::Action { next, actor: a, act: set, params: Some(set_bool(s.to_string())) })?;
    }
    f.entries.push(Entry { name, subs: vec![give], vars: Vec::new(), main: first, pad });
    Ok(())
}

pub fn edit_flow(f: &mut Flow, used: &BS<&'static str>) -> Res<()> {
    for tag in used {
        let r: &Room = room(tag).ok_or_else(|| E::Msg(format!("{tag} is not a silver rupee room")))?;
        credit(f, r)?;
    }
    if let Some(m) = sound(f).first() {
        return Err(E::Msg(format!("silver: the chest flow is broken after the new entries, {m}")));
    }
    Ok(())
}

fn flow_file(raw: &[u8], used: &BS<&'static str>) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    let f: &mut Flow = b.flows.first_mut().ok_or(E::Format("silver: the flow file has no flow"))?;
    edit_flow(f, used)?;
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(E::Format("silver: the written flow does not parse back to the edited model"));
    }
    Ok(out)
}

pub fn stage(rf: &mut Romfs, outs: &mut Vec<Out>, used: &BS<&'static str>) -> Res<()> {
    if used.is_empty() {
        return Ok(());
    }
    for path in [BIG_FLOW, SMALL_FLOW] {
        merge(rf, outs, path, |raw: &[u8]| flow_file(raw, used))?;
    }
    for table in [SETTING_TABLE, ANALYZED_TABLE] {
        let analyzed: bool = table == ANALYZED_TABLE;
        merge(rf, outs, table, |raw: &[u8]| {
            let mut y: Byml = Byml::parse(raw)?;
            for size in [Size::Small, Size::Big] {
                let rows: Vec<(String, Option<String>)> = used.iter().map(|t| (event_name(t, size), if analyzed { None } else { Some(entry_name(t)) })).collect();
                add_rows(&mut y, size.template(), &rows)?;
            }
            y.write()
        })?;
    }
    Ok(())
}

pub fn text_file(raw: &[u8], used: &BS<&'static str>) -> Res<Vec<u8>> {
    let mut es: Vec<sarc::Entry> = sarc::parse(raw)?;
    let e: &sarc::Entry = sarc::get(&es, FILE).ok_or_else(|| E::Msg(format!("the language archive has no {FILE}")))?;
    let mut m: Msbt = msbt::parse(&e.data)?;
    for tag in used {
        let r: &Room = room(tag).ok_or_else(|| E::Msg(format!("{tag} is not a silver rupee room")))?;
        m.put_markup(&label(tag), r.text)?;
    }
    sarc::set(&mut es, FILE, m.write()?);
    sarc::write(&es)
}

pub fn stage_text(rf: &mut Romfs, more: &mut Vec<Out>, used: &BS<&'static str>) -> Res<()> {
    if used.is_empty() {
        return Ok(());
    }
    for l in languages(rf)? {
        merge(rf, more, &l, |raw: &[u8]| text_file(raw, used))?;
    }
    Ok(())
}

pub fn spare_flags() -> Vec<&'static str> {
    ROOMS.iter().flat_map(|r| r.spare.iter().copied()).collect()
}

fn parts(run: &str, f: &mut dyn FnMut(&str)) {
    for tok in run.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
        if tok.is_empty() {
            continue;
        }
        let cuts: Vec<usize> = std::iter::once(0).chain(tok.match_indices('_').map(|(i, _)| i + 1)).collect();
        let ends: Vec<usize> = tok.match_indices('_').map(|(i, _)| i).chain(std::iter::once(tok.len())).collect();
        for a in &cuts {
            for b in &ends {
                if b > a {
                    f(&tok[*a..*b]);
                }
            }
        }
    }
}

pub fn refs(rf: &mut Romfs, names: &[String]) -> Res<BM<String, BS<String>>> {
    let want: HS<&str> = names.iter().map(|s| s.as_str()).collect();
    let mut out: BM<String, BS<String>> = BM::new();
    crate::flags_scan::walk(rf, &mut |p: &str, b: &[u8]| {
        if p.starts_with("GameData/") || p.starts_with("Mals/") {
            return;
        }
        crate::flags_scan::runs(b, 6, &mut |r: &str| {
            parts(r, &mut |t: &str| {
                if want.contains(t) {
                    out.entry(t.to_string()).or_default().insert(p.to_string());
                }
            });
        });
    })?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap as HM;
    use std::path::PathBuf as PB;

    use super::*;
    use crate::bfevfl::{Actor, Event};
    use crate::settings::Value;
    use crate::slice::generate::{allowed, generate_data, GenOpts, Generated};
    use crate::slice::logic::world_with;
    use crate::slice::support::plan;
    use crate::world::Placement;

    const FLAGS: &str = include_str!("../../../../data/flags.json");
    const EXTRA: &str = include_str!("../../../../data/flow_extra.json");

    fn root() -> Option<PB> {
        let p: PB = ootrr_game::test_romfs::path();
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Some(p)
        } else {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            None
        }
    }

    fn with(on: bool) -> Settings {
        let mut s: Settings = Settings::default();
        s.set(SETTING, Value::Bool(on)).unwrap();
        s
    }

    fn base() -> Flow {
        let a = |n: &str, acts: &[&str]| Actor {
            name: n.to_string(),
            sub: String::new(),
            arg: String::new(),
            actions: acts.iter().map(|s| s.to_string()).collect(),
            queries: Vec::new(),
            params: None,
            ep: 0xFFFF,
            cut: 1,
        };
        Flow {
            name: "Dm_SY_Treasurebox".to_string(),
            actors: vec![a("GameSystemActor", &["EventWait"])],
            events: vec![Event { name: "Event0".to_string(), kind: Ev::Action { next: NONE, actor: 0, act: 0, params: None } }],
            entries: vec![Entry { name: ROOT_ENTRY.to_string(), subs: Vec::new(), vars: Vec::new(), main: 0, pad: 3 }],
        }
    }

    fn name_of(c: &Option<Cont>) -> String {
        match c.as_ref().and_then(|c| c.get("GameDataName")) {
            Some(Val::Str(s)) => s.clone(),
            _ => panic!("no GameDataName"),
        }
    }

    fn walk(f: &Flow, entry: &str, st: &mut BS<String>) -> usize {
        let mut i: u16 = f.entries.iter().find(|e| e.name == entry).unwrap().main;
        let mut gave: usize = 0;
        let mut steps: usize = 0;
        while i != NONE {
            steps += 1;
            assert!(steps < 100, "the entry loops");
            match &f.events[i as usize].kind {
                Ev::Switch { actor, query, cases, params } => {
                    assert_eq!(f.actors[*actor as usize].queries[*query as usize], ASK);
                    let v: i64 = if st.contains(&name_of(params)) { 0 } else { 1 };
                    i = cases.iter().find(|c| c.val as i64 == v).unwrap().ev;
                }
                Ev::Action { next, actor, act, params } => {
                    let a: &Actor = &f.actors[*actor as usize];
                    if a.name == HOLDER {
                        assert_eq!(a.actions[*act as usize], SET);
                        assert_eq!(params.as_ref().and_then(|c| c.get("Value")), Some(&Val::Bool(1)));
                        st.insert(name_of(params));
                    } else {
                        assert_eq!((a.name.as_str(), a.actions[*act as usize].as_str()), (TALKER, TALK));
                    }
                    i = *next;
                }
                Ev::Sub { next, entry, flow, params } => {
                    assert!(flow.is_empty() && entry == ROOT_ENTRY);
                    assert_eq!(params.as_ref().and_then(|c| c.get("ContentName")), Some(&Val::Str(CONTENT.to_string())));
                    gave += 1;
                    i = *next;
                }
                k => panic!("unexpected event {k:?}"),
            }
        }
        gave
    }

    #[test]
    fn each_grant_sets_the_next_spare_flag_and_only_the_fifth_sets_the_solved_flags() {
        for r in &ROOMS {
            let mut f: Flow = base();
            edit_flow(&mut f, &BS::from([r.tag])).unwrap();
            assert!(sound(&f).is_empty(), "{}", r.tag);
            let e: &Entry = f.entries.iter().find(|e| e.name == entry_name(r.tag)).unwrap();
            assert_eq!(e.pad, 3);
            assert!(matches!(f.events[e.subs[0] as usize].kind, Ev::Sub { .. }));
            let mut st: BS<String> = BS::new();
            for k in 1..=5 {
                assert_eq!(walk(&f, &entry_name(r.tag), &mut st), 1, "{} grant {k}", r.tag);
                let spare: usize = r.spare.iter().filter(|s| st.contains(**s)).count();
                assert_eq!(spare, k.min(4), "{} grant {k}", r.tag);
                assert!(r.spare[..k.min(4)].iter().all(|s| st.contains(*s)), "{} grant {k} sets the flags in order", r.tag);
                let solved: bool = r.solved.iter().all(|s| st.contains(*s));
                assert_eq!(solved, k == 5, "{} grant {k}", r.tag);
                assert!(r.solved.iter().all(|s| st.contains(*s) == solved), "{} grant {k}", r.tag);
                assert_eq!(st.len(), spare + if solved { r.solved.len() } else { 0 }, "{} grant {k} writes nothing else", r.tag);
            }
            let before: BS<String> = st.clone();
            assert_eq!(walk(&f, &entry_name(r.tag), &mut st), 1, "{} a sixth grant still gives the rupee", r.tag);
            assert_eq!(st, before, "{} a sixth grant changes nothing", r.tag);
            let again: Flow = f.clone();
            edit_flow(&mut f, &BS::from([r.tag])).unwrap();
            assert_eq!(f, again);
        }
        let mut f: Flow = base();
        edit_flow(&mut f, &ROOMS.iter().map(|r| r.tag).collect()).unwrap();
        assert!(sound(&f).is_empty());
        assert_eq!(f.entries.len(), 1 + ROOMS.len());
        assert_eq!(f.actors.iter().filter(|a| a.name == HOLDER).count(), 1);
        assert!(edit_flow(&mut base(), &BS::from(["Nowhere"])).is_err());
    }

    #[test]
    fn the_table_is_five_rooms_of_five_spots_with_distinct_hashes_and_flags() {
        let d: Data = Data::load().unwrap();
        let mut hashes: BS<u64> = BS::new();
        let mut flags: BS<&str> = BS::new();
        let mut n: usize = 0;
        for r in &ROOMS {
            assert!(d.item(r.item).is_some_and(|i| i.kind == "SilverRupee" && i.adv && i.actor.is_none()), "{}", r.item);
            for (name, hs) in &r.spots {
                assert_eq!(d.vanilla_of(name), Some(r.item), "{name}");
                assert!(hs.len() == 1 || r.tag == "GtgSlopes", "{name}");
                for h in *hs {
                    assert!(hashes.insert(*h), "{h}");
                    n += 1;
                }
            }
            for f in r.spare.iter().chain(r.solved.iter()) {
                assert!(flags.insert(f), "{f}");
            }
            assert!(event_name(r.tag, Size::Small).len() <= 29 && label(r.tag).starts_with(OURS));
        }
        assert_eq!(n, 4 * 5 + 2 * 5);
        assert_eq!(room_of_item("Silver Rupee (Ganons Castle Fire Trial)"), None);
        assert!(spot("Ganons Castle Fire Trial Silver Rupee Inside Pillar").is_none());
        assert!(spot("Gerudo Training Ground Water Room Silver Rupee Top").is_none());
    }

    #[test]
    fn the_spare_flags_are_persistent_bools_no_other_edit_uses() {
        let fl: Vec<&str> = spare_flags();
        assert_eq!(fl.len(), 20);
        assert_eq!(fl.iter().collect::<BS<_>>().len(), 20);
        for f in &fl {
            let row: &str = FLAGS.lines().find(|l| l.contains(&format!("\"name\":\"{f}\""))).unwrap_or_else(|| panic!("{f} is not in flags.json"));
            assert!(row.contains("\"type\":\"Bool\"") && row.contains("\"default\":false") && row.contains("\"reset\":8") && row.contains("\"save\":0"), "{f}");
            assert!(f.starts_with("HavePlayedEvent."), "{f}");
            assert!(!EXTRA.contains(&format!("\"{f}\"")), "{f} is used by data/flow_extra.json");
        }
        for n in ["0022", "0023", "0024", "0025", "0026", "0027", "0028", "0029", "0030"] {
            assert!(!fl.contains(&format!("HavePlayedEvent.Dm_StaffCredit_{n}").as_str()), "{n} is reserved for the cows");
        }
        for c in crate::flow_extra::cows::LOCS {
            let cow: String = crate::flow_extra::cows::flag_of(c).unwrap();
            assert!(!fl.contains(&cow.as_str()), "{cow}");
        }
    }

    #[test]
    fn the_spare_flags_have_no_event_row_and_nothing_in_the_game_names_them() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let names: Vec<String> = spare_flags().iter().map(|f| f.trim_start_matches("HavePlayedEvent.").to_string()).collect();
        let got: BM<String, BS<String>> = refs(&mut rf, &names).unwrap();
        assert!(got.is_empty(), "{got:?}");
        let probe: Vec<String> = vec!["Dm_StaffCredit_0022".to_string()];
        assert!(refs(&mut rf, &probe).unwrap().get("Dm_StaffCredit_0022").is_some_and(|s| s.contains("Pack/EnvPalette.pack.zs")));
    }

    #[test]
    fn every_token_hash_is_a_silver_token_in_its_scene_file_and_nowhere_else() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let want: BS<u64> = ROOMS.iter().flat_map(|r| r.spots.iter().flat_map(|s| s.1.iter().copied())).collect();
        let mut seen: BM<u64, Vec<String>> = BM::new();
        for f in crate::placement::banc_files(&rf).unwrap() {
            let y: Byml = Byml::parse(&rf.read_zs(&f).unwrap()).unwrap();
            let none: Vec<Node> = Vec::new();
            for a in y.root.get("Actors").and_then(|a| a.as_arr()).unwrap_or(&none) {
                if let Some(h) = hash_of(a).filter(|h| want.contains(h)) {
                    assert_eq!(text(a, "Gyaml"), TOKEN, "{h} in {f}");
                    seen.entry(h).or_default().push(f.clone());
                }
            }
        }
        for r in &ROOMS {
            for (name, hs) in &r.spots {
                for h in *hs {
                    let files: Vec<String> = r.files.iter().map(|f| f.to_string()).collect();
                    assert_eq!(seen.get(h), Some(&files), "{name} {h}");
                }
            }
        }
    }

    #[test]
    fn the_setting_is_off_by_default_keeps_the_string_and_leaves_the_logic_setting_off() {
        assert!(!plan(&Settings::default()).unwrap().silver);
        assert!(plan(&with(true)).unwrap().silver);
        assert_eq!(Settings::default().encode().unwrap(), with(false).encode().unwrap());
        assert_ne!(Settings::default().encode().unwrap(), with(true).encode().unwrap());
        let on: Plan = plan(&with(true)).unwrap();
        assert!(!on.logic().iter().any(|(k, _)| *k == SETTING));
        assert!(on.notes.iter().any(|n| n == NOTE));
        assert!(!plan(&Settings::default()).unwrap().notes.iter().any(|n| n == NOTE));
        let mut s: Settings = with(true);
        s.set("shuffle_chests", Value::s("off")).unwrap();
        assert!(plan(&s).unwrap_err().to_string().contains(NEEDS_CHESTS));
        // Chests are shuffled by default, which is all the Silver Rupee items need.
        let mut s: Settings = Settings::default();
        s.set(SETTING, Value::Bool(true)).unwrap();
        assert!(plan(&s).unwrap().silver);
        let logic: &str = include_str!("../../../../data/logic/settings.json");
        assert!(logic.contains("\"shuffle_silver_rupees\": false"));
    }

    #[test]
    fn off_keeps_the_default_seeds_and_places_no_spot() {
        let d: Data = Data::load().unwrap();
        for n in 0..3 {
            let a: Generated = generate_data(&d, &Settings::default(), &format!("silver off {n}"), &GenOpts::default()).unwrap();
            let b: Generated = generate_data(&d, &with(false), &format!("silver off {n}"), &GenOpts::default()).unwrap();
            assert_eq!(a.seed_file.placements, b.seed_file.placements);
            assert_eq!(a.report, b.report);
            assert!(a.seed_file.placements.iter().all(|(l, i)| !is_spot(l) && !is_item(i)));
        }
    }

    fn check(d: &Data, s: &Settings, g: &Generated) {
        let p: Plan = plan(s).unwrap();
        let w: World = world_with(d, &p.logic()).unwrap();
        let mut count: BM<&str, usize> = BM::new();
        for r in &ROOMS {
            for (name, _) in &r.spots {
                let it: &str = &g.seed_file.placements[*name];
                assert!(junk(it).is_some(), "{name} holds {it}");
            }
        }
        for (loc, item) in &g.seed_file.placements {
            assert!(allowed(d, &w, &p, w.item(item).unwrap(), w.loc(loc).unwrap()), "{item} at {loc}");
            if let Some(r) = room_of_item(item) {
                *count.entry(r.tag).or_insert(0) += 1;
                assert!(chest_ok(d, loc), "{item} at {loc}");
                assert!(!is_spot(loc));
            }
        }
        assert_eq!(count, ROOMS.iter().map(|r| (r.tag, 5)).collect::<BM<&str, usize>>());
        let mut pl: Placement = crate::slice::fill::vanilla_place(&w, d).unwrap();
        for (loc, item) in &g.seed_file.placements {
            pl.put(w.loc(loc).unwrap(), w.item(item).unwrap());
        }
        assert!(crate::search::explore(&w, &pl, &[]).won());
    }

    #[test]
    fn on_moves_the_silver_rupees_into_chests_and_rupees_onto_their_spots() {
        let d: Data = Data::load().unwrap();
        for n in 0..4 {
            let g: Generated = generate_data(&d, &with(true), &format!("silver on {n}"), &GenOpts::default()).unwrap();
            check(&d, &with(true), &g);
            assert!(g.report.contains(NOTE));
        }
        let mut s: Settings = with(true);
        s.set("shuffle_chests", Value::s("dungeons")).unwrap();
        s.set("shuffle_freestanding_items", Value::s("all")).unwrap();
        for n in 0..2 {
            let g: Generated = generate_data(&d, &s, &format!("silver dungeons {n}"), &GenOpts::default()).unwrap();
            check(&d, &s, &g);
        }
    }

    #[test]
    fn spots_and_chests_refuse_what_the_writer_cannot_route() {
        let d: Data = Data::load().unwrap();
        let p: Plan = plan(&with(true)).unwrap();
        let w: World = world_with(&d, &p.logic()).unwrap();
        let spot_l: LocId = w.loc("Ganons Castle Light Trial Silver Rupee Center Top").unwrap();
        for it in ["Rupees (20)", "Rupee (1)", "Rupees (200)"] {
            if let Some(i) = w.item(it) {
                assert_eq!(allowed(&d, &w, &p, i, spot_l), junk(it).is_some(), "{it}");
            }
        }
        for it in ["Recovery Heart", "Progressive Hookshot", "Piece of Heart", "Silver Rupee (Ganons Castle Light Trial)", "Small Key (Ganons Castle)", "Deku Nuts (5)"] {
            assert!(!allowed(&d, &w, &p, w.item(it).unwrap(), spot_l), "{it}");
        }
        let item: ItemId = w.item("Silver Rupee (Ice Cavern Push Block)").unwrap();
        let mut chests: usize = 0;
        for l in 0..w.locs.len() {
            let ok: bool = allowed(&d, &w, &p, item, l);
            assert_eq!(ok, chest_ok(&d, &w.locs[l].name), "{}", w.locs[l].name);
            chests += ok as usize;
        }
        assert!(chests > 100, "{chests}");
        for n in ["Song from Impa", "Kak Impas House Cow", "KF Shop Item 1", "Queen Gohma", "GV Waterfall Freestanding PoH"] {
            if let Some(l) = w.loc(n) {
                assert!(!allowed(&d, &w, &p, item, l), "{n}");
            }
        }
        let off: Plan = plan(&Settings::default()).unwrap();
        assert!(at(&d, &w, &off, item, spot_l).is_none());
    }

    #[test]
    fn the_logic_counts_the_room_items_the_spots_held() {
        let d: Data = Data::load().unwrap();
        let p: Plan = plan(&with(true)).unwrap();
        let w: World = world_with(&d, &p.logic()).unwrap();
        let v: Placement = crate::slice::fill::vanilla_place(&w, &d).unwrap();
        let world: String = ["Ganons Castle", "Ice Cavern", "Bottom of the Well", "Gerudo Training Ground"]
            .iter()
            .map(|f| std::fs::read_to_string(PB::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../data/logic/World/{f}.json"))).unwrap())
            .collect();
        for r in &ROOMS {
            let i: ItemId = w.item(r.item).unwrap();
            for (name, _) in &r.spots {
                assert_eq!(v.get(w.loc(name).unwrap()), Some(i), "{name}");
            }
            let logic: String = r.item.replace("Silver Rupee (", "Silver_Rupee_").replace(')', "").replace(' ', "_");
            assert!(world.contains(&format!("({logic}, 5)")), "{logic}");
        }
    }

    #[test]
    fn a_built_mod_swaps_only_the_token_hashes_and_routes_every_silver_chest() {
        let Some(r) = root() else { return };
        let d: Data = Data::load().unwrap();
        let g: Generated = generate_data(&d, &with(true), "silver build", &GenOpts::default()).unwrap();
        let load: PB = PB::from(env!("CARGO_MANIFEST_DIR")).join("../../target/silver_mods");
        let _ = std::fs::remove_dir_all(&load);
        let b: crate::slice::edit::Built = crate::slice::edit::build_for(&g.seed_file, &r, &load, "ootrr-silver", crate::modpack::Host::Longshot).unwrap();
        let romfs: PB = b.root.join("romfs");
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let ours: BS<u64> = ROOMS.iter().flat_map(|r| r.spots.iter().flat_map(|s| s.1.iter().copied())).collect();
        let chests: BM<u64, &'static str> = grants(&d, &g.seed_file).unwrap();
        assert_eq!(chests.values().collect::<BS<_>>().len(), ROOMS.len());
        let mut swapped: BS<u64> = BS::new();
        let mut routed: BS<u64> = BS::new();
        for f in crate::placement::banc_files(&rf).unwrap() {
            let p: PB = romfs.join(&f);
            if !p.is_file() {
                continue;
            }
            let old: Byml = Byml::parse(&rf.read_zs(&f).unwrap()).unwrap();
            let new: Byml = Byml::parse(&rf.zs.decompress(&std::fs::read(&p).unwrap()).unwrap()).unwrap();
            let olds: &[Node] = old.root.get("Actors").unwrap().as_arr().unwrap();
            let news: &[Node] = new.root.get("Actors").unwrap().as_arr().unwrap();
            assert_eq!(olds.len(), news.len(), "{f}");
            for (a, b) in olds.iter().zip(news) {
                let h: u64 = hash_of(a).unwrap_or(0);
                if text(a, "Gyaml") == TOKEN {
                    if ours.contains(&h) {
                        assert!(text(b, "Gyaml").starts_with("Obj_Rupee_"), "{h} in {f} is {}", text(b, "Gyaml"));
                        let loc: &str = ROOMS.iter().flat_map(|r| r.spots.iter()).find(|s| s.1.contains(&h)).unwrap().0;
                        assert_eq!(junk(&g.seed_file.placements[loc]), Some(text(b, "Gyaml")), "{loc}");
                        swapped.insert(h);
                    } else {
                        assert_eq!(text(b, "Gyaml"), TOKEN, "a token outside the slice moved, {h} in {f}");
                    }
                }
                if let Some(tag) = chests.get(&h) {
                    let dy: &Node = b.get("Dynamic").unwrap();
                    assert_eq!(text(dy, DROP), CONTENT, "{h}");
                    assert_eq!(text(dy, "GetEventName"), event_name(tag, Size::of_gyaml(text(b, "Gyaml"))), "{h}");
                    routed.insert(h);
                }
            }
        }
        assert_eq!(swapped, ours);
        assert_eq!(routed, chests.keys().copied().collect::<BS<u64>>());
        let table: Byml = Byml::parse(&rf.zs.decompress(&std::fs::read(romfs.join(SETTING_TABLE)).unwrap()).unwrap()).unwrap();
        let mut flows: HM<String, Flow> = HM::new();
        for (path, stem) in [(BIG_FLOW, "Dm_SY_Treasurebox"), (SMALL_FLOW, "Dm_SY_TreasureboxSmall")] {
            let bf: Bf = Bf::parse(&rf.zs.decompress(&std::fs::read(romfs.join(path)).unwrap()).unwrap()).unwrap();
            let f: Flow = bf.flows[0].clone();
            assert!(sound(&f).is_empty(), "{path}");
            for room in &ROOMS {
                assert!(f.entries.iter().any(|e| e.name == entry_name(room.tag)), "{path} {}", room.tag);
            }
            flows.insert(stem.to_string(), f);
        }
        for room in &ROOMS {
            for size in [Size::Small, Size::Big] {
                let row: &Node = table.root.as_arr().unwrap().iter().find(|x| text(x, "__RowId") == event_name(room.tag, size)).unwrap();
                assert_eq!(text(row, "EntryPointName"), entry_name(room.tag));
            }
        }
        assert!(crate::flow_extra::analyzed::worked(&mut rf, &romfs).unwrap().is_none(), "an edited event calls a flow its row does not load");
        let mut s = crate::flow_extra::sim::sim(&r, flows).unwrap();
        for room in &ROOMS {
            for flow in ["Dm_SY_Treasurebox", "Dm_SY_TreasureboxSmall"] {
                for k in 0..=4 {
                    let set: Vec<(String, String)> = room.spare.iter().enumerate().map(|(i, f)| (f.to_string(), (i < k).to_string())).collect();
                    let sc = crate::flow_extra::sim::Scene { flow: flow.to_string(), entry: entry_name(room.tag), set, own: Vec::new() };
                    let fins = crate::flow_extra::sim::run(&mut s, &sc).unwrap();
                    assert!(!fins.is_empty(), "{} {flow} {k}", room.tag);
                    assert_eq!(crate::flow_extra::sim::broken(&fins), 0, "{} {flow} {k}", room.tag);
                    for fin in &fins {
                        let wrote: BS<String> = fin.m.writes.iter().filter(|(_, v)| v.as_str() == "true").map(|(n, _)| n.clone()).filter(|n| room.spare.contains(&n.as_str()) || room.solved.contains(&n.as_str())).collect();
                        let want: BS<String> = if k < 4 { BS::from([room.spare[k].to_string()]) } else { room.solved.iter().map(|x| x.to_string()).collect() };
                        assert_eq!(wrote, want, "{} {flow} {k}", room.tag);
                    }
                }
            }
        }
        let _ = std::fs::remove_dir_all(&load);
    }
}
