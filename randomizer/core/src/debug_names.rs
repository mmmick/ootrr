use std::collections::{BTreeMap as BM, BTreeSet as BS};
use std::fs;
use std::path::Path as P;

use crate::bfevfl::flow::NONE;
use crate::bfevfl::{Bf, Case, Cont, Entry, Flow, Kind, Val};
use crate::byml::{Byml, Node};
use crate::dungeon_credit::{action_slot, idx, push, set_bool, talk, HOLDER, SET, TALK, TALKER};
use crate::dungeon_text::{FILE, FILE_ID};
use crate::error::{Error as E, Res};
use crate::msbt::{self, Msbt};
use crate::placement::hash_of;
use crate::progressive::{add_children, add_rows, Size, ANALYZED_TABLE, SETTING_TABLE};
use crate::romfs::Romfs;
use crate::sarc;
use crate::slice::data::Data;
use crate::staging::Out;
use crate::story_skip::{stage_late, stage_late_file};

pub const NUM: u16 = 142;
pub const SETTING: &str = "debug_location_names";
pub const NOTE: &str = "Debug: chests, NPC, song and event rewards, Gold Skulltulas and items on the ground show their location name in a text box after the item get. Items on the ground need a button press, and placed items missing from the location list say so.";
const PREFIX: &str = "OOTRR_Dbg_";
const ENTRY_PREFIX: &str = "Dbg_";
const LINE_PREFIX: &str = "OOTRR_DbgLoc_";
const FLOW_PREFIX: &str = "OOTRR_DbgFlow_";
const CHEST: &str = "Obj_TBox";
const EVENT: &str = "GetEventName";
const DROP: &str = "Drop__DropActor";
const CHART: &str = "FlowchartFileName";
const ENTRY: &str = "EntryPointName";
const ROW_ID: &str = "__RowId";
const KIDS: &str = "ChildEventList";
const GET_FLOW: &str = "GetPouchContent";
const WINDOW_PREFIX: &str = "OpenGetWindow";
const SWEEP: &str = "OOTRRDbgSweep";
const ADD_FLOWS: [&str; 2] = ["Dm_SY_Get", "Dm_SY_Get_WithoutPlayer"];
const WAIT: &str = "EventWait";
const WAITER: &str = "EventSystemActor";
const ADD_DELAY: i32 = 5;
const SWEEP_MISS: &str = "OOTRRDbgMiss";
const WAITING: &str = "OOTRRDbgWait";
const TAG_ASK: &str = "EventQueryCheckTagOnPouchActor";
const TOKEN_TAG: u32 = 3;
const TAG_CASES: u32 = 16;
pub const MISS_SPAWN: &str = "OOTRR_DbgMiss_Spawned";
const MISS_SPAWN_TEXT: &str = "Not in database
not placed in a scene";
const SKULL_KIND: &str = "gs";
const ASK_BOOL: &str = "EventQueryIsGameDataBool";
const SCRUB_FLOW: &str = "Enemy_Scrub_Business_Common";
pub const PICK: &str = "OOTRRDbgPick";
const PICK_TEMPLATE: &str = "UI_MessageTargetActorName";
const ASK_TEXT: &str = "EventQueryIsGameDataString64";
const SET_TEXT: &str = "EventTriggerSetGameDataString64";
const ADD_OWN: &str = "EventTriggerAddOwnToPouch";
const OWN_LINK: &str = "QueryGenericGetOwnActorLink";
const ACTOR_NAME: &str = "QueryGenericGetActorName";
const SET_STRING: &str = "OneShotSetNonStructGameDataString64";
const GLOBAL_AI: &str = "Pack/AI.Global.Product.111.pack.zs";
const PICKUP_KINDS: [&str; 4] = ["freestanding", "heart", "bossheart", "key"];
const MISS_PREFIX: &str = "OOTRR_DbgMiss_";
const GET_TYPE: &str = "GetType";
const GET_MEDIUM: &str = "Medium";
const ITEM_KINDS: [&str; 32] = [
    "Obj_Armor_Goron",
    "Obj_Armor_Zora",
    "Obj_ArrowBundle_Large",
    "Obj_ArrowBundle_Medium",
    "Obj_ArrowBundle_Small",
    "Obj_Bomb",
    "Obj_Bombchu",
    "Obj_DekuNut",
    "Obj_DekuSeed",
    "Obj_DekuStick",
    "Obj_Dungeon_SmallKey",
    "Obj_Dungeon_SmallKey_GerudoFortress_01",
    "Obj_Dungeon_SmallKey_GerudoFortress_02",
    "Obj_Dungeon_SmallKey_GerudoFortress_03",
    "Obj_Dungeon_SmallKey_GerudoFortress_04",
    "Obj_GerudoValley_Heart_Piece",
    "Obj_Heart",
    "Obj_Heart_Container",
    "Obj_Heart_InWater",
    "Obj_Heart_Piece",
    "Obj_Heart_Piece_Digging_Hole",
    "Obj_Heart_Shootable",
    "Obj_MagicJar_Large",
    "Obj_MagicJar_Small",
    "Obj_Rupee_Blue",
    "Obj_Rupee_Gold",
    "Obj_Rupee_Green",
    "Obj_Rupee_Purple",
    "Obj_Rupee_Red",
    "Obj_ScrubBusinessSeed",
    "Obj_Shield_Deku",
    "Obj_Shield_Hylian",
];
const DONE: &str = "OOTRRDbgDone";
const PRESS_FROM: &str = "Obj_Heart_Piece";
const AUTO_CLIENT: &str = "Auto_Item_Catch";
const PRESS_CLIENTS: [&str; 2] = ["Attention/ClientParam/Item_Catch.game__attention__ClientParam", "Attention/ClientParam/LookOnly_High_Medium.game__attention__ClientParam"];

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Wrap {
    pub loc: usize,
    pub old: String,
    pub id: String,
    pub entry: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub stem: String,
    pub entry: String,
}

pub fn line_label(loc: usize) -> String {
    format!("{LINE_PREFIX}{loc}")
}

