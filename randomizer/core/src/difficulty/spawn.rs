use crate::ainb::build::{NIn, Spec, Src, PTR};
use crate::ainb::{audit, root, Ainb, Plug};
use crate::byml::{Byml, Node};
use crate::difficulty::backup::{flag, switch, GETB, IDLE, LIFT, MAT33, MAT34, PLACE, READY, SETB, SPLIT, WAKE};
use crate::error::{Error, Res};
use crate::qol::{bunny, edit_zs, Have};
use crate::romfs::Romfs;
use crate::sarc::{self, Entry};
use crate::staging::Out;

pub const ROOT: &str = "AI/Player.scheduler.root.ainb";
pub const HOLDER_DIR: &str = "Component/ActorHolderParam/";
pub const HOLDER_EXT: &str = ".game__component__ActorHolderParam.bgyml";
pub const HELD: &str = "ActorHoldSettings";
pub const HOLD_KEY: &str = "OotrrSpawn";
pub const BIT_KEYS: [&str; 6] = ["OotrrSpawnBit0", "OotrrSpawnBit1", "OotrrSpawnBit2", "OotrrSpawnBit3", "OotrrSpawnBit4", "OotrrSpawnBit5"];
pub const PENDING: &str = "OotrrSpawnPending";
pub const SLOT_KEY: &str = "OotrrSpawnSlot";
pub const ENEMIES: [&str; 43] = ["Enemy_Stalfos", "Enemy_Lizalfos", "Enemy_Dinolfos", "Enemy_Wolfos", "Enemy_Wolfos_White", "Enemy_Moblin_Club", "Enemy_Tektite_Red", "Enemy_Tektite_Blue", "Enemy_Keese", "Enemy_Keese_Fire", "Enemy_Keese_Ice", "Enemy_Skulltula", "Enemy_Skullwalltula", "Enemy_Dekubaba", "Enemy_Scrub_Mad", "Enemy_Octorok", "Enemy_Peahat", "Enemy_Leever", "Enemy_Leever_Big", "Enemy_Poe", "Enemy_Poe_Big", "Enemy_ReDead", "Enemy_Gibdo", "Enemy_LikeLike", "Enemy_Floormaster", "Enemy_Wallmaster", "Enemy_Amos", "Enemy_Beamos", "Enemy_Bubble_Blue", "Enemy_Bubble_Red", "Enemy_Bubble_Green", "Enemy_TorchSlug", "Enemy_Dodongo", "Enemy_BabyDodongo", "Enemy_Freezard", "Enemy_ShellBlade", "Enemy_Stinger", "Enemy_Biri", "Enemy_Bari", "Enemy_Stalchild", "Enemy_Guay", "Enemy_IronKnuckle_Silver", "Enemy_FlyingPot"];
pub const MAX_ACTORS: i32 = 12;
pub const AHEAD: f32 = 3.0;
pub const RISE: f32 = 1.0;
pub const DEPENDENCY: u32 = 3_707_532_803;
pub const MASK: &str = "QueryControllerGetMask";
pub const CHECK: &str = "QueryControllerTrigMaskChecker";
pub const CONJUNCTION: &str = "QueryLogicalConjunction";
pub const RANDOM: &str = "QueryGenericGetRandomBool";
pub const CREATE: &str = "OneShotActorHolderTryRequestCreateActorWithActorFileName";
pub const FORWARD: &str = "QueryActorGetForwardDir";
pub const SCALE: &str = "QueryMathMulF32AndVector3f";
pub const SUM: &str = "QueryMathAddVector3f";
pub const GET_S32: &str = "QueryBlackboardGetS32";
pub const SET_S32: &str = "ExecuteBlackboardSetS32";
pub const ADD_S32: &str = "QueryMathAddS32";
pub const LESS_S32: &str = "QueryMathLessS32";
pub const PICK_S32: &str = "QuerySelectS32ByBool";
pub const SLEEPER: &str = "QueryActorHolderGetEnableCancelSleepIndex";
pub const U32: &str = "const u32";
pub const OVER: &str = "bb::OverwriteParam";
pub const PARALLEL: u16 = 3;
pub const LOCAL: i32 = 0;
pub const BB_TYPES: usize = 7;
pub const BB_S32: usize = 1;
pub const BB_BOOL: usize = 4;
pub const BB_HEAD: usize = BB_TYPES * 8;
pub const BB_SIZES: [usize; BB_TYPES] = [4, 4, 4, 4, 4, 12, 0];
pub const BB_PLAIN: u32 = 0x0080_0000;

pub const NOTE: &str = "Hold both sticks pressed in (left stick click and right stick click) and press the D-pad Down to spawn a random ordinary enemy a few steps in front of Link. Press the D-pad Up with both sticks held in to spawn the same kind again. The pool has 43 kinds, from Stalfos and Lizalfos to Poes, Gibdos, Freezards and the silver Iron Knuckle. An enemy whose data the current area has not loaded will not appear, and at most twelve spawned enemies are kept at once. Experimental.";

