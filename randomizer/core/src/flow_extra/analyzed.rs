use std::collections::{BTreeMap as BM, BTreeSet as BS, HashMap as HM};
use std::fs;
use std::path::Path as P;

use crate::bfevfl::flow::NONE;
use crate::bfevfl::{Bf, Flow, Kind, Val};
use crate::byml::{Byml, Node};
use crate::error::{Error as E, Res};
use crate::event_chests::resolve::{row_of, Row};
use crate::flow_extra::sim::Over;
use crate::flow_sim::load::{Disk, Src};
use crate::progressive::{ANALYZED_TABLE, SETTING_TABLE};
use crate::qol::{edit_zs, Have};
use crate::romfs::Romfs;
use crate::staging::Out;

pub const SUBS: &str = "SubFlowList";
pub const KIDS: &str = "ChildEventList";
const MOVER: &str = "EventSystemActor";
const MOVE: &str = "EventTransition";
const NAME: &str = "EventName";
const FLOWS: &str = "Event/EventFlow";
const EXT: &str = ".bfevfl.zs";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Need {
    pub flows: BS<String>,
    pub kids: BS<String>,
}

fn targets(k: &Kind) -> Vec<u16> {
    match k {
        Kind::Action { next, .. } | Kind::Join { next } | Kind::Sub { next, .. } => vec![*next],
        Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => cases.iter().map(|c| c.ev).collect(),
        Kind::Fork { join, forks } => forks.iter().copied().chain([*join]).collect(),
        Kind::Label { .. } => Vec::new(),
    }
}

fn calls(k: &Kind) -> Option<(&str, &str)> {
    match k {
        Kind::Sub { flow, entry, .. } | Kind::SubSwitch { flow, entry, .. } => Some((flow.as_str(), entry.as_str())),
        _ => None,
    }
}

fn moves_to(f: &Flow, k: &Kind) -> Option<String> {
    let Kind::Action { actor, act, params, .. } = k else { return None };
    let a = f.actors.get(usize::from(*actor))?;
    if a.name != MOVER || a.actions.get(usize::from(*act)).map(|s| s.as_str()) != Some(MOVE) {
        return None;
    }
    match params.as_ref()?.get(NAME)? {
        Val::Str(s) => Some(s.clone()),
        _ => None,
    }
}

pub struct Reach<'a> {
    src: &'a mut dyn Src,
    cache: HM<String, Option<Flow>>,
}

impl<'a> Reach<'a> {
    pub fn new(src: &'a mut dyn Src) -> Reach<'a> {
        Reach { src, cache: HM::new() }
    }

    fn get(&mut self, name: &str) -> Res<Option<&Flow>> {
        if !self.cache.contains_key(name) {
            let f: Option<Flow> = self.src.flow(name)?;
            self.cache.insert(name.to_string(), f);
        }
        Ok(self.cache.get(name).and_then(|f| f.as_ref()))
    }

    pub fn need(&mut self, flow: &str, entry: &str) -> Res<Need> {
        let mut out: Need = Need::default();
        let mut seen: BS<(String, String)> = BS::new();
        let mut stack: Vec<(String, String)> = vec![(flow.to_string(), entry.to_string())];
        while let Some((fl, en)) = stack.pop() {
            if !seen.insert((fl.clone(), en.clone())) {
                continue;
            }
            let Some(f) = self.get(&fl)? else { continue };
            let Some(e) = f.entries.iter().find(|e| e.name == en) else { continue };
            let mut todo: Vec<u16> = vec![e.main];
            let mut vis: BS<u16> = BS::new();
            while let Some(i) = todo.pop() {
                if i == NONE || usize::from(i) >= f.events.len() || !vis.insert(i) {
                    continue;
                }
                let k: &Kind = &f.events[usize::from(i)].kind;
                if let Some((cf, ce)) = calls(k) {
                    let to: String = if cf.is_empty() { fl.clone() } else { cf.to_string() };
                    if to != flow {
                        out.flows.insert(to.clone());
                    }
                    stack.push((to, ce.to_string()));
                }
                if let Some(n) = moves_to(f, k) {
                    out.kids.insert(n);
                }
                todo.extend(targets(k));
            }
        }
        Ok(out)
    }

    /// What the event of `row` reaches. A scene a transition starts (no EntryPointName) can start
    /// at any entry of its flow, so it needs what every entry reaches.
    pub fn need_row(&mut self, row: &Row) -> Res<Need> {
        if !row.entry.is_empty() {
            return self.need(&row.stem, &row.entry);
        }
        let entries: Vec<String> = match self.get(&row.stem)? {
            Some(f) => f.entries.iter().map(|e| e.name.clone()).collect(),
            None => return Ok(Need::default()),
        };
        let mut out: Need = Need::default();
        for e in entries {
            let n: Need = self.need(&row.stem, &e)?;
            out.flows.extend(n.flows);
            out.kids.extend(n.kids);
        }
        Ok(out)
    }
}

fn id_of(n: &Node) -> &str {
    n.get("__RowId").and_then(|v| v.as_str()).unwrap_or("")
}

fn names(n: &Node, k: &str) -> Vec<String> {
    n.get(k).and_then(|v| v.as_arr()).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()).unwrap_or_default()
}

