use super::flows::{cont, finish, flow, marked, push, query, refusal, Step, DATA, PLAYER, PLAY_AS, PUT_AWAY, START};
use crate::bfevfl::flow::NONE;
use crate::bfevfl::{Bf, Case, Cont, Event, Flow, Kind, Val};
use crate::error::{Error, Res};

pub const CATCH_FILE: &str = "Event/EventFlow/Dm_SY_InsertBottle.bfevfl.zs";
pub const CATCH_FLOW: &str = "Dm_SY_InsertBottle";
pub const CATCH_ENTRIES: [&str; 1] = ["EntryPoint0"];
pub const GIVE_FILE: &str = "Event/EventFlow/Dm_SY_GetInEvent.bfevfl.zs";
pub const GIVE_FLOW: &str = "Dm_SY_GetInEvent";
pub const GIVE_ENTRIES: [&str; 5] = ["GetInEventInBottle", "GetInEventInEquipedBottle", "GetInEventInBottle_2DShop", "GetInEventInBottle_FindOutCucco", "GetInEventInBottle_BlueBottleShop"];
pub const SHOP_FILE: &str = "Event/EventFlow/DisplayShop.bfevfl.zs";
pub const SHOP_FLOW: &str = "DisplayShop";

pub const RED_POTION: i32 = 0;
pub const GREEN_POTION: i32 = 1;
pub const BLUE_POTION: i32 = 2;
pub const BLUE_FIRE: i32 = 3;
pub const FISH: i32 = 4;
pub const BIG_POE: i32 = 6;
pub const POE: i32 = 7;
pub const BUG: i32 = 8;
pub const FAIRY: i32 = 9;
pub const MILK: i32 = 10;
pub const HALF_MILK: i32 = 11;

pub const SELLERS: [(&str, &str, i32); 2] =
    [("Event/EventFlow/Minigame_FindOutCucco.bfevfl.zs", "Minigame_FindOutCucco", MILK), ("Event/EventFlow/Npc_Hylia_053_Adult.bfevfl.zs", "Npc_Hylia_053_Adult", BLUE_POTION)];
pub const GOODS: [(&str, i32); 11] = [
    ("Item_Bottle_RedPotion", RED_POTION),
    ("Item_Bottle_GreenPotion", GREEN_POTION),
    ("Item_Bottle_BluePotion", BLUE_POTION),
    ("Item_Bottle_BlueFire", BLUE_FIRE),
    ("Item_Bottle_Fish", FISH),
    ("Item_Bottle_BigPoe", BIG_POE),
    ("Item_Bottle_Poe", POE),
    ("Item_Bottle_Bug", BUG),
    ("Item_Bottle_Fairy", FAIRY),
    ("Item_Bottle_Milk", MILK),
    ("Item_Bottle_HalfMilk", HALF_MILK),
];

pub const CAMERA: &str = "EventCamera";
pub const TO_PLAYER_CAMERA: &str = "EventChangeTemporarilyToPlayerCamera";
pub const TALK_AS: &str = "EvtTalk";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    Catch,
    Give,
}

pub const POUCH: &str = "PouchSystemActor";
pub const HOLDS: &str = "EventQueryIsExistBottleTypeInPouch";
pub const ANY_EMPTY: &str = "EventQueryIsAnyBottleEmpty";
pub const IS_TEXT: &str = "EventQueryIsGameDataString64";
pub const PICKED: &str = "Shop_SelectItemName";
pub const ITEM: &str = "BottleItem";
pub const YES: u32 = 0;
pub const NO: u32 = 1;
pub const MARK: &str = "OOTRRNoDupe";

pub type Good<'a> = (Option<&'a str>, i32);

pub fn goods() -> Vec<Good<'static>> {
    GOODS.iter().map(|(n, i)| (Some(*n), *i)).collect()
}

fn passes(f: &Flow) -> bool {
    f.events.iter().any(|e| e.kind.params().is_some_and(|c| c.items.iter().any(|(_, v)| *v == Val::Arg(ITEM.to_string()))))
}

fn vanilla(f: &Flow, actor: &str, name: &str) -> Res<Option<Cont>> {
    let hit: Option<&Event> = f.events.iter().find(|e| {
        let Kind::Action { actor: a, act, .. } = &e.kind else {
            return false;
        };
        f.actors.get(*a as usize).is_some_and(|x| x.name == actor && x.sub.is_empty() && x.actions.get(*act as usize).is_some_and(|s| s == name))
    });
    match hit {
        Some(Event { kind: Kind::Action { params, .. }, .. }) => Ok(params.clone()),
        _ => Err(Error::Msg(format!("difficulty: the flow never runs {name}, so a refusal cannot copy it"))),
    }
}

