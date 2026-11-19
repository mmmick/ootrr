use std::collections::BTreeSet as Set;

use crate::ainb::build::Spec;
use crate::ainb::Ainb;
use crate::difficulty::hook::{self, flag, switch, Hook};
use crate::error::Res;
use crate::qol::Have;
use crate::romfs::Romfs;
use crate::staging::Out;

pub use hook::{actor_of, ACTOR_INFO, GLOBAL, GRAPH, GRAPHS, MODULES, STALFOS_GRAPH};

pub const HEAL: &str = "OneShotLifeSetOwnLife";
pub const TIMER: &str = "SelectorGenericFrameCounter";
pub const HAS_LIFE: &str = "QueryLifeHasComponent";
pub const IS_DEAD: &str = "QueryDeathIsDead";
pub const IDLE: &str = "ExecuteGenericDummyAction";
pub const HIT: &str = "QueryLifeIsExternalDamaged";
pub const HEAL_EVERY: f32 = 300.0;
pub const WAIT_FRAMES: f32 = 600.0;
pub const AMOUNT: i32 = 1;
pub const ADD: i32 = 0;
pub const HEAP_EXTRA: i32 = 0x4000;
pub const LABEL: &str = "enemy regen";
pub const ADDED: usize = 14;

pub const HOOK: Hook = Hook { label: LABEL, taken, build };

fn taken(a: &Ainb) -> bool {
    !a.find_els(HEAL).is_empty()
}

pub fn build(a: &mut Ainb, tag: &str) -> Res<usize> {
    let s = |n: &str| -> String { format!("{tag} {n}") };
    let has: usize = a.add_node(&Spec::query(HAS_LIFE, &s("has life")).out(1, "Bool"))?;
    let dead: usize = a.add_node(&Spec::query(IS_DEAD, &s("is dead")).spare(0, "DeathReason").spare(0, "SubDeathReason").out(1, "IsDead"))?;
    let heal: usize = a.add_node(
        &Spec::node(HEAL, &s("heal"))
            .prop(1, "ExecuteEveryFrame", flag(false))
            .constant(0, "SetType", ADD.to_le_bytes().to_vec())
            .constant(0, "SetValue", AMOUNT.to_le_bytes().to_vec()),
    )?;
    let gone: usize = a.add_node(&Spec::node(IDLE, &s("idle dead")))?;
    let alive: usize = a.add_node(&switch(&s("alive"), dead, gone, heal))?;
    let bare: usize = a.add_node(&Spec::node(IDLE, &s("idle no life")))?;
    let gate: usize = a.add_node(&switch(&s("gate"), has, alive, bare))?;
    let wait: usize = a.add_node(&Spec::node(IDLE, &s("idle wait")))?;
    let every: usize = a.add_node(&counter(&s("heal timer"), HEAL_EVERY, true, wait, gate))?;
    let pending: usize = a.add_node(&Spec::node(IDLE, &s("idle pending")))?;
    let quiet: usize = a.add_node(&counter(&s("quiet timer"), WAIT_FRAMES, false, pending, every))?;
    let hit: usize = a.add_node(&Spec::query(HIT, &s("hit")).out(1, "Bool"))?;
    let struck: usize = a.add_node(&Spec::node(IDLE, &s("idle hit")))?;
    a.add_node(&switch(&s("recently hit"), hit, struck, quiet))
}

fn counter<'a>(seed: &'a str, frames: f32, looped: bool, not_finished: usize, finished: usize) -> Spec<'a> {
    Spec::node(TIMER, seed)
        .prop(1, "IsCountUpdateOnReset", flag(false))
        .prop(1, "IsLoop", flag(looped))
        .prop(1, "NoChangeWhenChildIsBusy", flag(false))
        .prop(1, "SelectChildOnEnterOnly", flag(false))
        .prop(2, "CountFrames", frames.to_le_bytes().to_vec())
        .prop(2, "RandomAddCountFrames", 0f32.to_le_bytes().to_vec())
        .spare(2, "CurrentFrame")
        .spare(2, "FinishFrame")
        .kid(not_finished, "NotFinish")
        .kid(finished, "Finish")
}

