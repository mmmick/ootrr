use crate::ainb::Ainb;
use crate::bfevfl::flow::{Event, NONE};
use crate::bfevfl::{Actor, Bf, Case, Cont, Flow, Kind, Val};
use crate::byml::{Byml, Node};
use crate::difficulty::ambush::{actor, euler, fresh, list};
use crate::error::{Error, Res};
use crate::flow_locations::dict_id;
use crate::msbt::{self, Msbt};
use crate::qol::navi_return::{CURRENT, HOME_CHILD};
use crate::qol::{edit_plain, edit_zs, Have};
use crate::romfs::Romfs;
use crate::sarc;
use crate::staging::{Out, BANC_DICT};
use crate::zs;

pub const SETTING: &str = "owl_warps";
pub const NUM: u16 = 153;
pub const FILE: &str = "Event/EventFlow/Npc_Kaepora_DeathMountainTrail.bfevfl.zs";
pub const FLOW: &str = "Npc_Kaepora_DeathMountainTrail";
pub const MSG: &str = "EventFlowMsg/Npc_Kaepora_DeathMountainTrail.msbt";
pub const MSG_ID: &str = "EventFlowMsg/Npc_Kaepora_DeathMountainTrail";
pub const OWL: &str = "Npc_Kaepora_DeathMountainTrail";
pub const LAKE_OWL: &str = "Npc_Kaepora_LakeHylia";
pub const PRESENCE: &str = "Presence";
pub const SEED: &str = "ootrr owl warps";
pub const PREFIX: &str = "OOTRROwl";
pub const LABEL: &str = "OOTRR_Owl_";
pub const NOTE: &str = "The owls on the Death Mountain Trail and at Lake Hylia stay for the whole game, so you can ride them again and again, and the Death Mountain Trail owl offers his ride to Kakariko Village every time, with or without Din's Fire. A new owl stands in Kakariko Village and carries you to Kokiri Forest. The rides are for a child Link, as in the original game.";

pub const KAKARIKO_SPOT: [f32; 3] = [-49.33, 118.84, 21.58];
pub const KAKARIKO_TURN: f32 = 2.1206;

pub const STAY: [(&str, &str); 2] = [("DeathMountainTrail", "DeathMountainTrail_Child"), ("HyruleField", "HyruleField_Child")];
pub const LAKE_LOGIC: &str = "Logic/HyruleField_LakeHylia_Child_Prefab_32f1.logic.module.ainb";
pub const LAKE_READ: &str = "GameDataNameBool";
pub const LAKE_ELEMENT: &str = "HoldGameDataGetBoolSimple";
pub const LEFT: &str = "HavePlayedEvent.Dm_TimeTemple_0086";
pub const NEVER: &str = "HavePlayedEvent.Dm_Ending_0000";
const STR: usize = 3;
pub const STAY_OWLS: [&str; 2] = [OWL, LAKE_OWL];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ride {
    pub name: &'static str,
    pub dir: &'static str,
    pub map: &'static str,
    pub at: [f32; 3],
    pub turn: f32,
    pub to: (&'static str, &'static str),
    pub ask: &'static str,
}

pub const RIDES: [Ride; 1] = [Ride {
    name: "kakariko_to_kokiri_forest",
    dir: "KakarikoVillage",
    map: "KakarikoVillage_Child",
    at: KAKARIKO_SPOT,
    turn: KAKARIKO_TURN,
    to: HOME_CHILD,
    ask: "{3.0:0f0000}If you're heading for the forest, I can carry\nyou as far as Kokiri Forest. {5.0}What do you say?",
}];

pub fn file(dir: &str, map: &str) -> String {
    format!("Banc/{dir}/{map}_Static.bcett.byml.zs")
}

pub fn dynamic(dir: &str, map: &str) -> String {
    format!("Banc/{dir}/{map}_Dynamic.bcett.byml.zs")
}

pub fn label(i: usize) -> String {
    format!("{LABEL}{i}")
}