fn talk() -> Option<Cont> {
    cont(vec![
        ("ForceFadeFrame", Val::Int(0)),
        ("ClothWarpMode", Val::Int(-1)),
        ("ASCommand", Val::Str(TALK_AS.to_string())),
        ("UseAnmDriven", Val::Bool(0)),
        ("KeepOneTimeAnm", Val::Bool(0)),
        ("IsWaitEnd", Val::Bool(0)),
        ("IsAllSlot", Val::Bool(0x8000_0001)),
    ])
}

fn endings(f: &Flow, ending: Ending) -> Res<(Vec<Step<'static>>, Vec<Step<'static>>)> {
    match ending {
        Ending::Catch => Ok((vec![(CAMERA, TO_PLAYER_CAMERA, vanilla(f, CAMERA, TO_PLAYER_CAMERA)?), (PLAYER, START, vanilla(f, PLAYER, START)?)], vec![(PLAYER, PUT_AWAY, vanilla(f, PLAYER, PUT_AWAY)?)])),
        Ending::Give => Ok((Vec::new(), vec![(PLAYER, PLAY_AS, talk())])),
    }
}

pub fn entries(raw: &[u8], name: &str, list: &[&str], ending: Ending) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    let f: &mut Flow = flow(&mut b, name)?;
    if marked(f, MARK) {
        return Err(Error::Format("difficulty: a bottle filling flow is already edited"));
    }
    if !passes(f) {
        return Err(Error::Msg(format!("difficulty: the flow {name} never passes on which bottle content it fills in")));
    }
    let (pa, pq): (u16, u16) = query(f, POUCH, HOLDS)?;
    for want in list {
        let i: usize = f.entries.iter().position(|e| e.name == *want).ok_or_else(|| Error::Msg(format!("difficulty: the flow {name} has no entry {want}")))?;
        let main: u16 = f.entries[i].main;
        if main == NONE {
            return Err(Error::Msg(format!("difficulty: the entry {want} of {name} is empty")));
        }
        let (before, after): (Vec<Step>, Vec<Step>) = endings(f, ending)?;
        let deny: u16 = refusal(f, MARK, i, before, after)?;
        let gate: u16 = push(f, format!("{MARK}Entry{i}"), Kind::Switch { actor: pa, query: pq, cases: vec![Case { val: YES, ev: deny }, Case { val: NO, ev: main }], params: cont(vec![(ITEM, Val::Arg(ITEM.to_string()))]) })?;
        f.entries[i].main = gate;
    }
    finish(&b)
}

fn empty_check(f: &Flow, e: &Event) -> Option<(u16, u16)> {
    let Kind::Switch { actor, query, cases, .. } = &e.kind else {
        return None;
    };
    if !f.actors.get(*actor as usize).is_some_and(|a| a.name == POUCH && a.queries.get(*query as usize).is_some_and(|s| s == ANY_EMPTY)) {
        return None;
    }
    let free: u16 = cases.iter().find(|c| c.val == YES)?.ev;
    let held: u16 = cases.iter().find(|c| c.val == NO)?.ev;
    Some((free, held))
}

