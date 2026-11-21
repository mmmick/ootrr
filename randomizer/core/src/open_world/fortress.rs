//! Gerudo Fortress follows its own rescue, not the Spirit quest.
//!
//! The fortress (hostile guards, jailed carpenters, the thief fights, the jail-side guard, the
//! valley bridge, horseback archery) reads one thing: "SageOfSpirit is after Step01", through
//! HoldChallengeCurrentStepCheckOrder nodes in the fortress logic modules and through the expression [`CLEARED`]
//! (schedules, flows, the valley banc). The Spirit quest also moves on at the Colossus (Step04),
//! with child Nabooru (Step05 and on) and at the temple's end (Complete), so a seed that reaches
//! the Spirit Temple first found a friendly fortress whose carpenters, still in their cells, talked
//! as if rescued and never called the thief.
//!
//! Two states replace the quest step (user rule, 2026-11-21: the fortress follows the Membership
//! Card in the inventory):
//! - the fortress itself (guards, Gerudo NPCs, the valley bridge, horseback archery) is cleared
//!   while Link holds the card: [`CLEARED`] reads [`CARD`], and every Step01 node in
//!   [`CARD_MODULES`] is cut off for an OR over [`CLEARED`] (IsAfter becomes cleared, IsBefore and
//!   IsEqual, only ever read together, become not cleared);
//! - the rescue (the carpenters' jail behaviour, which also calls the thieves, and the jail-side
//!   guard of the card scene) is done once the clear event played ([`MARK`], which quest_order
//!   writes at the end of SageOfSpirit:Meet_FourCarpenter_ClearEvent and the open fortress preset
//!   sets). The rescue does not follow the card: a card found elsewhere must not shut the cells, or
//!   the card scene's shuffled item would be lost. [`RESCUE_MODULES`] read [`MARK`]; each
//!   carpenter's jail behaviour lived in the Step01 participant schedule
//!   (SageOfSpirit_Step01_Npc_Hylia_01x_Adult_GerudoFortress), which no later step carries, so their
//!   own schedule now holds it: its first condition is [`MARK`] (their own old behaviour), then the
//!   Step01 conditions, then the Step01 default. A schedule parses into about ten times its file
//!   size, so [`budget`] gives each merged schedule the sizes of both schedules it holds (the plain
//!   growth margin crashed the game's loader).
//!
//! The Requiem scene's map logic also asks for SageOfSpirit Step05 when the scene ends; it now only
//! does while the quest is not past Step05, so a late Requiem cannot roll the temple back.

use crate::ainb::build::{Spec, QUERY};
use crate::ainb::Ainb;
use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::gerudo_card::{give_queries, multi_node, qpos, queries, rewire, AND, BOOL, FALSE_OUT, OR, TRUE_OUT};
use crate::qol::{edit_plain, Have};
use crate::romfs::Romfs;
use crate::staging::Out;
use crate::start_defaults::Edit;

pub const CLEARED: &str = "SageOfSpirit_IsAfter_Step01_Exp";
pub const CARD: &str = crate::gerudo_card::CARD;
pub const MARK: &str = "HavePlayedEvent.Meet_FourCarpenter_ClearEvent";
pub const SPIRIT: &str = "Work/Challenge/SageOfSpirit.game__challenge__Challenge.gyml";
pub const STEP: &str = "Step01";
pub const STEP_CHECK: &str = "HoldChallengeCurrentStepCheckOrder";
pub const SET_STEP: &str = "TriggerChallengeSetStep";
const READ_EXP: &str = "HoldGameDataGetBoolExpSimple";
const EXP_IN: &str = "GameDataNameBoolExp";
const STR: usize = 3;
/// HoldChallengeCurrentStepCheckOrder outputs, in order.
const IS_AFTER: i16 = 0;
const IS_BEFORE: i16 = 1;
const IS_EQUAL: i16 = 2;