pub fn line_id(loc: usize) -> String {
    format!("{FILE_ID}:{}", line_label(loc))
}

pub fn flow_label(loc: usize) -> String {
    format!("{FLOW_PREFIX}{loc}")
}

pub fn flow_id(loc: usize) -> String {
    format!("{FILE_ID}:{}", flow_label(loc))
}

pub fn scene_line(d: &Data, loc: &str) -> Option<String> {
    d.flow.locs.iter().position(|l| l.name == loc).map(flow_id)
}

fn current(rf: &mut Romfs, more: &[Out], work: &P, path: &str) -> Res<Vec<u8>> {
    if let Some(o) = more.iter().find(|o| o.path == path) {
        return rf.zs.decompress(&o.z);
    }
    let at = path.split('/').fold(work.to_path_buf(), |a, s| a.join(s));
    let out: Res<Vec<u8>> = match fs::read(&at) {
        Ok(z) => rf.zs.decompress(&z),
        Err(_) => rf.read_zs(path),
    };
    out.map_err(|e| E::Msg(format!("debug names: {path}: {e}")))
}

fn text<'a>(n: &'a Node, k: &str) -> &'a str {
    n.get(k).and_then(|v| v.as_str()).unwrap_or("")
}

fn actors(y: &mut Byml) -> Option<&mut Vec<Node>> {
    y.root.get_mut("Actors")?.as_arr_mut()
}

pub fn chest_event(a: &Node) -> Option<String> {
    let g: &str = text(a, "Gyaml");
    if !g.starts_with(CHEST) {
        return None;
    }
    let d: &Node = a.get("Dynamic")?;
    d.get(DROP)?;
    let ev: &str = d.get(EVENT).and_then(|v| v.as_str()).unwrap_or("");
    Some(if ev.is_empty() { Size::of_gyaml(g).template().to_string() } else { ev.to_string() })
}

pub fn row_of(y: &Byml, id: &str) -> Option<Row> {
    let r: &Node = y.root.as_arr()?.iter().find(|r| text(r, ROW_ID) == id)?;
    let chart: &str = text(r, CHART);
    let stem: &str = chart.rsplit('/').next()?.strip_suffix(".evfc")?;
    Some(Row { stem: stem.to_string(), entry: text(r, ENTRY).to_string() })
}

pub fn parents(y: &Byml, ws: &[Wrap]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let Some(rows) = y.root.as_arr() else { return out };
    for r in rows {
        let Some(kids) = r.get(KIDS).and_then(|k| k.as_arr()) else { continue };
        for w in ws {
            if kids.iter().any(|k| k.as_str() == Some(w.old.as_str())) {
                out.push((text(r, ROW_ID).to_string(), w.id.clone()));
            }
        }
    }
    out
}

fn has_row(y: &Byml, id: &str) -> bool {
    y.root.as_arr().is_some_and(|a| a.iter().any(|r| text(r, ROW_ID) == id))
}

fn next_of(k: &Kind) -> Vec<u16> {
    match k {
        Kind::Action { next, .. } | Kind::Join { next } | Kind::Sub { next, .. } => vec![*next],
        Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => cases.iter().map(|c| c.ev).collect(),
        Kind::Fork { join, forks } => forks.iter().copied().chain([*join]).collect(),
        Kind::Label { .. } => Vec::new(),
    }
}

pub fn args_from(f: &Flow, main: u16) -> BS<String> {
    let mut out: BS<String> = BS::new();
    let mut seen: BS<u16> = BS::new();
    let mut todo: Vec<u16> = vec![main];
    while let Some(i) = todo.pop() {
        if i == NONE || !seen.insert(i) {
            continue;
        }
        let Some(e) = f.events.get(usize::from(i)) else { continue };
        if let Some(c) = e.kind.params() {
            for (_, v) in &c.items {
                if let Val::Arg(a) = v {
                    out.insert(a.clone());
                }
            }
        }
        todo.extend(next_of(&e.kind));
    }
    out
}

pub fn wrap_flow(b: &mut Bf, stem: &str, ws: &[(&Wrap, &Row)]) -> Res<()> {
    let f: &mut Flow = match b.flows.iter().position(|f| f.name == stem) {
        Some(i) => &mut b.flows[i],
        None => b.flows.first_mut().ok_or_else(|| E::Msg(format!("debug names: {stem} has no flow")))?,
    };
    for (w, r) in ws {
        if f.entries.iter().any(|e| e.name == w.entry) {
            continue;
        }
        let (vars, pad, main) = {
            let e: &Entry = f.entries.iter().find(|e| e.name == r.entry).ok_or_else(|| E::Msg(format!("debug names: {stem} has no entry {}", r.entry)))?;
            (e.vars.clone(), e.pad, e.main)
        };
        let args: BS<String> = args_from(f, main);
        let params: Option<Cont> = if args.is_empty() { None } else { Some(Cont { items: args.iter().map(|a| (a.clone(), Val::Arg(a.clone()))).collect() }) };
        let (t, say): (u16, u16) = action_slot(f, TALKER, TALK)?;
        let at: u16 = idx(f.events.len())?;
        push(f, Kind::Sub { next: at + 1, params, flow: String::new(), entry: r.entry.clone() })?;
        push(f, Kind::Action { next: NONE, actor: t, act: say, params: Some(talk(line_id(w.loc))) })?;
        f.entries.push(Entry { name: w.entry.clone(), subs: vec![at], vars, main: at, pad });
    }
    Ok(())
}

pub fn put_lines(raw: &[u8], lines: &BM<String, String>) -> Res<Vec<u8>> {
    let mut es: Vec<sarc::Entry> = sarc::parse(raw)?;
    let e: &sarc::Entry = sarc::get(&es, FILE).ok_or_else(|| E::Msg(format!("the language archive has no {FILE}")))?;
    let mut m: Msbt = msbt::parse(&e.data)?;
    for (label, name) in lines {
        let t = msbt::from_markup(&format!("[{name}]"))?;
        m.put_tokens(label, &t)?;
    }
    sarc::set(&mut es, FILE, m.write()?);
    sarc::write(&es)
}

