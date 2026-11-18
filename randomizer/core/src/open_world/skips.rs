//! Quest skips that edit game files: Epona's Song registering Epona, the Gerudo Valley bridge
//! and the one-sale bean salesman. The mask quest and the planted beans are flag list edits
//! (data/start_presets.json).

use crate::ainb::Ainb;
use crate::bfevfl::flow::NONE;
use crate::bfevfl::{Actor, Bf, Case, Cont, Entry, Event, Flow, Kind, Val};
use crate::byml::{Byml, Node};
use crate::error::{Error, Res};

pub const EPONA_FILE: &str = "Event/EventFlow/Dm_SY_Ocarina_EponaSong.bfevfl.zs";
pub const EPONA_FLOW: &str = "Dm_SY_Ocarina_EponaSong";
/// The adult song's first question: does Link own a horse (case 0) or not (case 1).
pub const ASK_OWNED: &str = "Event1";
pub const HAS_HORSE: &str = "EventQueryHasOwnedHorse";
pub const CAN_RIDE: &str = "EventQueryIsCurrentSceneEnableRide";
pub const EPONA_ACTOR: &str = "Animal_Epona_Adult";
/// Owning a horse is engine state (the owned horse list), not game data: only this action on an
/// Epona actor writes it, and Epona's Song only calls an owned horse. In the original game it runs
/// during the ranch escape (Get_Epona's Sub_Get_Epona_EscapeRanch_*_Warp entries).
pub const REGISTER: &str = "EventCopyAndRegisterOwnedHorse";
pub const WARP: &str = "EventWarpToTargetActor";
pub const OFFSTAGE: &str = "EventGoOffstageForUser";
pub const SET_ENUM: &str = "EventTriggerSetGameDataEnum";
pub const EPONA_STEP: &str = "Step_Get_Epona";
pub const EPONA_PREFIX: &str = "OOTRREpona";
/// The event that makes and registers Epona: its event name, flow, entry and flow file.
pub const OWN_EVENT: &str = "OOTRREponaOwn";
pub const OWN_FILE: &str = "Event/EventFlow/OOTRREponaOwn.bfevfl.zs";
/// The Ocarina event's child event (Pierre's) whose table rows the new event copies.
pub const OWN_TEMPLATE: &str = "AppearPierre_ByScarecrowSong";
const MOVER: &str = "EventSystemActor";
const MOVE: &str = "EventTransition";
pub const EPONA_NOTE: &str = "Epona's Song is enough to own Epona: the first time an adult Link plays it where a horse can be ridden, the song starts an event that makes an Epona and registers her as Link's horse the way the ranch escape does, then calls her. Only that first song makes an Epona. The Epona quest is set complete at the same time, so Ingo's race and the ranch visit are skipped. Experimental and untested in the game.";
pub const BRIDGE_MODULE: &str = "Logic/GerudoValley_Adult_Prefab_9e43.logic.module.ainb";
pub const BEAN_FILE: &str = "Event/EventFlow/Npc_Hylia_047_Child.bfevfl.zs";
pub const BEAN_FLOW: &str = "Npc_Hylia_047_Child";

pub const BRIDGE_EXP: &str = "SageOfSpirit_IsAfter_Step01_Exp";
pub const BRIDGE_FIXED: &str = "Obj_GerudoValley_Bridge_A_02_e280";

/// The bean salesman's last sale (the tenth bean). With Get_MagicBeans at 9 at new game it is
/// the only one left, and its 100 rupees would not fit the child's wallet.
pub const BEAN_SALES: [(&str, &str); 2] = [("Event14", "Price"), ("Event15", "Price")];
pub const BEAN_MINUS: &str = "Event15";
pub const BEAN_OLD_PRICE: i32 = 100;
pub const BEAN_PRICE: i32 = 60;
pub const BEAN_SALESMAN: &str = "ZR Magic Bean Salesman";
pub const BEAN_ITEMS: [&str; 2] = ["Buy Magic Bean", "Magic Bean"];

pub const BRIDGE_NOTE: &str = "The broken Gerudo Valley bridge is repaired from the start, as if the carpenters had been rescued, so adult Link can cross without Epona or the Longshot. The carpenters in the Thieves' Hideout still need rescuing, the guards and the Gerudo Membership Card are unchanged. Experimental and untested in the game.";

