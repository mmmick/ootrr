use std::collections::BTreeSet as Set;

use crate::ainb::build::Spec;
use crate::ainb::{audit, Ainb, Plug};
use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::pickup_pack::rows;
use crate::qol::{edit_zs, Have};
use crate::romfs::Romfs;
use crate::sarc::{self, Entry};
use crate::staging::Out;
use crate::zs;

pub const GLOBAL: &str = "Pack/AI.Global.Product.111.pack.zs";
pub const ACTOR_INFO: &str = rows::ACTOR_INFO;
pub const GRAPH: &str = "AI/EnemyCommon.common.reactiondamagecolor.module.ainb";
pub const STALFOS_GRAPH: &str = "AI/Stalfos.utility.reactiondamagecolor.module.ainb";
pub const GRAPHS: [&str; 2] = [GRAPH, STALFOS_GRAPH];
pub const MODULES: [&str; 2] = ["EnemyCommon.common.reactiondamagecolor.module", "Stalfos.utility.reactiondamagecolor.module"];
pub const REFS: [&str; 2] = ["Component/BSAParam/", "AI/AIInfo/"];
pub const PARALLEL: u16 = 3;
pub const SWITCH: u16 = 7;

const LINK_EXTRA: usize = 8;

pub type Build = fn(&mut Ainb, &str) -> Res<usize>;
pub type Taken = fn(&Ainb) -> bool;

#[derive(Clone, Copy)]
pub struct Hook {
    pub label: &'static str,
    pub taken: Taken,
    pub build: Build,
}

pub fn flag(v: bool) -> Vec<u8> {
    (v as u32).to_le_bytes().to_vec()
}

pub fn switch<'a>(seed: &'a str, input: usize, yes: usize, no: usize) -> Spec<'a> {
    let mut s: Spec = Spec::composite(SWITCH, seed)
        .prop(0, "CalculateTiming", vec![0; 4])
        .prop(1, "ChildFrameSync", flag(false))
        .prop(1, "InputValue", flag(false))
        .prop(1, "IsNoSelectWhenChildBusy", flag(false))
        .linked(1, "Input", input, 0)
        .kid(yes, "True")
        .kid(no, "False");
    s.link_extra = LINK_EXTRA;
    s
}

pub fn both<'a>(seed: &'a str, first: usize, second: usize) -> Spec<'a> {
    Spec::composite(PARALLEL, seed).prop(0, "EndPolicy", vec![0; 4]).prop(0, "ResultPolicy", vec![0; 4]).kid(first, "").kid(second, "")
}

pub fn has(d: &[u8], name: &str) -> bool {
    let m: &[u8] = name.as_bytes();
    d.windows(m.len() + 1).any(|w| &w[..m.len()] == m && w[m.len()] == 0)
}

pub fn calls(d: &[u8]) -> bool {
    MODULES.iter().any(|m| has(d, m))
}

pub fn is_caller(e: &Entry) -> bool {
    e.name.ends_with(".ainb") && !GRAPHS.contains(&e.name.as_str()) && calls(&e.data)
}

pub fn callers(es: &[Entry]) -> Set<String> {
    es.iter().filter(|e| is_caller(e)).map(|e| e.name.clone()).collect()
}

pub fn graph_of(path: &str) -> Option<String> {
    let f: &str = path.rsplit('/').next()?;
    f.ends_with(".ain").then(|| format!("AI/{f}b"))
}

fn named(n: &Node, shared: &Set<String>, out: &mut Set<String>) {
    match n {
        Node::Str(s) => {
            if let Some(g) = graph_of(s).filter(|g| shared.contains(g)) {
                out.insert(g);
            }
        }
        Node::Arr(v) => v.iter().for_each(|x| named(x, shared, out)),
        Node::Map(v) => v.iter().for_each(|(_, x)| named(x, shared, out)),
        _ => {}
    }
}

pub fn shared_of(es: &[Entry], shared: &Set<String>) -> Set<String> {
    let mut out: Set<String> = Set::new();
    for e in es.iter().filter(|e| REFS.iter().any(|d| e.name.starts_with(d))) {
        if let Ok(y) = Byml::parse(&e.data) {
            named(&y.root, shared, &mut out);
        }
    }
    out
}

pub fn runs(es: &[Entry], shared: &Set<String>) -> bool {
    es.iter().any(is_caller) || !shared_of(es, shared).is_empty()
}

pub fn actor_of(pack: &str) -> Option<&str> {
    pack.rsplit('/').next()?.strip_suffix(".pack.zs")
}

fn heap_of(row: &Node) -> Option<i32> {
    match row.get(rows::HEAP)? {
        Node::I32(v) => Some(*v),
        Node::U32(v) => i32::try_from(*v).ok(),
        _ => None,
    }
}

