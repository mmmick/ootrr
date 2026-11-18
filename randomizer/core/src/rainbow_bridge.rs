//! Rainbow Bridge: "Dungeon rewards". The bridge to Ganon's Castle appears once the player holds
//! `bridge_rewards` Stones and Medallions together.
//!
//! The bridge's logic module in the castle field creates its trigger areas while
//! Defeat_Ganon_IsAfter_Ready_Exp holds (the Light Arrows scene set the Defeat_Ganon quest to
//! Step01). Walking in plays GanonCastle_Adult_Field_Rainbow, which builds the bridge and sets the
//! quest to Step02, and the bridge stays once that event has played. That formula is read by event
//! flows too, so the module's read is pointed at Defeat_Ganon_IsAfter_Step03_Exp, which nothing in
//! the game reads, and that formula is rewritten to the reward count (crate::medal_gate builds
//! "at least n of 9") and the event not played yet.
//!
//! The bridge can now come before the Light Arrows scene, whose Defeat_Ganon Step01 write would then
//! move the quest back from Step02. That write is skipped when the quest is already past Ready.

use crate::ainb::Ainb;
use crate::bfevfl::{Bf, Case, Cont, Event, Flow, Kind, Val};
use crate::error::{Error, Res};
use crate::romfs::Romfs;
use crate::settings::Settings;
use crate::staging::Out;
use crate::start_defaults::Edit;

pub const SETTING: &str = "bridge";
pub const REWARDS: &str = "bridge_rewards";
pub const VALUE: &str = "dungeons";
pub const MODULE: &str = "Logic/GanonCastle_Level_Field_00_Prefab_14ba.logic.module.ainb";
pub const READY_EXP: &str = "Defeat_Ganon_IsAfter_Ready_Exp";
pub const BRIDGE_EXP: &str = "Defeat_Ganon_IsAfter_Step03_Exp";
pub const PLAYED: &str = "HavePlayedEvent.GanonCastle_Adult_Field_Rainbow";
pub const NOTE: &str = "The Rainbow Bridge to Ganon's Castle appears once you hold the set number of Stones and Medallions together, with or without the Light Arrows Cutscene. The bridge's logic module reads a rewritten computed flag (Defeat_Ganon_IsAfter_Step03_Exp) in place of the quest step, and the Light Arrows scene no longer moves the Defeat_Ganon quest back once the bridge is built. Experimental and untested in the game.";
const COUNTS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
const READ_EXP: &str = "HoldGameDataGetBoolExpSimple";
const EXP_IN: &str = "GameDataNameBoolExp";
const STR: usize = 3;
const NOT_OP: i32 = 5;
const AND_OP: i32 = 3;
const LACS_FLOW: &str = "AllSage";
/// AllSage's last write after the Light Arrows scene: Defeat_Ganon to Step01.
const STEP01: (&str, &str) = ("Event11", "ChallengeSystemActor.EventExecuteProgressChallengeStep");

/// Some(n) when the bridge waits for n dungeon rewards. Vanilla keeps the game's own condition.
pub fn read(s: &Settings, bad: &mut Vec<String>) -> Option<u8> {
    match s.text(SETTING) {
        Ok("vanilla") => None,
        Ok(VALUE) => match s.int(REWARDS) {
            Ok(n) if (0..=9).contains(&n) => Some(n as u8),
            Ok(n) => {
                bad.push(format!("{REWARDS} is {n}, which is not between 0 and 9"));
                None
            }
            Err(e) => {
                bad.push(e.to_string());
                None
            }
        },
        Ok(v) => {
            bad.push(format!("{SETTING} is {v}, which this build cannot apply yet (only vanilla and dungeons work)"));
            None
        }
        Err(e) => {
            bad.push(e.to_string());
            None
        }
    }
}

pub fn logic(n: u8) -> [(&'static str, &'static str); 2] {
    [(SETTING, VALUE), (REWARDS, COUNTS[usize::from(n).min(9)])]
}

