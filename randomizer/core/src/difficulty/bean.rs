use super::flows::{action, finish, flow, marked, push, query};
use crate::bfevfl::flow::NONE;
use crate::bfevfl::{Bf, Case, Cont, Flow, Kind, Val};
use crate::error::{Error, Res};
use crate::msbt::{self, Msbt};
use crate::sarc::{self, Entry};

pub const FILE: &str = crate::open_world::skips::BEAN_FILE;
pub const FLOW: &str = crate::open_world::skips::BEAN_FLOW;
pub const ENTRY: &str = "Talk";
pub const GREET: &str = "Mask_All";
pub const MSG: &str = "EventFlowMsg/Npc_Hylia_047_Child.msbt";
pub const MSG_ID: &str = "EventFlowMsg/Npc_Hylia_047_Child";
pub const LABEL: &str = "OOTRR_BeanPitch";
pub const MARK: &str = "OOTRRBeanPitch";
pub const SELLER: &str = "Npc_EventStarter";
pub const TALK: &str = "EventTalk";
pub const SYSTEM: &str = "GameSystemActor";
pub const RANDOM: &str = "EventQueryRandomChoice5";
pub const ODDS: u32 = 5;
pub const CHEW: &str = "Act_Eat_Talk";
pub const WORDS: &str = "Chomp Chomp...{5.1}\nwould you like some {0.3:0000}Difficulty Mods{0.3:ffff}?";
const VOICE: &str = "{3.0:0f0100}{4.0:18004265616e4d65726368616e745f4d616d6554616c6b5f303000}";

pub fn message_id() -> String {
    format!("{MSG_ID}:{LABEL}")
}

fn greet(f: &Flow) -> Res<usize> {
    let mut i: u16 = f.entries.iter().find(|e| e.name == ENTRY).ok_or(Error::Format("difficulty: the bean seller has no Talk entry"))?.main;
    for _ in 0..f.events.len() {
        match f.events.get(i as usize).map(|e| &e.kind) {
            Some(Kind::Sub { flow, entry, .. }) if flow.is_empty() && entry == GREET => return Ok(i as usize),
            Some(Kind::Action { next, .. }) | Some(Kind::Sub { next, .. }) => i = *next,
            _ => break,
        }
    }
    Err(Error::Format("difficulty: the bean seller's talk does not start with his greeting, the flow differs"))
}

fn is_line(f: &Flow, k: &Kind) -> bool {
    let Kind::Action { actor, act, params: Some(c), .. } = k else {
        return false;
    };
    let named: bool = f.actors.get(*actor as usize).is_some_and(|a| a.name == SELLER && a.actions.get(*act as usize).is_some_and(|s| s == TALK));
    named && c.get("ASName") == Some(&Val::Str(CHEW.to_string())) && c.get("ChoiceNumber") == Some(&Val::Int(0)) && matches!(c.get("MessageId"), Some(Val::Str(_)))
}

fn line(f: &Flow) -> Res<Cont> {
    let mut c: Cont = f.events.iter().find(|e| is_line(f, &e.kind)).and_then(|e| e.kind.params().cloned()).ok_or(Error::Format("difficulty: the bean seller has no spoken line to copy"))?;
    for (k, v) in c.items.iter_mut() {
        if k == "MessageId" {
            *v = Val::Str(message_id());
        }
    }
    Ok(c)
}

pub fn pitch(raw: &[u8]) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    let f: &mut Flow = flow(&mut b, FLOW)?;
    if marked(f, MARK) {
        return Err(Error::Format("difficulty: the bean seller's flow is already edited"));
    }
    let at: usize = greet(f)?;
    let old: u16 = match &f.events[at].kind {
        Kind::Sub { next, .. } => *next,
        _ => NONE,
    };
    if old == NONE {
        return Err(Error::Format("difficulty: the bean seller's greeting leads nowhere, the flow differs"));
    }
    let c: Cont = line(f)?;
    let (ta, t): (u16, u16) = action(f, SELLER, TALK)?;
    let (ra, r): (u16, u16) = query(f, SYSTEM, RANDOM)?;
    let say: u16 = push(f, format!("{MARK}Say"), Kind::Action { next: old, actor: ta, act: t, params: Some(c) })?;
    let cases: Vec<Case> = (0..ODDS).map(|v| Case { val: v, ev: if v == 0 { say } else { old } }).collect();
    let roll: u16 = push(f, format!("{MARK}Roll"), Kind::Switch { actor: ra, query: r, cases, params: None })?;
    if let Kind::Sub { next, .. } = &mut f.events[at].kind {
        *next = roll;
    }
    finish(&b)
}