/// The guards and the archery range: cleared while Link holds the card.
pub const CARD_MODULES: [&str; 7] = [
    "Logic/GerudoFortress_Common_178b.logic.module.ainb",
    "Logic/GerudoFortress_Common_1d80.logic.module.ainb",
    "Logic/GerudoFortress_Level_Exterior_01_a401.logic.module.ainb",
    "Logic/GerudoFortress_Level_Room_02_5d06.logic.module.ainb",
    "Logic/GerudoFortress_Level_Room_05_ee1c.logic.module.ainb",
    "Logic/GerudoFortress_Level_Room_06_ade9.logic.module.ainb",
    "Logic/MiniGame_HorsebackArchery_8380.logic.module.ainb",
];
/// The jail-side guard (Npc_Gerudo_001) of the card scene: done once the clear event played.
pub const RESCUE_MODULES: [&str; 5] = [
    "Logic/GerudoFortress_Common_58a8.logic.module.ainb",
    "Logic/GerudoFortress_Common_Level_CarpenterSet_01_e139.logic.module.ainb",
    "Logic/GerudoFortress_Common_Level_CarpenterSet_02_9477.logic.module.ainb",
    "Logic/GerudoFortress_Common_Level_CarpenterSet_03_c7f2.logic.module.ainb",
    "Logic/GerudoFortress_Common_Level_CarpenterSet_04_e9a4.logic.module.ainb",
];

/// What a module's quest step reads become.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Read {
    Exp(&'static str),
    Bool(&'static str),
}

pub fn read_of(path: &str) -> Option<Read> {
    if CARD_MODULES.contains(&path) {
        Some(Read::Exp(CLEARED))
    } else if RESCUE_MODULES.contains(&path) {
        Some(Read::Bool(MARK))
    } else {
        None
    }
}

pub const COLOSSUS: &str = "Logic/DesertColossus_Common_665d.logic.module.ainb";
pub const REQUIEM_STEP: &str = "Step05";
pub const REQUIEM_AFTER: &str = "SageOfSpirit_IsAfter_Step05_Exp";

pub const CARPENTERS: [&str; 4] = ["013", "014", "015", "016"];

pub fn exps() -> Vec<Edit> {
    vec![Edit::Exp(CLEARED.to_string(), vec![(0, CARD.to_string())])]
}

pub fn own_schedule(n: &str) -> String {
    crate::gerudo_card::schedule_path(&format!("Npc_Hylia_{n}_Adult"))
}

pub fn jail_schedule(n: &str) -> String {
    crate::gerudo_card::schedule_path(&format!("SageOfSpirit_Step01_Npc_Hylia_{n}_Adult_GerudoFortress"))
}

/// Every logic module this file edits.
pub fn modules() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = CARD_MODULES.to_vec();
    v.extend(RESCUE_MODULES);
    v.push(COLOSSUS);
    v
}

/// Every plain file this file edits (logic modules and the carpenters' schedules).
pub fn files() -> Vec<String> {
    let mut v: Vec<String> = modules().into_iter().map(str::to_string).collect();
    v.extend(CARPENTERS.iter().map(|n| own_schedule(n)));
    v
}

fn exp_node(a: &mut Ainb, exp: &str, seed: &str) -> Res<usize> {
    let name: Vec<u8> = a.pool.add(exp).to_le_bytes().to_vec();
    let mut s: Spec = Spec::node(READ_EXP, seed).constant(STR, EXP_IN, name).out(BOOL, "Bool");
    s.role = QUERY;
    a.add_node(&s)
}

fn read_node(a: &mut Ainb, r: Read, seed: &str) -> Res<usize> {
    match r {
        Read::Exp(e) => exp_node(a, e, seed),
        Read::Bool(b) => crate::gerudo_card::bool_node(a, b, seed),
    }
}

fn entry(a: &Ainb, j: usize) -> Res<(u16, u16)> {
    crate::door_open::multi_entry(a, j).ok_or(Error::Format("gerudo fortress: the multi link table is too short"))
}

fn set_entry(a: &mut Ainb, j: usize, el: usize, out: i16) -> Res<()> {
    let e: u16 = u16::try_from(el).map_err(|_| Error::Format("gerudo fortress: element index out of range"))?;
    a.multi[8 * j..8 * j + 2].copy_from_slice(&e.to_le_bytes());
    a.multi[8 * j + 2..8 * j + 4].copy_from_slice(&(out as u16).to_le_bytes());
    Ok(())
}

fn cleared_out(plug: i16) -> Res<i16> {
    match plug {
        IS_AFTER => Ok(TRUE_OUT),
        IS_BEFORE | IS_EQUAL => Ok(FALSE_OUT as i16),
        _ => Err(Error::Format("gerudo fortress: a step check output the game does not have")),
    }
}