pub fn stage(rf: &mut Romfs, d: &Data, work: &P, more: &mut Vec<Out>) -> Res<usize> {
    let mut by_file: BM<String, BM<u64, usize>> = BM::new();
    for (i, l) in d.cat.locs.iter().enumerate() {
        for r in &l.refs {
            for f in &r.files {
                by_file.entry(f.clone()).or_default().insert(r.hash, i);
            }
        }
    }
    let mut found: BM<String, Vec<(u64, usize, String)>> = BM::new();
    for (f, want) in &by_file {
        let raw: Vec<u8> = current(rf, more, work, f)?;
        let mut y: Byml = Byml::parse(&raw)?;
        let Some(list) = actors(&mut y) else { continue };
        for a in list.iter() {
            let Some(h) = hash_of(a) else { continue };
            let Some(i) = want.get(&h) else { continue };
            if let Some(ev) = chest_event(a) {
                found.entry(f.clone()).or_default().push((h, *i, ev));
            }
        }
    }
    let pairs: BS<(usize, String)> = found.values().flatten().map(|(_, i, ev)| (*i, ev.clone())).collect();
    if pairs.is_empty() {
        return Ok(0);
    }
    let wraps: Vec<Wrap> = pairs.iter().enumerate().map(|(n, (i, ev))| Wrap { loc: *i, old: ev.clone(), id: format!("{PREFIX}{n}"), entry: format!("{ENTRY_PREFIX}{n}") }).collect();
    let id_of: BM<(usize, String), String> = wraps.iter().map(|w| ((w.loc, w.old.clone()), w.id.clone())).collect();
    let table: Byml = Byml::parse(&current(rf, more, work, SETTING_TABLE)?)?;
    let mut rows: Vec<(&Wrap, Row)> = Vec::new();
    for w in &wraps {
        let r: Row = row_of(&table, &w.old).ok_or_else(|| E::Msg(format!("debug names: the event table has no {} row for {}", w.old, d.cat.locs[w.loc].name)))?;
        rows.push((w, r));
    }
    let mut by_flow: BM<String, Vec<(&Wrap, &Row)>> = BM::new();
    for (w, r) in &rows {
        by_flow.entry(r.stem.clone()).or_default().push((w, r));
    }
    for (stem, ws) in &by_flow {
        stage_late(rf, more, work, stem, |b| wrap_flow(b, stem, ws))?;
    }
    for t in [SETTING_TABLE, ANALYZED_TABLE] {
        let analyzed: bool = t == ANALYZED_TABLE;
        stage_late_file(rf, more, work, t, |raw: &[u8]| {
            let mut y: Byml = Byml::parse(raw)?;
            for w in &wraps {
                if analyzed && !has_row(&y, &w.old) {
                    continue;
                }
                add_rows(&mut y, &w.old, &[(w.id.clone(), if analyzed { None } else { Some(w.entry.clone()) })])?;
            }
            if analyzed {
                let kids: Vec<(String, String)> = parents(&y, &wraps);
                add_children(&mut y, &kids);
            }
            y.write()
        })?;
    }
    for (f, hits) in &found {
        stage_late_file(rf, more, work, f, |raw: &[u8]| {
            let mut y: Byml = Byml::parse(raw)?;
            if let Some(list) = actors(&mut y) {
                for a in list.iter_mut() {
                    let Some(h) = hash_of(a) else { continue };
                    let Some((_, i, ev)) = hits.iter().find(|x| x.0 == h) else { continue };
                    let Some(id) = id_of.get(&(*i, ev.clone())) else { continue };
                    if let Some(dy) = a.get_mut("Dynamic") {
                        dy.set(EVENT, Node::Str(id.clone()));
                    }
                }
            }
            y.write()
        })?;
    }
    let lines: BM<String, String> = wraps.iter().map(|w| (line_label(w.loc), d.cat.locs[w.loc].name.clone())).collect();
    for l in crate::hints::languages(rf)? {
        stage_late_file(rf, more, work, &l, |raw: &[u8]| put_lines(raw, &lines))?;
    }
    Ok(found.values().map(|v| v.len()).sum())
}

fn say_after(f: &mut Flow, at: usize, id: String) -> Res<bool> {
    let then: u16 = match &f.events[at].kind {
        Kind::Action { next, .. } | Kind::Sub { next, .. } => *next,
        _ => return Ok(false),
    };
    let (t, say): (u16, u16) = action_slot(f, TALKER, TALK)?;
    let n: u16 = push(f, Kind::Action { next: then, actor: t, act: say, params: Some(talk(id)) })?;
    match &mut f.events[at].kind {
        Kind::Action { next, .. } | Kind::Sub { next, .. } => *next = n,
        _ => {}
    }
    Ok(true)
}

fn say_before(f: &mut Flow, at: usize, id: String) -> Res<()> {
    let (t, say): (u16, u16) = action_slot(f, TALKER, TALK)?;
    let w: u16 = idx(at)?;
    before(f, at, Kind::Action { next: w, actor: t, act: say, params: Some(talk(id)) })?;
    Ok(())
}

fn before(f: &mut Flow, at: usize, k: Kind) -> Res<u16> {
    let w: u16 = idx(at)?;
    let n: u16 = push(f, k)?;
    for (i, ev) in f.events.iter_mut().enumerate() {
        if i == usize::from(n) {
            continue;
        }
        match &mut ev.kind {
            Kind::Action { next, .. } | Kind::Sub { next, .. } | Kind::Join { next } if *next == w => *next = n,
            Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => {
                for c in cases.iter_mut().filter(|c| c.ev == w) {
                    c.ev = n;
                }
            }
            Kind::Fork { forks, join } => {
                for x in forks.iter_mut().filter(|x| **x == w) {
                    *x = n;
                }
                if *join == w {
                    *join = n;
                }
            }
            _ => {}
        }
    }
    for e in f.entries.iter_mut() {
        if e.main == w {
            e.main = n;
        }
    }
    Ok(n)
}

