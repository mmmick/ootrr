use std::collections::BTreeSet as Set;

use crate::ainb::build::{Spec, QUERY};
use crate::ainb::Ainb;
use crate::difficulty::fx;
use crate::difficulty::hook::{self, both, flag, switch, Hook};
use crate::error::Res;
use crate::qol::Have;
use crate::romfs::Romfs;
use crate::staging::Out;

pub use hook::{actor_of, ACTOR_INFO, GLOBAL, GRAPH, GRAPHS, MODULES, STALFOS_GRAPH};

pub const LIST: &str = crate::chest_slot::LIST;
pub const FLAG: &str = "MiniGame_Diving_Test";
pub const GET: &str = "QueryGetNonStructGameDataBool";
pub const CLEAR: &str = "OneShotSetNonStructGameDataBool";
pub const HEAL: &str = "OneShotLifeSetOwnLife";
pub const TIMER: &str = "SelectorGenericFrameCounter";
pub const HAS_LIFE: &str = "QueryLifeHasComponent";
pub const IS_DEAD: &str = "QueryDeathIsDead";
pub const IDLE: &str = "ExecuteGenericDummyAction";
pub const FRAMES: f32 = 30.0;
pub const TO_MAX: i32 = 1;
pub const HEAP_EXTRA: i32 = 0x8000;
pub const ADDED: usize = 14 + 1 + fx::HURT_ADDED;
pub const LABEL: &str = "fairy helps enemies";

pub const HOOK: Hook = Hook { label: LABEL, taken, build };

fn taken(a: &Ainb) -> bool {
    a.find_els(GET).into_iter().chain(a.find_els(CLEAR)).any(|i| a.in_str(i, "GameDataFullName") == Some(FLAG))
}

pub fn build(a: &mut Ainb, tag: &str) -> Res<usize> {
    let s = |n: &str| -> String { format!("{tag} {n}") };
    let key: Vec<u8> = a.pool.add(FLAG).to_le_bytes().to_vec();
    let name: String = s("used");
    let mut get: Spec = Spec::node(GET, &name)
        .prop(1, "DispError", flag(false))
        .prop(1, "OutputOnlyOnEnter", flag(false))
        .constant(0, "GameDataIndex", (-1i32).to_le_bytes().to_vec())
        .constant(1, "IsDirect", flag(false))
        .constant(3, "GameDataFullName", key.clone())
        .out(1, "Value");
    get.role = QUERY;
    let used: usize = a.add_node(&get)?;
    let has: usize = a.add_node(&Spec::query(HAS_LIFE, &s("has life")).out(1, "Bool"))?;
    let dead: usize = a.add_node(&Spec::query(IS_DEAD, &s("is dead")).spare(0, "DeathReason").spare(0, "SubDeathReason").out(1, "IsDead"))?;
    let heal: usize = a.add_node(&heal_spec(&s("heal")))?;
    let topped: usize = a.add_node(&heal_spec(&s("heal at full")))?;
    let shown: usize = fx::when_hurt(a, &s("heal"), topped, heal)?;
    let gone: usize = a.add_node(&Spec::node(IDLE, &s("idle dead")))?;
    let alive: usize = a.add_node(&switch(&s("alive"), dead, gone, shown))?;
    let bare: usize = a.add_node(&Spec::node(IDLE, &s("idle no life")))?;
    let gate: usize = a.add_node(&switch(&s("gate"), has, alive, bare))?;
    let clear: usize = a.add_node(
        &Spec::node(CLEAR, &s("clear"))
            .prop(1, "ExecuteEveryFrame", flag(false))
            .constant(0, "GameDataIndex", (-1i32).to_le_bytes().to_vec())
            .constant(1, "Value", flag(false))
            .constant(3, "GameDataFullName", key),
    )?;
    let wait: usize = a.add_node(&Spec::node(IDLE, &s("idle wait")))?;
    let timer: usize = a.add_node(
        &Spec::node(TIMER, &s("timer"))
            .prop(1, "IsCountUpdateOnReset", flag(false))
            .prop(1, "IsLoop", flag(true))
            .prop(1, "NoChangeWhenChildIsBusy", flag(false))
            .prop(1, "SelectChildOnEnterOnly", flag(false))
            .prop(2, "CountFrames", FRAMES.to_le_bytes().to_vec())
            .prop(2, "RandomAddCountFrames", 0f32.to_le_bytes().to_vec())
            .spare(2, "CurrentFrame")
            .spare(2, "FinishFrame")
            .kid(wait, "NotFinish")
            .kid(clear, "Finish"),
    )?;
    let work: usize = a.add_node(&both(&s("work"), gate, timer))?;
    let calm: usize = a.add_node(&Spec::node(IDLE, &s("idle calm")))?;
    a.add_node(&switch(&s("poll"), used, work, calm))
}

