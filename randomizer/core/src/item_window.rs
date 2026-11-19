//! Window-only copies of the game's give routines, for dungeon items.
//!
//! Every reward event hands its item out through a give routine (Dm_SY_GetInEvent:GetInEvent_*,
//! NpcCommon:NpcCommon_HandOver_*, a local TakenReward_* entry and so on) that shows the item,
//! plays Link's item get, and ends in the game's generic pouch get (GetPouchContent:
//! GetPouchContentByActorName*, or a bare PouchSystemActor add after its own window). The pouch add
//! does not know which dungeon a key belongs to, so a dungeon item runs a copy of the routine whose
//! pouch get is only the get window (GetPouchContent:DisplayGetByActorName) and whose bare adds are
//! a zero-frame wait. Everything else (the item model, the animation, the camera, the clean up)
//! stays. crate::gift_keys writes the dungeon credit after the copy returns.
//!
//! A copy takes every event its entry reaches. A call into another routine that reaches a pouch get
//! (in the same file or another one) calls that routine's copy instead.

use std::collections::{BTreeMap as BM, BTreeSet as BS};

use crate::bfevfl::{Bf, Cont, Entry, Event, Flow, Kind, Val};
use crate::dungeon_credit::action_slot;
use crate::error::{Error as E, Res};
use crate::flow_locations::StagedFile;
use crate::romfs::Romfs;

pub const PREFIX: &str = "OOTRR_W_";
/// The small key's copy: it holds up the actor it is given (the minigame key, the one key the game
/// holds up itself) while its get window names the dungeon key (crate::gift_keys).
pub const KEY_PREFIX: &str = "OOTRR_K_";
pub const KEY_WINDOW: &str = "Item_Dungeon_SmallKey";
const POUCH: &str = "GetPouchContent";
const WINDOW: &str = "DisplayGetByActorName";
const ADD: (&str, &str) = ("PouchSystemActor", "EventTriggerAddOrRemovePouchContent");
const WAIT: (&str, &str) = ("EventSystemActor", "EventWait");
const NONE: u16 = 0xFFFF;

pub fn name(entry: &str) -> String {
    format!("{PREFIX}{entry}")
}

pub fn key_name(entry: &str) -> String {
    format!("{KEY_PREFIX}{entry}")
}

pub fn path(flow: &str) -> String {
    crate::flow_locations::path_of(&format!("{flow}.bfevfl"))
}

/// The flows read so far, from the staged files when there, else from the game.
pub struct Cache {
    flows: BM<String, Bf>,
    missing: BS<String>,
    dirty: BS<String>,
}

impl Default for Cache {
    fn default() -> Cache {
        Cache::new()
    }
}

impl Cache {
    pub fn new() -> Cache {
        Cache { flows: BM::new(), missing: BS::new(), dirty: BS::new() }
    }

    fn load(&mut self, rf: &mut Romfs, files: &[StagedFile], flow: &str) -> Res<bool> {
        if self.flows.contains_key(flow) {
            return Ok(true);
        }
        if self.missing.contains(flow) {
            return Ok(false);
        }
        let p: String = path(flow);
        let raw: Vec<u8> = match files.iter().find(|f| f.path == p) {
            Some(s) => rf.zs.decompress(&s.z)?,
            None => match rf.read(&p) {
                Ok(z) => rf.zs.decompress(&z)?,
                Err(_) => {
                    self.missing.insert(flow.to_string());
                    return Ok(false);
                }
            },
        };
        self.flows.insert(flow.to_string(), Bf::parse(&raw)?);
        Ok(true)
    }

    /// The flow named `flow`, loaded, for editing.
    pub fn flow_mut(&mut self, rf: &mut Romfs, files: &[StagedFile], flow: &str) -> Res<&mut Flow> {
        if !self.load(rf, files, flow)? {
            return Err(E::Msg(format!("item window: there is no flow file for {flow}")));
        }
        self.dirty.insert(flow.to_string());
        let b: &mut Bf = self.flows.get_mut(flow).ok_or(E::Format("item window: a loaded flow vanished"))?;
        b.flows.first_mut().ok_or_else(|| E::Msg(format!("item window: {flow} holds no flow")))
    }

    fn flow(&self, flow: &str) -> Option<&Flow> {
        self.flows.get(flow).and_then(|b| b.flows.first())
    }