pub fn files() -> Vec<String> {
    let mut out: Vec<String> = vec![FILE.to_string(), LAKE_LOGIC.to_string()];
    for (d, m) in STAY {
        out.push(file(d, m));
    }
    for r in RIDES {
        let f: String = file(r.dir, r.map);
        if !out.contains(&f) {
            out.push(f);
        }
    }
    out
}

pub fn lake_logic(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    let hits: Vec<usize> = a.find_els(LAKE_ELEMENT).into_iter().filter(|&i| a.in_str(i, LAKE_READ) == Some(LEFT)).collect();
    let [i] = hits.as_slice() else {
        return Err(Error::Format("owl warps: the Lake Hylia owl logic has no single Temple of Time owl check, the file is edited or differs"));
    };
    let g: usize = a.find_in(*i, STR, LAKE_READ).ok_or(Error::Format("owl warps: the Lake Hylia owl check has no flag input"))?;
    let o: u32 = a.pool.add(NEVER);
    a.set_in_val(STR, g, &o.to_le_bytes())?;
    let out: Vec<u8> = a.write()?;
    if Ainb::parse(&out)? != a {
        return Err(Error::Format("owl warps: the edited Lake Hylia owl logic does not parse back"));
    }
    Ok(out)
}

fn source(rf: &Romfs, have: Have, p: &str) -> Res<Vec<u8>> {
    match have(p) {
        Some(b) => Ok(b),
        None => rf.read(p),
    }
}

pub fn keep(y: &mut Byml) -> Res<usize> {
    let acts: &mut Vec<Node> = list(y, "Actors")?;
    let mut n: usize = 0;
    for a in acts.iter_mut() {
        let owl: bool = a.get("Gyaml").and_then(|g| g.as_str()).is_some_and(|g| STAY_OWLS.contains(&g));
        if owl && a.remove(PRESENCE) {
            n += 1;
        }
    }
    Ok(n)
}

pub fn place(y: &mut Byml, r: &Ride, raws: &[&[u8]]) -> Res<()> {
    let h: u64 = fresh(raws, &format!("{SEED} {}", r.name));
    let a: Node = actor(OWL, h, r.at, euler(0.0, r.turn));
    list(y, "Actors")?.push(a);
    Ok(())
}

fn map_out(rf: &mut Romfs, have: Have, dir: &str, map: &str, f: impl FnOnce(&mut Byml, &[&[u8]]) -> Res<()>) -> Res<Out> {
    let p: String = file(dir, map);
    let z: Vec<u8> = source(rf, have, &p)?;
    let dict: u32 = dict_id(&z).unwrap_or(BANC_DICT);
    let raw: Vec<u8> = rf.zs.decompress(&z)?;
    let land: Vec<u8> = rf.zs.decompress(&source(rf, have, &dynamic(dir, map))?)?;
    let mut y: Byml = Byml::parse(&raw)?;
    f(&mut y, &[&raw, &land])?;
    let bytes: Vec<u8> = y.write()?;
    Ok(Out { path: p, old: raw.len(), new: bytes.len(), z: zs::compress_raw(&bytes, Some(dict))? })
}

pub fn stage_maps(rf: &mut Romfs, have: Have) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    let mut dirs: Vec<(&str, &str)> = STAY.to_vec();
    for r in RIDES {
        if !dirs.contains(&(r.dir, r.map)) {
            dirs.push((r.dir, r.map));
        }
    }
    for (dir, map) in dirs {
        let stays: bool = STAY.contains(&(dir, map));
        let here: Vec<Ride> = RIDES.iter().filter(|r| r.dir == dir && r.map == map).copied().collect();
        outs.push(map_out(rf, have, dir, map, |y, raws| {
            if stays && keep(y)? == 0 {
                return Err(Error::Format("owl warps: the map has no owl with a presence flag, the game files differ"));
            }
            for r in &here {
                place(y, r, raws)?;
            }
            Ok(())
        })?);
    }
    Ok(outs)
}

fn idx(n: usize) -> Res<u16> {
    u16::try_from(n).ok().filter(|&x| x != NONE).ok_or(Error::Format("owl warps: too many events in the flow"))
}