pub fn hook(b: &mut Bf, event: &str, id: &str) -> Res<bool> {
    for f in b.flows.iter_mut() {
        let Some(at) = f.events.iter().position(|e| e.name == event) else { continue };
        if !say_after(f, at, id.to_string())? {
            say_before(f, at, id.to_string())?;
        }
        return Ok(true);
    }
    Ok(false)
}

pub fn stage_flows(rf: &mut Romfs, d: &Data, work: &P, late: &mut Vec<Out>, scenes: &BS<&str>, sales: &[(String, String)]) -> Res<Vec<String>> {
    let mut missed: Vec<String> = Vec::new();
    let mut by_file: BM<String, Vec<(String, String)>> = BM::new();
    for (i, l) in d.flow.locs.iter().enumerate() {
        if scenes.contains(l.name.as_str()) {
            continue;
        }
        if sales.iter().any(|(_, loc)| loc == &l.name) {
            continue;
        }
        let mut grants: Vec<&crate::flow_locations::Edit> = l.edits.iter().filter(|e| e.role == "grant").collect();
        if grants.is_empty() {
            grants = l.edits.iter().filter(|e| e.role == "offer").collect();
        }
        if grants.is_empty() {
            missed.push(l.name.clone());
            continue;
        }
        for e in grants {
            for f in std::iter::once(&e.file).chain(e.also.iter()) {
                let stem: String = f.trim_end_matches(".bfevfl").to_string();
                by_file.entry(stem).or_default().push((e.event.clone(), flow_id(i)));
            }
        }
    }
    for (stem, hs) in &by_file {
        let path: String = format!("Event/EventFlow/{stem}.bfevfl.zs");
        let staged: bool = late.iter().any(|o| o.path == path) || path.split('/').fold(work.to_path_buf(), |a, s| a.join(s)).is_file();
        if !staged && rf.read(&path).is_err() {
            missed.extend(hs.iter().map(|(ev, _)| format!("{stem}:{ev}")));
            continue;
        }
        let mut found: Vec<bool> = Vec::new();
        stage_late(rf, late, work, stem, |b| {
            for (ev, id) in hs {
                found.push(hook(b, ev, id)?);
            }
            Ok(())
        })?;
        for ((ev, _), ok) in hs.iter().zip(found) {
            if !ok {
                missed.push(format!("{stem}:{ev}"));
            }
        }
    }
    if !sales.is_empty() {
        let mut found: Vec<bool> = Vec::new();
        stage_late(rf, late, work, SCRUB_FLOW, |b| {
            for (entry, loc) in sales {
                let i: Option<usize> = d.flow.locs.iter().position(|l| &l.name == loc);
                found.push(match i {
                    Some(i) => hook_sale(b, entry, &flow_id(i))?,
                    None => false,
                });
            }
            Ok(())
        })?;
        for ((_, loc), ok) in sales.iter().zip(found) {
            if !ok {
                missed.push(loc.clone());
            }
        }
    }
    let lines: BM<String, String> = d.flow.locs.iter().enumerate().map(|(i, l)| (flow_label(i), l.name.clone())).collect();
    for l in crate::hints::languages(rf)? {
        stage_late_file(rf, late, work, &l, |raw: &[u8]| put_lines(raw, &lines))?;
    }
    Ok(missed)
}

pub fn skull_flag(h: u64) -> String {
    format!("IsGoldSkullDead.IsGoldSkullDead_{h}")
}

pub fn shown_flag(h: u64) -> String {
    format!("OOTRRDbgGs_{h}")
}

pub fn skulls(d: &Data) -> Vec<(u64, usize)> {
    let mut out: Vec<(u64, usize)> = Vec::new();
    for (i, l) in d.cat.locs.iter().enumerate().filter(|(_, l)| l.kind == SKULL_KIND) {
        for r in &l.refs {
            if !out.iter().any(|x| x.0 == r.hash) {
                out.push((r.hash, i));
            }
        }
    }
    out
}

fn row_hash(r: &Node) -> Option<u32> {
    match r.get("Hash") {
        Some(Node::U32(h)) => Some(*h),
        _ => None,
    }
}

pub fn add_shown(raw: &[u8], gs: &[(u64, usize)]) -> Res<(Vec<u8>, Vec<u64>)> {
    let mut y: Byml = Byml::parse(raw)?;
    let list: &mut Vec<Node> = y.root.get_mut("Data").and_then(|d| d.get_mut("Bool")).and_then(|b| b.as_arr_mut()).ok_or(E::Format("debug names: the game data list has no Bool table"))?;
    let have: BS<u32> = list.iter().filter_map(row_hash).collect();
    let mut added: Vec<u64> = Vec::new();
    for (h, _) in gs {
        let k: u32 = crate::flags::hash_name(&skull_flag(*h));
        let Some(base) = list.iter().find(|r| row_hash(r) == Some(k)).cloned() else { continue };
        let n: u32 = crate::flags::hash_name(&shown_flag(*h));
        if have.contains(&n) {
            return Err(E::Msg(format!("debug names: the flag {} clashes with a game flag", shown_flag(*h))));
        }
        let mut r: Node = base;
        r.set("Hash", Node::U32(n));
        r.set("DefaultValue", Node::Bool(false));
        list.push(r);
        added.push(*h);
    }
    Ok((y.write()?, added))
}

fn ask(f: &mut Flow, name: &str, yes: u16, no: u16) -> Res<Kind> {
    let (a, q): (u16, u16) = crate::start_state::query_slot(f, "GameDataActor", ASK_BOOL)?;
    Ok(Kind::Switch {
        actor: a,
        query: q,
        cases: vec![Case { val: 0, ev: yes }, Case { val: 1, ev: no }],
        params: Some(Cont { items: vec![("GameDataName".to_string(), Val::Str(name.to_string())), ("Index".to_string(), Val::Int(-1))] }),
    })
}