/// At least `n` of the nine rewards, and the bridge event not played yet.
pub fn terms(n: u8) -> Vec<(i32, String)> {
    let mut t: Vec<(i32, String)> = crate::medal_gate::at_least(&crate::medal_gate::REWARD_ITEMS, usize::from(n).min(9)).unwrap_or_default();
    let counted: bool = !t.is_empty();
    t.push((0, PLAYED.to_string()));
    t.push((NOT_OP, String::new()));
    if counted {
        t.push((AND_OP, String::new()));
    }
    t
}

pub fn exp(n: u8) -> Edit {
    Edit::Exp(BRIDGE_EXP.to_string(), terms(n))
}

/// The bridge module reads BRIDGE_EXP where it read the quest step formula.
pub fn patch_module(a: &mut Ainb) -> Res<()> {
    let hits: Vec<usize> = a.find_els(READ_EXP).into_iter().filter(|&i| a.in_str(i, EXP_IN) == Some(READY_EXP)).collect();
    let [i] = hits.as_slice() else {
        return Err(Error::Msg(format!("rainbow bridge: expected one read of {READY_EXP} in the bridge module, found {}", hits.len())));
    };
    let g: usize = a.find_in(*i, STR, EXP_IN).ok_or(Error::Format("rainbow bridge: the read has no flag input"))?;
    let o: u32 = a.pool.add(BRIDGE_EXP);
    a.set_in_val(STR, g, &o.to_le_bytes())
}

pub fn stage(rf: &mut Romfs, more: &mut Vec<Out>) -> Res<()> {
    more.push(crate::medal_gate::module(rf, MODULE, patch_module)?);
    Ok(())
}

/// After the Light Arrows scene, the Defeat_Ganon Step01 write runs only while the quest is not
/// past Ready (the bridge not built yet).
pub fn patch_lacs_step(b: &mut Bf) -> Res<()> {
    let f: &mut Flow = b.flows.iter_mut().find(|f| f.name == LACS_FLOW).ok_or(Error::Format("rainbow bridge: AllSage has no AllSage flow"))?;
    let i: usize = f.events.iter().position(|e| e.name == STEP01.0).ok_or(Error::Format("rainbow bridge: AllSage has no Defeat_Ganon Step01 write"))?;
    let ok: bool = matches!(&f.events[i].kind, Kind::Action { actor, act, params, .. }
        if f.actors.get(*actor as usize).is_some_and(|a| format!("{}.{}", a.name, a.actions.get(*act as usize).cloned().unwrap_or_default()) == STEP01.1)
            && params.as_ref().is_some_and(|p| p.items.iter().any(|(k, v)| k == "ChallengeName" && *v == Val::Str("Defeat_Ganon".to_string()))));
    if !ok {
        return Err(Error::Format("rainbow bridge: AllSage's Event11 is not the Defeat_Ganon step write, the game files differ"));
    }
    let preds: Vec<usize> = (0..f.events.len())
        .filter(|&j| matches!(&f.events[j].kind, Kind::Action { next, .. } | Kind::Sub { next, .. } | Kind::Join { next } if *next as usize == i))
        .collect();
    if preds.is_empty() {
        return Err(Error::Format("rainbow bridge: nothing leads to the Defeat_Ganon step write"));
    }
    let (a, _): (u16, u16) = crate::dungeon_credit::action_slot(f, "GameDataActor", "EventTriggerSetGameDataBool")?;
    let q: u16 = {
        let actor = &mut f.actors[a as usize];
        let j: usize = match actor.queries.iter().position(|x| x == "EventQueryIsGameDataBoolExp") {
            Some(j) => j,
            None => {
                actor.queries.push("EventQueryIsGameDataBoolExp".to_string());
                actor.queries.len() - 1
            }
        };
        crate::dungeon_credit::idx(j)?
    };
    let at: u16 = crate::dungeon_credit::idx(f.events.len())?;
    f.events.push(Event {
        name: "OOTRRBridgeStep".to_string(),
        kind: Kind::Switch {
            actor: a,
            query: q,
            cases: vec![Case { val: 1, ev: crate::dungeon_credit::idx(i)? }],
            params: Some(Cont { items: vec![("GameDataName".to_string(), Val::Str(READY_EXP.to_string()))] }),
        },
    });
    for j in preds {
        if let Kind::Action { next, .. } | Kind::Sub { next, .. } | Kind::Join { next } = &mut f.events[j].kind {
            *next = at;
        }
    }
    Ok(())
}
