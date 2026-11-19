use std::collections::BTreeMap as BM;

use crate::ainb::build::{Spec, QUERY};
use crate::ainb::Ainb;
use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::start_defaults::Edit;
use crate::qol::{edit_plain, edit_zs, Have};
use crate::romfs::Romfs;
use crate::staging::Out;

pub const CARD: &str = "IsGet.Item_Token_Gerudo";
pub const GUARD: &str = "ActorLogicNpcSecurityGuard";
pub const ENABLE: &str = "EnableSecurityGuard";
pub const GET_BOOL: &str = "HoldGameDataGetBoolSimple";
pub const AND: &str = "OperateAND";
pub const OR: &str = "OperateOR";
pub const MULTI: &str = "BoolMulti";
pub const BOOL: usize = 1;
pub const BOOL_IN: usize = 16;
pub const MULTI_COUNT: usize = 0x1c;
pub const TRUE_OUT: i16 = 1;
pub const FALSE_OUT: u16 = 0;
pub const AREA: &str = "ActorLogicAreaActor";
pub const CREATE: &str = "Logic_Create";
pub const STEP_CHECK: &str = "HoldChallengeCurrentStepCheckOrder";
pub const ARCHERY: (&str, &str) = ("Logic/MiniGame_HorsebackArchery_8380.logic.module.ainb", "Area_a5fa");
pub const QUEST_EXP: &str = "SageOfSpirit_IsAfter_Step01_Exp";
pub const MEMBER_EXP: &str = "Defeat_Ganon_IsAfter_Step09_Exp";
pub const READ_OP: i32 = 0;
pub const READ_EXP_OP: i32 = 10;
pub const OR_OP: i32 = 4;
pub const GATE_FLOW: &str = "Npc_Gerudo_006.bfevfl";
pub const SCHEDULE: (&str, &str) = ("Component/AIScheduleParam/", ".game__component__AIScheduleParam.bgyml");

pub const MEMBERS: [&str; 14] = [
    "Npc_Gerudo_003_Adult",
    "Npc_Gerudo_004",
    "Npc_Gerudo_006",
    "Npc_Gerudo_007",
    "Npc_Gerudo_009",
    "Npc_Gerudo_010",
    "Npc_Gerudo_011",
    "Npc_Gerudo_012",
    "Npc_Gerudo_013",
    "Npc_Gerudo_014",
    "Npc_Gerudo_015",
    "Npc_Gerudo_016",
    "Npc_Gerudo_017",
    "Npc_Gerudo_018",
];

pub const MODULES: [&str; 7] = [
    "Logic/GerudoFortress_Common_178b.logic.module.ainb",
    "Logic/GerudoFortress_Common_1d80.logic.module.ainb",
    "Logic/GerudoFortress_Level_Exterior_01_a401.logic.module.ainb",
    "Logic/GerudoFortress_Level_Room_02_5d06.logic.module.ainb",
    "Logic/GerudoFortress_Level_Room_05_ee1c.logic.module.ainb",
    "Logic/GerudoFortress_Level_Room_06_ade9.logic.module.ainb",
    "Logic/MiniGame_HorsebackArchery_8380.logic.module.ainb",
];

pub const ALWAYS_OFF: [&str; 6] = [
    "Logic/GerudoFortress_Common_58a8.logic.module.ainb",
    "Logic/GerudoFortress_Common_Level_CarpenterSet_01_e139.logic.module.ainb",
    "Logic/GerudoFortress_Common_Level_CarpenterSet_02_9477.logic.module.ainb",
    "Logic/GerudoFortress_Common_Level_CarpenterSet_03_c7f2.logic.module.ainb",
    "Logic/GerudoFortress_Common_Level_CarpenterSet_04_e9a4.logic.module.ainb",
    "Logic/GerudoFortress_Level_Room_06_1561.logic.module.ainb",
];

fn i16_at(b: &mut [u8], o: usize, v: i16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}