fn merge(have: &[String], add: &BS<String>) -> Vec<String> {
    let mut out: Vec<String> = have.to_vec();
    for a in add {
        if out.contains(a) {
            continue;
        }
        let low: String = a.to_lowercase();
        let at: usize = out.partition_point(|x| x.to_lowercase() < low);
        out.insert(at, a.clone());
    }
    out
}

fn list(v: Vec<String>) -> Node {
    Node::Arr(v.into_iter().map(Node::Str).collect())
}

pub type Added = BM<String, (Vec<String>, Vec<String>)>;

pub fn cover(y: &mut Byml, rows: &HM<String, Row>, r: &mut Reach, changed: &BS<String>) -> Res<Added> {
    let all: &mut Vec<Node> = y.root.as_arr_mut().ok_or(E::Format("analyzed events: the table root is not a list"))?;
    let mut out: Added = Added::new();
    for n in all.iter_mut() {
        let id: String = id_of(n).to_string();
        let Some(row) = rows.get(&id) else { continue };
        let subs: Vec<String> = names(n, SUBS);
        if !changed.contains(&row.stem) && !subs.iter().any(|s| changed.contains(s)) {
            continue;
        }
        let need: Need = r.need_row(row)?;
        let kids: Vec<String> = names(n, KIDS);
        let new_subs: BS<String> = need.flows.iter().filter(|x| !subs.contains(x)).cloned().collect();
        let new_kids: BS<String> = need.kids.iter().filter(|x| !kids.contains(x)).cloned().collect();
        if new_subs.is_empty() && new_kids.is_empty() {
            continue;
        }
        if !new_subs.is_empty() {
            n.set(SUBS, list(merge(&subs, &new_subs)));
        }
        if !new_kids.is_empty() {
            let mut k: Vec<String> = kids.clone();
            k.extend(new_kids.iter().cloned());
            n.set(KIDS, list(k));
        }
        out.insert(id, (new_subs.into_iter().collect(), new_kids.into_iter().collect()));
    }
    Ok(out)
}

pub fn settings(rf: &mut Romfs, have: Have) -> Res<HM<String, Row>> {
    let raw: Vec<u8> = match have(SETTING_TABLE) {
        Some(z) => rf.zs.decompress(&z)?,
        None => rf.read_zs(SETTING_TABLE)?,
    };
    let y: Byml = Byml::parse(&raw)?;
    Ok(y.root.as_arr().unwrap_or(&[]).iter().map(row_of).collect())
}