fn flow_mut<'a>(b: &'a mut Bf, name: &str) -> Res<&'a mut Flow> {
    b.flows.iter_mut().find(|x| x.name == name).ok_or_else(|| Error::Msg(format!("skips: no flow named {name}")))
}

const BOOL: usize = 1;
const CONST: u32 = 0x180_0000;

/// Turns a linked bool input into the constant `v`, dropping the plug that fed it.
pub fn force_bool(a: &mut Ainb, i: usize, name: &str, v: bool) -> Res<()> {
    let g: usize = a.find_in(i, BOOL, name).ok_or_else(|| Error::Msg(format!("skips: node {i} has no {name} input")))?;
    let x = a.input(BOOL, g).ok_or(Error::Format("skips: input out of range"))?;
    let src: u32 = u32::try_from(x.src).map_err(|_| Error::Format("skips: the input is not a single link"))?;
    let mut hit: Option<(usize, usize)> = None;
    for k in 0..10 {
        if let Some(pos) = a.params[i].group(k).iter().position(|p| p.a == src && a.s(p.b) == name) {
            hit = Some((k, pos));
            break;
        }
    }
    let (k, pos): (usize, usize) = hit.ok_or(Error::Format("skips: the link into the input has no plug"))?;
    let p = &mut a.params[i];
    p.plugs.remove(p.pl[k][1] as usize + pos);
    p.pl[k][0] -= 1;
    for j in k + 1..p.pl.len() {
        p.pl[j][1] -= 1;
    }
    let o: usize = g * 16;
    let rec: &mut [u8] = a.ins[BOOL].get_mut(o..o + 16).ok_or(Error::Format("skips: input out of range"))?;
    rec[4..6].copy_from_slice(&(-1i16).to_le_bytes());
    rec[6..8].copy_from_slice(&0i16.to_le_bytes());
    rec[8..12].copy_from_slice(&CONST.to_le_bytes());
    rec[12..16].copy_from_slice(&u32::from(v).to_le_bytes());
    Ok(())
}

/// Points a linked bool input at output `out` of node `src`, moving the plug that fed it.
pub fn relink_bool(a: &mut Ainb, i: usize, name: &str, src: usize, out: u16) -> Res<()> {
    let g: usize = a.find_in(i, BOOL, name).ok_or_else(|| Error::Msg(format!("skips: node {i} has no {name} input")))?;
    let x = a.input(BOOL, g).ok_or(Error::Format("skips: input out of range"))?;
    let old: u32 = u32::try_from(x.src).map_err(|_| Error::Format("skips: the input is not a single link"))?;
    let to: i16 = i16::try_from(src).map_err(|_| Error::Format("skips: node index out of range"))?;
    let out: i16 = i16::try_from(out).map_err(|_| Error::Format("skips: output index out of range"))?;
    let mut hit: Option<(usize, usize)> = None;
    for k in 0..10 {
        if let Some(pos) = a.params[i].group(k).iter().position(|p| p.a == old && a.s(p.b) == name) {
            hit = Some((k, pos));
            break;
        }
    }
    let (k, pos): (usize, usize) = hit.ok_or(Error::Format("skips: the link into the input has no plug"))?;
    let at: usize = a.params[i].pl[k][1] as usize + pos;
    a.params[i].plugs[at].a = src as u32;
    let o: usize = g * 16;
    let rec: &mut [u8] = a.ins[BOOL].get_mut(o..o + 16).ok_or(Error::Format("skips: input out of range"))?;
    rec[4..6].copy_from_slice(&to.to_le_bytes());
    rec[6..8].copy_from_slice(&out.to_le_bytes());
    Ok(())
}

/// The AND node that reads the Spirit quest step and picks the broken or the repaired bridge.
pub fn bridge_gate(a: &Ainb) -> Res<usize> {
    let reads: Vec<usize> = a.find_els("HoldGameDataGetBoolExpSimple").into_iter().filter(|&i| a.in_str(i, "GameDataNameBoolExp") == Some(BRIDGE_EXP)).collect();
    let hits: Vec<usize> = a
        .find_els("OperateAND")
        .into_iter()
        .filter(|&i| a.find_in(i, BOOL, "BoolMulti").and_then(|g| a.input(BOOL, g)).is_some_and(|x| x.src >= 0 && reads.contains(&(x.src as usize))))
        .collect();
    match hits.as_slice() {
        [i] => Ok(*i),
        _ => Err(Error::Msg(format!("skips: expected one bridge gate in {BRIDGE_MODULE}, found {}", hits.len()))),
    }
}