/// The Spirit quest Step01 checks of a module.
pub fn step_checks(a: &Ainb) -> Vec<usize> {
    a.find_els(STEP_CHECK).into_iter().filter(|&i| a.in_str(i, "ChallengeName") == Some(SPIRIT) && a.in_str(i, "ChallengeStepName") == Some(STEP)).collect()
}

/// Points every reader of the module's Step01 checks at one OR over `r`: its True output
/// for IsAfter, its False output for IsBefore and IsEqual. Returns how many links moved.
pub fn decouple(a: &mut Ainb, tag: &str, r: Read) -> Res<usize> {
    let steps: Vec<usize> = step_checks(a);
    if steps.is_empty() {
        return Err(Error::Msg(format!("gerudo fortress: {tag} reads no Spirit quest Step01, the game files differ")));
    }
    let read: usize = read_node(a, r, &format!("ootrr fortress {tag} cleared read"))?;
    let gate: usize = multi_node(a, OR, &format!("ootrr fortress {tag} cleared"), &[(read, 0)])?;
    let is_step = |src: i64| src >= 0 && steps.contains(&(src as usize));
    let mut moved: usize = 0;
    for i in 0..a.els.len() {
        if i == gate || i == read || steps.contains(&i) {
            continue;
        }
        for g in a.input_ids(i, BOOL) {
            let x = a.input(BOOL, g).ok_or(Error::Format("gerudo fortress: input out of range"))?;
            if x.src >= 0 {
                if is_step(i64::from(x.src)) {
                    rewire(a, i, g, x.src, x.name, gate, cleared_out(x.plug)?)?;
                    moved += 1;
                }
                continue;
            }
            if x.src > -100 || x.plug < 1 {
                continue;
            }
            let start: usize = (-100 - i32::from(x.src)) as usize;
            let n: usize = x.plug as usize;
            let mut hit: Vec<(usize, u16)> = Vec::new();
            for j in start..start + n {
                let (src, out) = entry(a, j)?;
                if is_step(i64::from(src)) {
                    hit.push((j, src));
                    set_entry(a, j, gate, cleared_out(out as i16)?)?;
                    moved += 1;
                }
            }
            if hit.is_empty() {
                continue;
            }
            let p = &mut a.params[i];
            let (n0, s0): (usize, usize) = (p.pl[0][0] as usize, p.pl[0][1] as usize);
            for k in s0..s0 + n0 {
                if p.plugs[k].b == x.name && hit.iter().any(|(_, s)| p.plugs[k].a == u32::from(*s)) {
                    p.plugs[k].a = gate as u32;
                }
            }
            let q: u16 = qpos(a, gate)?;
            let mut want: Vec<u16> = queries(a, i);
            if want.len() == n {
                for (j, _) in &hit {
                    want[j - start] = q;
                }
            } else {
                let old: Vec<u16> = hit.iter().filter_map(|(_, s)| a.query_pos(usize::from(*s)).and_then(|p| u16::try_from(p).ok())).collect();
                want.retain(|w| !old.contains(w));
                if !want.contains(&q) {
                    want.push(q);
                }
            }
            give_queries(a, i, &want)?;
        }
    }
    if moved == 0 {
        return Err(Error::Msg(format!("gerudo fortress: nothing in {tag} reads its Spirit quest check, the game files differ")));
    }
    Ok(moved)
}

/// The Colossus map logic's Step05 request after the Requiem scene runs only while the quest is not
/// past Step05.
pub fn guard_requiem(a: &mut Ainb) -> Res<()> {
    let set: Vec<usize> = a.find_els(SET_STEP).into_iter().filter(|&i| a.in_str(i, "ChallengeName") == Some(SPIRIT) && a.in_str(i, "StepName") == Some(REQUIEM_STEP)).collect();
    let [t] = set[..] else {
        return Err(Error::Format("gerudo fortress: the Colossus does not set Spirit Step05 once, the game files differ"));
    };
    let g: usize = a.find_in(t, BOOL, "IsDrive").ok_or(Error::Format("gerudo fortress: the Step05 request has no IsDrive"))?;
    let x = a.input(BOOL, g).ok_or(Error::Format("gerudo fortress: input out of range"))?;
    if x.src < 0 {
        return Err(Error::Format("gerudo fortress: the Step05 request is not driven by the Requiem event"));
    }
    let read: usize = exp_node(a, REQUIEM_AFTER, "ootrr requiem after read")?;
    let not: usize = multi_node(a, OR, "ootrr requiem not after", &[(read, 0)])?;
    // The AND's second source is the event actor's output, which is not a query: the AND is built
    // over the query twice, then its second entry is pointed at the actor.
    let and: usize = multi_node(a, AND, "ootrr requiem step", &[(not, FALSE_OUT), (not, FALSE_OUT)])?;
    let m: usize = a.find_in(and, BOOL, crate::gerudo_card::MULTI).ok_or(Error::Format("gerudo fortress: the new gate has no multi input"))?;
    let y = a.input(BOOL, m).ok_or(Error::Format("gerudo fortress: input out of range"))?;
    let start: usize = (-100 - i32::from(y.src)) as usize;
    set_entry(a, start + 1, x.src as usize, x.plug)?;
    rewire(a, t, g, x.src, x.name, and, TRUE_OUT)?;
    Ok(())
}