pub fn add_sweep(b: &mut Bf, gs: &[(u64, usize)], picks: &[(String, String)]) -> Res<usize> {
    let f: &mut Flow = b.flows.iter_mut().find(|f| f.name == GET_FLOW).ok_or(E::Format("debug names: GetPouchContent has no flow"))?;
    if f.entries.iter().any(|e| e.name == SWEEP) || (gs.is_empty() && picks.is_empty()) {
        return Ok(0);
    }
    let pad: u16 = f.entries.first().map(|e| e.pad).unwrap_or(0);
    let base: usize = f.events.len();
    let (ta, ts): (u16, u16) = action_slot(f, TALKER, TALK)?;
    let (sa, ss): (u16, u16) = action_slot(f, HOLDER, SET)?;
    let tail: usize = base + 4 * gs.len() + 3 * picks.len();
    for (k, (h, i)) in gs.iter().enumerate() {
        let at: usize = base + 4 * k;
        let after: u16 = idx(at + 4)?;
        let dead: Kind = ask(f, &skull_flag(*h), idx(at + 1)?, after)?;
        let seen: Kind = ask(f, &shown_flag(*h), after, idx(at + 2)?)?;
        push(f, dead)?;
        push(f, seen)?;
        push(f, Kind::Action { next: idx(at + 3)?, actor: ta, act: ts, params: Some(talk(line_id(*i))) })?;
        push(f, Kind::Action { next: after, actor: sa, act: ss, params: Some(set_bool(shown_flag(*h))) })?;
    }
    let (qa, qt): (u16, u16) = crate::start_state::query_slot(f, HOLDER, ASK_TEXT)?;
    let (_, st): (u16, u16) = action_slot(f, HOLDER, SET_TEXT)?;
    let text_of = |v: &str| -> Cont { Cont { items: vec![("GameDataName".to_string(), Val::Str(PICK.to_string())), ("Index".to_string(), Val::Int(-1)), ("Value".to_string(), Val::Str(v.to_string()))] } };
    for (k, (name, label)) in picks.iter().enumerate() {
        let at: usize = base + 4 * gs.len() + 3 * k;
        let after: u16 = idx(at + 3)?;
        push(f, Kind::Switch { actor: qa, query: qt, cases: vec![Case { val: 0, ev: idx(at + 1)? }, Case { val: 1, ev: after }], params: Some(text_of(name)) })?;
        push(f, Kind::Action { next: idx(at + 2)?, actor: qa, act: st, params: Some(text_of(DONE)) })?;
        push(f, Kind::Action { next: after, actor: ta, act: ts, params: Some(talk(format!("{FILE_ID}:{label}"))) })?;
    }
    let (wa, wt): (u16, u16) = wait_slot(f)?;
    if push(f, Kind::Action { next: NONE, actor: wa, act: wt, params: Some(wait(0)) })? != idx(tail)? {
        return Err(E::Format("debug names: the sweep events are out of order"));
    }
    f.entries.push(Entry { name: SWEEP.to_string(), subs: Vec::new(), vars: Vec::new(), main: idx(base)?, pad });
    let m: usize = f.events.len();
    push(f, Kind::Switch { actor: qa, query: qt, cases: vec![Case { val: 0, ev: NONE }, Case { val: 1, ev: idx(m + 1)? }], params: Some(text_of(DONE)) })?;
    push(f, Kind::Switch { actor: qa, query: qt, cases: vec![Case { val: 0, ev: NONE }, Case { val: 1, ev: idx(m + 2)? }], params: Some(text_of(WAITING)) })?;
    push(f, Kind::Action { next: idx(m + 3)?, actor: ta, act: ts, params: Some(talk(format!("{FILE_ID}:{MISS_SPAWN}"))) })?;
    push(f, Kind::Action { next: NONE, actor: qa, act: st, params: Some(text_of(DONE)) })?;
    f.entries.push(Entry { name: SWEEP_MISS.to_string(), subs: Vec::new(), vars: Vec::new(), main: idx(m)?, pad });
    let starts: Vec<u16> = f.entries.iter().filter(|e| e.name.starts_with(WINDOW_PREFIX)).map(|e| e.main).collect();
    let mut seen: BS<u16> = BS::new();
    let mut todo: Vec<u16> = starts;
    let mut ends: Vec<u16> = Vec::new();
    while let Some(i) = todo.pop() {
        if i == NONE || usize::from(i) >= base || !seen.insert(i) {
            continue;
        }
        let k: &Kind = &f.events[usize::from(i)].kind;
        let nx: Vec<u16> = next_of(k);
        if matches!(k, Kind::Action { .. } | Kind::Sub { .. } | Kind::Join { .. } | Kind::Switch { .. } | Kind::SubSwitch { .. }) && nx.contains(&NONE) {
            ends.push(i);
        }
        todo.extend(nx);
    }
    let mut n: usize = 0;
    for i in ends {
        let call: u16 = push(f, Kind::Sub { next: NONE, params: None, flow: String::new(), entry: SWEEP.to_string() })?;
        let owners: Vec<usize> = f.entries.iter().enumerate().filter(|(_, e)| e.name.starts_with(WINDOW_PREFIX)).map(|(j, _)| j).collect();
        for j in owners {
            if !f.entries[j].subs.contains(&call) {
                f.entries[j].subs.push(call);
            }
        }
        match &mut f.events[usize::from(i)].kind {
            Kind::Action { next, .. } | Kind::Sub { next, .. } | Kind::Join { next } => *next = call,
            Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => {
                for c in cases.iter_mut().filter(|c| c.ev == NONE) {
                    c.ev = call;
                }
            }
            _ => {}
        }
        n += 1;
    }
    Ok(n)
}

fn reach(f: &Flow, from: u16) -> BS<u16> {
    let mut seen: BS<u16> = BS::new();
    let mut todo: Vec<u16> = vec![from];
    while let Some(i) = todo.pop() {
        if i == NONE || usize::from(i) >= f.events.len() || !seen.insert(i) {
            continue;
        }
        todo.extend(next_of(&f.events[usize::from(i)].kind));
    }
    seen
}