pub fn bridge_bytes(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    let fixed: Vec<usize> = a.find_els("ActorLogicSimpleActor").into_iter().filter(|&i| a.prop_str(i, "InstanceName") == Some(BRIDGE_FIXED)).collect();
    if fixed.len() != 1 {
        return Err(Error::Msg(format!("skips: expected one repaired bridge actor in {BRIDGE_MODULE}, found {}", fixed.len())));
    }
    let g: usize = bridge_gate(&a)?;
    force_bool(&mut a, g, "BoolMulti", true)?;
    let out: Vec<u8> = a.write()?;
    let b: Ainb = Ainb::parse(&out)?;
    let audit: Vec<String> = crate::ainb::audit(&b);
    if !audit.is_empty() {
        return Err(Error::Msg(format!("skips: the bridge module fails its audit: {}", audit.join("; "))));
    }
    let forced = b.find_in(g, BOOL, "BoolMulti").and_then(|x| b.input(BOOL, x));
    if b.write()? != out || !forced.is_some_and(|x| x.src < 0 && x.val == 1u32.to_le_bytes()) {
        return Err(Error::Format("skips: the bridge module does not read back"));
    }
    Ok(out)
}

fn set_int(c: &mut Cont, key: &str, old: i32, new: i32, at: &str) -> Res<()> {
    let v: &mut Val = c.items.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v).ok_or_else(|| Error::Msg(format!("skips: {at} has no {key}")))?;
    if *v != Val::Int(old) {
        return Err(Error::Msg(format!("skips: {at} {key} is {v:?}, expected {old}, the game files differ")));
    }
    *v = Val::Int(new);
    Ok(())
}

pub fn patch_beans(b: &mut Bf) -> Res<()> {
    let f: &mut Flow = flow_mut(b, BEAN_FLOW)?;
    for (ev, key) in BEAN_SALES {
        let e: &mut Event = f.events.iter_mut().find(|e| e.name == ev).ok_or_else(|| Error::Msg(format!("skips: no event {ev} in {BEAN_FLOW}")))?;
        let c: &mut Cont = e.kind.params_mut().ok_or_else(|| Error::Msg(format!("skips: {ev} has no parameters")))?;
        set_int(c, key, BEAN_OLD_PRICE, BEAN_PRICE, ev)?;
        if ev == BEAN_MINUS {
            set_int(c, "MinusRupeeValue", -BEAN_OLD_PRICE, -BEAN_PRICE, ev)?;
        }
    }
    Ok(())
}

pub fn bean_bytes(raw: &[u8]) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    patch_beans(&mut b)?;
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(Error::Format("skips: the bean salesman flow does not parse back"));
    }
    Ok(out)
}

const TRUE: u32 = 0x8000_0001;
const FALSE: u32 = 0;

fn event_at(f: &Flow, name: &str) -> Res<usize> {
    let v: Vec<usize> = (0..f.events.len()).filter(|&i| f.events[i].name == name).collect();
    match v.as_slice() {
        [i] => Ok(*i),
        _ => Err(Error::Msg(format!("skips: expected one event named {name} in {EPONA_FLOW}, found {}", v.len()))),
    }
}

#[cfg(test)]
fn text<'a>(e: &'a Event, key: &str) -> Option<&'a str> {
    match e.kind.params()?.get(key)? {
        Val::Str(s) => Some(s.as_str()),
        _ => None,
    }
}

fn kv(k: &str, v: Val) -> (String, Val) {
    (k.to_string(), v)
}

fn action_slot(f: &mut Flow, actor: usize, action: &str) -> u16 {
    let a: &mut Actor = &mut f.actors[actor];
    match a.actions.iter().position(|x| x == action) {
        Some(j) => j as u16,
        None => {
            a.actions.push(action.to_string());
            (a.actions.len() - 1) as u16
        }
    }
}