fn vec3(x: f32, y: f32, z: f32) -> Vec<u8> {
    [x, y, z].iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn par(a: &mut Ainb, seed: &str, kids: &[usize]) -> Res<usize> {
    let mut s: Spec = Spec::composite(PARALLEL, seed).prop(0, "EndPolicy", vec![0; 4]).prop(0, "ResultPolicy", vec![0; 4]);
    for k in kids {
        s = s.kid(*k, "");
    }
    a.add_node(&s)
}

fn idle(a: &mut Ainb, what: &str) -> Res<usize> {
    a.add_node(&Spec::node(IDLE, &format!("ootrr spawn idle {what}")))
}

fn route(a: &mut Ainb, from: usize, to: usize, out: &str, input: &str) -> Res<()> {
    let want: u32 = a.pool.add(input);
    let name: u32 = a.pool.add(out);
    let at: usize = a.params.get(to).ok_or(Error::Format("spawn combo: no such reader"))?.group(0).iter().position(|p| p.a as usize == from && p.b == want).ok_or(Error::Format("spawn combo: the reader does not have that link"))?;
    a.params[to].remove_plug(0, at)?;
    let n: usize = a.params.get(from).ok_or(Error::Format("spawn combo: no such source"))?.pl[0][0] as usize;
    a.params[from].add_plug(0, n, Plug { a: to as u32, b: name, extra: Vec::new() })?;
    Ok(())
}

fn key_of(a: &mut Ainb, k: &str) -> Vec<u8> {
    a.pool.add(k).to_le_bytes().to_vec()
}

fn local_keys(a: &mut Ainb, keys: &[(usize, &str)]) -> Res<Vec<u8>> {
    let notes: u32 = a.pool.add("");
    let mut head: Vec<u8> = Vec::new();
    let mut rows: Vec<u8> = Vec::new();
    let mut values: Vec<u8> = Vec::new();
    let mut index: u16 = 0;
    for t in 0..BB_TYPES {
        let mine: Vec<&str> = keys.iter().filter(|k| k.0 == t).map(|k| k.1).collect();
        if !mine.is_empty() && BB_SIZES[t] == 0 {
            return Err(Error::Format("spawn combo: that kind of local key is not supported"));
        }
        let (at, offset): (u16, u16) = if index == 0 && mine.is_empty() { (0, 0) } else { (index, values.len() as u16) };
        head.extend_from_slice(&(mine.len() as u16).to_le_bytes());
        head.extend_from_slice(&at.to_le_bytes());
        head.extend_from_slice(&offset.to_le_bytes());
        head.extend_from_slice(&[0; 2]);
        for k in mine {
            rows.extend_from_slice(&(a.pool.add(k) | BB_PLAIN).to_le_bytes());
            rows.extend_from_slice(&notes.to_le_bytes());
            values.extend(std::iter::repeat_n(0u8, BB_SIZES[t]));
            index += 1;
        }
    }
    Ok([head, rows, values].concat())
}

fn get_bool<'a>(seed: &'a str, key: Vec<u8>) -> Spec<'a> {
    Spec::query(GETB, seed).prop(0, "BlackboardType", LOCAL.to_le_bytes().to_vec()).prop(3, "BlackboardKey", key).out(1, "Bool")
}

fn set_bool(a: &mut Ainb, seed: &str, key: &str, value: Result<bool, usize>) -> Res<usize> {
    let k: Vec<u8> = key_of(a, key);
    let mut s: Spec = Spec::node(SETB, seed)
        .prop(0, "BlackboardType", LOCAL.to_le_bytes().to_vec())
        .prop(1, "ExecuteOnce", flag(false))
        .prop(1, "IsIgnoreNotExistsError", flag(false))
        .prop(3, "BlackboardKey", k);
    s = match value {
        Err(q) => s.linked(1, "Bool", q, 0),
        Ok(v) => s.constant(1, "Bool", flag(v)),
    };
    a.add_node(&s)
}

fn create(a: &mut Ainb, enemy: &str, index: usize, slot: usize) -> Res<usize> {
    let name: Vec<u8> = key_of(a, enemy);
    let key: Vec<u8> = key_of(a, HOLD_KEY);
    a.add_node(&Spec::node(CREATE, &format!("ootrr spawn create {index} {enemy}")).prop(1, "ExecuteEveryFrame", flag(false)).constant(3, "ActorFileName", name).linked(0, "Index", slot, 0).constant(3, "Key", key))
}

fn tree(a: &mut Ainb, gets: &[usize; 6], slot: usize, level: usize, index: usize) -> Res<usize> {
    if level == gets.len() {
        return create(a, ENEMIES[index * ENEMIES.len() / (1 << gets.len())], index, slot);
    }
    let yes: usize = tree(a, gets, slot, level + 1, index * 2 + 1)?;
    let no: usize = tree(a, gets, slot, level + 1, index * 2)?;
    a.add_node(&switch(&format!("ootrr spawn tree {level} {index}"), gets[level], yes, no))
}

fn trig_checker<'a>(seed: &'a str, mask: usize, at: i16, read: [bool; 16]) -> Spec<'a> {
    let mut s: Spec = Spec::query(CHECK, seed);
    s.ins.push(NIn { k: PTR, name: "ControllerTrigMask", class: U32, src: Src::Link(mask, at) });
    let names: [&str; 16] = ["TrigA", "TrigB", "TrigDown", "TrigL", "TrigLeft", "TrigLSPush", "TrigMinus", "TrigPlus", "TrigR", "TrigRight", "TrigRSPush", "TrigUp", "TrigX", "TrigY", "TrigZL", "TrigZR"];
    for (n, r) in names.iter().zip(read) {
        s = if r { s.out(1, n) } else { s.spare(1, n) };
    }
    s
}