fn checked(a: &Ainb, what: &str) -> Res<Vec<u8>> {
    let out: Vec<u8> = a.write()?;
    let back: Ainb = Ainb::parse(&out)?;
    if back.write()? != out || !crate::ainb::audit(&back).is_empty() {
        return Err(Error::Msg(format!("gerudo fortress: {what} does not read back")));
    }
    Ok(out)
}

pub fn module_bytes(raw: &[u8], path: &str) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    if path == COLOSSUS {
        guard_requiem(&mut a)?;
    } else {
        let r: Read = read_of(path).ok_or_else(|| Error::Msg(format!("gerudo fortress: {path} is not a fortress module")))?;
        decouple(&mut a, path, r)?;
    }
    checked(&a, path)
}

/// A carpenter's own schedule with the jail behaviour of its Step01 schedule: rescue done first (the
/// old own behaviour), then the Step01 conditions, then the Step01 default.
pub fn jailed(own: &[u8], step: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(own)?;
    let s: Byml = Byml::parse(step)?;
    // Carpenter 016's own schedule carries an empty condition list.
    if y.root.get("ConditionalSchedule").and_then(|c| c.as_arr()).is_some_and(|c| !c.is_empty()) {
        return Err(Error::Format("gerudo fortress: a carpenter's own schedule already has conditions, the game files differ"));
    }
    let mut first: Node = y.root.get("DefaultSchedule").cloned().ok_or(Error::Format("gerudo fortress: a carpenter's own schedule has no default"))?;
    first.set("ConditionFlag", Node::Str(MARK.to_string()));
    let jail: &[Node] = s.root.get("ConditionalSchedule").and_then(|c| c.as_arr()).ok_or(Error::Format("gerudo fortress: a Step01 carpenter schedule has no conditions"))?;
    let default: Node = s.root.get("DefaultSchedule").cloned().ok_or(Error::Format("gerudo fortress: a Step01 carpenter schedule has no default"))?;
    let mut list: Vec<Node> = vec![first];
    list.extend(jail.iter().cloned());
    y.root.set("ConditionalSchedule", Node::Arr(list));
    y.root.set("DefaultSchedule", default);
    y.write()
}

fn merge_plain(rf: &Romfs, outs: &mut Vec<Out>, have: Have, path: &str, f: impl FnOnce(&[u8]) -> Res<Vec<u8>>) -> Res<()> {
    match outs.iter_mut().find(|o| o.path == path) {
        Some(o) => {
            let b: Vec<u8> = f(&o.z)?;
            o.new = b.len();
            o.z = b;
        }
        None => outs.push(edit_plain(rf, have, path, f)?),
    }
    Ok(())
}

/// Stages every edit on top of `outs` (the Gerudo Card stage edits some of the same modules).
pub fn stage(rf: &mut Romfs, outs: &mut Vec<Out>, have: Have) -> Res<()> {
    for m in modules() {
        merge_plain(rf, outs, have, m, |raw| module_bytes(raw, m))?;
    }
    let pack: Vec<u8> = rf.read_zs(crate::gerudo_card::SCHEDULE_PACK)?;
    let es: Vec<crate::sarc::Entry> = crate::sarc::parse(&pack)?;
    for n in CARPENTERS {
        let f: String = jail_schedule(n);
        let step: Vec<u8> = crate::sarc::get(&es, &f).map(|e| e.data.clone()).ok_or_else(|| Error::Msg(format!("gerudo fortress: the schedule pack has no {f}, the game files differ")))?;
        merge_plain(rf, outs, have, &own_schedule(n), |raw| jailed(raw, &step))?;
    }
    Ok(())
}