/// An Epona the event itself creates (member create method 1, as events do for Kaepora or
/// Sheik), with the member settings Get_Epona gives its ranch Epona.
fn epona_member(base: &Actor) -> Actor {
    let mut items: Vec<(String, Val)> = vec![
        kv("EventMemberCreateMethod", Val::Int(1)),
        kv("IsBindNearestActor", Val::Bool(FALSE)),
        kv("IsGrounding", Val::Bool(FALSE)),
        kv("IsWorld", Val::Bool(FALSE)),
    ];
    for k in ["UseEventModelAnime", "UseEventAS"] {
        if let Some(v) = base.params.as_ref().and_then(|c| c.get(k)) {
            items.push(kv(k, v.clone()));
        }
    }
    for slot in ["Weapon", "SubWeapon", "Shield", "Bow"] {
        items.push(kv(&format!("EquipmentUser_{slot}"), Val::Str(String::new())));
        items.push(kv(&format!("EquipmentUser_{slot}_EquipmentState"), Val::Int(0)));
    }
    for slot in ["Tool", "SubTool", "Accessory1", "Accessory2", "Accessory3", "Accessory4"] {
        items.push(kv(&format!("EquipmentUser_{slot}"), Val::Str(String::new())));
    }
    Actor {
        name: EPONA_ACTOR.to_string(),
        sub: String::new(),
        arg: String::new(),
        actions: Vec::new(),
        queries: Vec::new(),
        params: Some(Cont { items }),
        ep: base.ep,
        cut: base.cut,
    }
}

/// When an adult plays Epona's Song without owning a horse, in a place where riding is allowed:
/// start the event that makes and registers Epona ([`OWN_EVENT`]), then go on to the game's own
/// "owned horse" path, which calls her. Elsewhere, and once she is owned, the song is unchanged.
///
/// The Ocarina event loads this flow as soon as the Ocarina comes out (its
/// StartToPrepareCallOwnedHorse entry), and an event creates every member of the flows it loads
/// that it is told to create. An Epona member here made a new Epona at every Ocarina use, so she
/// lives in her own event, which only this path starts (as the Ocarina event starts Pierre's).
/// Returns the added events.
pub fn patch_epona(b: &mut Bf) -> Res<usize> {
    let f: &mut Flow = flow_mut(b, EPONA_FLOW)?;
    if f.events.iter().any(|e| e.name.starts_with(EPONA_PREFIX)) {
        return Err(Error::Msg(format!("skips: {EPONA_FLOW} already registers Epona")));
    }
    let ask: usize = event_at(f, ASK_OWNED)?;
    let (owned, unowned): (u16, u16) = match &f.events[ask].kind {
        Kind::Switch { actor, query, cases, .. } if f.actors.get(*actor as usize).and_then(|a| a.queries.get(*query as usize)).is_some_and(|q| q == HAS_HORSE) => {
            match (cases.iter().find(|c| c.val == 0), cases.iter().find(|c| c.val == 1)) {
                (Some(a), Some(b)) => (a.ev, b.ev),
                _ => return Err(Error::Msg(format!("skips: {ASK_OWNED} has no owned and unowned case"))),
            }
        }
        _ => return Err(Error::Msg(format!("skips: {ASK_OWNED} does not ask whether Link owns a horse, the game files differ"))),
    };
    let (ra, rq): (u16, u16) = match &f.events[owned as usize].kind {
        Kind::Switch { actor, query, .. } if f.actors.get(*actor as usize).and_then(|a| a.queries.get(*query as usize)).is_some_and(|q| q == CAN_RIDE) => (*actor, *query),
        _ => return Err(Error::Msg(format!("skips: the owned horse path of {EPONA_FLOW} does not start with the riding check"))),
    };
    if f.actors.iter().any(|a| a.name == EPONA_ACTOR) {
        return Err(Error::Msg(format!("skips: {EPONA_FLOW} has an Epona member, the game files differ")));
    }
    let base: Actor = f.actors.first().cloned().ok_or(Error::Format("skips: the song flow has no actors"))?;
    let mv: usize = match f.actors.iter().position(|a| a.name == MOVER && a.sub.is_empty()) {
        Some(i) => i,
        None => {
            f.actors.push(Actor { name: MOVER.to_string(), sub: String::new(), arg: String::new(), actions: Vec::new(), queries: Vec::new(), params: base.params.clone(), ep: base.ep, cut: base.cut });
            f.actors.len() - 1
        }
    };
    let go: u16 = action_slot(f, mv, MOVE);
    let n: usize = f.events.len();
    if n + 2 >= NONE as usize {
        return Err(Error::Format("skips: too many events in the song flow"));
    }
    let at = |k: usize| (n + k) as u16;
    let chain: Vec<Kind> = vec![
        Kind::Switch { actor: ra, query: rq, cases: vec![Case { val: 0, ev: at(1) }, Case { val: 1, ev: unowned }], params: None },
        Kind::Action {
            next: owned,
            actor: mv as u16,
            act: go,
            params: Some(Cont {
                items: vec![kv("IsWaitEnd", Val::Bool(TRUE)), kv("SpecialEventActionType", Val::Int(1)), kv("FadeInTypeEventEnd", Val::Int(0)), kv("EventName", Val::Str(OWN_EVENT.to_string()))],
            }),
        },
    ];
    let added: usize = chain.len();
    for (k, kind) in chain.into_iter().enumerate() {
        f.events.push(Event { name: format!("{EPONA_PREFIX}{k}"), kind });
    }
    if let Kind::Switch { cases, .. } = &mut f.events[ask].kind {
        for c in cases.iter_mut().filter(|c| c.val == 1) {
            c.ev = at(0);
        }
    }
    Ok(added)
}