    /// Writes every edited flow back into `files` (adding the ones not staged yet).
    pub fn store(&self, rf: &mut Romfs, files: &mut Vec<StagedFile>) -> Res<()> {
        for name in &self.dirty {
            let b: &Bf = &self.flows[name];
            if let Some(m) = b.flows.first().and_then(|f| crate::dungeon_credit::sound(f).into_iter().next()) {
                return Err(E::Msg(format!("item window: {name} is broken after the edit, {m}")));
            }
            let out: Vec<u8> = b.write()?;
            if Bf::parse(&out)? != *b {
                return Err(E::Msg(format!("item window: the edited {name} does not parse back")));
            }
            let p: String = path(name);
            match files.iter().position(|f| f.path == p) {
                Some(i) => {
                    let id: Option<u32> = crate::flow_locations::dict_id(&files[i].z);
                    files[i].new = out.len();
                    files[i].z = crate::zs::compress_raw(&out, id)?;
                }
                None => {
                    let z: Vec<u8> = rf.read(&p)?;
                    let id: Option<u32> = crate::flow_locations::dict_id(&z);
                    let old: usize = rf.zs.decompress(&z)?.len();
                    files.push(StagedFile { path: p, z: crate::zs::compress_raw(&out, id)?, old, new: out.len() });
                }
            }
        }
        Ok(())
    }
}

/// An NPC routine takes the NPC it hands over from through an argument actor: sub
/// "<entry>(<arg>)", arg the parameter's name, ep the entry's index (GetInEvent_NotTurn declares
/// Npc_EventStarter with actor 19, ep 17; its events act on actor 26, Fado's, of the same argument).
/// A copy is a new entry that declares no argument, so the caller's NPC is bound to nothing: the
/// copy's NPC actions (the hand over AS, the go offstage) never end, the fork waiting for them never
/// joins, and Link stays in the get pose with no item and no window (GC Rolling Goron as Adult, in
/// game, with every item form). Each argument `entry` declares gets a twin declared by the copy `w`
/// (the next entry index), and every argument actor the copy acts on becomes the twin of its
/// argument, with the action or query found by name. Returns argument name -> twin.
fn twins(f: &mut Flow, entry: &str, w: &str) -> Res<BM<String, u16>> {
    let src: u16 = f.entries.iter().position(|e| e.name == entry).ok_or_else(|| E::Msg(format!("item window: {} has no entry {entry}", f.name)))? as u16;
    let at: u16 = u16::try_from(f.entries.len()).map_err(|_| E::Format("item window: the flow has too many entries"))?;
    let mut out: BM<String, u16> = BM::new();
    let own: Vec<usize> = (0..f.actors.len()).filter(|&i| f.actors[i].ep == src && !f.actors[i].arg.is_empty()).collect();
    for i in own {
        let mut a: crate::bfevfl::Actor = f.actors[i].clone();
        if a.sub != format!("{entry}({})", a.arg) {
            return Err(E::Msg(format!("item window: the argument actor {} of {entry} is named {}, the game files differ", a.name, a.sub)));
        }
        if out.contains_key(&a.arg) {
            return Err(E::Msg(format!("item window: {entry} declares {} twice", a.arg)));
        }
        a.sub = format!("{w}({})", a.arg);
        a.ep = at;
        let to: u16 = u16::try_from(f.actors.len()).map_err(|_| E::Format("item window: the flow has too many actors"))?;
        out.insert(a.arg.clone(), to);
        f.actors.push(a);
    }
    Ok(out)
}

/// Points an action or query of an argument actor at the copy's twin of that argument.
fn rebind(f: &mut Flow, k: &mut Kind, twins: &BM<String, u16>, entry: &str) -> Res<()> {
    let (actor, slot, query): (&mut u16, &mut u16, bool) = match k {
        Kind::Action { actor, act, .. } => (actor, act, false),
        Kind::Switch { actor, query, .. } => (actor, query, true),
        _ => return Ok(()),
    };
    let a = f.actors.get(*actor as usize).ok_or(E::Format("item window: an event names a missing actor"))?;
    if a.arg.is_empty() {
        return Ok(());
    }
    let to: u16 = *twins.get(&a.arg).ok_or_else(|| E::Msg(format!("item window: {entry} acts on argument {} without declaring it", a.arg)))?;
    let list: &Vec<String> = if query { &a.queries } else { &a.actions };
    let name: String = list.get(*slot as usize).cloned().ok_or(E::Format("item window: an event names a missing action"))?;
    let t = &mut f.actors[to as usize];
    let list: &mut Vec<String> = if query { &mut t.queries } else { &mut t.actions };
    let j: usize = match list.iter().position(|x| *x == name) {
        Some(j) => j,
        None => {
            list.push(name);
            list.len() - 1
        }
    };
    *actor = to;
    *slot = u16::try_from(j).map_err(|_| E::Format("item window: the actor has too many actions"))?;
    Ok(())
}