fn heal_spec(seed: &str) -> Spec<'_> {
    Spec::node(HEAL, seed)
        .prop(1, "ExecuteEveryFrame", flag(false))
        .constant(0, "SetType", TO_MAX.to_le_bytes().to_vec())
        .constant(0, "SetValue", 0i32.to_le_bytes().to_vec())
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
    fn the_game_has_the_flag_and_does_not_save_it() {
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let y: Byml = rf.read_byml(LIST).unwrap();
        let want: i64 = crate::chest_slot::key(FLAG) as i64;
        let num = |n: &Node| -> Option<i64> {
            match n {
                Node::U32(v) => Some(*v as i64),
                Node::I32(v) => Some(*v as i64),
                _ => None,
            }
        };
        let bools: &[Node] = y.root.get("Data").unwrap().get("Bool").unwrap().as_arr().unwrap();
        let mine: Vec<&Node> = bools.iter().filter(|b| b.get("Hash").and_then(num) == Some(want)).collect();
        assert_eq!(mine.len(), 1, "{FLAG} is a bool of the game");
        assert_eq!(mine[0].get("SaveFileIndex").and_then(num), Some(-1), "{FLAG} is not saved");
        assert_eq!(mine[0].get("DefaultValue"), Some(&Node::Bool(false)));
        let name: &[u8] = FLAG.as_bytes();
        let es: Vec<Entry> = sarc::parse(&rf.read_zs(GLOBAL).unwrap()).unwrap();
        assert!(!es.iter().any(|e| e.data.windows(name.len()).any(|w| w == name)), "no shared graph uses {FLAG}");
    }

    #[test]
    fn a_caller_heals_while_the_flag_is_set_and_then_clears_it() {
        let Some(dir) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&dir).unwrap();
        let es: Vec<Entry> = sarc::parse(&rf.read_zs(GLOBAL).unwrap()).unwrap();
        let name: &str = "AI/EnemyBase.utility.root.ainb";
        let old: Ainb = Ainb::parse(&sarc::get(&es, name).unwrap().data).unwrap();
        let (out, a): (Vec<u8>, Ainb) = patched(&old, name);
        assert_eq!(a.els.len(), old.els.len() + ADDED);
        assert_eq!(root(&a), root(&old));
        let call: usize = a.find_els(MODULES[0])[0];
        let (parent, _): (usize, usize) = a.parent_of(call).unwrap();
        assert_eq!(a.el_type(parent), PARALLEL);
        let kids = |i: usize| -> Vec<usize> { a.params[i].group(2).iter().map(|g| g.a as usize).collect() };
        let before: usize = old.params[parent].group(2).len();
        let poll: usize = kids(parent)[before];
        assert_eq!((kids(parent).len(), a.el_type(poll)), (before + 1, SWITCH));
        let work: usize = kids(poll)[0];
        assert_eq!((a.el_type(work), a.el_name(kids(poll)[1])), (PARALLEL, IDLE));
        let timer: usize = kids(work)[1];
        assert_eq!((a.el_type(kids(work)[0]), a.el_name(timer)), (SWITCH, TIMER));
        assert_eq!((a.el_name(kids(timer)[0]), a.el_name(kids(timer)[1])), (IDLE, CLEAR));
        let heal: usize = a.find_els(HEAL)[0];
        assert_eq!((a.in_s32(heal, "SetType"), a.in_s32(heal, "SetValue")), (Some(TO_MAX), Some(0)));
        let shown: String = show(&a);
        for line in ["prop bool IsLoop = true", "prop f32 CountFrames = 30", "out bool IsDead flags 0x2", "out bool Value flags 0x2", "in bool Value = false", "\"NotFinish\"", "\"Finish\""] {
            assert!(shown.contains(line), "{line} is missing from\n{shown}");
        }
        assert_eq!(shown.matches(&format!("in str GameDataFullName = \"{FLAG}\"")).count(), 2, "{shown}");
        assert!(hook::patch_graph(&out, &HOOK).is_err(), "a patched graph is not patched twice");
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
        let ours: usize = mine.find_els(TIMER).into_iter().find(|i| shape(&mine, *i).iter().any(|l| l.contains("IsLoop"))).unwrap();
        assert_eq!(shape(&mine, ours), shape(&twin, theirs));
        let glass: Ainb = Ainb::parse(&sarc::get(&es, "AI/EnemyCommon.common.enableglasstruth.module.ainb").unwrap().data).unwrap();
        assert_eq!(shape(&mine, mine.find_els(IS_DEAD)[0]), shape(&glass, glass.find_els(IS_DEAD)[0]));
        let sw: usize = (0..glass.els.len()).find(|i| glass.el_type(*i) == SWITCH).unwrap();
        let my_sw: usize = (base.els.len()..mine.els.len()).find(|i| mine.el_type(*i) == SWITCH).unwrap();
        assert_eq!(shape(&mine, my_sw), shape(&glass, sw));
        assert_eq!(mine.params[my_sw].pl, glass.params[sw].pl);
        assert_eq!(mine.params[my_sw].plugs[0].extra.len(), glass.params[sw].plugs[0].extra.len());
        let npc: Ainb = Ainb::parse(&sarc::get(&es, "AI/SimpleNpc.ExecuteInterruptReaction.module.ainb").unwrap().data).unwrap();
        assert_eq!(shape(&mine, mine.find_els(HAS_LIFE)[0]), shape(&npc, npc.find_els(HAS_LIFE)[0]));
        let floor: Vec<Entry> = sarc::parse(&rf.read_zs("Pack/Actor/Enemy_Floormaster.pack.zs").unwrap()).unwrap();
        let down: Ainb = Ainb::parse(&sarc::get(&floor, "AI/Floormaster.action.interruptdown.module.ainb").unwrap().data).unwrap();
        assert_eq!(shape(&mine, mine.find_els(HEAL)[0]), shape(&down, down.find_els(HEAL)[0]));
        let stone: Vec<Entry> = sarc::parse(&rf.read_zs("Pack/Actor/AgonyStoneReactArea.pack.zs").unwrap()).unwrap();
        let area: Ainb = Ainb::parse(&sarc::get(&stone, "AI/AgonyStoneReactArea.root.ainb").unwrap().data).unwrap();
        assert_eq!(shape(&mine, mine.find_els(CLEAR)[0]), shape(&area, area.find_els(CLEAR)[0]));
        let away: Ainb = Ainb::parse(&sarc::get(&es, "AI/NpcCommon.IsLookAway.module.ainb").unwrap().data).unwrap();
        assert_eq!(shape(&mine, mine.find_els(GET)[0]), shape(&away, away.find_els(GET)[0]));
        let player: Ainb = Ainb::parse(&sarc::get(&es, "AI/Player.event.root.ainb").unwrap().data).unwrap();
        let full: usize = player.find_els("EventTriggerHealLifeMax")[0];
        let does: usize = player.params[full].group(2)[0].a as usize;
        assert_eq!((player.el_name(does), player.in_s32(does, "SetType"), player.in_s32(does, "SetValue")), (HEAL, Some(TO_MAX), Some(0)), "the player is healed to full the same way");
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
        assert_eq!((outs[0].path.as_str(), outs[outs.len() - 1].path.as_str()), (GLOBAL, ACTOR_INFO));
        assert_eq!(zs::dict_of(&outs[0].z), zs::dict_of(&rf.read(GLOBAL).unwrap()), "the AI pack keeps its dictionary");
        let last: usize = outs.len() - 1;
        assert!(outs[..last].iter().all(|x| x.new > x.old) && outs[last].new == outs[last].old);
        for x in &outs[..last] {
            let now: Vec<Entry> = sarc::parse(&rf.zs.decompress(&x.z).unwrap()).unwrap();
            let old: Vec<Entry> = sarc::parse(&rf.read_zs(&x.path).unwrap()).unwrap();
            for g in GRAPHS {
                assert_eq!(sarc::get(&now, g).map(|e| &e.data), sarc::get(&old, g).map(|e| &e.data), "{g} is vanilla in {}", x.path);
            }
            for (a, b) in now.iter().zip(old.iter()).filter(|(a, b)| a.data != b.data) {
                assert!(hook::is_caller(b), "{} in {} is not a caller", a.name, x.path);
            }
        }
        let was: Byml = rf.read_byml(ACTOR_INFO).unwrap();
        let now: Byml = Byml::parse(&rf.zs.decompress(&outs[last].z).unwrap()).unwrap();
        let grown: usize = was.root.as_arr().unwrap().iter().zip(now.root.as_arr().unwrap().iter()).filter(|(x, y)| x != y).count();
        assert!(grown >= 80, "{grown} actors grew");
    }
}