pub fn grow_heap(raw: &[u8], actors: &Set<String>, extra: i32, label: &str) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    let list: &mut Vec<Node> = y.root.as_arr_mut().ok_or_else(|| Error::Msg(format!("{label}: the actor table is not a row list")))?;
    let mut want: Vec<(String, i32)> = Vec::new();
    for row in list.iter_mut() {
        let Some(name) = rows::id(row).filter(|n| actors.contains(*n)).map(str::to_string) else {
            continue;
        };
        let Some(old) = heap_of(row).filter(|v| *v > 0) else {
            continue;
        };
        let new: i32 = old.checked_add(extra).ok_or_else(|| Error::Msg(format!("{label}: the heap size does not fit")))?;
        let v: Node = match row.get(rows::HEAP) {
            Some(Node::U32(_)) => Node::U32(new as u32),
            _ => Node::I32(new),
        };
        row.set(rows::HEAP, v);
        want.push((name, new));
    }
    let out: Vec<u8> = y.write()?;
    let back: Byml = Byml::parse(&out)?;
    if want.iter().any(|(name, new)| rows::find(&back, name).and_then(heap_of) != Some(*new)) {
        return Err(Error::Msg(format!("{label}: the actor table does not read back")));
    }
    Ok(out)
}

pub fn zipped(rf: &Romfs, have: Have, path: &str) -> Res<Vec<u8>> {
    match have(path) {
        Some(b) => Ok(b),
        None => rf.read(path),
    }
}

pub fn opened(rf: &mut Romfs, have: Have, path: &str) -> Res<(Vec<u8>, usize, Vec<Entry>)> {
    let z: Vec<u8> = zipped(rf, have, path)?;
    let raw: Vec<u8> = rf.zs.decompress(&z)?;
    let es: Vec<Entry> = sarc::parse(&raw)?;
    Ok((z, raw.len(), es))
}

pub fn patch_graph(raw: &[u8], h: &Hook) -> Res<Option<Vec<u8>>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    let sites: Vec<usize> = MODULES.iter().flat_map(|m| a.find_els(m)).collect();
    if sites.is_empty() {
        return Ok(None);
    }
    if (h.taken)(&a) {
        return Err(Error::Msg(format!("{}: the graph is already patched", h.label)));
    }
    for site in sites {
        let (parent, _): (usize, usize) = a.parent_of(site).map_err(|_| Error::Msg(format!("{}: a damage color call has no single parent", h.label)))?;
        if a.el_type(parent) != PARALLEL {
            return Err(Error::Msg(format!("{}: a damage color call no longer sits in a parallel", h.label)));
        }
        let top: usize = (h.build)(&mut a, &format!("ootrr {} {site}", h.label))?;
        let at: usize = a.params[parent].pl[2][0] as usize;
        let name: u32 = a.pool.add("");
        a.params[parent].add_plug(2, at, Plug { a: top as u32, b: name, extra: Vec::new() })?;
    }
    let bad: Vec<String> = audit(&a);
    if !bad.is_empty() {
        return Err(Error::Msg(format!("{}: {}", h.label, bad.join("; "))));
    }
    let out: Vec<u8> = a.write()?;
    let back: Ainb = Ainb::parse(&out)?;
    if back.write()? != out || back.params != a.params || back.ins != a.ins || back.outs != a.outs || back.qids != a.qids {
        return Err(Error::Msg(format!("{}: the written graph does not read back", h.label)));
    }
    Ok(Some(out))
}

pub fn patch_entries(es: &mut [Entry], h: &Hook, allow: &dyn Fn(&str) -> bool) -> Res<bool> {
    let mut hit: bool = false;
    for e in es.iter_mut().filter(|e| is_caller(e) && allow(&e.name)) {
        if let Some(out) = patch_graph(&e.data, h)? {
            e.data = out;
            hit = true;
        }
    }
    Ok(hit)
}

fn visit(rf: &mut Romfs, have: Have, path: &str, shared: &Set<String>, h: &Hook) -> Res<(bool, Option<Out>)> {
    let z: Vec<u8> = zipped(rf, have, path)?;
    let raw: Vec<u8> = rf.zs.decompress(&z)?;
    let mut es: Vec<Entry> = sarc::parse(&raw)?;
    let used: bool = runs(&es, shared);
    if !patch_entries(&mut es, h, &|_| true)? {
        return Ok((used, None));
    }
    let out: Vec<u8> = sarc::write(&es)?;
    Ok((used, Some(Out { path: path.to_string(), old: raw.len(), new: out.len(), z: zs::compress_raw(&out, zs::dict_of(&z))? })))
}