fn targets(k: &Kind) -> Vec<u16> {
    match k {
        Kind::Action { next, .. } | Kind::Sub { next, .. } | Kind::Join { next } => vec![*next],
        Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => cases.iter().map(|c| c.ev).collect(),
        Kind::Fork { join, forks } => std::iter::once(*join).chain(forks.iter().copied()).collect(),
        Kind::Label { .. } => Vec::new(),
    }
}

fn remap(k: &mut Kind, m: &dyn Fn(u16) -> u16) {
    match k {
        Kind::Action { next, .. } | Kind::Sub { next, .. } | Kind::Join { next } => *next = m(*next),
        Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => cases.iter_mut().for_each(|c| c.ev = m(c.ev)),
        Kind::Fork { join, forks } => {
            *join = m(*join);
            forks.iter_mut().for_each(|x| *x = m(*x));
        }
        Kind::Label { .. } => {}
    }
}

fn reach(f: &Flow, main: u16) -> Vec<usize> {
    let mut seen: BS<usize> = BS::new();
    let mut out: Vec<usize> = Vec::new();
    let mut todo: Vec<u16> = vec![main];
    while let Some(i) = todo.pop() {
        if i == NONE || (i as usize) >= f.events.len() || !seen.insert(i as usize) {
            continue;
        }
        out.push(i as usize);
        todo.extend(targets(&f.events[i as usize].kind));
    }
    out.sort();
    out
}

fn is_pouch_get(k: &Kind) -> bool {
    matches!(k, Kind::Sub { flow, entry, .. } if flow == POUCH && entry.starts_with("GetPouchContent"))
}

fn is_add(f: &Flow, k: &Kind) -> bool {
    match k {
        Kind::Action { actor, act, .. } => f.actors.get(*actor as usize).is_some_and(|a| a.name == ADD.0 && a.sub.is_empty() && a.actions.get(*act as usize).is_some_and(|x| x == ADD.1)),
        _ => false,
    }
}

/// The routine a sub call runs: (flow, entry), the flow being `here` for a local call.
fn callee(k: &Kind, here: &str) -> Option<(String, String)> {
    match k {
        Kind::Sub { flow, entry, .. } | Kind::SubSwitch { flow, entry, .. } => Some((if flow.is_empty() { here.to_string() } else { flow.clone() }, entry.clone())),
        _ => None,
    }
}

fn entry_main(f: &Flow, entry: &str) -> Option<u16> {
    f.entries.iter().find(|e| e.name == entry).map(|e| e.main)
}