pub fn stage(rf: &mut Romfs, have: Have, dir: &str) -> Res<(Set<String>, Vec<Out>)> {
    hook::stage(rf, have, dir, &HOOK, HEAP_EXTRA)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ainb::root;
    use crate::ainb::show::show;
    use crate::byml::{Byml, Node};
    use crate::pickup_pack::rows;
    use crate::difficulty::hook::{PARALLEL, SWITCH};
    use crate::sarc::{self, Entry};
    use crate::zs;

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        if p.is_dir() { Some(p) } else { None }
    }

    fn patched(a: &Ainb, name: &str) -> (Vec<u8>, Ainb) {
        let raw: Vec<u8> = a.write().unwrap();
        let out: Vec<u8> = hook::patch_graph(&raw, &HOOK).unwrap().unwrap_or_else(|| panic!("{name} has no damage color call"));
        let back: Ainb = Ainb::parse(&out).unwrap();
        (out, back)
    }

    #[test]
    fn a_file_that_is_not_a_graph_is_refused() {
        assert!(hook::patch_graph(b"nope", &HOOK).is_err());
    }

    #[test]
    fn a_caller_gains_a_looping_heal_beside_the_damage_color_call() {
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let es: Vec<Entry> = sarc::parse(&rf.read_zs(GLOBAL).unwrap()).unwrap();
        let name: &str = "AI/EnemyBase.utility.root.ainb";
        let was: &Entry = sarc::get(&es, name).unwrap();
        let old: Ainb = Ainb::parse(&was.data).unwrap();
        assert_eq!(old.write().unwrap(), was.data, "the untouched graph rewrites byte for byte");
        let (out, a): (Vec<u8>, Ainb) = patched(&old, name);
        assert_eq!(a.els.len(), old.els.len() + ADDED);
        assert_eq!(root(&a), root(&old));
        let call: usize = a.find_els(MODULES[0])[0];
        assert_eq!(call, old.find_els(MODULES[0])[0]);
        let (parent, _): (usize, usize) = a.parent_of(call).unwrap();
        let (was_parent, _): (usize, usize) = old.parent_of(call).unwrap();
        assert_eq!((parent, a.el_type(parent)), (was_parent, PARALLEL));
        let kids: Vec<usize> = a.params[parent].group(2).iter().map(|g| g.a as usize).collect();
        let before: Vec<usize> = old.params[parent].group(2).iter().map(|g| g.a as usize).collect();
        assert_eq!(&kids[..before.len()], &before[..], "the old children keep their places");
        assert_eq!((kids.len(), a.el_type(kids[kids.len() - 1])), (before.len() + 1, SWITCH));
        let heal: usize = a.find_els(HEAL)[0];
        assert_eq!((a.in_s32(heal, "SetType"), a.in_s32(heal, "SetValue")), (Some(ADD), Some(AMOUNT)));
        let shown: String = show(&a);
        for line in ["prop bool IsLoop = true", "prop f32 CountFrames = 300", "prop f32 CountFrames = 600", "prop bool IsLoop = false", "\"True\"", "\"False\"", "out f32 CurrentFrame flags 0x0", "out bool IsDead flags 0x2", "out s32 DeathReason flags 0x0", "\"NotFinish\"", "\"Finish\""] {
            assert!(shown.contains(line), "{line} is missing from\n{shown}");
        }
        assert_eq!(shown.matches("Blackboard").count(), show(&old).matches("Blackboard").count(), "regen reads and writes no key");
        let hit: usize = a.find_els(HIT)[0];
        let quiet: usize = a.find_els(TIMER).into_iter().find(|i| a.parent_of(*i).is_ok_and(|p| a.el_type(p.0) == SWITCH)).unwrap();
        let (gate, _): (usize, usize) = a.parent_of(quiet).unwrap();
        assert_eq!(a.parent_of(gate).unwrap().0, parent);
        assert_eq!(a.params[gate].group(2).len(), 2);
        assert!(shown.contains(&format!("in bool Input = false from {hit}.0")), "the hit query feeds the switch");
        assert!(hook::patch_graph(&out, &HOOK).is_err(), "a patched graph is not patched twice");
        let module: &Entry = sarc::get(&es, GRAPH).unwrap();
        assert_eq!(hook::patch_graph(&module.data, &HOOK).unwrap(), None, "the module itself has no call to host in");
        let mut packed: Vec<Entry> = es.clone();
        assert!(hook::patch_entries(&mut packed, &HOOK, &|_| true).unwrap());
        let changed: Vec<&str> = packed.iter().zip(es.iter()).filter(|(x, y)| x.data != y.data).map(|(x, _)| x.name.as_str()).collect();
        assert_eq!(changed.len(), 16, "{changed:?}");
        assert!(changed.iter().all(|n| !GRAPHS.contains(n)));
    }

    #[test]
    fn the_game_nodes_have_the_shapes_the_patch_copies() {
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let es: Vec<Entry> = sarc::parse(&rf.read_zs(GLOBAL).unwrap()).unwrap();
        let base: Ainb = Ainb::parse(&sarc::get(&es, "AI/EnemyBase.utility.root.ainb").unwrap().data).unwrap();
        let (_, mine): (Vec<u8>, Ainb) = patched(&base, "EnemyBase");
        let shape = |a: &Ainb, i: usize| -> Vec<String> {
            let all: String = show(a);
            let from: usize = all.find(&format!("element {i} type")).unwrap();
            let body: &str = &all[from..];
            let end: usize = body.find(&format!("\nelement {} type", i + 1)).unwrap_or(body.len());
            body[..end].lines().skip(1).filter(|l| !l.contains("plug ")).map(|l| l.split(" = ").next().unwrap_or(l).split(" from ").next().unwrap_or(l).to_string()).collect()
        };
        let boss: Vec<Entry> = sarc::parse(&rf.read_zs("Pack/Actor/Enemy_DungeonBoss_Twinrova.pack.zs").unwrap()).unwrap();
        let twin: Ainb = Ainb::parse(&sarc::get(&boss, "AI/Enemy_DungeonBoss_Twinrova.utility.root.ainb").unwrap().data).unwrap();
        let theirs: usize = twin.find_els(TIMER).into_iter().find(|i| shape(&twin, *i).iter().any(|l| l.contains("IsLoop"))).unwrap();
        for t in mine.find_els(TIMER) {
            assert_eq!(shape(&mine, t), shape(&twin, theirs));
        }
        assert_eq!(mine.find_els(TIMER).len(), 2);
        let stalfos: Ainb = Ainb::parse(&sarc::get(&es, "AI/Enemy_Stalfos.utility.root.ainb").unwrap().data).unwrap();
        assert_eq!(shape(&mine, mine.find_els(HIT)[0]), shape(&stalfos, stalfos.find_els(HIT)[0]));
        let glass: Ainb = Ainb::parse(&sarc::get(&es, "AI/EnemyCommon.common.enableglasstruth.module.ainb").unwrap().data).unwrap();
        assert_eq!(shape(&mine, mine.find_els(IS_DEAD)[0]), shape(&glass, glass.find_els(IS_DEAD)[0]));
        let sw: usize = (0..glass.els.len()).find(|i| glass.el_type(*i) == SWITCH).unwrap();
        let my_sw: usize = (base.els.len()..mine.els.len()).find(|i| mine.el_type(*i) == SWITCH).unwrap();
        assert_eq!(shape(&mine, my_sw), shape(&glass, sw));
        let st_sw: usize = (0..stalfos.els.len()).find(|i| stalfos.el_type(*i) == SWITCH && stalfos.params[*i].plugs[0].extra.len() == 8).unwrap();
        assert_eq!(shape(&mine, my_sw), shape(&stalfos, st_sw));
        assert_eq!(mine.params[my_sw].pl, glass.params[sw].pl);
        assert_eq!(mine.params[my_sw].plugs[0].extra.len(), glass.params[sw].plugs[0].extra.len());
        let npc: Ainb = Ainb::parse(&sarc::get(&es, "AI/SimpleNpc.ExecuteInterruptReaction.module.ainb").unwrap().data).unwrap();
        assert_eq!(shape(&mine, mine.find_els(HAS_LIFE)[0]), shape(&npc, npc.find_els(HAS_LIFE)[0]));
        let floor: Vec<Entry> = sarc::parse(&rf.read_zs("Pack/Actor/Enemy_Floormaster.pack.zs").unwrap()).unwrap();
        let down: Ainb = Ainb::parse(&sarc::get(&floor, "AI/Floormaster.action.interruptdown.module.ainb").unwrap().data).unwrap();
        assert_eq!(shape(&mine, mine.find_els(HEAL)[0]), shape(&down, down.find_els(HEAL)[0]));
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn the_game_actors_that_run_a_caller_get_room_and_the_module_stays_vanilla() {
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let (who, outs): (Set<String>, Vec<Out>) = stage(&mut rf, &none, "Pack/Actor").unwrap();
        for a in ["Enemy_Stalfos", "Enemy_Stalfos_ForChild", "Enemy_Keese", "Enemy_Octorok", "Enemy_Spike", "Enemy_DungeonBoss_Gohma"] {
            assert!(who.contains(a), "{a} runs a caller");
        }
        for a in ["Player_Child", "Player_Adult", "Npc_Navi", "Enemy_Beamos", "Enemy_Stalfos_Dictionary"] {
            assert!(!who.contains(a), "{a} does not run a caller");
        }
        let paths: Vec<&str> = outs.iter().map(|x| x.path.as_str()).collect();
        assert_eq!(zs::dict_of(&outs[0].z), zs::dict_of(&rf.read(GLOBAL).unwrap()), "the AI pack keeps its dictionary");
        assert_eq!((paths[0], paths[paths.len() - 1]), (GLOBAL, ACTOR_INFO));
        let last: usize = outs.len() - 1;
        assert!(outs[..last].iter().all(|x| x.new > x.old) && outs[last].new == outs[last].old);
        for x in &outs[..last] {
            let now: Vec<Entry> = sarc::parse(&rf.zs.decompress(&x.z).unwrap()).unwrap();
            let old: Vec<Entry> = sarc::parse(&rf.read_zs(&x.path).unwrap()).unwrap();
            assert_eq!(now.len(), old.len());
            let changed: Vec<&Entry> = now.iter().zip(old.iter()).filter(|(a, b)| a.name != b.name || a.data != b.data).map(|(a, _)| a).collect();
            assert!(!changed.is_empty(), "{}", x.path);
            for g in GRAPHS {
                assert_eq!(sarc::get(&now, g).map(|e| &e.data), sarc::get(&old, g).map(|e| &e.data), "{g} is vanilla in {}", x.path);
            }
            for e in changed {
                assert!(old.iter().any(|o| o.name == e.name && hook::is_caller(o)), "{} in {} is not a caller", e.name, x.path);
            }
        }
        let was: Byml = rf.read_byml(ACTOR_INFO).unwrap();
        let now: Byml = Byml::parse(&rf.zs.decompress(&outs[last].z).unwrap()).unwrap();
        let (a, b): (&[Node], &[Node]) = (was.root.as_arr().unwrap(), now.root.as_arr().unwrap());
        assert_eq!(a.len(), b.len());
        let heap = |row: &Node| -> Option<i32> {
            match row.get(rows::HEAP)? {
                Node::I32(v) => Some(*v),
                Node::U32(v) => i32::try_from(*v).ok(),
                _ => None,
            }
        };
        let mut grown: usize = 0;
        for (x, y) in a.iter().zip(b.iter()) {
            if x == y {
                continue;
            }
            let name: &str = rows::id(x).unwrap();
            assert!(who.contains(name), "{name} changed");
            assert_eq!(heap(y), Some(heap(x).unwrap() + HEAP_EXTRA));
            let mut z: Node = y.clone();
            z.set(rows::HEAP, x.get(rows::HEAP).unwrap().clone());
            assert_eq!(&z, x, "only the heap of {name} changed");
            grown += 1;
        }
        assert!(grown >= 80, "{grown} actors grew");
        let mut t: crate::rsizetable::Rst = crate::rsizetable::load(&mut rf).unwrap();
        let caller: &str = "AI/EnemyBase.utility.root.ainb";
        let before: Option<u32> = t.get(caller);
        assert!(before.is_some(), "the caller has its own size table entry");
        for o in &outs {
            crate::staging::grow_out(&mut rf, &mut t, o).unwrap();
        }
        assert!(t.get(caller) > before, "the size table entry of {caller} grew");
    }
}