fn range(a: &Ainb, i: usize) -> (usize, usize) {
    let e = &a.els[i];
    (u16::from_le_bytes([e[0x24], e[0x25]]) as usize, u16::from_le_bytes([e[0x26], e[0x27]]) as usize)
}

fn set_range(a: &mut Ainb, i: usize, base: usize, n: usize) -> Res<()> {
    let b: u16 = u16::try_from(base).map_err(|_| Error::Format("gerudo card: query list too long"))?;
    let c: u16 = u16::try_from(n).map_err(|_| Error::Format("gerudo card: query list too long"))?;
    a.els[i][0x24..0x26].copy_from_slice(&b.to_le_bytes());
    a.els[i][0x26..0x28].copy_from_slice(&c.to_le_bytes());
    Ok(())
}

fn qpos(a: &Ainb, i: usize) -> Res<u16> {
    let p: usize = a.query_pos(i).ok_or(Error::Format("gerudo card: a source is not a query node"))?;
    u16::try_from(p).map_err(|_| Error::Format("gerudo card: too many query nodes"))
}

pub fn queries(a: &Ainb, i: usize) -> Vec<u16> {
    let (s, n) = range(a, i);
    (s..s + n).map(|j| u16::from_le_bytes([a.qids[4 * j], a.qids[4 * j + 1]])).collect()
}

fn give_queries(a: &mut Ainb, i: usize, want: &[u16]) -> Res<()> {
    let base: usize = a.qids.len() / 4;
    for q in want {
        a.qids.extend_from_slice(&q.to_le_bytes());
        a.qids.extend_from_slice(&0u16.to_le_bytes());
    }
    set_range(a, i, base, want.len())
}

pub fn multi_node(a: &mut Ainb, name: &str, seed: &str, srcs: &[(usize, u16)]) -> Res<usize> {
    let first: usize = srcs.first().ok_or(Error::Format("gerudo card: a gate needs a source"))?.0;
    let mut s: Spec = Spec::node(name, seed).linked(BOOL, MULTI, first, 0).constant(BOOL, "UpdateOnce", vec![0; 4]).out(BOOL, "False").out(BOOL, "True");
    s.role = QUERY;
    let id: usize = a.add_node(&s)?;
    let start: usize = a.multi.len() / 8;
    for (el, out) in srcs {
        let e: u16 = u16::try_from(*el).map_err(|_| Error::Format("gerudo card: element index too large"))?;
        a.multi.extend_from_slice(&e.to_le_bytes());
        a.multi.extend_from_slice(&out.to_le_bytes());
        a.multi.extend_from_slice(&[0; 4]);
    }
    let g: usize = a.find_in(id, BOOL, MULTI).ok_or(Error::Format("gerudo card: the gate has no multi input"))?;
    let at: usize = g * BOOL_IN;
    let src: i16 = i16::try_from(-100 - start as i64).map_err(|_| Error::Format("gerudo card: multi table too long"))?;
    let n: i16 = i16::try_from(srcs.len()).map_err(|_| Error::Format("gerudo card: too many gate sources"))?;
    i16_at(&mut a.ins[BOOL], at + 4, src);
    i16_at(&mut a.ins[BOOL], at + 6, n);
    a.els[id][MULTI_COUNT..MULTI_COUNT + 4].copy_from_slice(&(srcs.len() as u32).to_le_bytes());
    let mut want: Vec<u16> = Vec::new();
    for (el, _) in srcs {
        let q: u16 = qpos(a, *el)?;
        if !want.contains(&q) {
            want.push(q);
        }
    }
    give_queries(a, id, &want)?;
    Ok(id)
}

pub fn card_node(a: &mut Ainb, seed: &str) -> Res<usize> {
    bool_node(a, CARD, seed)
}