/// True when the routine reaches a pouch get, directly or through the routines it calls.
pub fn gives(c: &mut Cache, rf: &mut Romfs, files: &[StagedFile], flow: &str, entry: &str, seen: &mut BS<(String, String)>) -> Res<bool> {
    if flow == POUCH || !seen.insert((flow.to_string(), entry.to_string())) || !c.load(rf, files, flow)? {
        return Ok(false);
    }
    let (direct, calls): (bool, Vec<(String, String)>) = {
        let f: &Flow = c.flow(flow).ok_or(E::Format("item window: a loaded flow vanished"))?;
        let Some(main) = entry_main(f, entry) else { return Ok(false) };
        let evs: Vec<usize> = reach(f, main);
        let direct: bool = evs.iter().any(|&i| is_pouch_get(&f.events[i].kind) || is_add(f, &f.events[i].kind));
        (direct, evs.iter().filter_map(|&i| callee(&f.events[i].kind, flow)).collect())
    };
    if direct {
        return Ok(true);
    }
    for (f2, e2) in calls {
        if gives(c, rf, files, &f2, &e2, seen)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Makes (once) the window-only copy of `flow`:`entry` and returns its entry name.
pub fn window(c: &mut Cache, rf: &mut Romfs, files: &[StagedFile], flow: &str, entry: &str) -> Res<String> {
    copy(c, rf, files, flow, entry, false)
}

/// Makes (once) the small key's window-only copy of `flow`:`entry` (KEY_PREFIX): its get window
/// names KEY_WINDOW whatever actor the routine holds up.
pub fn key_window(c: &mut Cache, rf: &mut Romfs, files: &[StagedFile], flow: &str, entry: &str) -> Res<String> {
    copy(c, rf, files, flow, entry, true)
}

fn copy(c: &mut Cache, rf: &mut Romfs, files: &[StagedFile], flow: &str, entry: &str, key: bool) -> Res<String> {
    let w: String = if key { key_name(entry) } else { name(entry) };
    if !c.load(rf, files, flow)? {
        return Err(E::Msg(format!("item window: there is no flow file for {flow}")));
    }
    let (src, evs, calls): (Entry, Vec<usize>, Vec<(usize, String, String)>) = {
        let f: &Flow = c.flow(flow).ok_or(E::Format("item window: a loaded flow vanished"))?;
        if f.entries.iter().any(|e| e.name == w) {
            return Ok(w);
        }
        let src: Entry = f.entries.iter().find(|e| e.name == entry).cloned().ok_or_else(|| E::Msg(format!("item window: {flow} has no entry {entry}")))?;
        let evs: Vec<usize> = reach(f, src.main);
        let calls: Vec<(usize, String, String)> = evs
            .iter()
            .filter(|&&i| !is_pouch_get(&f.events[i].kind))
            .filter_map(|&i| callee(&f.events[i].kind, flow).map(|(a, b)| (i, a, b)))
            .collect();
        (src, evs, calls)
    };
    // The routines this one calls that give the item get their own copies first.
    let mut redirect: BM<usize, String> = BM::new();
    for (i, f2, e2) in calls {
        if f2 != POUCH && gives(c, rf, files, &f2, &e2, &mut BS::new())? {
            redirect.insert(i, copy(c, rf, files, &f2, &e2, key)?);
        }
    }
    let f: &mut Flow = c.flow_mut(rf, files, flow)?;
    if f.entries.iter().any(|e| e.name == w) {
        return Ok(w);
    }
    let base: usize = f.events.len();
    let at: BM<usize, u16> = evs.iter().enumerate().map(|(k, &i)| (i, (base + k) as u16)).collect();
    let tw: BM<String, u16> = twins(f, &src.name, &w)?;
    if base + evs.len() >= NONE as usize {
        return Err(E::Msg(format!("item window: {flow} would hold too many events")));
    }
    let m = |x: u16| -> u16 { at.get(&(x as usize)).copied().unwrap_or(x) };
    let (wa, wait): (u16, u16) = action_slot(f, WAIT.0, WAIT.1)?;
    for &i in &evs {
        let mut k: Kind = f.events[i].kind.clone();
        remap(&mut k, &m);
        rebind(f, &mut k, &tw, entry)?;
        if is_pouch_get(&f.events[i].kind) {
            let Kind::Sub { next, params, .. } = &k else { unreachable!() };
            let actor: Val = params
                .as_ref()
                .and_then(|p| p.items.iter().find(|(n, _)| n == "ActorName").map(|(_, v)| v.clone()))
                .ok_or_else(|| E::Msg(format!("item window: the pouch get {} in {flow} names no actor", f.events[i].name)))?;
            let actor: Val = if key { Val::Str(KEY_WINDOW.to_string()) } else { actor };
            k = Kind::Sub { next: *next, params: Some(Cont { items: vec![("ActorName".to_string(), actor)] }), flow: POUCH.to_string(), entry: WINDOW.to_string() };
        } else if is_add(f, &f.events[i].kind) {
            let Kind::Action { next, .. } = &k else { unreachable!() };
            k = Kind::Action { next: *next, actor: wa, act: wait, params: Some(Cont { items: vec![("IsWaitEnd".to_string(), Val::Bool(1)), ("Frames".to_string(), Val::Int(0))] }) };
        } else if let Some(to) = redirect.get(&i) {
            match &mut k {
                Kind::Sub { entry, .. } | Kind::SubSwitch { entry, .. } => *entry = to.clone(),
                _ => {}
            }
        }
        f.events.push(Event { name: format!("OOTRRW{}", f.events.len()), kind: k });
    }
    let subs: Vec<u16> = src.subs.iter().map(|&x| m(x)).collect();
    f.entries.push(Entry { name: w.clone(), subs, vars: src.vars.clone(), main: m(src.main), pad: src.pad });
    Ok(w)
}