fn wait_slot(f: &mut Flow) -> Res<(u16, u16)> {
    match f.actors.iter().find(|a| a.sub.is_empty() && a.actions.iter().any(|x| x == WAIT)) {
        Some(a) => {
            let who: String = a.name.clone();
            action_slot(f, &who, WAIT)
        }
        None => action_slot(f, WAITER, WAIT),
    }
}

fn wait(frames: i32) -> Cont {
    Cont { items: vec![("IsWaitEnd".to_string(), Val::Bool(1)), ("Frames".to_string(), Val::Int(frames))] }
}

pub fn sweep_after_add(b: &mut Bf) -> Res<usize> {
    let mut n: usize = 0;
    for f in b.flows.iter_mut() {
        let Some((ai, a)) = f.actors.iter().enumerate().find(|(_, a)| a.actions.iter().any(|x| x == ADD_OWN)) else { continue };
        let Some(add) = a.actions.iter().position(|x| x == ADD_OWN) else { continue };
        let adder: String = a.name.clone();
        let (who, what): (u16, u16) = (idx(ai)?, idx(add)?);
        if f.events.iter().any(|e| matches!(&e.kind, Kind::Sub { entry, .. } if entry == SWEEP)) {
            continue;
        }
        let sites: Vec<usize> = f.events.iter().enumerate().filter(|(_, e)| matches!(e.kind, Kind::Action { actor, act, .. } if actor == who && act == what)).map(|(i, _)| i).collect();
        let (wa, wt): (u16, u16) = wait_slot(f)?;
        let (ga, gt): (u16, u16) = crate::start_state::query_slot(f, &adder, TAG_ASK)?;
        let (ha, hs): (u16, u16) = action_slot(f, HOLDER, SET_TEXT)?;
        let reset: Cont = Cont { items: vec![("GameDataName".to_string(), Val::Str(PICK.to_string())), ("Index".to_string(), Val::Int(-1)), ("Value".to_string(), Val::Str(WAITING.to_string()))] };
        for at in sites {
            let then: u16 = match &f.events[at].kind {
                Kind::Action { next, .. } => *next,
                _ => continue,
            };
            let owners: Vec<usize> = f.entries.iter().enumerate().filter(|(_, e)| reach(f, e.main).contains(&idx(at).unwrap_or(NONE))).map(|(j, _)| j).collect();
            let miss: u16 = push(f, Kind::Sub { next: then, params: None, flow: GET_FLOW.to_string(), entry: SWEEP_MISS.to_string() })?;
            let cases: Vec<Case> = (0..TAG_CASES).map(|v| Case { val: v, ev: if v == TOKEN_TAG { then } else { miss } }).collect();
            let tag: u16 = push(f, Kind::Switch { actor: ga, query: gt, cases, params: None })?;
            let call: u16 = push(f, Kind::Sub { next: tag, params: None, flow: GET_FLOW.to_string(), entry: SWEEP.to_string() })?;
            let hold: u16 = push(f, Kind::Action { next: call, actor: wa, act: wt, params: Some(wait(ADD_DELAY)) })?;
            if let Kind::Action { next, .. } = &mut f.events[at].kind {
                *next = hold;
            }
            before(f, at, Kind::Action { next: idx(at)?, actor: ha, act: hs, params: Some(reset.clone()) })?;
            for j in owners {
                f.entries[j].subs.push(call);
                f.entries[j].subs.push(miss);
            }
            n += 1;
        }
    }
    Ok(n)
}

pub fn add_pick_flag(raw: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    let list: &mut Vec<Node> = y.root.get_mut("Data").and_then(|d| d.get_mut("String64")).and_then(|b| b.as_arr_mut()).ok_or(E::Format("debug names: the game data list has no String64 table"))?;
    let k: u32 = crate::flags::hash_name(PICK_TEMPLATE);
    let n: u32 = crate::flags::hash_name(PICK);
    if !list.iter().any(|r| row_hash(r) == Some(n)) {
        let mut r: Node = list.iter().find(|r| row_hash(r) == Some(k)).cloned().ok_or(E::Format("debug names: the game data list has no UI_MessageTargetActorName row"))?;
        r.set("Hash", Node::U32(n));
        r.set("DefaultValue", Node::Str(String::new()));
        list.push(r);
    }
    y.write()
}

fn ptr_class(a: &crate::ainb::Ainb, el: usize) -> Option<String> {
    let st: usize = a.params.get(el)?.ioe[5][2] as usize;
    let rec: &[u8] = a.outs[5].get(st * 8..st * 8 + 8)?;
    let o: u32 = u32::from_le_bytes([rec[4], rec[5], rec[6], rec[7]]);
    a.pool.get(o).map(|x| x.to_string())
}