fn push(f: &mut Flow, kind: Kind) -> Res<u16> {
    let i: u16 = idx(f.events.len())?;
    f.events.push(Event { name: format!("{PREFIX}{i}"), kind });
    Ok(i)
}

fn named(f: &Flow, name: &str) -> Res<usize> {
    f.events.iter().position(|e| e.name == name).ok_or(Error::Format("owl warps: the owl flow differs from the game's"))
}

fn two(yes: u16, no: u16) -> Vec<Case> {
    vec![Case { val: 0, ev: yes }, Case { val: 1, ev: no }]
}

fn member(f: &mut Flow, actor: &str, name: &str, query: bool) -> Res<(u16, u16)> {
    let i: usize = f.actors.iter().position(|a| a.name == actor && a.sub.is_empty()).ok_or(Error::Format("owl warps: the owl flow lacks an actor it needs"))?;
    let a: &mut Actor = &mut f.actors[i];
    let l: &mut Vec<String> = if query { &mut a.queries } else { &mut a.actions };
    let j: usize = match l.iter().position(|s| s == name) {
        Some(j) => j,
        None => {
            l.push(name.to_string());
            l.len() - 1
        }
    };
    Ok((idx(i)?, idx(j)?))
}

fn with_next(k: &Kind, to: u16) -> Res<Kind> {
    match k.clone() {
        Kind::Action { actor, act, params, .. } => Ok(Kind::Action { next: to, actor, act, params }),
        Kind::Sub { params, flow, entry, .. } => Ok(Kind::Sub { next: to, params, flow, entry }),
        _ => Err(Error::Format("owl warps: an owl event is not an action or a sub call")),
    }
}

fn with_text(k: &Kind, text: &str, to: u16) -> Res<Kind> {
    let mut k: Kind = with_next(k, to)?;
    let Some(p) = k.params_mut() else {
        return Err(Error::Format("owl warps: the owl question has no parameters"));
    };
    let Some(i) = p.items.iter().position(|(n, _)| n == "MessageId") else {
        return Err(Error::Format("owl warps: the owl question has no message"));
    };
    p.items[i].1 = Val::Str(format!("{MSG_ID}:{text}"));
    Ok(k)
}

fn warp(banc: &str, start: &str) -> Cont {
    let items: Vec<(&str, Val)> = vec![
        ("FadePattern", Val::Int(1)),
        ("IsWaitEnd", Val::Bool(1)),
        ("NextPosition", Val::Floats(vec![0.0, 0.0, 0.0])),
        ("NextStartRot", Val::Floats(vec![0.0, 0.0, 0.0])),
        ("BootEventName", Val::Str(String::new())),
        ("IsStartOnEvent", Val::Bool(0)),
        ("NextBanc", Val::Str(banc.to_string())),
        ("NextStartPos", Val::Str(start.to_string())),
        ("EventStarterName", Val::Str(String::new())),
        ("IsItemWarp", Val::Bool(0)),
    ];
    Cont { items: items.into_iter().map(|(k, v)| (k.to_string(), v)).collect() }
}