pub fn stage(rf: &mut Romfs, have: Have, dir: &str, h: &Hook, extra: i32) -> Res<(Set<String>, Vec<Out>)> {
    let z: Vec<u8> = zipped(rf, have, GLOBAL)?;
    let shared: Set<String> = callers(&sarc::parse(&rf.zs.decompress(&z)?)?);
    let mut outs: Vec<Out> = Vec::new();
    outs.push(visit(rf, have, GLOBAL, &shared, h)?.1.ok_or_else(|| Error::Msg(format!("{}: the AI pack has no graph that runs the damage color module", h.label)))?);
    let mut who: Set<String> = Set::new();
    for f in rf.list(dir)? {
        let Some(name) = actor_of(&f).map(str::to_string) else {
            continue;
        };
        let (used, out): (bool, Option<Out>) = visit(rf, have, &f, &shared, h)?;
        if used {
            who.insert(name);
        }
        outs.extend(out);
    }
    if who.is_empty() {
        return Err(Error::Msg(format!("{}: no actor runs the damage color module", h.label)));
    }
    outs.push(edit_zs(rf, have, ACTOR_INFO, |raw| grow_heap(raw, &who, extra, h.label))?);
    Ok((who, outs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_call_is_a_whole_name_in_the_string_pool() {
        let mut d: Vec<u8> = b"x\0".to_vec();
        d.extend_from_slice(MODULES[0].as_bytes());
        assert!(!calls(&d));
        d.push(0);
        assert!(calls(&d));
        let mut longer: Vec<u8> = MODULES[1].as_bytes().to_vec();
        longer.extend_from_slice(b".other\0");
        assert!(!calls(&longer));
    }

    #[test]
    fn a_graph_path_turns_into_its_packed_name() {
        assert_eq!(graph_of("Work/AI/Root/Enemy/Enemy_Stalfos.utility.root.ain").as_deref(), Some("AI/Enemy_Stalfos.utility.root.ainb"));
        assert_eq!(graph_of("Work/BSA/SchedulerParam/Enemy_Stalfos.game__bsa__SchedulerParam.gyml"), None);
        assert_eq!(graph_of(""), None);
        assert_eq!(actor_of("Pack/Actor/Enemy_Stalfos.pack.zs"), Some("Enemy_Stalfos"));
        assert_eq!(actor_of("Pack/Actor/readme.txt"), None);
    }

    fn bsa(name: &str, graph: &str) -> Entry {
        let y: Byml = Byml { root: Node::Map(vec![("UtilityAinPath".to_string(), Node::Str(graph.to_string()))]), version: 7, big: false };
        Entry { name: name.to_string(), data: y.write().unwrap() }
    }

    fn graph(name: &str, pool: &[u8]) -> Entry {
        Entry { name: name.to_string(), data: pool.to_vec() }
    }

    #[test]
    fn an_actor_runs_the_module_through_its_own_graph_or_a_shared_one() {
        let mut call: Vec<u8> = MODULES[0].as_bytes().to_vec();
        call.push(0);
        let global: Vec<Entry> = vec![graph(GRAPH, &call), graph("AI/Enemy_Stalfos.utility.root.ainb", &call), graph("AI/Npc.utility.root.ainb", b"none\0")];
        let shared: Set<String> = callers(&global);
        assert_eq!(shared.iter().cloned().collect::<Vec<String>>(), vec!["AI/Enemy_Stalfos.utility.root.ainb".to_string()]);
        let stalfos: Entry = bsa("Component/BSAParam/Enemy_Stalfos.game__component__BSAParam.bgyml", "Work/AI/Root/Enemy/Enemy_Stalfos.utility.root.ain");
        assert!(runs(&[stalfos.clone()], &shared));
        assert_eq!(shared_of(&[stalfos], &shared), shared);
        assert!(!runs(&[bsa("Component/BSAParam/Npc.game__component__BSAParam.bgyml", "Work/AI/Root/Npc.utility.root.ain")], &shared));
        assert!(!runs(&[bsa("Component/Other/Enemy_Stalfos.bgyml", "Work/AI/Root/Enemy/Enemy_Stalfos.utility.root.ain")], &shared));
        assert!(runs(&[graph("AI/Enemy_Octorok.utility.root.ainb", &call)], &shared));
        assert!(!runs(&[graph(GRAPH, &call)], &shared));
    }

    fn row(name: &str, heap: Node) -> Node {
        Node::Map(vec![(rows::HEAP.to_string(), heap), (rows::ROW_ID.to_string(), Node::Str(name.to_string()))])
    }

    #[test]
    fn only_the_named_actors_with_a_heap_grow() {
        let y: Byml = Byml { root: Node::Arr(vec![row("Enemy_A", Node::I32(1000)), row("Enemy_B", Node::I32(0)), row("Npc_C", Node::I32(500)), row("Enemy_D", Node::U32(700))]), version: 7, big: false };
        let actors: Set<String> = ["Enemy_A", "Enemy_B", "Enemy_D", "Enemy_Missing"].iter().map(|s| s.to_string()).collect();
        let back: Byml = Byml::parse(&grow_heap(&y.write().unwrap(), &actors, 0x4000, "t").unwrap()).unwrap();
        let at = |n: &str| -> Node { rows::find(&back, n).unwrap().get(rows::HEAP).unwrap().clone() };
        assert_eq!(at("Enemy_A"), Node::I32(1000 + 0x4000));
        assert_eq!(at("Enemy_B"), Node::I32(0));
        assert_eq!(at("Npc_C"), Node::I32(500));
        assert_eq!(at("Enemy_D"), Node::U32(700 + 0x4000));
    }

    #[test]
    fn garbage_is_refused() {
        let h: Hook = Hook { label: "t", taken: |_| false, build: |a, tag| a.add_node(&Spec::node("ExecuteGenericDummyAction", &format!("{tag} idle"))) };
        assert!(patch_graph(b"nope", &h).is_err());
    }
}