pub fn patch_pick(raw: &[u8], tag: &str) -> Res<Option<Vec<u8>>> {
    use crate::ainb::build::Spec;
    use crate::ainb::{audit, Ainb, Plug};
    let mut a: Ainb = Ainb::parse(raw)?;
    let sites: Vec<usize> = a.find_els(ADD_OWN);
    if sites.is_empty() || a.find_els(SET_STRING).iter().any(|&i| a.in_str(i, "GameDataFullName") == Some(PICK)) {
        return Ok(None);
    }
    let class: String = a.find_els(OWN_LINK).into_iter().find_map(|i| ptr_class(&a, i)).ok_or_else(|| E::Msg(format!("debug names: {tag} has no actor link to copy")))?;
    for (n, site) in sites.into_iter().enumerate() {
        let kid: usize = {
            let p = &a.params[site];
            p.plugs.get(p.pl[2][1] as usize).map(|x| x.a as usize).ok_or_else(|| E::Msg(format!("debug names: the add handler in {tag} has no body")))?
        };
        if a.el_type(kid) != 3 {
            return Err(E::Msg(format!("debug names: the add handler in {tag} is not a parallel")));
        }
        let seed: String = format!("ootrr debug pick {tag} {n}");
        let s1: String = format!("{seed} link");
        let s2: String = format!("{seed} name");
        let s3: String = format!("{seed} set");
        let link: usize = a.add_node(&Spec::query(OWN_LINK, &s1).out_ptr("Actor", &class))?;
        let name: usize = a.add_node(&Spec::query(ACTOR_NAME, &s2).linked_ptr("Actor", &class, link).spare(3, "ActorFileName").out(3, "ActorInstanceName").spare(3, "ActorName"))?;
        let key: Vec<u8> = a.pool.add(PICK).to_le_bytes().to_vec();
        let set: usize = a.add_node(
            &Spec::node(SET_STRING, &s3)
                .prop(1, "ExecuteEveryFrame", vec![0; 4])
                .constant(0, "GameDataIndex", (-1i32).to_le_bytes().to_vec())
                .constant(3, "GameDataFullName", key)
                .linked(3, "Value", name, 1),
        )?;
        let at: usize = a.params[kid].pl[2][0] as usize;
        let nm: u32 = a.pool.add("");
        a.params[kid].add_plug(2, at, Plug { a: set as u32, b: nm, extra: Vec::new() })?;
    }
    let bad: Vec<String> = audit(&a);
    if !bad.is_empty() {
        return Err(E::Msg(format!("debug names: {tag}: {}", bad.join("; "))));
    }
    let out: Vec<u8> = a.write()?;
    let back: Ainb = Ainb::parse(&out)?;
    if back.write()? != out {
        return Err(E::Msg(format!("debug names: {tag} does not read back")));
    }
    Ok(Some(out))
}

fn work_path(f: &str) -> String {
    format!("Work/{f}.gyml")
}

pub fn press(es: &mut Vec<sarc::Entry>, clients: &[sarc::Entry]) -> Res<bool> {
    let mut hit: bool = false;
    for e in es.iter_mut().filter(|e| e.name.contains("AttentionSenderParam/") && e.name.ends_with(".bgyml")) {
        let mut y: Byml = Byml::parse(&e.data)?;
        let auto: bool = y.root.get("ClientList").and_then(|l| l.as_arr()).is_some_and(|l| l.iter().any(|c| c.as_str().is_some_and(|s| s.contains(AUTO_CLIENT))));
        if !auto {
            continue;
        }
        y.root.set("ClientList", Node::Arr(PRESS_CLIENTS.iter().map(|c| Node::Str(work_path(c))).collect()));
        e.data = y.write()?;
        hit = true;
    }
    if hit {
        for c in clients {
            if sarc::get(es, &c.name).is_none() {
                es.push(c.clone());
            }
        }
    }
    let pressed: bool = es.iter().filter(|e| e.name.contains("AttentionSenderParam/") && e.name.ends_with(".bgyml")).any(|e| Byml::parse(&e.data).ok().and_then(|y| y.root.get("ClientList").and_then(|l| l.as_arr()).map(|l| l.iter().any(|c| c.as_str().is_some_and(|s| s.contains("/Item_Catch."))))).unwrap_or(false));
    if !pressed {
        return Ok(hit);
    }
    for e in es.iter_mut().filter(|e| e.name.contains("PouchContentParam/") && e.name.ends_with(".bgyml")) {
        let mut y: Byml = Byml::parse(&e.data)?;
        let want: [(&str, bool); 3] = [("DefaultAutoGet", false), ("PlayGetEventManyTimes", true), ("IsPlayGetEventPouchMax", true)];
        let typed: bool = y.root.get(GET_TYPE).is_some();
        if typed && want.iter().all(|(k, v)| y.root.get(k) == Some(&Node::Bool(*v))) {
            continue;
        }
        for (k, v) in want {
            y.root.set(k, Node::Bool(v));
        }
        if !typed {
            y.root.set(GET_TYPE, Node::U32(crate::flags::hash_name(GET_MEDIUM)));
        }
        e.data = y.write()?;
        hit = true;
    }
    Ok(hit)
}

pub fn press_clients(rf: &mut Romfs) -> Res<Vec<sarc::Entry>> {
    let raw: Vec<u8> = rf.read_zs(&crate::pickup_pack::pack_of(PRESS_FROM))?;
    let es: Vec<sarc::Entry> = sarc::parse(&raw)?;
    PRESS_CLIENTS
        .iter()
        .map(|c| sarc::get(&es, &format!("{c}.bgyml")).cloned().ok_or_else(|| E::Msg(format!("debug names: {PRESS_FROM} has no {c}"))))
        .collect()
}

pub fn patch_pack(raw: &[u8], tag: &str, clients: &[sarc::Entry]) -> Res<Option<Vec<u8>>> {
    let mut es: Vec<sarc::Entry> = sarc::parse(raw)?;
    let mut hit: bool = !clients.is_empty() && press(&mut es, clients)?;
    for e in es.iter_mut().filter(|e| e.name.ends_with(".event.root.ainb")) {
        if let Some(out) = patch_pick(&e.data, &format!("{tag} {}", e.name))? {
            e.data = out;
            hit = true;
        }
    }
    if hit {
        Ok(Some(sarc::write(&es)?))
    } else {
        Ok(None)
    }
}

fn edit_pack(rf: &mut Romfs, tail: &mut Vec<Out>, work: &P, path: &str, f: impl FnOnce(&[u8]) -> Res<Vec<u8>>) -> Res<()> {
    let at = path.split('/').fold(work.to_path_buf(), |a, s| a.join(s));
    let z: Vec<u8> = match tail.iter().find(|o| o.path == path) {
        Some(o) => o.z.clone(),
        None => match fs::read(&at) {
            Ok(z) => z,
            Err(_) => rf.read(path)?,
        },
    };
    let id: Option<u32> = crate::zs::dict_of(&z);
    let raw: Vec<u8> = rf.zs.decompress(&z)?;
    let out: Vec<u8> = f(&raw)?;
    let o: Out = Out { path: path.to_string(), z: crate::zs::compress_raw(&out, id)?, old: raw.len(), new: out.len() };
    match tail.iter().position(|x| x.path == path) {
        Some(i) => tail[i] = o,
        None => tail.push(o),
    }
    Ok(())
}