pub fn patch_flow(b: &mut Bf) -> Res<usize> {
    let f: &mut Flow = b.flows.iter_mut().find(|f| f.name == FLOW).ok_or(Error::Format("owl warps: the Death Mountain owl flow is not in the file"))?;
    if f.events.iter().any(|e| e.name.starts_with(PREFIX)) {
        return Err(Error::Format("owl warps: the owl flow is already edited"));
    }
    let before: usize = f.events.len();
    let mask: usize = named(f, "Event41")?;
    let offer: usize = named(f, "Event13")?;
    let choice: usize = named(f, "Event14")?;
    let decline: usize = named(f, "Event54")?;
    let yes: usize = named(f, "Event0")?;
    let boarding: usize = named(f, "Event15")?;
    match &f.events[mask].kind {
        Kind::Sub { entry, .. } if entry == "Mask_All" => {}
        _ => return Err(Error::Format("owl warps: the owl's mask talk is not where the game puts it, the flow is already edited or differs")),
    }
    let q: (u16, u16) = member(f, "GameDataActor", "EventQueryIsGameDataString64", true)?;
    let w: (u16, u16) = member(f, "GameSystemActor", "EventReserveBancChange", false)?;
    let mut tail: u16 = idx(offer)?;
    for (i, r) in RIDES.iter().enumerate().rev() {
        let go: u16 = push(f, Kind::Action { next: NONE, actor: w.0, act: w.1, params: Some(warp(r.to.0, r.to.1)) })?;
        let board: u16 = push(f, with_next(&f.events[boarding].kind.clone(), go)?)?;
        let sure: u16 = push(f, with_next(&f.events[yes].kind.clone(), board)?)?;
        let (actor, query, params): (u16, u16, Option<Cont>) = match &f.events[choice].kind {
            Kind::Switch { actor, query, params, .. } => (*actor, *query, params.clone()),
            _ => return Err(Error::Format("owl warps: the owl's answer is not a switch")),
        };
        let pick: u16 = push(f, Kind::Switch { actor, query, cases: two(sure, idx(decline)?), params })?;
        let ask: u16 = push(f, with_text(&f.events[offer].kind.clone(), &label(i), pick)?)?;
        let c: Cont = Cont { items: vec![("Index".to_string(), Val::Int(-1)), ("GameDataName".to_string(), Val::Str(CURRENT.to_string())), ("Value".to_string(), Val::Str(r.map.to_string()))] };
        tail = push(f, Kind::Switch { actor: q.0, query: q.1, cases: two(ask, tail), params: Some(c) })?;
        let calls: Vec<u16> = vec![sure];
        for e in f.entries.iter_mut().filter(|e| e.name == "Talk") {
            for c in &calls {
                if !e.subs.contains(c) {
                    e.subs.push(*c);
                }
            }
        }
    }
    match &mut f.events[mask].kind {
        Kind::Sub { next, .. } => *next = tail,
        _ => return Err(Error::Format("owl warps: the owl's mask talk is not a sub call")),
    }
    Ok(f.events.len() - before)
}

pub fn patch(raw: &[u8]) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    patch_flow(&mut b)?;
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(Error::Format("owl warps: the written flow does not parse back to the edited model"));
    }
    Ok(out)
}

pub fn apply_text(raw: &[u8]) -> Res<Vec<u8>> {
    let mut es: Vec<sarc::Entry> = sarc::parse(raw)?;
    let mut m: Msbt = msbt::parse(&sarc::get(&es, MSG).ok_or(Error::Format("owl warps: the language archive has no owl texts"))?.data)?;
    for (i, r) in RIDES.iter().enumerate() {
        m.put_markup(&label(i), r.ask)?;
    }
    sarc::set(&mut es, MSG, m.write()?);
    sarc::write(&es)
}