pub fn text(raw: &[u8]) -> Res<Vec<u8>> {
    let mut es: Vec<Entry> = sarc::parse(raw)?;
    let mut m: Msbt = msbt::parse(&sarc::get(&es, MSG).ok_or(Error::Format("difficulty: the language archive has no bean seller texts"))?.data)?;
    if !m.put_markup(LABEL, &format!("{VOICE}{WORDS}"))? {
        return Err(Error::Format("difficulty: the bean seller's texts are already edited"));
    }
    sarc::set(&mut es, MSG, m.write()?);
    sarc::write(&es)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::Actor;
    use crate::bfevfl::flow::{Entry as Start, Event};

    fn actor(n: &str, acts: &[&str], qs: &[&str]) -> Actor {
        Actor {
            name: n.to_string(),
            sub: String::new(),
            arg: String::new(),
            actions: acts.iter().map(|s| s.to_string()).collect(),
            queries: qs.iter().map(|s| s.to_string()).collect(),
            params: Some(Cont { items: vec![("EventMemberCreateMethod".to_string(), Val::Int(0))] }),
            ep: 0xFFFF,
            cut: 1,
        }
    }

    fn said(id: &str, anim: &str) -> Option<Cont> {
        Some(Cont {
            items: vec![
                ("IsWaitEnd".to_string(), Val::Bool(0x8000_0001)),
                ("ChoiceNumber".to_string(), Val::Int(0)),
                ("MessageId".to_string(), Val::Str(id.to_string())),
                ("ASName".to_string(), Val::Str(anim.to_string())),
            ],
        })
    }

    fn sample() -> Bf {
        let f: Flow = Flow {
            name: FLOW.to_string(),
            actors: vec![actor("GameDataActor", &["EventTriggerSetGameDataBool"], &[]), actor(SELLER, &[TALK], &[]), actor(SYSTEM, &[], &["EventQueryRandomChoice2"])],
            events: vec![
                Event { name: "Event54".to_string(), kind: Kind::Action { next: 1, actor: 0, act: 0, params: None } },
                Event { name: "Event42".to_string(), kind: Kind::Sub { next: 2, params: None, flow: String::new(), entry: GREET.to_string() } },
                Event { name: "Event20".to_string(), kind: Kind::Action { next: 3, actor: 1, act: 0, params: said("EventFlowMsg/Npc_Hylia_047_Child:Talk_00_00", "") } },
                Event { name: "Event30".to_string(), kind: Kind::Action { next: NONE, actor: 1, act: 0, params: said("EventFlowMsg/Npc_Hylia_047_Child:Talk_17", CHEW) } },
                Event { name: "Event47".to_string(), kind: Kind::Action { next: NONE, actor: 1, act: 0, params: None } },
            ],
            entries: vec![
                Start { name: ENTRY.to_string(), subs: vec![1], vars: Vec::new(), main: 0, pad: 0 },
                Start { name: GREET.to_string(), subs: Vec::new(), vars: Vec::new(), main: 4, pad: 0 },
            ],
        };
        Bf { ver: [0, 4, 0, 0], bom: 0xFEFF, shift: 3, pad: 0, name: FLOW.to_string(), flows: vec![f], tls: Vec::new() }
    }

    fn spoken(f: &Flow, roll: u32) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut i: u16 = f.entries.iter().find(|e| e.name == ENTRY).unwrap().main;
        for _ in 0..50 {
            if i == NONE {
                return out;
            }
            match &f.events[i as usize].kind {
                Kind::Action { next, params, .. } => {
                    if let Some(Val::Str(s)) = params.as_ref().and_then(|c| c.get("MessageId")) {
                        out.push(s.clone());
                    }
                    i = *next;
                }
                Kind::Sub { next, .. } => i = *next,
                Kind::Switch { actor, query, cases, .. } => {
                    assert_eq!(f.actors[*actor as usize].name, SYSTEM);
                    assert_eq!(f.actors[*actor as usize].queries[*query as usize], RANDOM);
                    assert_eq!(cases.len(), ODDS as usize);
                    i = cases.iter().find(|c| c.val == roll).unwrap().ev;
                }
                k => panic!("unexpected event {k:?}"),
            }
        }
        panic!("the walk does not end");
    }

    #[test]
    fn one_roll_in_five_adds_the_pitch_and_every_roll_reaches_the_usual_talk() {
        let was: Bf = sample();
        let usual: Vec<String> = vec!["EventFlowMsg/Npc_Hylia_047_Child:Talk_00_00".to_string(), "EventFlowMsg/Npc_Hylia_047_Child:Talk_17".to_string()];
        let b: Bf = Bf::parse(&pitch(&was.write().unwrap()).unwrap()).unwrap();
        let f: &Flow = &b.flows[0];
        let mut with: Vec<String> = vec![message_id()];
        with.extend(usual.clone());
        assert_eq!(spoken(f, 0), with);
        for roll in 1..ODDS {
            assert_eq!(spoken(f, roll), usual, "roll {roll}");
        }
        assert_eq!(f.events.len(), was.flows[0].events.len() + 2);
        assert_eq!(f.events[..5].iter().filter(|e| !was.flows[0].events.contains(e)).count(), 1);
        let Kind::Action { params: Some(c), .. } = &f.events[5].kind else { panic!() };
        assert_eq!(c.get("ASName"), Some(&Val::Str(CHEW.to_string())));
        assert_eq!(c.get("IsWaitEnd"), Some(&Val::Bool(0x8000_0001)));
    }

    #[test]
    fn the_edit_refuses_a_second_run_and_a_flow_without_the_greeting() {
        let once: Vec<u8> = pitch(&sample().write().unwrap()).unwrap();
        assert!(pitch(&once).is_err());
        let mut b: Bf = sample();
        b.flows[0].entries[0].main = 2;
        assert!(pitch(&b.write().unwrap()).is_err());
    }

    #[test]
    fn the_line_is_valid_markup_and_keeps_the_wording() {
        let t: Vec<msbt::Tok> = msbt::from_markup(&format!("{VOICE}{WORDS}")).unwrap();
        let plain: String = t.iter().filter_map(|k| if let msbt::Tok::Text(s) = k { Some(s.as_str()) } else { None }).collect();
        assert_eq!(plain, "Chomp Chomp...\nwould you like some Difficulty Mods?");
        assert!(plain.lines().all(|l| l.len() <= 50));
    }
}