pub fn stage(rf: &mut Romfs, have: Have, flows: HM<String, Flow>) -> Res<Option<(Out, Added)>> {
    if flows.is_empty() {
        return Ok(None);
    }
    let rows: HM<String, Row> = settings(rf, have)?;
    let changed: BS<String> = flows.keys().cloned().collect();
    let mut src: Over = Over { disk: Disk::new(Romfs::open(&rf.root)?, None, Vec::new()), flows };
    let mut r: Reach = Reach::new(&mut src);
    let mut added: Added = Added::new();
    let o: Out = edit_zs(rf, have, ANALYZED_TABLE, |raw: &[u8]| {
        let mut y: Byml = Byml::parse(raw)?;
        added = cover(&mut y, &rows, &mut r, &changed)?;
        y.write()
    })?;
    if added.is_empty() {
        return Ok(None);
    }
    Ok(Some((o, added)))
}

pub fn worked(rf: &mut Romfs, root: &P) -> Res<Option<(Out, Added)>> {
    let flows: HM<String, Flow> = flows_in(rf, root)?;
    let have = |p: &str| -> Option<Vec<u8>> { fs::read(p.split('/').fold(root.to_path_buf(), |a, s| a.join(s))).ok() };
    stage(rf, &have, flows)
}

pub fn flows_in(rf: &mut Romfs, root: &P) -> Res<HM<String, Flow>> {
    let dir = root.join(FLOWS);
    let mut out: HM<String, Flow> = HM::new();
    let Ok(rd) = fs::read_dir(&dir) else { return Ok(out) };
    for e in rd {
        let p = e?.path();
        let Some(n) = p.file_name().and_then(|s| s.to_str()).and_then(|s| s.strip_suffix(EXT)) else { continue };
        if n.contains('.') {
            continue;
        }
        let mut b: Bf = Bf::parse(&rf.zs.decompress(&fs::read(&p)?)?)?;
        let i: usize = b.flows.iter().position(|f| f.name == n).unwrap_or(0);
        if i < b.flows.len() {
            out.insert(n.to_string(), b.flows.swap_remove(i));
        }
    }
    Ok(out)
}

pub fn flows_of(rf: &mut Romfs, files: &[(String, Vec<u8>)]) -> Res<HM<String, Flow>> {
    let mut out: HM<String, Flow> = HM::new();
    for (path, z) in files {
        let Some(n) = path.strip_prefix(&format!("{FLOWS}/")).and_then(|s| s.strip_suffix(EXT)) else { continue };
        if n.contains('.') {
            continue;
        }
        let mut b: Bf = Bf::parse(&rf.zs.decompress(z)?)?;
        let i: usize = b.flows.iter().position(|f| f.name == n).unwrap_or(0);
        if i < b.flows.len() {
            out.insert(n.to_string(), b.flows.swap_remove(i));
        }
    }
    Ok(out)
}