pub fn stage(rf: &mut Romfs, have: Have, outs: &mut Vec<Out>) -> Res<()> {
    outs.push(edit_zs(rf, have, FILE, patch)?);
    outs.push(edit_plain(rf, have, LAKE_LOGIC, lake_logic)?);
    outs.extend(stage_maps(rf, have)?);
    for l in crate::hints::languages(rf)? {
        match outs.iter().position(|o| o.path == l) {
            Some(i) => {
                let id: Option<u32> = dict_id(&outs[i].z);
                let raw: Vec<u8> = rf.zs.decompress(&outs[i].z)?;
                let text: Vec<u8> = apply_text(&raw)?;
                outs[i] = Out { path: l, old: outs[i].old, new: text.len(), z: zs::compress_raw(&text, id)? };
            }
            None => outs.push(edit_zs(rf, have, &l, apply_text)?),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qol::Opts;

    fn root() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        if p.is_dir() { Some(p) } else { None }
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    fn name(f: &Flow, a: u16, q: u16, query: bool) -> String {
        let x: &Actor = &f.actors[a as usize];
        format!("{}.{}", x.name, if query { &x.queries[q as usize] } else { &x.actions[q as usize] })
    }

    fn message(k: &Kind) -> Option<String> {
        match k.params()?.get("MessageId") {
            Some(Val::Str(s)) => Some(s.clone()),
            _ => None,
        }
    }

    fn walk(f: &Flow, map: &str, yes: bool) -> Option<(Vec<String>, Option<(String, String)>, bool)> {
        let mut i: u16 = f.entries.iter().find(|e| e.name == "Talk")?.main;
        let mut said: Vec<String> = Vec::new();
        let mut asked: bool = false;
        for _ in 0..200 {
            if i == NONE {
                return Some((said, None, asked));
            }
            let e: &Event = f.events.get(i as usize)?;
            if let Some(m) = message(&e.kind) {
                said.push(m);
            }
            match &e.kind {
                Kind::Switch { actor, query, cases, params } => {
                    let q: String = name(f, *actor, *query, true);
                    let pick: usize = match q.as_str() {
                        "GameDataActor.EventQueryIsGameDataString64" => {
                            let c: &Cont = params.as_ref()?;
                            if c.get("Value") == Some(&Val::Str(map.to_string())) { 0 } else { 1 }
                        }
                        "GameSystemActor.EventQueryTalkChoice2" => {
                            asked = true;
                            if yes { 0 } else { 1 }
                        }
                        _ => return None,
                    };
                    i = cases.iter().find(|c| c.val as usize == pick)?.ev;
                }
                Kind::Action { next, actor, act, params } => {
                    if name(f, *actor, *act, false) == "GameSystemActor.EventReserveBancChange" {
                        let c: &Cont = params.as_ref()?;
                        let s = |k: &str| match c.get(k) {
                            Some(Val::Str(s)) => s.clone(),
                            _ => String::new(),
                        };
                        return Some((said, Some((s("NextBanc"), s("NextStartPos"))), asked));
                    }
                    i = *next;
                }
                Kind::Sub { next, .. } => i = *next,
                _ => return None,
            }
        }
        None
    }

    #[test]
    fn the_table_names_each_ride_once_and_the_setting_is_a_new_number() {
        for (i, r) in RIDES.iter().enumerate() {
            assert_eq!(RIDES.iter().filter(|x| x.name == r.name).count(), 1);
            assert!(r.map.starts_with(r.dir) && r.map.ends_with("_Child"));
            assert!(label(i).starts_with(LABEL));
        }
        assert_eq!(RIDES[0].at, KAKARIKO_SPOT);
        assert_eq!(NUM, 153);
        assert!(files().contains(&FILE.to_string()));
        assert_eq!(files().len(), 5);
    }

    #[test]
    fn the_owl_flow_offers_every_ride_and_each_path_ends_in_the_games_own_way() {
        let Some(root) = root() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.zs.decompress(&rf.read(FILE).unwrap()).unwrap();
        let out: Vec<u8> = patch(&raw).unwrap();
        assert!(patch(&out).is_err());
        let b: Bf = Bf::parse(&out).unwrap();
        let f: &Flow = &b.flows[0];
        let mask: usize = named(f, "Event41").unwrap();
        for e in f.entries.iter().filter(|e| e.name == "Talk") {
            for s in &e.subs {
                assert!(matches!(f.events[*s as usize].kind, Kind::Sub { .. }));
            }
        }
        assert!(matches!(f.events[mask].kind, Kind::Sub { .. }));
        let (said, to, asked) = walk(f, "KakarikoVillage_Child", true).unwrap();
        assert_eq!(to, Some((HOME_CHILD.0.to_string(), HOME_CHILD.1.to_string())));
        assert!(asked && said[0].ends_with(&label(0)) && said.last().unwrap().ends_with(":Talk_05"));
        let (said, to, asked) = walk(f, "KakarikoVillage_Child", false).unwrap();
        assert!(to.is_none() && asked && said.last().unwrap().ends_with(":Talk_06"));
        let (said, to, asked) = walk(f, "DeathMountainTrail_Child", true).unwrap();
        assert!(asked && said[0].ends_with(":Talk_04") && said.last().unwrap().ends_with(":Talk_05"));
        assert_eq!(to.map(|t| t.0), Some("KakarikoVillage_Child".to_string()));
        let (said, to, _) = walk(f, "DeathMountainTrail_Child", false).unwrap();
        assert!(to.is_none() && said.last().unwrap().ends_with(":Talk_06"));
    }

    #[test]
    fn the_maps_keep_the_two_owls_and_gain_the_new_one() {
        let Some(root) = root() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let outs: Vec<Out> = stage_maps(&mut rf, &none).unwrap();
        assert_eq!(outs.len(), 3);
        for o in &outs {
            let was: Byml = rf.read_byml(&o.path).unwrap();
            let now: Byml = Byml::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap();
            let owls = |y: &Byml, n: &str| -> Vec<bool> { y.root.get("Actors").and_then(|a| a.as_arr()).unwrap().iter().filter(|a| a.get("Gyaml").and_then(|g| g.as_str()) == Some(n)).map(|a| a.get(PRESENCE).is_some()).collect() };
            for n in STAY_OWLS {
                assert!(owls(&now, n).iter().all(|p| !p));
            }
            let extra: usize = owls(&now, OWL).len() - owls(&was, OWL).len();
            assert_eq!(extra, if o.path.contains("KakarikoVillage") { 1 } else { 0 });
            assert_eq!(now.root.get("Actors").and_then(|a| a.as_arr()).unwrap().len(), was.root.get("Actors").and_then(|a| a.as_arr()).unwrap().len() + extra);
        }
    }

    #[test]
    fn the_lake_logic_reads_another_flag_and_nothing_else_changes() {
        let Some(root) = root() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read(LAKE_LOGIC).unwrap();
        let out: Vec<u8> = lake_logic(&raw).unwrap();
        assert!(lake_logic(&out).is_err());
        let (a, b): (Ainb, Ainb) = (Ainb::parse(&raw).unwrap(), Ainb::parse(&out).unwrap());
        assert_eq!(a.els, b.els);
        let reads = |x: &Ainb| -> Vec<String> { x.find_els(LAKE_ELEMENT).into_iter().filter_map(|i| x.in_str(i, LAKE_READ).map(|s| s.to_string())).collect() };
        let (before, after): (Vec<String>, Vec<String>) = (reads(&a), reads(&b));
        assert_eq!(before.len(), after.len());
        assert!(before.contains(&LEFT.to_string()) && !after.contains(&LEFT.to_string()) && after.contains(&NEVER.to_string()));
        assert_eq!(before.iter().zip(&after).filter(|(x, y)| x != y).count(), 1);
    }

    #[test]
    fn the_game_builds_with_the_owls_and_without() {
        let Some(root) = root() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let off: Vec<Out> = crate::qol::stage(&mut rf, &Opts::default(), &none).unwrap();
        assert!(off.iter().all(|o| o.path != FILE));
        let on: Vec<Out> = crate::qol::stage(&mut rf, &Opts { owls: true, ..Opts::default() }, &none).unwrap();
        for f in files() {
            assert!(on.iter().any(|o| o.path == f), "{f} not staged");
        }
        let langs: usize = crate::hints::languages(&rf).unwrap().len();
        assert_eq!(on.len() - off.len(), 1 + 1 + 3 + langs);
        let both: Vec<Out> = crate::qol::stage(&mut rf, &Opts { owls: true, navi: crate::qol::navi_return::Mode::Everywhere, ..Opts::default() }, &none).unwrap();
        let mut paths: Vec<&String> = both.iter().map(|o| &o.path).collect();
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), both.len());
        for l in crate::hints::languages(&rf).unwrap() {
            let o: &Out = both.iter().find(|o| o.path == l).unwrap();
            let es: Vec<sarc::Entry> = sarc::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap();
            let owl: Msbt = msbt::parse(&sarc::get(&es, MSG).unwrap().data).unwrap();
            let navi: Msbt = msbt::parse(&sarc::get(&es, crate::qol::navi_return::MSG).unwrap().data).unwrap();
            assert!(owl.find(&label(0)).is_some() && navi.find(crate::qol::navi_return::ASK_CHILD).is_some(), "{l}");
        }
    }
}