pub fn bool_node(a: &mut Ainb, flag: &str, seed: &str) -> Res<usize> {
    let name: Vec<u8> = a.pool.add(flag).to_le_bytes().to_vec();
    let mut s: Spec = Spec::node(GET_BOOL, seed).constant(0, "GameDataArrayIndex", (-1i32).to_le_bytes().to_vec()).constant(3, "GameDataNameBool", name).out(BOOL, "Bool");
    s.role = QUERY;
    a.add_node(&s)
}

pub fn guards(a: &Ainb) -> Vec<(usize, usize)> {
    a.find_els(GUARD).into_iter().filter_map(|i| a.find_in(i, BOOL, ENABLE).map(|g| (i, g))).collect()
}

pub fn patch(a: &mut Ainb, tag: &str) -> Res<usize> {
    let linked: Vec<(usize, usize, i16, i16)> = guards(a)
        .into_iter()
        .filter_map(|(i, g)| a.input(BOOL, g).map(|x| (i, g, x.src, x.plug)))
        .filter(|(_, _, src, _)| *src >= 0)
        .collect();
    if guards(a).iter().any(|(_, g)| a.input(BOOL, *g).is_some_and(|x| x.src <= -100)) {
        return Err(Error::Format("gerudo card: a guard reads a multi input, the game files differ"));
    }
    if linked.is_empty() {
        return Err(Error::Format("gerudo card: no guard's security is linked, the game files differ"));
    }
    let card: usize = card_node(a, &format!("ootrr gerudo card {tag} read"))?;
    let lacks: usize = multi_node(a, OR, &format!("ootrr gerudo card {tag} lacks"), &[(card, 0)])?;
    let mut gates: BM<(i16, i16), usize> = BM::new();
    let enable: u32 = a.pool.add(ENABLE);
    for (i, g, src, plug) in &linked {
        let gate: usize = match gates.get(&(*src, *plug)) {
            Some(x) => *x,
            None => {
                let from: u16 = u16::try_from(*plug).map_err(|_| Error::Format("gerudo card: bad output index"))?;
                let n: usize = gates.len();
                let x: usize = multi_node(a, AND, &format!("ootrr gerudo card {tag} gate {n}"), &[(*src as usize, from), (lacks, FALSE_OUT)])?;
                gates.insert((*src, *plug), x);
                x
            }
        };
        rewire(a, *i, *g, *src, enable, gate, TRUE_OUT)?;
    }
    Ok(linked.len())
}

pub fn rewire(a: &mut Ainb, i: usize, g: usize, src: i16, name: u32, gate: usize, out: i16) -> Res<()> {
    let at: usize = g * BOOL_IN;
    let to: i16 = i16::try_from(gate).map_err(|_| Error::Format("gerudo card: element index too large"))?;
    i16_at(&mut a.ins[BOOL], at + 4, to);
    i16_at(&mut a.ins[BOOL], at + 6, out);
    let p = &mut a.params[i];
    let (n0, s0): (usize, usize) = (p.pl[0][0] as usize, p.pl[0][1] as usize);
    let hit: usize = (s0..s0 + n0)
        .find(|&k| p.plugs[k].a == src as u32 && p.plugs[k].b == name)
        .ok_or(Error::Format("gerudo card: a rewired input has no plug"))?;
    p.plugs[hit].a = gate as u32;
    let mut want: Vec<u16> = queries(a, i);
    let q: u16 = qpos(a, gate)?;
    if !want.contains(&q) {
        want.push(q);
        give_queries(a, i, &want)?;
    }
    Ok(())
}

pub fn area_of(a: &Ainb, instance: &str) -> Option<(usize, usize)> {
    a.find_els(AREA).into_iter().find(|&i| a.prop_str(i, "InstanceName") == Some(instance)).and_then(|i| a.find_in(i, BOOL, CREATE).map(|g| (i, g)))
}