/// The event's one flow: create an Epona next to Link, register her as Link's horse the way the
/// ranch escape does, take the created one off stage and mark the Epona quest complete. Built from
/// the game's Epona's Song flow `song` (its file layout and member settings).
pub fn own_flow(song: &Bf) -> Res<Bf> {
    let s: &Flow = song.flows.iter().find(|x| x.name == EPONA_FLOW).ok_or_else(|| Error::Msg(format!("skips: no flow named {EPONA_FLOW}")))?;
    let base: Actor = s.actors.first().cloned().ok_or(Error::Format("skips: the song flow has no actors"))?;
    let like = |name: &str| -> Res<Actor> {
        let mut a: Actor = s.actors.iter().find(|a| a.name == name && a.sub.is_empty()).cloned().ok_or_else(|| Error::Msg(format!("skips: the song flow has no {name}")))?;
        a.actions.clear();
        a.queries.clear();
        Ok(a)
    };
    let mut f: Flow = Flow {
        name: OWN_EVENT.to_string(),
        actors: vec![epona_member(&base), like("GameDataActor")?, like("Player_Event")?],
        events: Vec::new(),
        entries: vec![Entry { name: OWN_EVENT.to_string(), subs: Vec::new(), vars: Vec::new(), main: 0, pad: 0 }],
    };
    let (warp, reg, off): (u16, u16, u16) = (action_slot(&mut f, 0, WARP), action_slot(&mut f, 0, REGISTER), action_slot(&mut f, 0, OFFSTAGE));
    let set: u16 = action_slot(&mut f, 1, SET_ENUM);
    let chain: Vec<Kind> = vec![
        Kind::Action {
            next: 1,
            actor: 0,
            act: warp,
            params: Some(Cont {
                items: vec![
                    kv("IsWaitEnd", Val::Bool(TRUE)),
                    kv("TargetActorKind", Val::Int(1)),
                    kv("ActorName", Val::Str("Player_Event".to_string())),
                    kv("InstanceName", Val::Str(String::new())),
                    kv("OffsetBase", Val::Int(1)),
                    kv("PositionOffset", Val::Floats(vec![-2.0, 0.0, 0.0])),
                    kv("YAngle", Val::Int(0)),
                    kv("ClothWarpMode", Val::Int(-1)),
                ],
            }),
        },
        Kind::Action { next: 2, actor: 0, act: reg, params: Some(Cont { items: vec![kv("IsWaitEnd", Val::Bool(TRUE))] }) },
        Kind::Action {
            next: 3,
            actor: 0,
            act: off,
            params: Some(Cont {
                items: vec![
                    kv("IsWaitEnd", Val::Bool(TRUE)),
                    kv("IsSleepXLink", Val::Bool(TRUE)),
                    kv("ExecuteSetSpeedZeroEveryFrame", Val::Bool(FALSE)),
                    kv("IsRidingTargetGoOffstage", Val::Bool(FALSE)),
                ],
            }),
        },
        Kind::Action {
            next: NONE,
            actor: 1,
            act: set,
            params: Some(Cont { items: vec![kv("GameDataName", Val::Str(EPONA_STEP.to_string())), kv("Value", Val::Str("Complete".to_string())), kv("Index", Val::Int(-1))] }),
        },
    ];
    for (k, kind) in chain.into_iter().enumerate() {
        f.events.push(Event { name: format!("{EPONA_PREFIX}Own{k}"), kind });
    }
    Ok(Bf { name: OWN_EVENT.to_string(), flows: vec![f], tls: Vec::new(), ..song.clone() })
}