/// Gives each merged carpenter schedule at least the game's own budgets of the two schedules it now
/// holds. A schedule's parsed form is about ten times its file, so the usual growth (file bytes
/// plus a margin) left the loader short and the game crashed entering the fortress. Returns the
/// entries raised.
pub fn budget(rf: &mut Romfs, t: &mut crate::rsizetable::Rst) -> Res<usize> {
    let base: crate::rsizetable::Rst = crate::rsizetable::load(rf)?;
    let mut n: usize = 0;
    for c in CARPENTERS {
        let own: String = own_schedule(c);
        let jail: String = jail_schedule(c);
        let need: u32 = base.get(&own).ok_or_else(|| Error::Msg(format!("gerudo fortress: the size table has no {own}")))? + base.get(&jail).ok_or_else(|| Error::Msg(format!("gerudo fortress: the size table has no {jail}")))?;
        if t.get(&own).is_none_or(|cur| cur < need) {
            t.set_size(&own, need)?;
            n += 1;
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf as PB;

    const READ_BOOL: &str = "HoldGameDataGetBoolSimple";
    const BOOL_IN_NAME: &str = "GameDataNameBool";

    fn real() -> Option<PB> {
        let p: PB = ootrr_game::test_romfs::path();
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Some(p)
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    /// Every (element, input or multi entry) still reading a Step01 check.
    fn step_readers(a: &Ainb) -> usize {
        let steps: Vec<usize> = step_checks(a);
        let mut n: usize = 0;
        for i in 0..a.els.len() {
            for g in a.input_ids(i, BOOL) {
                let x = a.input(BOOL, g).unwrap();
                if x.src >= 0 && steps.contains(&(x.src as usize)) {
                    n += 1;
                } else if x.src <= -100 {
                    let s: usize = (-100 - i32::from(x.src)) as usize;
                    n += (s..s + x.plug as usize).filter(|&j| steps.contains(&(entry(a, j).unwrap().0 as usize))).count();
                }
            }
        }
        n
    }

    #[test]
    fn no_fortress_module_reads_the_spirit_quest_after_the_edit() {
        let Some(root) = real() else {
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        let mut total: usize = 0;
        for m in CARD_MODULES.iter().chain(RESCUE_MODULES.iter()) {
            let raw: Vec<u8> = rf.read(m).unwrap();
            let before: usize = step_readers(&Ainb::parse(&raw).unwrap());
            assert!(before > 0, "{m}");
            let out: Vec<u8> = module_bytes(&raw, m).unwrap();
            let a: Ainb = Ainb::parse(&out).unwrap();
            assert_eq!(step_readers(&a), 0, "{m}");
            match read_of(m).unwrap() {
                Read::Exp(e) => assert!(a.find_els(READ_EXP).iter().any(|&i| a.in_str(i, EXP_IN) == Some(e)), "{m}"),
                Read::Bool(f) => assert!(a.find_els(READ_BOOL).iter().any(|&i| a.in_str(i, BOOL_IN_NAME) == Some(f)), "{m}"),
            }
            total += before;
        }
        assert!(total >= 13, "{total}");
    }

    #[test]
    fn the_exterior_guards_read_not_cleared_where_they_read_before_or_equal() {
        let Some(root) = real() else {
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        let m: &str = CARD_MODULES[2];
        let raw: Vec<u8> = rf.read(m).unwrap();
        let a0: Ainb = Ainb::parse(&raw).unwrap();
        let step: usize = step_checks(&a0)[0];
        let or: usize = a0.find_els(OR).into_iter().find(|&i| {
            let x = a0.input(BOOL, a0.find_in(i, BOOL, crate::gerudo_card::MULTI).unwrap()).unwrap();
            x.src <= -100 && entry(&a0, (-100 - i32::from(x.src)) as usize).unwrap().0 as usize == step
        }).unwrap();
        let x = a0.input(BOOL, a0.find_in(or, BOOL, crate::gerudo_card::MULTI).unwrap()).unwrap();
        let s: usize = (-100 - i32::from(x.src)) as usize;
        let mut outs: Vec<u16> = (s..s + x.plug as usize).map(|j| entry(&a0, j).unwrap().1).collect();
        outs.sort();
        assert_eq!(outs, vec![IS_BEFORE as u16, IS_EQUAL as u16]);
        let a: Ainb = Ainb::parse(&module_bytes(&raw, m).unwrap()).unwrap();
        let y = a.input(BOOL, a.find_in(or, BOOL, crate::gerudo_card::MULTI).unwrap()).unwrap();
        let t: usize = (-100 - i32::from(y.src)) as usize;
        let now: Vec<(u16, u16)> = (t..t + y.plug as usize).map(|j| entry(&a, j).unwrap()).collect();
        assert_eq!(now.len(), 2);
        assert!(now.iter().all(|(e, o)| a.el_name(*e as usize) == OR && *o == FALSE_OUT));
    }

    #[test]
    fn the_requiem_step_needs_the_event_and_a_quest_not_past_step05() {
        let Some(root) = real() else {
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read(COLOSSUS).unwrap();
        let a0: Ainb = Ainb::parse(&raw).unwrap();
        let t: usize = a0.find_els(SET_STEP)[0];
        let old = a0.input(BOOL, a0.find_in(t, BOOL, "IsDrive").unwrap()).unwrap();
        let a: Ainb = Ainb::parse(&module_bytes(&raw, COLOSSUS).unwrap()).unwrap();
        let x = a.input(BOOL, a.find_in(t, BOOL, "IsDrive").unwrap()).unwrap();
        assert_eq!(a.el_name(x.src as usize), AND);
        assert_eq!(x.plug, TRUE_OUT);
        let m = a.input(BOOL, a.find_in(x.src as usize, BOOL, crate::gerudo_card::MULTI).unwrap()).unwrap();
        let s: usize = (-100 - i32::from(m.src)) as usize;
        let e: Vec<(u16, u16)> = (s..s + m.plug as usize).map(|j| entry(&a, j).unwrap()).collect();
        assert_eq!(e[1], (old.src as u16, old.plug as u16));
        assert_eq!(a.el_name(e[0].0 as usize), OR);
        assert_eq!(e[0].1, FALSE_OUT);
    }

    #[test]
    fn a_carpenter_keeps_his_jail_schedule_until_the_fortress_is_cleared() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let es: Vec<crate::sarc::Entry> = crate::sarc::parse(&rf.read_zs(crate::gerudo_card::SCHEDULE_PACK).unwrap()).unwrap();
        for n in CARPENTERS {
            let own: Vec<u8> = rf.read(&own_schedule(n)).unwrap();
            let step: Vec<u8> = crate::sarc::get(&es, &jail_schedule(n)).unwrap().data.clone();
            let y: Byml = Byml::parse(&jailed(&own, &step).unwrap()).unwrap();
            let o: Byml = Byml::parse(&own).unwrap();
            let s: Byml = Byml::parse(&step).unwrap();
            let list: &[Node] = y.root.get("ConditionalSchedule").unwrap().as_arr().unwrap();
            assert_eq!(list[0].get("ConditionFlag").and_then(|f| f.as_str()), Some(MARK));
            assert_eq!(list[0].get("EventEntry"), o.root.get("DefaultSchedule").unwrap().get("EventEntry"));
            assert_eq!(&list[1..], s.root.get("ConditionalSchedule").unwrap().as_arr().unwrap());
            assert_eq!(y.root.get("DefaultSchedule"), s.root.get("DefaultSchedule"));
            assert!(jailed(&jailed(&own, &step).unwrap(), &step).is_err());
        }
    }

    #[test]
    fn every_merged_schedule_gets_both_budgets_and_the_split_covers_every_step_check() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let base: crate::rsizetable::Rst = crate::rsizetable::load(&mut rf).unwrap();
        let mut t: crate::rsizetable::Rst = base.clone();
        assert_eq!(budget(&mut rf, &mut t).unwrap(), CARPENTERS.len());
        for c in CARPENTERS {
            assert_eq!(t.get(&own_schedule(c)).unwrap(), base.get(&own_schedule(c)).unwrap() + base.get(&jail_schedule(c)).unwrap());
        }
        assert_eq!(budget(&mut rf, &mut t).unwrap(), 0);
        for m in modules() {
            assert!(m == COLOSSUS || read_of(m).is_some(), "{m}");
        }
        assert_eq!(CARD_MODULES.len() + RESCUE_MODULES.len(), 12);
    }

    #[test]
    fn the_cleared_formula_names_a_flag_the_game_has() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut y: Byml = rf.read_byml(crate::chest_slot::LIST).unwrap();
        for e in exps() {
            assert!(crate::start_defaults::apply_one(&mut y, &e).unwrap());
        }
    }
}