pub fn open_area(a: &mut Ainb, instance: &str, tag: &str) -> Res<usize> {
    let (i, g): (usize, usize) = area_of(a, instance).ok_or(Error::Format("gerudo card: the member area is missing, the game files differ"))?;
    let x = a.input(BOOL, g).ok_or(Error::Format("gerudo card: the member area has no create input"))?;
    let (src, plug): (i16, i16) = (x.src, x.plug);
    if src < 0 || a.el_name(src as usize) != STEP_CHECK || plug != 0 {
        return Err(Error::Format("gerudo card: the member area is not created by the quest step, the game files differ"));
    }
    let held: Option<usize> = a.find_els(GET_BOOL).into_iter().filter(|&k| a.in_str(k, "GameDataNameBool") == Some(CARD)).last();
    let card: usize = match held {
        Some(k) => k,
        None => card_node(a, &format!("ootrr gerudo card {tag} member read"))?,
    };
    let gate: usize = multi_node(a, OR, &format!("ootrr gerudo card {tag} member"), &[(src as usize, 0), (card, 0)])?;
    let create: u32 = a.pool.add(CREATE);
    rewire(a, i, g, src, create, gate, TRUE_OUT)?;
    Ok(gate)
}

pub fn patch_bytes(raw: &[u8], tag: &str) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    patch(&mut a, tag)?;
    if tag == ARCHERY.0 {
        open_area(&mut a, ARCHERY.1, tag)?;
    }
    let out: Vec<u8> = a.write()?;
    let back: Ainb = Ainb::parse(&out)?;
    if back.write()? != out || !crate::ainb::audit(&back).is_empty() {
        return Err(Error::Format("gerudo card: the patched module does not read back"));
    }
    Ok(out)
}

pub fn exps() -> Vec<Edit> {
    vec![Edit::Exp(MEMBER_EXP.to_string(), vec![(READ_EXP_OP, QUEST_EXP.to_string()), (READ_OP, CARD.to_string()), (OR_OP, String::new())])]
}

pub fn schedule_path(npc: &str) -> String {
    format!("{}{npc}{}", SCHEDULE.0, SCHEDULE.1)
}

pub fn member_schedule(raw: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    let list: &mut Vec<Node> = y
        .root
        .get_mut("ConditionalSchedule")
        .and_then(|c| c.as_arr_mut())
        .ok_or(Error::Format("gerudo card: a schedule has no conditional list, the game files differ"))?;
    let mut n: usize = 0;
    for c in list.iter_mut() {
        if c.get("ConditionFlag").and_then(|v| v.as_str()) == Some(QUEST_EXP) {
            c.set("ConditionFlag", Node::Str(MEMBER_EXP.to_string()));
            n += 1;
        }
    }
    if n != 1 {
        return Err(Error::Format("gerudo card: a schedule does not check membership once, the game files differ"));
    }
    y.write()
}

pub const SCHEDULE_PACK: &str = "Pack/AIScheduleParam.pack.zs";
pub const PACKED: [&str; 1] = ["Minigame_HorsebackArchery_Ready_Npc_Gerudo_008"];

pub fn member_pack(raw: &[u8]) -> Res<Vec<u8>> {
    let mut es: Vec<crate::sarc::Entry> = crate::sarc::parse(raw)?;
    for name in PACKED {
        let f: String = schedule_path(name);
        let e: &crate::sarc::Entry = crate::sarc::get(&es, &f).ok_or_else(|| Error::Msg(format!("gerudo card: the schedule pack has no {name}, the game files differ")))?;
        let out: Vec<u8> = member_schedule(&e.data)?;
        crate::sarc::set(&mut es, &f, out);
    }
    crate::sarc::write(&es)
}