pub fn lines(a: &Added) -> Vec<String> {
    a.iter()
        .map(|(id, (s, k))| {
            let mut t: String = format!("  {id}:");
            if !s.is_empty() {
                t.push_str(&format!(" loads {}", s.join(", ")));
            }
            if !k.is_empty() {
                t.push_str(&format!(" starts {}", k.join(", ")));
            }
            t
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow_extra::sim::edited;
    use std::path::PathBuf as PB;


    fn root() -> Option<PB> {
        let p: PB = ootrr_game::test_romfs::path();
        if p.join("Event/EventFlow").is_dir() {
            Some(p)
        } else {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            None
        }
    }

    use crate::bfevfl::{Actor, Case, Cont, Entry, Event};
    use crate::flow_extra::sim::{beyond_always, stem};
    use crate::flow_extra::Extra;
    use crate::flow_sim::load::Mem;

    const NEW: &str = "Item_Longshot";
    const GET: &str = "Dm_SY_Get";

    fn actor(n: &str, acts: &[&str]) -> Actor {
        Actor { name: n.to_string(), sub: String::new(), arg: String::new(), actions: acts.iter().map(|s| s.to_string()).collect(), queries: Vec::new(), params: None, ep: 0xFFFF, cut: 1 }
    }

    fn ev(n: &str, kind: Kind) -> Event {
        Event { name: n.to_string(), kind }
    }

    fn sub(next: u16, flow: &str, entry: &str) -> Kind {
        Kind::Sub { next, params: None, flow: flow.to_string(), entry: entry.to_string() }
    }

    fn flow(name: &str, events: Vec<Event>, entries: Vec<(&str, u16)>) -> Flow {
        Flow {
            name: name.to_string(),
            actors: vec![actor(MOVER, &[MOVE])],
            events,
            entries: entries.into_iter().map(|(n, m)| Entry { name: n.to_string(), subs: Vec::new(), vars: Vec::new(), main: m, pad: 0 }).collect(),
        }
    }

    fn mem() -> Mem {
        let mut m: Mem = Mem::default();
        let go = Some(Cont { items: vec![(NAME.to_string(), Val::Str("Next_Event".to_string()))] });
        m.flows.insert(
            "Cow".to_string(),
            flow(
                "Cow",
                vec![
                    ev("E0", Kind::Switch { actor: 0, query: 0, cases: vec![Case { val: 0, ev: 1 }, Case { val: 1, ev: 2 }], params: None }),
                    ev("E1", sub(NONE, "", "Milk")),
                    ev("E2", sub(4, "Gift", "Give")),
                    ev("E3", sub(NONE, "Bottle", "Fill")),
                    ev("E4", Kind::Action { next: NONE, actor: 0, act: 0, params: go }),
                    ev("E5", sub(NONE, "Unused", "X")),
                ],
                vec![("Ocarina", 0), ("Milk", 3), ("Dead", 5)],
            ),
        );
        m.flows.insert("Gift".to_string(), flow("Gift", vec![ev("G0", sub(NONE, GET, "Show"))], vec![("Give", 0)]));
        m.flows.insert("Bottle".to_string(), flow("Bottle", vec![ev("B0", Kind::Action { next: NONE, actor: 0, act: 1, params: None })], vec![("Fill", 0)]));
        m.flows.insert(GET.to_string(), flow(GET, vec![ev("S0", Kind::Action { next: NONE, actor: 0, act: 1, params: None })], vec![("Show", 0)]));
        m
    }

    #[test]
    fn the_reach_follows_local_and_foreign_calls_and_scene_changes() {
        let mut m: Mem = mem();
        let mut r: Reach = Reach::new(&mut m);
        let n: Need = r.need("Cow", "Ocarina").unwrap();
        assert_eq!(n.flows, ["Bottle", "Gift", GET].iter().map(|s| s.to_string()).collect::<BS<String>>());
        assert_eq!(n.kids, ["Next_Event".to_string()].into_iter().collect::<BS<String>>());
        assert_eq!(r.need("Cow", "Milk").unwrap().flows, ["Bottle".to_string()].into_iter().collect::<BS<String>>());
        assert_eq!(r.need("Cow", "Missing").unwrap(), Need::default());
    }

    fn row(id: &str, subs: &[&str]) -> Node {
        Node::Map(vec![(SUBS.to_string(), Node::Arr(subs.iter().map(|s| Node::Str(s.to_string())).collect())), ("__RowId".to_string(), Node::Str(id.to_string()))])
    }

    fn rows() -> HM<String, Row> {
        let r = |e: &str| Row { stem: "Cow".to_string(), entry: e.to_string(), comp: String::new() };
        let mut m: HM<String, Row> = HM::new();
        m.insert("Cow_Ocarina".to_string(), r("Ocarina"));
        m.insert("Cow_Milk".to_string(), r("Milk"));
        m.insert("Other".to_string(), Row { stem: "Talk".to_string(), entry: "Talk".to_string(), comp: String::new() });
        m
    }

    #[test]
    fn cover_adds_only_what_an_edited_event_now_reaches_and_keeps_the_order() {
        let mut y: Byml = Byml { root: Node::Arr(vec![row("Cow_Milk", &["Bottle"]), row("Cow_Ocarina", &["ActionGuide", "Bottle", "Gift", "NaviEventCommon"]), row("Other", &["Gift"])]), version: 7, big: false };
        let mut m: Mem = mem();
        let mut r: Reach = Reach::new(&mut m);
        let changed: BS<String> = ["Cow".to_string()].into_iter().collect();
        let a: Added = cover(&mut y, &rows(), &mut r, &changed).unwrap();
        assert_eq!(a.keys().cloned().collect::<Vec<String>>(), vec!["Cow_Ocarina".to_string()]);
        assert_eq!(a["Cow_Ocarina"], (vec![GET.to_string()], vec!["Next_Event".to_string()]));
        let list: &[Node] = y.root.as_arr().unwrap();
        assert_eq!(names(&list[1], SUBS), vec!["ActionGuide", "Bottle", GET, "Gift", "NaviEventCommon"]);
        assert_eq!(names(&list[1], KIDS), vec!["Next_Event"]);
        assert_eq!(names(&list[0], SUBS), vec!["Bottle"]);
        assert!(list[0].get(KIDS).is_none());
        assert_eq!(names(&list[2], SUBS), vec!["Gift"]);
        assert!(cover(&mut y, &rows(), &mut r, &changed).unwrap().is_empty());
        let none: BS<String> = BS::new();
        let mut fresh: Byml = Byml { root: Node::Arr(vec![row("Cow_Ocarina", &[])]), version: 7, big: false };
        assert!(cover(&mut fresh, &rows(), &mut r, &none).unwrap().is_empty());
    }

    #[test]
    fn merge_keeps_the_game_order_of_the_names() {
        let have: Vec<String> = ["ActionGuide", "Dm_SY_GetInEvent", "Get_Epona", "GetPouchContent", "InitTalk"].iter().map(|s| s.to_string()).collect();
        let add: BS<String> = [GET.to_string()].into_iter().collect();
        assert_eq!(merge(&have, &add), vec!["ActionGuide", GET, "Dm_SY_GetInEvent", "Get_Epona", "GetPouchContent", "InitTalk"]);
        assert_eq!(merge(&have, &BS::new()), have);
    }

    fn table(rf: &mut Romfs) -> Byml {
        rf.read_byml(ANALYZED_TABLE).unwrap()
    }

    fn touched() -> BS<String> {
        let x: Extra = Extra::load().unwrap();
        x.specs.iter().chain(x.always.iter()).flat_map(|s| s.ops.iter().map(|o| stem(&o.file))).collect()
    }

    #[test]
    fn the_game_rows_of_every_edited_flow_list_exactly_what_the_reach_finds() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let es: HM<String, Row> = settings(&mut rf, &|_: &str| None).unwrap();
        let y: Byml = table(&mut rf);
        let mut d: Disk = Disk::new(Romfs::open(&r).unwrap(), None, Vec::new());
        let mut re: Reach = Reach::new(&mut d);
        let t: BS<String> = touched();
        let mut checked: usize = 0;
        let mut bad: Vec<String> = Vec::new();
        for n in y.root.as_arr().unwrap() {
            let id: &str = id_of(n);
            let Some(row) = es.get(id) else { continue };
            if row.entry.is_empty() || !t.contains(&row.stem) {
                continue;
            }
            let have: BS<String> = names(n, SUBS).into_iter().collect();
            let need: Need = re.need(&row.stem, &row.entry).unwrap();
            if need.flows != have {
                bad.push(format!("{id}: reach {:?} row {have:?}", need.flows));
            }
            let kids: BS<String> = names(n, KIDS).into_iter().collect();
            if !need.kids.is_subset(&kids) {
                bad.push(format!("{id}: starts {:?} row {kids:?}", need.kids));
            }
            checked += 1;
        }
        assert!(checked > 20, "{checked}");
        assert!(bad.is_empty(), "{bad:#?}");
    }

    fn added_for(r: &P, rf: &mut Romfs, es: &HM<String, Row>, loc: &str) -> (Added, Byml, HM<String, Flow>) {
        let fl: HM<String, Flow> = edited(rf, loc, NEW).unwrap();
        let own: BS<String> = beyond_always(&fl).into_iter().collect();
        let mut y: Byml = table(rf);
        let mut src: Over = Over { disk: Disk::new(Romfs::open(r).unwrap(), None, Vec::new()), flows: fl.clone() };
        let mut re: Reach = Reach::new(&mut src);
        let a: Added = cover(&mut y, es, &mut re, &own).unwrap();
        assert!(cover(&mut y, es, &mut re, &own).unwrap().is_empty(), "{loc} is not fully covered after one pass");
        (a, y, fl)
    }

    #[test]
    fn every_gated_edit_leaves_its_events_able_to_load_what_they_call() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let es: HM<String, Row> = settings(&mut rf, &|_: &str| None).unwrap();
        let x: Extra = Extra::load().unwrap();
        let mut seen: BM<String, Added> = BM::new();
        for s in &x.specs {
            let (a, _, _) = added_for(&r, &mut rf, &es, &s.name);
            if !a.is_empty() {
                seen.insert(s.name.clone(), a);
            }
        }
        let rows = |loc: &str| -> Vec<String> { seen.get(loc).map(|a| a.keys().cloned().collect()).unwrap_or_default() };
        assert_eq!(rows("GV Cow"), vec!["GiveMilk_GerudoFortress".to_string(), "GiveMilk_GerudoValley".to_string()]);
        assert_eq!(rows("KF Links House Cow"), vec!["Animal_Cow_PushToBottleMilk_LinkHouse".to_string()]);
        for loc in ["GV Cow", "KF Links House Cow"] {
            assert!(seen[loc].values().all(|(s, k)| s == &vec![GET.to_string()] && k.is_empty()), "{loc} {:?}", seen[loc]);
        }
        if let Ok(p) = std::env::var("OOTRR_LOADS_OUT") {
            let mut t: String = String::new();
            for (loc, a) in &seen {
                t.push_str(&format!("{loc}\n{}\n", lines(a).join("\n")));
            }
            std::fs::write(p, t).unwrap();
        }
    }

    #[test]
    fn the_always_edits_call_nothing_new() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let fl: HM<String, Flow> = edited(&mut rf, "Kak 20 Gold Skulltula Reward", "Item_AgonyStone").unwrap();
        assert!(beyond_always(&fl).is_empty());
        let got = stage(&mut rf, &|_: &str| None, fl).unwrap();
        assert!(got.is_none(), "{:?}", got.map(|g| g.1));
    }

    #[test]
    fn the_cow_table_edit_reaches_the_valley_ocarina_event_through_stage() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let fl: HM<String, Flow> = edited(&mut rf, "GV Cow", NEW).unwrap();
        let (o, a): (Out, Added) = stage(&mut rf, &|_: &str| None, fl).unwrap().unwrap();
        assert_eq!(o.path, ANALYZED_TABLE);
        assert!(o.new > o.old);
        assert!(a.contains_key("GiveMilk_GerudoValley"));
        let y: Byml = Byml::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap();
        let v: Byml = table(&mut rf);
        let find = |b: &Byml, id: &str| -> Vec<String> { b.root.as_arr().unwrap().iter().find(|n| id_of(n) == id).map(|n| names(n, SUBS)).unwrap() };
        let after: Vec<String> = find(&y, "GiveMilk_GerudoValley");
        let before: Vec<String> = find(&v, "GiveMilk_GerudoValley");
        assert!(after.contains(&GET.to_string()) && !before.contains(&GET.to_string()));
        assert_eq!(after.iter().filter(|s| *s != GET).cloned().collect::<Vec<String>>(), before);
        assert_eq!(y.root.as_arr().unwrap().len(), v.root.as_arr().unwrap().len());
        assert_eq!(find(&y, "Animal_Cow_D_T"), find(&v, "Animal_Cow_D_T"));
    }
}