pub fn patch_graph(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    if !a.find_els(MASK).is_empty() {
        return Err(Error::Format("spawn combo: the graph is already patched"));
    }
    let top_parent: usize = root(&a).ok_or(Error::Format("spawn combo: the player scheduler has no root"))?;
    if a.el_type(top_parent) != PARALLEL || a.params[top_parent].group(2).len() != 3 {
        return Err(Error::Format("spawn combo: the player scheduler is not the three way parallel this edit was written for"));
    }
    if a.bb.len() != BB_HEAD || a.bb.iter().any(|b| *b != 0) {
        return Err(Error::Format("spawn combo: the player scheduler already has local keys"));
    }
    let mut keys: Vec<(usize, &str)> = BIT_KEYS.iter().chain([PENDING].iter()).map(|k| (BB_BOOL, *k)).collect();
    keys.insert(0, (BB_S32, SLOT_KEY));
    a.bb = local_keys(&mut a, &keys)?;
    let seed = |n: &str| format!("ootrr spawn {n}");
    let user: u32 = a.pool.add("");
    let mask_seed: String = seed("mask");
    let mut mask_spec: Spec = Spec::query(MASK, &mask_seed).constant(0, "ControllerIndex", vec![0; 4]).idle(3, "ControllerUserName", user.to_le_bytes().to_vec());
    mask_spec = mask_spec.out_ptr("ControllerHoldMask", U32).spare_ptr("ControllerReleaseMask", U32).spare_ptr("ControllerRepeatMask", U32).out_ptr("ControllerTrigMask", U32).spare_ptr("ControllerTrigWithRepeatMask", U32);
    let mask: usize = a.add_node(&mask_spec)?;
    let mut held: [bool; 16] = [false; 16];
    held[5] = true;
    held[10] = true;
    let mut pressed: [bool; 16] = [false; 16];
    pressed[2] = true;
    pressed[11] = true;
    let hold_check: usize = a.add_node(&trig_checker(&seed("hold check"), mask, 0, held))?;
    route(&mut a, mask, hold_check, "ControllerHoldMask", "ControllerTrigMask")?;
    let trig_check: usize = a.add_node(&trig_checker(&seed("press check"), mask, 3, pressed))?;
    route(&mut a, mask, trig_check, "ControllerTrigMask", "ControllerTrigMask")?;
    let both: usize = a.add_node(&Spec::query(CONJUNCTION, &seed("both sticks")).linked(1, "ValueA", hold_check, 5).linked(1, "ValueB", hold_check, 10).out(1, "Result"))?;
    let down: usize = a.add_node(&Spec::query(CONJUNCTION, &seed("down")).linked(1, "ValueA", both, 0).linked(1, "ValueB", trig_check, 2).out(1, "Result"))?;
    let up: usize = a.add_node(&Spec::query(CONJUNCTION, &seed("up")).linked(1, "ValueA", both, 0).linked(1, "ValueB", trig_check, 11).out(1, "Result"))?;

    let mut rolls: Vec<usize> = Vec::new();
    for (i, key) in BIT_KEYS.iter().enumerate() {
        let r: usize = a.add_node(&Spec::query(RANDOM, &seed(&format!("random {i}"))).idle(2, "FalseProbability", 1f32.to_le_bytes().to_vec()).idle(2, "TrueProbability", 1f32.to_le_bytes().to_vec()).out(1, "Bool"))?;
        rolls.push(set_bool(&mut a, &seed(&format!("store {i}")), key, Err(r))?);
    }
    rolls.push(set_bool(&mut a, &seed("pending on after down"), PENDING, Ok(true))?);
    let roll: usize = par(&mut a, &seed("roll"), &rolls)?;
    let again: usize = set_bool(&mut a, &seed("pending on after up"), PENDING, Ok(true))?;
    let nothing: usize = idle(&mut a, "press")?;
    let up_gate: usize = a.add_node(&switch(&seed("up gate"), up, again, nothing))?;
    let press: usize = a.add_node(&switch(&seed("press gate"), down, roll, up_gate))?;

    let mut gets: [usize; 6] = [0; 6];
    for (i, key) in BIT_KEYS.iter().enumerate() {
        let k: Vec<u8> = key_of(&mut a, key);
        gets[i] = a.add_node(&get_bool(&seed(&format!("load {i}")), k))?;
    }
    let pending_key: Vec<u8> = key_of(&mut a, PENDING);
    let pending: usize = a.add_node(&get_bool(&seed("pending"), pending_key))?;
    let slot_key: Vec<u8> = key_of(&mut a, SLOT_KEY);
    let slot: usize = a.add_node(&Spec::query(GET_S32, &seed("slot")).prop(0, "BlackboardType", LOCAL.to_le_bytes().to_vec()).prop(3, "BlackboardKey", slot_key.clone()).out(0, "S32"))?;
    let picked: usize = tree(&mut a, &gets, slot, 0, 0)?;
    let next: usize = a.add_node(&Spec::query(ADD_S32, &seed("next slot")).linked(0, "InputA", slot, 0).constant(0, "InputB", 1i32.to_le_bytes().to_vec()).out(0, "Result"))?;
    let inside: usize = a.add_node(&Spec::query(LESS_S32, &seed("slot inside")).linked(0, "S32", next, 0).constant(0, "S32Base", MAX_ACTORS.to_le_bytes().to_vec()).out(1, "Bool"))?;
    let wrapped: usize = a.add_node(&Spec::query(PICK_S32, &seed("slot wrapped")).constant(0, "FalseValue", 0i32.to_le_bytes().to_vec()).linked(0, "TrueValue", next, 0).linked(1, "Bool", inside, 0).out(0, "S32"))?;
    let step_slot: usize = a.add_node(
        &Spec::node(SET_S32, &seed("slot on"))
            .prop(0, "BlackboardType", LOCAL.to_le_bytes().to_vec())
            .prop(1, "ExecuteOnce", flag(false))
            .prop(1, "IsIgnoreNotExistsError", flag(false))
            .prop(3, "BlackboardKey", slot_key)
            .linked(0, "S32", wrapped, 0),
    )?;
    let clear: usize = set_bool(&mut a, &seed("pending off"), PENDING, Ok(false))?;
    let fired: usize = par(&mut a, &seed("fire"), &[clear, picked, step_slot])?;
    let waiting: usize = idle(&mut a, "fire")?;
    let fire: usize = a.add_node(&switch(&seed("fire gate"), pending, fired, waiting))?;

    let hold: Vec<u8> = key_of(&mut a, HOLD_KEY);
    let ready: usize = a.add_node(&Spec::query(READY, &seed("ready")).constant(3, "Key", hold.clone()).out(1, "IsExist"))?;
    let sleeper: usize = a.add_node(&Spec::query(SLEEPER, &seed("sleeper")).constant(3, "Key", hold.clone()).out(0, "Index").spare(1, "IsExist"))?;
    let place: usize = a.add_node(&Spec::query(PLACE, &seed("place")).out_ptr("Mat34F", MAT34))?;
    let split: usize = a.add_node(&Spec::query(SPLIT, &seed("split")).linked_ptr("Matrix34f", MAT34, place).out(4, "Translation").spare_ptr("Rotation", MAT33))?;
    route(&mut a, place, split, "Mat34F", "Matrix34f")?;
    let forward: usize = a.add_node(&Spec::query(FORWARD, &seed("forward")).out(4, "Forward"))?;
    let step: usize = a.add_node(&Spec::query(SCALE, &seed("step")).constant(2, "F32", AHEAD.to_le_bytes().to_vec()).linked(4, "Vector", forward, 0).out(4, "Vector"))?;
    route(&mut a, forward, step, "Forward", "Vector")?;
    let ahead: usize = a.add_node(&Spec::query(SUM, &seed("ahead")).linked(4, "InputA", split, 0).linked(4, "InputB", step, 0).out(4, "Result"))?;
    route(&mut a, split, ahead, "Translation", "InputA")?;
    route(&mut a, step, ahead, "Vector", "InputB")?;
    let lift: usize = a.add_node(&Spec::query(LIFT, &seed("lift")).linked(4, "InputA", ahead, 0).constant(4, "InputB", vec3(0.0, RISE, 0.0)).out(4, "Result"))?;
    route(&mut a, ahead, lift, "Result", "InputA")?;
    let wake: usize = a.add_node(
        &Spec::node(WAKE, &seed("wake"))
            .prop(1, "ExecuteEveryFrame", flag(true))
            .prop(1, "IsForeachActor", flag(false))
            .linked(0, "Index", sleeper, 0)
            .constant(3, "Key", hold)
            .linked(4, "Pos", lift, 0)
            .idle(4, "Scale", vec3(1.0, 1.0, 1.0))
            .idle_ptr("OverwriteParam", OVER)
            .idle_ptr("Rot", MAT33),
    )?;
    route(&mut a, lift, wake, "Result", "Pos")?;
    let asleep: usize = idle(&mut a, "wake")?;
    let wake_gate: usize = a.add_node(&switch(&seed("wake gate"), ready, wake, asleep))?;

    let top: usize = par(&mut a, &seed("top"), &[wake_gate, press, fire])?;
    let name: u32 = a.pool.add("");
    a.params.get_mut(top_parent).ok_or(Error::Format("spawn combo: no such node"))?.add_plug(2, 3, Plug { a: top as u32, b: name, extra: Vec::new() })?;
    let bad: Vec<String> = audit(&a);
    if !bad.is_empty() {
        return Err(Error::Msg(format!("spawn combo: {}", bad.join("; "))));
    }
    let out: Vec<u8> = a.write()?;
    let back: Ainb = Ainb::parse(&out)?;
    if back.write()? != out || back.bb != a.bb || back.params != a.params || back.ins != a.ins || back.outs != a.outs || back.qids != a.qids {
        return Err(Error::Format("spawn combo: the written graph does not read back"));
    }
    Ok(out)
}