/// [`own_flow`] as file bytes, from the game's song flow file.
pub fn own_bytes(song_raw: &[u8]) -> Res<Vec<u8>> {
    let b: Bf = own_flow(&Bf::parse(song_raw)?)?;
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(Error::Format("skips: the Epona event flow does not parse back"));
    }
    Ok(out)
}

fn row_id(n: &Node) -> &str {
    n.get("__RowId").and_then(|v| v.as_str()).unwrap_or("")
}

/// Adds [`OWN_EVENT`]'s row to the event settings table, or (`analyzed`) to the analyzed events
/// table, copied from Pierre's row: a plain flow chart event the Ocarina event starts.
pub fn own_row(y: &mut Byml, analyzed: bool) -> Res<()> {
    let list: &mut Vec<Node> = y.root.as_arr_mut().ok_or(Error::Format("skips: an event table root is not a list"))?;
    if list.iter().any(|r| row_id(r) == OWN_EVENT) {
        return Err(Error::Msg(format!("skips: the event table already holds {OWN_EVENT}")));
    }
    let mut r: Node = list.iter().find(|r| row_id(r) == OWN_TEMPLATE).cloned().ok_or_else(|| Error::Msg(format!("skips: the event table has no {OWN_TEMPLATE} row, the game files differ")))?;
    r.set("__RowId", Node::Str(OWN_EVENT.to_string()));
    if analyzed {
        if let Node::Map(m) = &mut r {
            m.retain(|(k, _)| k != "SubFlowList" && k != "ChildEventList");
        }
    } else {
        r.set("FlowchartFileName", Node::Str(format!("Work/Event/EventFlow/{OWN_EVENT}.evfc")));
        r.set("EntryPointName", Node::Str(OWN_EVENT.to_string()));
    }
    let at: usize = list.partition_point(|x| row_id(x) < OWN_EVENT);
    list.insert(at, r);
    Ok(())
}

pub fn epona_bytes(raw: &[u8]) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    patch_epona(&mut b)?;
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(Error::Format("skips: the Epona's Song flow does not parse back"));
    }
    Ok(out)
}

