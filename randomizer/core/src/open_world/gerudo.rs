use crate::bfevfl::flow::NONE;
use crate::bfevfl::{Bf, Case, Cont, Flow, Kind, Val};
use crate::error::{Error, Res};

pub const GUARD_FILE: &str = "Event/EventFlow/Npc_Gerudo_001.bfevfl.zs";
pub const GUARD_FLOW: &str = "Npc_Gerudo_001";
pub const GUARD_ENTRY: &str = "FIrstTalkCleard";
pub const GUARD_TALK_EVENT: &str = "Event121";
pub const CARD_EVENT: &str = "Dm_Gerudo_0025";
pub const CHECK_FLAG: &str = "HavePlayedEvent.Dm_Gerudo_0025";
pub const CARD_QUERY: &str = "EventQueryIsGameDataBool";
pub const TRANSITION: &str = "EventTransition";
pub const CARD_PREFIX: &str = "OOTRRGerudoCard";

const TRUE: u32 = 0x8000_0001;

fn flow_mut<'a>(b: &'a mut Bf, name: &str) -> Res<&'a mut Flow> {
    b.flows
        .iter_mut()
        .find(|f| f.name == name)
        .ok_or_else(|| Error::Msg(format!("gerudo: no flow named {name}")))
}

fn index(n: usize) -> Res<u16> {
    u16::try_from(n).map_err(|_| Error::Format("gerudo: too many events in the flow"))
}

fn push(f: &mut Flow, name: &str, kind: Kind) -> Res<u16> {
    let at: u16 = index(f.events.len())?;
    f.events.push(crate::bfevfl::Event {
        name: format!("{name}{at}"),
        kind,
    });
    Ok(at)
}

pub fn patch_open_guard(b: &mut Bf) -> Res<()> {
    let f: &mut Flow = flow_mut(b, GUARD_FLOW)?;
    if f.events.iter().any(|e| e.name.starts_with(CARD_PREFIX)) {
        return Err(Error::Msg(format!(
            "gerudo: {GUARD_FLOW} already has the card trigger"
        )));
    }
    let old: u16 = f
        .entries
        .iter()
        .find(|e| e.name == GUARD_ENTRY)
        .ok_or_else(|| Error::Msg(format!("gerudo: {GUARD_FLOW} has no {GUARD_ENTRY} entry")))?
        .main;
    if f.events.get(usize::from(old)).map(|e| e.name.as_str()) != Some(GUARD_TALK_EVENT) {
        return Err(Error::Msg(format!(
            "gerudo: {GUARD_ENTRY} no longer starts at {GUARD_TALK_EVENT}, the game files differ"
        )));
    }
    let (qa, qi): (u16, u16) = crate::start_state::query_slot(f, "GameDataActor", CARD_QUERY)?;
    let (ta, ti): (u16, u16) = crate::start_state::slot(f, "EventSystemActor", TRANSITION)?;
    let give: u16 = push(
        f,
        &format!("{CARD_PREFIX}Event"),
        Kind::Action {
            next: NONE,
            actor: ta,
            act: ti,
            params: Some(Cont {
                items: vec![
                    ("IsWaitEnd".to_string(), Val::Bool(TRUE)),
                    ("SpecialEventActionType".to_string(), Val::Int(1)),
                    ("EventName".to_string(), Val::Str(CARD_EVENT.to_string())),
                    ("FadeInTypeEventEnd".to_string(), Val::Int(0)),
                ],
            }),
        },
    )?;
    let ask: u16 = push(
        f,
        &format!("{CARD_PREFIX}Ask"),
        Kind::Switch {
            actor: qa,
            query: qi,
            cases: vec![Case { val: 0, ev: give }, Case { val: 1, ev: old }],
            params: Some(Cont {
                items: vec![
                    ("Index".to_string(), Val::Int(-1)),
                    ("GameDataName".to_string(), Val::Str(CHECK_FLAG.to_string())),
                ],
            }),
        },
    )?;
    let entry: &mut crate::bfevfl::Entry = f
        .entries
        .iter_mut()
        .find(|e| e.name == GUARD_ENTRY)
        .ok_or_else(|| Error::Msg(format!("gerudo: {GUARD_FLOW} has no {GUARD_ENTRY} entry")))?;
    entry.main = ask;
    Ok(())
}