pub fn holder_entry() -> Node {
    Node::Map(vec![
        ("DependencyWithHolder".to_string(), Node::U32(DEPENDENCY)),
        ("IsCreateFirstWithHolder".to_string(), Node::Bool(false)),
        ("IsEnableModelAlphaFadeIn".to_string(), Node::Bool(false)),
        ("IsEnableSleepDeleteWithHolder".to_string(), Node::Bool(true)),
        ("IsEnableWaitCreateAndIgnition".to_string(), Node::Bool(false)),
        ("IsWildActor".to_string(), Node::Bool(true)),
        ("MaxActors".to_string(), Node::I32(MAX_ACTORS)),
    ])
}

pub fn patch_holder(raw: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    let held: &mut Node = y.root.get_mut(HELD).ok_or(Error::Format("spawn combo: the player holder has no entries"))?;
    if held.get(HOLD_KEY).is_some() {
        return Err(Error::Format("spawn combo: the player holder already has the spawn entry"));
    }
    held.set(HOLD_KEY, holder_entry());
    let out: Vec<u8> = y.write()?;
    if Byml::parse(&out)?.root.get(HELD).and_then(|h| h.get(HOLD_KEY)) != Some(&holder_entry()) {
        return Err(Error::Format("spawn combo: the holder does not read back"));
    }
    Ok(out)
}