pub fn is_bean_sale(name: &str) -> bool {
    name == BEAN_SALESMAN
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::romfs::Romfs;
    use std::path::PathBuf as PB;

    fn real() -> Option<Romfs> {
        let p: PB = ootrr_game::test_romfs::path();
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Romfs::open(&p).ok()
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    #[test]
    fn the_bridge_module_creates_the_repaired_bridge_from_the_start() {
        let Some(rf) = real() else {
            return;
        };
        let raw: Vec<u8> = rf.read(BRIDGE_MODULE).unwrap();
        let a: Ainb = Ainb::parse(&raw).unwrap();
        let g: usize = bridge_gate(&a).unwrap();
        let out: Vec<u8> = bridge_bytes(&raw).unwrap();
        let b: Ainb = Ainb::parse(&out).unwrap();
        let x = b.input(BOOL, b.find_in(g, BOOL, "BoolMulti").unwrap()).unwrap();
        assert!(x.src < 0);
        assert_eq!(x.val, 1u32.to_le_bytes().to_vec());
        assert!(bridge_gate(&b).is_err());
        assert!(crate::ainb::audit(&b).is_empty());
        assert_eq!(b.els.len(), a.els.len());
    }

    #[test]
    fn an_unowned_adult_song_registers_epona_and_then_calls_her() {
        let Some(mut rf) = real() else {
            return;
        };
        let raw: Vec<u8> = rf.read_zs(EPONA_FILE).unwrap();
        let old: Bf = Bf::parse(&raw).unwrap();
        let of: &Flow = old.flows.iter().find(|x| x.name == EPONA_FLOW).unwrap();
        let out: Vec<u8> = epona_bytes(&raw).unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        let f: &Flow = b.flows.iter().find(|x| x.name == EPONA_FLOW).unwrap();
        assert_eq!(f.events.len(), of.events.len() + 2);
        let ask: usize = event_at(f, ASK_OWNED).unwrap();
        for i in 0..of.events.len() {
            if i != ask {
                assert_eq!(f.events[i], of.events[i], "{}", of.events[i].name);
            }
        }
        let Kind::Switch { cases, .. } = &f.events[ask].kind else { panic!() };
        let Kind::Switch { cases: was, .. } = &of.events[ask].kind else { panic!() };
        assert_eq!(cases[0], was[0]);
        let first: &Event = &f.events[cases.iter().find(|c| c.val == 1).unwrap().ev as usize];
        assert_eq!(first.name, format!("{EPONA_PREFIX}0"));
        let names = |e: &Event| -> (String, String) {
            match &e.kind {
                Kind::Action { actor, act, .. } => (f.actors[*actor as usize].name.clone(), f.actors[*actor as usize].actions[*act as usize].clone()),
                Kind::Switch { actor, query, .. } => (f.actors[*actor as usize].name.clone(), f.actors[*actor as usize].queries[*query as usize].clone()),
                _ => (String::new(), String::new()),
            }
        };
        let got: Vec<(String, String)> = (0..2).map(|k| names(&f.events[of.events.len() + k])).collect();
        assert_eq!(got, vec![("GameSystemActor".to_string(), CAN_RIDE.to_string()), (MOVER.to_string(), MOVE.to_string())]);
        let Kind::Action { next, .. } = &f.events[of.events.len() + 1].kind else { panic!() };
        assert_eq!(*next, was[0].ev, "after the event, the owned horse path calls her");
        assert_eq!(text(&f.events[of.events.len() + 1], "EventName"), Some(OWN_EVENT));
        assert!(!f.actors.iter().any(|a| a.name == EPONA_ACTOR), "no Epona is made whenever the song flow loads");
        let own: Bf = Bf::parse(&own_bytes(&raw).unwrap()).unwrap();
        let g: &Flow = &own.flows[0];
        assert_eq!((own.name.as_str(), g.name.as_str(), g.entries[0].name.as_str(), g.entries[0].main), (OWN_EVENT, OWN_EVENT, OWN_EVENT, 0));
        let acts: Vec<(String, String)> = g
            .events
            .iter()
            .map(|e| match &e.kind {
                Kind::Action { actor, act, .. } => (g.actors[*actor as usize].name.clone(), g.actors[*actor as usize].actions[*act as usize].clone()),
                _ => panic!(),
            })
            .collect();
        let want: Vec<(String, String)> = vec![
            (EPONA_ACTOR.to_string(), WARP.to_string()),
            (EPONA_ACTOR.to_string(), REGISTER.to_string()),
            (EPONA_ACTOR.to_string(), OFFSTAGE.to_string()),
            ("GameDataActor".to_string(), SET_ENUM.to_string()),
        ];
        assert_eq!(acts, want);
        assert_eq!(text(&g.events[3], "Value"), Some("Complete"));
        let ep = g.actors.iter().find(|a| a.name == EPONA_ACTOR).unwrap();
        assert_eq!(ep.params.as_ref().unwrap().get("EventMemberCreateMethod"), Some(&Val::Int(1)));
        for (t, analyzed) in [(crate::progressive::SETTING_TABLE, false), (crate::progressive::ANALYZED_TABLE, true)] {
            let mut y: Byml = rf.read_byml(t).unwrap();
            own_row(&mut y, analyzed).unwrap();
            let r: &Node = y.root.as_arr().unwrap().iter().find(|r| row_id(r) == OWN_EVENT).unwrap();
            if analyzed {
                assert!(r.get("SubFlowList").is_none() && r.get("ChildEventList").is_none());
            } else {
                assert_eq!(r.get("EntryPointName").and_then(|v| v.as_str()), Some(OWN_EVENT));
            }
            assert!(own_row(&mut y, analyzed).is_err());
        }
        assert!(epona_bytes(&out).is_err());
    }

    #[test]
    fn the_bean_salesman_keeps_one_sale_at_sixty_rupees() {
        let Some(mut rf) = real() else {
            return;
        };
        let raw: Vec<u8> = rf.read_zs(BEAN_FILE).unwrap();
        let out: Vec<u8> = bean_bytes(&raw).unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        let f: &Flow = b.flows.iter().find(|x| x.name == BEAN_FLOW).unwrap();
        for (ev, key) in BEAN_SALES {
            let e: &Event = f.events.iter().find(|e| e.name == ev).unwrap();
            assert_eq!(e.kind.params().unwrap().get(key), Some(&Val::Int(BEAN_PRICE)));
        }
        assert!(bean_bytes(&out).is_err());
    }
}