pub fn stage(rf: &mut Romfs, have: Have) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = MODULES.iter().map(|m| edit_plain(rf, have, m, |raw| patch_bytes(raw, m))).collect::<Res<_>>()?;
    for npc in MEMBERS {
        outs.push(edit_plain(rf, have, &schedule_path(npc), member_schedule)?);
    }
    outs.push(edit_zs(rf, have, SCHEDULE_PACK, member_pack)?);
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf as PB;

    fn real() -> Option<PB> {
        let p: PB = ootrr_game::test_romfs::path();
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Some(p)
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    fn src_of(a: &Ainb, g: usize) -> (i16, i16) {
        let x = a.input(BOOL, g).unwrap();
        (x.src, x.plug)
    }

    fn multi_of(a: &Ainb, i: usize) -> Vec<(u16, u16)> {
        let g: usize = a.find_in(i, BOOL, MULTI).unwrap();
        let x = a.input(BOOL, g).unwrap();
        let start: usize = (-100 - i32::from(x.src)) as usize;
        (start..start + x.plug as usize).map(|j| crate::door_open::multi_entry(a, j).unwrap()).collect()
    }

    #[test]
    fn every_multi_input_node_in_the_game_counts_its_entries_at_0x1c() {
        let Some(root) = real() else {
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        for m in MODULES.iter().chain(ALWAYS_OFF.iter()).chain(["Logic/GerudoFortress_Common_35d7.logic.module.ainb"].iter()) {
            let a: Ainb = Ainb::parse(&rf.read(m).unwrap()).unwrap();
            for i in a.find_els(AND).into_iter().chain(a.find_els(OR)) {
                let x = a.input(BOOL, a.find_in(i, BOOL, MULTI).unwrap()).unwrap();
                let want: u8 = if x.src <= -100 { x.plug as u8 } else { 0 };
                assert_eq!(a.els[i][MULTI_COUNT], want, "{m} element {i}");
            }
        }
    }

    #[test]
    fn every_fortress_guard_is_switched_off_by_the_card_and_keeps_its_quest_check() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let outs: Vec<Out> = stage(&mut rf, &|_: &str| None).unwrap();
        assert_eq!(outs.len(), MODULES.len() + MEMBERS.len() + 1);
        let mut total: usize = 0;
        for (m, o) in MODULES.iter().zip(&outs) {
            let skip: Option<usize> = if *m == ARCHERY.0 { area_of(&Ainb::parse(&rf.read(m).unwrap()).unwrap(), ARCHERY.1).map(|x| x.0) } else { None };
            let old: Ainb = Ainb::parse(&rf.read(m).unwrap()).unwrap();
            let new: Ainb = Ainb::parse(&o.z).unwrap();
            assert!(crate::ainb::audit(&new).is_empty(), "{m}");
            let card: Vec<usize> = new.find_els(GET_BOOL).into_iter().filter(|&i| new.in_str(i, "GameDataNameBool") == Some(CARD)).collect();
            assert_eq!(card.len(), 1 + old.find_els(GET_BOOL).into_iter().filter(|&i| old.in_str(i, "GameDataNameBool") == Some(CARD)).count(), "{m}");
            let card: usize = *card.last().unwrap();
            let lacks: Vec<usize> = new.find_els(OR).into_iter().filter(|&i| i >= old.els.len() && multi_of(&new, i) == vec![(card as u16, 0)]).collect();
            assert_eq!(lacks.len(), 1, "{m}");
            for ((i, g), (j, h)) in guards(&old).into_iter().zip(guards(&new)) {
                assert_eq!(i, j, "{m}");
                let (s, p) = src_of(&old, g);
                if s < 0 {
                    assert_eq!(src_of(&new, h), (s, p), "{m}");
                    continue;
                }
                let (gate, out) = src_of(&new, h);
                assert_eq!(out, TRUE_OUT, "{m}");
                assert_eq!(new.el_name(gate as usize), AND, "{m}");
                assert_eq!(multi_of(&new, gate as usize), vec![(s as u16, p as u16), (lacks[0] as u16, FALSE_OUT)], "{m}");
                assert!(queries(&new, i).contains(&(gate as u16)), "{m}");
                assert_eq!(new.els[gate as usize][MULTI_COUNT], 2, "{m}");
                total += 1;
            }
            for i in 0..old.els.len() {
                if old.el_name(i) == GUARD || Some(i) == skip {
                    continue;
                }
                for k in 0..6 {
                    for (g, h) in old.input_ids(i, k).into_iter().zip(new.input_ids(i, k)) {
                        assert_eq!(old.input(k, g).map(|x| (x.src, x.plug)), new.input(k, h).map(|x| (x.src, x.plug)), "{m} element {i}");
                    }
                }
            }
        }
        assert_eq!(total, 17);
        for m in ALWAYS_OFF {
            let a: Ainb = Ainb::parse(&rf.read(m).unwrap()).unwrap();
            let gs: Vec<(usize, usize)> = guards(&a);
            assert!(!gs.is_empty(), "{m}");
            for (_, g) in gs {
                let x = a.input(BOOL, g).unwrap();
                assert!(x.src < 0 && x.val == vec![0; 4], "{m}");
            }
            assert!(patch(&mut a.clone(), m).is_err(), "{m}");
        }
    }

    #[test]
    fn the_archery_area_is_created_for_a_member_or_a_card_holder() {
        let Some(root) = real() else {
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        let old: Ainb = Ainb::parse(&rf.read(ARCHERY.0).unwrap()).unwrap();
        let new: Ainb = Ainb::parse(&patch_bytes(&rf.read(ARCHERY.0).unwrap(), ARCHERY.0).unwrap()).unwrap();
        let (i, g) = area_of(&old, ARCHERY.1).unwrap();
        let (step, plug) = src_of(&old, g);
        assert_eq!((old.el_name(step as usize), plug), (STEP_CHECK, 0));
        let (j, h) = area_of(&new, ARCHERY.1).unwrap();
        assert_eq!(i, j);
        let (gate, out) = src_of(&new, h);
        assert_eq!((new.el_name(gate as usize), out), (OR, TRUE_OUT));
        let srcs: Vec<(u16, u16)> = multi_of(&new, gate as usize);
        assert_eq!(srcs.len(), 2);
        assert_eq!(srcs[0], (step as u16, 0));
        assert_eq!(new.in_str(srcs[1].0 as usize, "GameDataNameBool"), Some(CARD));
        assert_eq!(new.els[gate as usize][MULTI_COUNT], 2);
        assert!(queries(&new, j).contains(&(gate as u16)));
        assert!(crate::ainb::audit(&new).is_empty());
    }

    #[test]
    fn only_the_gerudo_schedules_swap_the_quest_check_for_the_member_check() {
        let Some(root) = real() else {
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        let dir: PB = root.join(SCHEDULE.0.trim_end_matches('/'));
        let mut named: Vec<String> = Vec::new();
        for e in std::fs::read_dir(&dir).unwrap() {
            let p: PB = e.unwrap().path();
            let raw: Vec<u8> = std::fs::read(&p).unwrap();
            if raw.windows(QUEST_EXP.len()).any(|w| w == QUEST_EXP.as_bytes()) {
                named.push(p.file_name().unwrap().to_string_lossy().trim_end_matches(SCHEDULE.1).to_string());
            }
        }
        named.sort();
        let mut want: Vec<String> = MEMBERS.iter().map(|s| s.to_string()).collect();
        want.push("Npc_Hylia_012_Adult".to_string());
        want.sort();
        assert_eq!(named, want);
        for npc in MEMBERS {
            let raw: Vec<u8> = rf.read(&schedule_path(npc)).unwrap();
            let old: Byml = Byml::parse(&raw).unwrap();
            let mut new: Byml = Byml::parse(&member_schedule(&raw).unwrap()).unwrap();
            let list: &mut Vec<Node> = new.root.get_mut("ConditionalSchedule").unwrap().as_arr_mut().unwrap();
            let hits: Vec<usize> = (0..list.len()).filter(|&k| list[k].get("ConditionFlag").and_then(|v| v.as_str()) == Some(MEMBER_EXP)).collect();
            assert_eq!(hits.len(), 1, "{npc}");
            list[hits[0]].set("ConditionFlag", Node::Str(QUEST_EXP.to_string()));
            assert_eq!(new.root, old.root, "{npc}");
        }
    }

    #[test]
    fn the_member_check_is_a_spare_formula_made_of_the_quest_step_or_the_card() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read_zs(crate::chest_slot::LIST).unwrap();
        let mut y: Byml = Byml::parse(&raw).unwrap();
        let num = |n: &Node| -> u32 {
            match n {
                Node::I32(x) => *x as u32,
                Node::U32(x) => *x,
                Node::I64(x) => *x as u32,
                Node::U64(x) => *x as u32,
                _ => panic!("not a number"),
            }
        };
        let rows = |y: &Byml| -> Vec<Node> { y.root.get("Data").and_then(|d| d.get("BoolExp")).and_then(|v| v.as_arr()).unwrap().to_vec() };
        let key = |n: &str| crate::start_defaults::key(n);
        let hash = |r: &Node| r.get("Hash").map(num);
        let terms = |r: &Node| -> Vec<Vec<u32>> { r.get("Values").and_then(|v| v.as_arr()).unwrap().iter().map(|t| t.as_arr().unwrap().iter().map(num).collect()).collect() };
        let exps0: Vec<Node> = rows(&y);
        let names: Vec<u32> = exps0.iter().filter_map(hash).collect();
        assert!(names.contains(&key(MEMBER_EXP)) && names.contains(&key(QUEST_EXP)));
        assert!(exps0.iter().all(|r| terms(r).iter().all(|t| !(t.len() == 2 && t[1] == key(MEMBER_EXP)))));
        assert!(exps0.iter().any(|r| terms(r).iter().any(|t| t.len() == 2 && t[0] == READ_EXP_OP as u32 && names.contains(&t[1]))));
        assert_eq!(crate::start_defaults::apply(&mut y, &exps()).unwrap(), 1);
        let row: Node = rows(&y).into_iter().find(|r| hash(r) == Some(key(MEMBER_EXP))).unwrap();
        assert_eq!(terms(&row), vec![vec![READ_EXP_OP as u32, key(QUEST_EXP)], vec![READ_OP as u32, key(CARD)], vec![OR_OP as u32]]);
        let Ok(a) = Byml::parse(&y.write().unwrap()) else { panic!() };
        assert_eq!(a.root, y.root);
    }

    #[test]
    fn the_archery_ready_schedule_in_the_pack_takes_the_member_check_and_nothing_else_changes() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read_zs(SCHEDULE_PACK).unwrap();
        let old: Vec<crate::sarc::Entry> = crate::sarc::parse(&raw).unwrap();
        let named: Vec<&str> = old.iter().filter(|e| e.data.windows(QUEST_EXP.len()).any(|w| w == QUEST_EXP.as_bytes())).map(|e| e.name.as_str()).collect();
        assert_eq!(named, PACKED.iter().map(|n| schedule_path(n)).collect::<Vec<String>>().iter().map(|s| s.as_str()).collect::<Vec<&str>>());
        let new: Vec<crate::sarc::Entry> = crate::sarc::parse(&member_pack(&raw).unwrap()).unwrap();
        assert_eq!(old.len(), new.len());
        for (a, b) in old.iter().zip(&new) {
            assert_eq!(a.name, b.name);
            if PACKED.iter().any(|n| schedule_path(n) == a.name) {
                let y: Byml = Byml::parse(&b.data).unwrap();
                let flags: Vec<&str> = y.root.get("ConditionalSchedule").and_then(|c| c.as_arr()).unwrap().iter().filter_map(|c| c.get("ConditionFlag").and_then(|v| v.as_str())).collect();
                assert!(flags.contains(&MEMBER_EXP) && !flags.contains(&QUEST_EXP), "{flags:?}");
            } else {
                assert_eq!(a.data, b.data, "{}", a.name);
            }
        }
    }
}