pub fn fronts(raw: &[u8], name: &str, goods: &[Good]) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    let f: &mut Flow = flow(&mut b, name)?;
    if marked(f, MARK) {
        return Err(Error::Format("difficulty: a bottle seller flow is already edited"));
    }
    let at: Vec<(usize, u16, u16)> = (0..f.events.len()).filter_map(|i| empty_check(f, &f.events[i]).map(|(free, held)| (i, free, held))).collect();
    if at.is_empty() || goods.is_empty() {
        return Err(Error::Msg(format!("difficulty: the flow {name} never asks for an empty bottle")));
    }
    let (pa, pq): (u16, u16) = query(f, POUCH, HOLDS)?;
    for (i, free, held) in at {
        let mut next: u16 = free;
        for (j, (good, item)) in goods.iter().enumerate().rev() {
            let has: u16 = push(
                f,
                format!("{MARK}Has{i}_{j}"),
                Kind::Switch { actor: pa, query: pq, cases: vec![Case { val: YES, ev: held }, Case { val: NO, ev: free }], params: cont(vec![(ITEM, Val::Int(*item))]) },
            )?;
            next = match good {
                None => has,
                Some(n) => {
                    let (da, dq): (u16, u16) = query(f, DATA, IS_TEXT)?;
                    push(
                        f,
                        format!("{MARK}Is{i}_{j}"),
                        Kind::Switch {
                            actor: da,
                            query: dq,
                            cases: vec![Case { val: YES, ev: has }, Case { val: NO, ev: next }],
                            params: cont(vec![("GameDataName", Val::Str(PICKED.to_string())), ("Index", Val::Int(-1)), ("Value", Val::Str(n.to_string()))]),
                        },
                    )?
                }
            };
        }
        if let Kind::Switch { cases, .. } = &mut f.events[i].kind {
            for c in cases.iter_mut().filter(|c| c.val == YES) {
                c.ev = next;
            }
        }
    }
    finish(&b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::{Actor, Entry};
    use crate::difficulty::flows::REFUSE_AS;

    fn sys(name: &str, actions: &[&str], queries: &[&str]) -> Actor {
        Actor {
            name: name.to_string(),
            sub: String::new(),
            arg: String::new(),
            actions: actions.iter().map(|s| s.to_string()).collect(),
            queries: queries.iter().map(|s| s.to_string()).collect(),
            params: None,
            ep: 0xFFFF,
            cut: 1,
        }
    }

    fn file(name: &str, actors: Vec<Actor>, events: Vec<Event>, mains: &[(&str, u16)]) -> Vec<u8> {
        let entries: Vec<Entry> = mains.iter().map(|(n, m)| Entry { name: n.to_string(), subs: Vec::new(), vars: Vec::new(), main: *m, pad: 0 }).collect();
        Bf { ver: [0, 4, 0, 0], bom: 0xFEFF, shift: 3, pad: 0, name: name.to_string(), flows: vec![Flow { name: name.to_string(), actors, events, entries }], tls: Vec::new() }.write().unwrap()
    }

    fn act(name: &str, next: u16, items: Vec<(&str, Val)>) -> Event {
        Event { name: name.to_string(), kind: Kind::Action { next, actor: 0, act: 0, params: cont(items) } }
    }

    fn switch(f: &Flow, i: u16) -> (&str, &str, &Vec<Case>, Option<&Val>, Option<&Val>) {
        let Kind::Switch { actor, query, cases, params } = &f.events[i as usize].kind else { panic!("event {i} is not a switch") };
        let a: &Actor = &f.actors[*actor as usize];
        (a.name.as_str(), a.queries[*query as usize].as_str(), cases, params.as_ref().and_then(|c| c.get(ITEM)), params.as_ref().and_then(|c| c.get("Value")))
    }

    fn chain(f: &Flow, from: u16) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();
        let mut at: u16 = from;
        while at != NONE {
            let Kind::Action { next, actor, act, params } = &f.events[at as usize].kind else { panic!("event {at} is not an action") };
            let a: &Actor = &f.actors[*actor as usize];
            let asked: String = match params.as_ref().and_then(|c| c.get("ASCommand")) {
                Some(Val::Str(s)) => s.clone(),
                _ => String::new(),
            };
            out.push((format!("{}.{}", a.name, a.actions[*act as usize]), asked));
            at = *next;
        }
        out
    }

    fn step(actor: &str, name: &str, asked: &str) -> (String, String) {
        (format!("{actor}.{name}"), asked.to_string())
    }

    fn gate(f: &Flow, i: usize) -> (u16, u16) {
        let (actor, q, cases, item, _) = switch(f, f.entries[i].main);
        assert_eq!((actor, q), (POUCH, HOLDS));
        assert_eq!(item, Some(&Val::Arg(ITEM.into())));
        assert_eq!((cases[0].val, cases[1].val), (YES, NO));
        (cases[0].ev, cases[1].ev)
    }

    #[test]
    fn a_catch_that_is_refused_ends_like_a_catch_that_is_done() {
        let camera: Event = Event { name: "Cam".into(), kind: Kind::Action { next: 1, actor: 1, act: 0, params: cont(vec![("EnableControle", Val::Bool(0x8000_0001)), ("IsWaitEnd", Val::Bool(0))]) } };
        let begin: Event = Event { name: "Start".into(), kind: Kind::Action { next: 2, actor: 0, act: 0, params: None } };
        let push_it: Event = act("Push", NONE, vec![("Item", Val::Arg(ITEM.into()))]);
        let away: Event = Event { name: "Away".into(), kind: Kind::Action { next: NONE, actor: 0, act: 2, params: cont(vec![("IsWaitEnd", Val::Bool(0x8000_0001)), ("IsImmediateDrawn", Val::Bool(0x8000_0001))]) } };
        let actors: Vec<Actor> = vec![sys(PLAYER, &[START, PLAY_AS, PUT_AWAY], &[]), sys(CAMERA, &[TO_PLAYER_CAMERA], &[]), sys(POUCH, &["EventTriggerPushToEmptyBottle"], &[])];
        let raw: Vec<u8> = file("Fill", actors, vec![camera, begin, Event { name: "Push".into(), kind: Kind::Action { next: 3, actor: 2, act: 0, params: push_it.kind.params().cloned() } }, away], &[("In", 0)]);
        let out: Vec<u8> = entries(&raw, "Fill", &["In"], Ending::Catch).unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        let f: &Flow = &b.flows[0];
        assert_eq!(f.events.len(), 4 + 5);
        let (deny, go) = gate(f, 0);
        assert_eq!(go, 0, "a content that is not held goes through the vanilla flow");
        assert_eq!(
            chain(f, deny),
            vec![step(CAMERA, TO_PLAYER_CAMERA, ""), step(PLAYER, START, ""), step(PLAYER, PLAY_AS, REFUSE_AS), step(PLAYER, PUT_AWAY, "")],
            "the refusal opens and closes the way the catch does, and ends there"
        );
        let Kind::Action { params, .. } = &f.events[deny as usize].kind else { panic!() };
        assert_eq!(params.as_ref().unwrap().get("EnableControle"), Some(&Val::Bool(0x8000_0001)));
        let Kind::Action { params, .. } = &f.events[chain_end(f, deny) as usize].kind else { panic!() };
        assert_eq!(params.as_ref().unwrap().get("IsImmediateDrawn"), Some(&Val::Bool(0x8000_0001)));
        assert!(entries(&out, "Fill", &["In"], Ending::Catch).is_err());
        assert!(entries(&raw, "Fill", &["Missing"], Ending::Catch).is_err());
        assert!(entries(&raw, "Other", &["In"], Ending::Catch).is_err());
        let bare: Vec<u8> = file("Fill", vec![sys(PLAYER, &[START], &[]), sys(POUCH, &["Use"], &[])], vec![act("Other", NONE, vec![("Item", Val::Arg(ITEM.into()))])], &[("In", 0)]);
        assert!(entries(&bare, "Fill", &["In"], Ending::Catch).is_err());
        let blind: Vec<u8> = file("Fill", vec![sys(POUCH, &["Use"], &[])], vec![act("Other", NONE, vec![])], &[("In", 0)]);
        assert!(entries(&blind, "Fill", &["In"], Ending::Catch).is_err());
    }

    fn chain_end(f: &Flow, from: u16) -> u16 {
        let mut at: u16 = from;
        loop {
            let Kind::Action { next, .. } = &f.events[at as usize].kind else { panic!() };
            if *next == NONE {
                return at;
            }
            at = *next;
        }
    }

    #[test]
    fn a_hand_over_that_is_refused_shakes_its_head_and_returns_to_the_caller() {
        let push_it: Event = act("Push", NONE, vec![("Item", Val::Arg(ITEM.into()))]);
        let raw: Vec<u8> = file("Give", vec![sys(PLAYER, &[PLAY_AS], &[]), sys(POUCH, &["EventTriggerPushToEmptyBottle"], &[])], vec![act("Other", NONE, vec![]), Event { name: "Push".into(), kind: Kind::Action { next: NONE, actor: 1, act: 0, params: push_it.kind.params().cloned() } }], &[("In", 1), ("Plain", 0)]);
        let out: Vec<u8> = entries(&raw, "Give", &["In"], Ending::Give).unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        let f: &Flow = &b.flows[0];
        assert_eq!(f.events.len(), 2 + 3);
        assert_eq!(f.entries[1].main, 0, "an entry that is not guarded is left alone");
        let (deny, go) = gate(f, 0);
        assert_eq!(go, 1);
        assert_eq!(chain(f, deny), vec![step(PLAYER, PLAY_AS, REFUSE_AS), step(PLAYER, PLAY_AS, TALK_AS)], "the refusal answers no, goes back to talking and then ends, which returns to the caller");
        let Kind::Action { params, .. } = &f.events[chain_end(f, deny) as usize].kind else { panic!() };
        assert_eq!(params.as_ref().unwrap().get("IsWaitEnd"), Some(&Val::Bool(0)), "the talk pose is not waited for, as in the vanilla tail");
        assert!(entries(&out, "Give", &["In"], Ending::Give).is_err());
        let blind: Vec<u8> = file("Give", vec![sys(POUCH, &["Use"], &[])], vec![act("Other", NONE, vec![])], &[("In", 0)]);
        assert!(entries(&blind, "Give", &["In"], Ending::Give).is_err());
    }

    fn asks(free: u16, held: u16) -> Event {
        Event { name: "Ask".into(), kind: Kind::Switch { actor: 0, query: 0, cases: vec![Case { val: YES, ev: free }, Case { val: NO, ev: held }], params: None } }
    }

    #[test]
    fn a_seller_treats_held_content_like_no_empty_bottle() {
        let raw: Vec<u8> = file("Seller", vec![sys(POUCH, &[], &[ANY_EMPTY])], vec![act("Sell", NONE, vec![]), act("Refuse", NONE, vec![]), asks(0, 1)], &[("Talk", 2)]);
        let out: Vec<u8> = fronts(&raw, "Seller", &[(None, MILK)]).unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        let f: &Flow = &b.flows[0];
        assert_eq!(f.events.len(), 4);
        assert_eq!(f.entries[0].main, 2);
        let (_, q, cases, _, _) = switch(f, 2);
        assert_eq!(q, ANY_EMPTY);
        assert_eq!(cases, &vec![Case { val: YES, ev: 3 }, Case { val: NO, ev: 1 }]);
        let (actor, q, cases, item, _) = switch(f, 3);
        assert_eq!((actor, q), (POUCH, HOLDS));
        assert_eq!(cases, &vec![Case { val: YES, ev: 1 }, Case { val: NO, ev: 0 }]);
        assert_eq!(item, Some(&Val::Int(MILK)));
        assert!(fronts(&out, "Seller", &[(None, MILK)]).is_err());
        let plain: Vec<u8> = file("Seller", vec![sys(POUCH, &["Use"], &[])], vec![act("Sell", NONE, vec![])], &[("Talk", 0)]);
        assert!(fronts(&plain, "Seller", &[(None, MILK)]).is_err());
    }

    #[test]
    fn a_shop_checks_the_picked_good_by_name() {
        let raw: Vec<u8> = file("Shop", vec![sys(POUCH, &[], &[ANY_EMPTY])], vec![act("Sell", NONE, vec![]), act("Refuse", NONE, vec![]), asks(0, 1)], &[("Talk", 2)]);
        let b: Bf = Bf::parse(&fronts(&raw, "Shop", &goods()).unwrap()).unwrap();
        let f: &Flow = &b.flows[0];
        assert_eq!(f.events.len(), 3 + 2 * GOODS.len());
        assert_eq!(f.actors.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), vec![POUCH, DATA]);
        let (_, _, cases, _, _) = switch(f, 2);
        let mut at: u16 = cases[0].ev;
        for (name, item) in GOODS {
            let (actor, q, cases, _, value) = switch(f, at);
            assert_eq!((actor, q), (DATA, IS_TEXT));
            assert_eq!(value, Some(&Val::Str(name.to_string())));
            let (has, next): (u16, u16) = (cases[0].ev, cases[1].ev);
            assert_eq!((cases[0].val, cases[1].val), (YES, NO));
            let (actor, q, cases, held, _) = switch(f, has);
            assert_eq!((actor, q), (POUCH, HOLDS));
            assert_eq!(held, Some(&Val::Int(item)));
            assert_eq!(cases, &vec![Case { val: YES, ev: 1 }, Case { val: NO, ev: 0 }]);
            at = next;
        }
        assert_eq!(at, 0, "a good that is no bottle content is sold as before");
    }

    #[test]
    fn the_contents_are_distinct_and_leave_out_the_letter() {
        let mut ids: Vec<i32> = GOODS.iter().map(|(_, i)| *i).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), GOODS.len());
        assert!(!ids.contains(&5) && !ids.contains(&-1));
        assert!(GOODS.iter().all(|(n, _)| n.starts_with("Item_Bottle_") && *n != "Item_Bottle_Letter"));
    }
}