pub fn holder_of(actor: &str) -> String {
    format!("{HOLDER_DIR}{actor}{HOLDER_EXT}")
}

pub fn patch_pack(raw: &[u8], actor: &str) -> Res<Vec<u8>> {
    let mut es: Vec<Entry> = sarc::parse(raw)?;
    let holder: String = holder_of(actor);
    for (name, f) in [(ROOT, patch_graph as fn(&[u8]) -> Res<Vec<u8>>), (holder.as_str(), patch_holder)] {
        let data: Vec<u8> = sarc::get(&es, name).ok_or(Error::Msg(format!("spawn combo: the player pack has no {name}")))?.data.clone();
        sarc::set(&mut es, name, f(&data)?);
    }
    sarc::write(&es)
}

pub fn stage(rf: &mut Romfs, have: Have) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    for (file, actor) in bunny::PACKS {
        outs.push(edit_zs(rf, have, file, |raw| patch_pack(raw, actor))?);
    }
    outs.push(edit_zs(rf, have, bunny::ACTOR_INFO, bunny::grow_heap)?);
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        if p.is_dir() { Some(p) } else { None }
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    fn graph(rf: &mut Romfs, pack: &str, name: &str) -> Ainb {
        let es: Vec<Entry> = sarc::parse(&rf.read_zs(pack).unwrap()).unwrap();
        Ainb::parse(&sarc::get(&es, name).unwrap().data).unwrap()
    }

    fn shape(a: &Ainb, i: usize) -> Vec<String> {
        let mut out: Vec<String> = vec![format!("{} role {} type {}", a.el_name(i), a.role(i), a.el_type(i))];
        let p: &crate::ainb::Param = &a.params[i];
        for k in 0..6 {
            for g in p.pe[k][0] as usize..(p.pe[k][0] + p.pe[k][1]) as usize {
                out.push(format!("prop {k} {}", a.prop_name(k, g).unwrap()));
            }
            for g in a.input_ids(i, k) {
                let n: u32 = if k == 5 { u32::from_le_bytes(a.ins[5][g * 20..g * 20 + 4].try_into().unwrap()) } else { a.input(k, g).unwrap().name };
                let class: String = if k == 5 { a.s(u32::from_le_bytes(a.ins[5][g * 20 + 4..g * 20 + 8].try_into().unwrap())).to_string() } else { String::new() };
                out.push(format!("in {k} {} {class}", a.s(n)));
            }
            for g in p.ioe[k][2] as usize..(p.ioe[k][2] + p.ioe[k][3]) as usize {
                let sz: usize = if k == 5 { 8 } else { 4 };
                let w: u32 = u32::from_le_bytes(a.outs[k][g * sz..g * sz + 4].try_into().unwrap());
                let class: String = if k == 5 { a.s(u32::from_le_bytes(a.outs[5][g * 8 + 4..g * 8 + 8].try_into().unwrap())).to_string() } else { String::new() };
                out.push(format!("out {k} {} {class}", a.s(w & 0x3fff_ffff)));
            }
        }
        out
    }

    fn same(rf: &mut Romfs, new: &Ainb, node: &str, pack: &str, file: &str) {
        let old: Ainb = graph(rf, pack, file);
        let theirs: Vec<String> = shape(&old, old.find_els(node)[0]);
        let ours: Vec<Vec<String>> = new.find_els(node).into_iter().map(|i| shape(new, i)).collect();
        assert!(!ours.is_empty(), "{node}");
        for o in ours {
            assert_eq!(o, theirs, "{node} differs from {file}");
        }
    }

    fn same_anywhere(rf: &mut Romfs, new: &Ainb, node: &str) {
        let mut packs: Vec<String> = vec![crate::difficulty::backup::GLOBAL.to_string()];
        packs.extend(rf.list("Pack/Actor").unwrap());
        for p in packs {
            let es: Vec<Entry> = sarc::parse(&rf.read_zs(&p).unwrap()).unwrap();
            if let Some(e) = es.iter().filter(|e| e.name.ends_with(".ainb")).find(|e| Ainb::parse(&e.data).is_ok_and(|a| !a.find_els(node).is_empty())) {
                let name: String = e.name.clone();
                return same(rf, new, node, &p, &name);
            }
        }
        panic!("{node} is in no vanilla graph");
    }

    #[test]
    fn each_spawn_takes_the_next_holder_slot_and_the_wake_takes_the_sleeping_one() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let (_, new): (Ainb, Ainb) = built(&mut rf);
        for n in [GET_S32, SET_S32, ADD_S32, LESS_S32, PICK_S32, SLEEPER] {
            same_anywhere(&mut rf, &new, n);
        }
        let slot: usize = new.find_els(GET_S32)[0];
        assert_eq!(new.prop_str(slot, "BlackboardKey"), Some(SLOT_KEY));
        let creates: Vec<usize> = new.find_els(CREATE);
        assert_eq!(creates.len(), 64);
        for c in &creates {
            assert_eq!(new.params[*c].group(0).iter().map(|p| p.a as usize).collect::<Vec<usize>>(), vec![slot], "every create reads the slot");
        }
        let sleeper: usize = new.find_els(SLEEPER)[0];
        let wake: usize = new.find_els(WAKE)[0];
        assert!(new.params[wake].group(0).iter().any(|p| p.a as usize == sleeper), "the wake reads the sleeping actor's index");
        let step: usize = new.find_els(SET_S32)[0];
        assert_eq!(new.prop_str(step, "BlackboardKey"), Some(SLOT_KEY));
        let fire: usize = (0..new.els.len()).find(|i| new.el_type(*i) == PARALLEL && kids(&new, *i).contains(&step)).unwrap();
        assert_eq!(kids(&new, fire).len(), 3, "clear pending, create, step the slot");
    }

    fn built(rf: &mut Romfs) -> (Ainb, Ainb) {
        let old: Ainb = graph(rf, bunny::FILE, ROOT);
        let raw: Vec<u8> = sarc::get(&sarc::parse(&rf.read_zs(bunny::FILE).unwrap()).unwrap(), ROOT).unwrap().data.clone();
        (old, Ainb::parse(&patch_graph(&raw).unwrap()).unwrap())
    }

    fn local_names(a: &Ainb) -> Vec<(usize, String)> {
        let mut out: Vec<(usize, String)> = Vec::new();
        let mut at: usize = BB_HEAD;
        for t in 0..BB_TYPES {
            for _ in 0..u16::from_le_bytes([a.bb[t * 8], a.bb[t * 8 + 1]]) {
                let w: u32 = u32::from_le_bytes(a.bb[at..at + 4].try_into().unwrap());
                assert_eq!(w & !0x3f_ffff, BB_PLAIN);
                out.push((t, a.s(w & 0x3f_ffff).to_string()));
                at += 8;
            }
        }
        out
    }

    #[test]
    fn the_local_keys_are_written_byte_for_byte_like_the_vanilla_player_modules() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut seen: usize = 0;
        for file in ["AI/Player.sub.action.hookshot.module.ainb", "AI/Player.action.grabpushpull.module.ainb", "AI/Player.sub.action.lift.module.ainb", "AI/Player.action.hookshotshoot.module.ainb"] {
            let mut a: Ainb = graph(&mut rf, bunny::FILE, file);
            let names: Vec<(usize, String)> = local_names(&a);
            let keys: Vec<(usize, &str)> = names.iter().map(|k| (k.0, k.1.as_str())).collect();
            let theirs: Vec<u8> = a.bb.clone();
            let rows: usize = BB_HEAD + keys.len() * 8;
            let notes: u32 = u32::from_le_bytes(theirs[BB_HEAD + 4..BB_HEAD + 8].try_into().unwrap());
            assert_eq!(a.s(notes), "", "{file}");
            let mut ours: Vec<u8> = local_keys(&mut a, &keys).unwrap();
            for i in 0..keys.len() {
                ours[BB_HEAD + i * 8 + 4..BB_HEAD + i * 8 + 8].copy_from_slice(&notes.to_le_bytes());
            }
            assert_eq!(ours.len(), theirs.len(), "{file}");
            assert_eq!(ours[..rows], theirs[..rows], "{file}");
            seen += keys.len();
        }
        assert_eq!(seen, 12);
        let plain = |a: &Ainb| -> bool {
            let n: usize = (0..BB_TYPES).map(|t| u16::from_le_bytes([a.bb[t * 8], a.bb[t * 8 + 1]]) as usize).sum();
            let s32: u16 = u16::from_le_bytes([a.bb[BB_S32 * 8], a.bb[BB_S32 * 8 + 1]]);
            let bools: u16 = u16::from_le_bytes([a.bb[BB_BOOL * 8], a.bb[BB_BOOL * 8 + 1]]);
            let only: bool = (0..BB_TYPES).all(|t| t == BB_S32 || t == BB_BOOL || a.bb[t * 8] == 0 && a.bb[t * 8 + 1] == 0);
            only && s32 > 0 && bools > 0 && (0..n).all(|i| u32::from_le_bytes(a.bb[BB_HEAD + i * 8..BB_HEAD + i * 8 + 4].try_into().unwrap()) & !0x3f_ffff == BB_PLAIN)
        };
        let es: Vec<Entry> = sarc::parse(&rf.read_zs(crate::difficulty::backup::GLOBAL).unwrap()).unwrap();
        let mut mixed: Ainb = es.iter().filter(|e| e.name.ends_with(".ainb")).filter_map(|e| Ainb::parse(&e.data).ok()).find(|a| plain(a)).unwrap();
        let names: Vec<(usize, String)> = local_names(&mixed);
        let keys: Vec<(usize, &str)> = names.iter().map(|k| (k.0, k.1.as_str())).collect();
        let theirs: Vec<u8> = mixed.bb.clone();
        let notes: u32 = u32::from_le_bytes(theirs[BB_HEAD + 4..BB_HEAD + 8].try_into().unwrap());
        let mut ours: Vec<u8> = local_keys(&mut mixed, &keys).unwrap();
        for i in 0..keys.len() {
            ours[BB_HEAD + i * 8 + 4..BB_HEAD + i * 8 + 8].copy_from_slice(&notes.to_le_bytes());
        }
        assert_eq!(ours[..BB_HEAD + keys.len() * 8], theirs[..BB_HEAD + keys.len() * 8], "an s32 key and bools");
        let (old, new) = built(&mut rf);
        assert_eq!(old.bb, vec![0u8; BB_HEAD]);
        assert_eq!(new.bb.len(), BB_HEAD + 8 * 8 + 8 * 4);
        let type_of = |a: &Ainb, i: usize| -> String { crate::ainb::show::show(a).split("\nelement ").nth(i + 1).unwrap().lines().find(|l| l.contains("BlackboardType")).unwrap().trim().to_string() };
        for i in new.find_els(SETB).into_iter().chain(new.find_els(GETB)) {
            assert_eq!(type_of(&new, i), "prop s32 BlackboardType = 0");
        }
        let vanilla: Ainb = graph(&mut rf, bunny::FILE, "AI/Player.sub.action.hookshot.module.ainb");
        assert!(!vanilla.find_els(SETB).is_empty());
        for i in vanilla.find_els(SETB) {
            assert_eq!(type_of(&vanilla, i), "prop s32 BlackboardType = 0");
        }
    }

    fn waits_on(a: &Ainb, i: usize) -> Vec<usize> {
        let queries: Vec<usize> = (0..a.els.len()).filter(|j| a.query_pos(*j).is_some()).collect();
        let base: usize = u16::from_le_bytes([a.els[i][0x24], a.els[i][0x25]]) as usize;
        let n: usize = u16::from_le_bytes([a.els[i][0x26], a.els[i][0x27]]) as usize;
        (base..base + n).map(|k| queries[u16::from_le_bytes([a.qids[k * 4], a.qids[k * 4 + 1]]) as usize]).collect()
    }

    #[test]
    fn a_node_that_reads_one_query_twice_lists_it_twice_as_the_game_does() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let game: Ainb = graph(&mut rf, "Pack/Actor/GameSystemActor.pack.zs", "AI/GameSystemActor.event.root.ainb");
        let check: usize = game.find_els(CHECK)[0];
        let either: usize = (0..game.els.len()).find(|i| game.el_name(*i) == "QueryLogicalDisjunction" && game.params[*i].group(0).len() == 2 && game.params[*i].group(0).iter().all(|p| p.a as usize == check)).unwrap();
        assert_eq!(waits_on(&game, either), vec![check, check]);
        let (_, new): (Ainb, Ainb) = built(&mut rf);
        let checks: Vec<usize> = new.find_els(CHECK);
        let both: usize = new.find_els(CONJUNCTION)[0];
        assert_eq!(new.params[both].group(0).iter().map(|p| p.a as usize).collect::<Vec<usize>>(), vec![checks[0], checks[0]]);
        assert_eq!(waits_on(&new, both), vec![checks[0], checks[0]]);
    }

    fn kids(a: &Ainb, i: usize) -> Vec<usize> {
        a.params[i].group(2).iter().map(|p| p.a as usize).collect()
    }

    #[test]
    fn the_pool_fits_three_coin_flips() {
        assert!(ENEMIES.len() <= 1 << BIT_KEYS.len());
        let mut seen: Vec<&str> = ENEMIES.to_vec();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), ENEMIES.len());
        let mut reached: Vec<usize> = (0..1usize << BIT_KEYS.len()).map(|i| i * ENEMIES.len() / (1 << BIT_KEYS.len())).collect();
        reached.dedup();
        assert_eq!(reached, (0..ENEMIES.len()).collect::<Vec<usize>>());
    }

    #[test]
    fn the_holder_entry_has_the_wild_actor_shape_of_the_treasure_chest_entry() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let es: Vec<Entry> = sarc::parse(&rf.read_zs("Pack/Actor/Obj_TBox_A_02.pack.zs").unwrap()).unwrap();
        let y: Byml = Byml::parse(&sarc::get(&es, "Component/ActorHolderParam/Obj_TBox.game__component__ActorHolderParam.bgyml").unwrap().data).unwrap();
        let theirs: Node = y.root.get(HELD).unwrap().get("Content").unwrap().clone();
        let Node::Map(mut want) = theirs else {
            panic!("not a map");
        };
        want.push(("MaxActors".to_string(), Node::I32(MAX_ACTORS)));
        assert_eq!(holder_entry(), Node::Map(want));
    }

    #[test]
    fn the_nodes_have_the_shapes_of_the_vanilla_nodes_they_copy() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let (_, new) = built(&mut rf);
        let global: &str = "Pack/AI.Global.Product.111.pack.zs";
        same(&mut rf, &new, MASK, "Pack/Actor/GameSystemActor.pack.zs", "AI/GameSystemActor.event.root.ainb");
        same(&mut rf, &new, CHECK, "Pack/Actor/GameSystemActor.pack.zs", "AI/GameSystemActor.event.root.ainb");
        same(&mut rf, &new, RANDOM, global, "AI/ExecuteDarkLinkMirrorAction.module.ainb");
        same(&mut rf, &new, CONJUNCTION, global, "AI/ElevatorLiftMove.module.ainb");
        same(&mut rf, &new, SETB, global, "AI/ElevatorLiftMove.module.ainb");
        same(&mut rf, &new, SCALE, global, "AI/ElevatorLiftMove.module.ainb");
        same(&mut rf, &new, SUM, global, "AI/ElevatorLiftMove.module.ainb");
        same(&mut rf, &new, FORWARD, global, "AI/Enemy_Leever.utility.postsensor.root.ainb");
        same(&mut rf, &new, CREATE, global, "AI/Obj_TBox_A_01.root.ainb");
        same(&mut rf, &new, PLACE, global, "AI/Enemy_Dekubaba.utility.root.ainb");
        same(&mut rf, &new, GETB, global, "AI/NpcCommon.event.root.ainb");
    }

    #[test]
    fn the_scheduler_gains_one_fourth_child_and_the_old_three_are_kept() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let (old, new) = built(&mut rf);
        assert!(audit(&new).is_empty());
        let r: usize = root_of(&new);
        let before: Vec<usize> = kids(&old, root_of(&old));
        let after: Vec<usize> = kids(&new, r);
        assert_eq!(after.len(), 4);
        assert_eq!(&after[..3], &before[..]);
        let top: usize = after[3];
        assert_eq!(kids(&new, top).len(), 3);
        assert_eq!(new.find_els(CREATE).len(), 1 << BIT_KEYS.len());
        assert_eq!(new.find_els(RANDOM).len(), BIT_KEYS.len());
        assert_eq!(new.find_els(MASK).len(), 1);
        assert_eq!(new.find_els(CHECK).len(), 2);
    }

    fn root_of(a: &Ainb) -> usize {
        root(a).unwrap()
    }

    #[test]
    fn the_buttons_are_read_from_the_hold_mask_and_the_press_mask() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let (_, new) = built(&mut rf);
        let mask: usize = new.find_els(MASK)[0];
        let plugs: Vec<(usize, String)> = new.params[mask].group(0).iter().map(|p| (p.a as usize, new.s(p.b).to_string())).collect();
        let checks: Vec<usize> = new.find_els(CHECK);
        assert_eq!(plugs, vec![(checks[0], "ControllerHoldMask".to_string()), (checks[1], "ControllerTrigMask".to_string())]);
        let at = |c: usize| -> i16 { new.input(5, new.find_in(c, 5, "ControllerTrigMask").unwrap()).unwrap().plug };
        assert_eq!((at(checks[0]), at(checks[1])), (0, 3));
        let reads = |c: usize| -> Vec<String> {
            let p: &crate::ainb::Param = &new.params[c];
            (p.ioe[1][2] as usize..(p.ioe[1][2] + p.ioe[1][3]) as usize).filter(|g| u32::from_le_bytes(new.outs[1][g * 4..g * 4 + 4].try_into().unwrap()) >> 31 == 1).map(|g| new.s(u32::from_le_bytes(new.outs[1][g * 4..g * 4 + 4].try_into().unwrap()) & 0x3fff_ffff).to_string()).collect()
        };
        assert_eq!(reads(checks[0]), vec!["TrigLSPush", "TrigRSPush"]);
        assert_eq!(reads(checks[1]), vec!["TrigDown", "TrigUp"]);
    }

    #[test]
    fn every_enemy_has_its_own_create_node_and_the_tree_reaches_each_once() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let (_, new) = built(&mut rf);
        let mut names: Vec<String> = new.find_els(CREATE).into_iter().map(|i| new.in_str(i, "ActorFileName").unwrap().to_string()).collect();
        names.sort();
        names.dedup();
        let mut want: Vec<String> = ENEMIES.iter().map(|s| s.to_string()).collect();
        want.sort();
        assert_eq!(names, want);
        for i in new.find_els(CREATE) {
            assert_eq!(new.in_str(i, "Key"), Some(HOLD_KEY));
            assert_eq!(new.params[i].group(2).len(), 0);
        }
        let leaves: Vec<usize> = new.find_els(CREATE);
        for l in leaves {
            assert!(new.parent_of(l).is_ok());
        }
    }

    #[test]
    fn the_pack_gets_the_graph_the_blackboard_keys_and_the_holder_entry() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        for (file, actor) in bunny::PACKS {
            let was: Vec<Entry> = sarc::parse(&rf.read_zs(file).unwrap()).unwrap();
            let raw: Vec<u8> = rf.zs.decompress(&rf.read(file).unwrap()).unwrap();
            let now: Vec<Entry> = sarc::parse(&patch_pack(&raw, actor).unwrap()).unwrap();
            let changed: Vec<&str> = now.iter().filter(|e| sarc::get(&was, &e.name).is_none_or(|w| w.data != e.data)).map(|e| e.name.as_str()).collect();
            let holder: String = holder_of(actor);
            assert_eq!(changed.len(), 2, "{file}");
            assert!(changed.contains(&ROOT) && changed.contains(&holder.as_str()));
            let g: Ainb = Ainb::parse(&sarc::get(&now, ROOT).unwrap().data).unwrap();
            let mut want: Vec<&str> = BIT_KEYS.to_vec();
            want.push(PENDING);
            let mut keys: Vec<(usize, String)> = vec![(BB_S32, SLOT_KEY.to_string())];
            keys.extend(want.iter().map(|k| (BB_BOOL, k.to_string())));
            assert_eq!(local_names(&g), keys);
            let h: Byml = Byml::parse(&sarc::get(&now, &holder).unwrap().data).unwrap();
            assert_eq!(h.root.get(HELD).unwrap().get(HOLD_KEY), Some(&holder_entry()));
        }
    }

    #[test]
    fn a_pack_patched_twice_is_refused() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let (file, actor) = bunny::PACKS[0];
        let raw: Vec<u8> = rf.zs.decompress(&rf.read(file).unwrap()).unwrap();
        let once: Vec<u8> = patch_pack(&raw, actor).unwrap();
        assert!(patch_pack(&once, actor).is_err());
    }

    #[test]
    fn staging_writes_both_player_packs_and_the_actor_table() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let outs: Vec<Out> = stage(&mut rf, &none).unwrap();
        let paths: Vec<&str> = outs.iter().map(|o| o.path.as_str()).collect();
        assert_eq!(paths, vec![bunny::FILE, bunny::ADULT_FILE, bunny::ACTOR_INFO]);
    }
}