pub fn pick_name(h: u64) -> String {
    format!("OOTRRDbg{h}")
}

pub fn miss_label(h: u64) -> String {
    format!("{MISS_PREFIX}{h}")
}

fn miss_text(g: &str, h: u64, f: &str) -> String {
    let short: &str = f.trim_start_matches("Banc/").trim_end_matches(".bcett.byml.zs");
    format!("Not in database
{g} {h}
{short}")
}

pub fn stage_sweep(rf: &mut Romfs, d: &Data, work: &P, tail: &mut Vec<Out>) -> Res<(usize, usize)> {
    let gs: Vec<(u64, usize)> = skulls(d);
    let mut listed: BM<u64, usize> = BM::new();
    for (i, l) in d.cat.locs.iter().enumerate().filter(|(_, l)| PICKUP_KINDS.contains(&l.kind.as_str())) {
        for r in &l.refs {
            listed.insert(r.hash, i);
        }
    }
    let known: BS<u64> = d.cat.locs.iter().flat_map(|l| l.refs.iter().map(|r| r.hash)).collect();
    let files: Vec<String> = rf.list("Banc")?.into_iter().filter(|f| f.ends_with(".bcett.byml.zs")).collect();
    let mut picks: BM<String, String> = BM::new();
    let mut lines: BM<String, String> = BM::new();
    let mut seen: BS<String> = ITEM_KINDS.iter().map(|g| g.to_string()).collect();
    for f in &files {
        let raw: Vec<u8> = current(rf, tail, work, f)?;
        let y: Byml = Byml::parse(&raw)?;
        let Some(list) = y.root.get("Actors").and_then(|x| x.as_arr()) else { continue };
        let mut names: BM<u64, String> = BM::new();
        for a in list {
            let Some(h) = hash_of(a) else { continue };
            let g: &str = text(a, "Gyaml");
            if g.starts_with(CHEST) {
                continue;
            }
            let label: String = match listed.get(&h) {
                Some(i) => {
                    lines.insert(line_label(*i), d.cat.locs[*i].name.clone());
                    line_label(*i)
                }
                None if !known.contains(&h) && ITEM_KINDS.contains(&g) => {
                    lines.entry(miss_label(h)).or_insert_with(|| miss_text(g, h, f));
                    miss_label(h)
                }
                None => continue,
            };
            let name: String = match a.get("Name").and_then(|v| v.as_str()) {
                Some(n) if !n.is_empty() => n.to_string(),
                _ => pick_name(h),
            };
            seen.insert(g.to_string());
            picks.insert(name.clone(), label);
            names.insert(h, name);
        }
        if names.is_empty() {
            continue;
        }
        stage_late_file(rf, tail, work, f, |raw: &[u8]| {
            let mut y: Byml = Byml::parse(raw)?;
            if let Some(list) = actors(&mut y) {
                for a in list.iter_mut() {
                    let Some(h) = hash_of(a) else { continue };
                    if let Some(n) = names.get(&h)
                        && a.get("Name").is_none()
                    {
                        a.set("Name", Node::Str(n.clone()));
                    }
                }
            }
            y.write()
        })?;
    }
    let clients: Vec<sarc::Entry> = press_clients(rf)?;
    let mut packs: Vec<(String, bool)> = vec![(GLOBAL_AI.to_string(), false)];
    packs.extend(seen.iter().map(|g| (crate::pickup_pack::pack_of(g), true)));
    for (p, own) in &packs {
        let cl: &[sarc::Entry] = if *own { &clients } else { &[] };
        let Ok(raw) = current(rf, tail, work, p) else { continue };
        if patch_pack(&raw, p, cl)?.is_none() {
            continue;
        }
        edit_pack(rf, tail, work, p, |raw: &[u8]| Ok(patch_pack(raw, p, cl)?.unwrap_or_else(|| raw.to_vec())))?;
    }
    let mut added: Vec<u64> = Vec::new();
    stage_late_file(rf, tail, work, crate::chest_slot::LIST, |raw: &[u8]| {
        let (b, a) = add_shown(raw, &gs)?;
        added = a;
        add_pick_flag(&b)
    })?;
    let live: Vec<(u64, usize)> = gs.into_iter().filter(|x| added.contains(&x.0)).collect();
    let pl: Vec<(String, String)> = picks.into_iter().collect();
    stage_late(rf, tail, work, GET_FLOW, |b| add_sweep(b, &live, &pl).map(|_| ()))?;
    if !pl.is_empty() {
        for stem in ADD_FLOWS {
            stage_late(rf, tail, work, stem, |b| sweep_after_add(b).map(|_| ()))?;
        }
    }
    lines.extend(live.iter().map(|(_, i)| (line_label(*i), d.cat.locs[*i].name.clone())));
    lines.insert(MISS_SPAWN.to_string(), MISS_SPAWN_TEXT.to_string());
    for l in crate::hints::languages(rf)? {
        stage_late_file(rf, tail, work, &l, |raw: &[u8]| put_lines(raw, &lines))?;
    }
    Ok((live.len(), pl.len()))
}

fn case_to(k: &Kind, val: u32) -> Option<u16> {
    match k {
        Kind::Switch { cases, .. } => cases.iter().find(|c| c.val == val).map(|c| c.ev),
        _ => None,
    }
}

pub fn hook_sale(b: &mut Bf, entry: &str, id: &str) -> Res<bool> {
    let Some(f) = b.flows.iter_mut().find(|f| f.name == SCRUB_FLOW) else { return Ok(false) };
    let Some(gate) = f.entries.iter().find(|e| e.name == entry).map(|e| e.main) else { return Ok(false) };
    let Some(sub) = case_to(&f.events[usize::from(gate)].kind, 1) else { return Ok(false) };
    let chk: u16 = match &f.events[usize::from(sub)].kind {
        Kind::Sub { next, .. } => *next,
        _ => return Ok(false),
    };
    let Some(set) = case_to(&f.events[usize::from(chk)].kind, 0) else { return Ok(false) };
    say_after(f, usize::from(set), id.to_string())
}
