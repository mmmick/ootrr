use crate::ainb::build::{Spec, QUERY};
use crate::ainb::{audit, Ainb, Plug};
use crate::byml::{Byml, Node};
use crate::difficulty::backup::{feed, flag, flag_query, flag_row, switch, MAT33, MAT34};
use crate::error::{Error, Res};
use crate::romfs::Romfs;
use crate::sarc::{self, Entry};
use crate::staging::Out;
use crate::zs;

pub const PACK: &str = "Pack/Actor/Enemy_IronKnuckle_Silver.pack.zs";
pub const SIT: &str = "AI/Enemy_IronKnuckle.action.Sit.module.ainb";
pub const TABLE: &str = "Component/Blackboard/BlackboardParamTable/Enemy_IronKnuckle.engine__component__BlackboardParamTable.bgyml";
pub const BOOLS: &str = "BlackboardParamBoolArray";
pub const KEY: &str = "OotrrKnightWake";
pub const RADIUS: f32 = 9.0;
pub const STACK: &str = "SelectorStackNode";
pub const SPHERE: &str = "QueryUserPerceptionIsInsideSphereByInput";
pub const BONE: &str = "QueryModelGetBoneMatrix34f";
pub const SPLIT: &str = "QueryMathMtx34fToVec3fMat33f";
pub const WIN: &str = "ExecuteGenericSetResultSuccess";
pub const HOLD: &str = "ExecuteGenericSetResultNone";
pub const SPINE: &str = "Spine_2";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wake {
    SeatedUntilNear,
    Awake,
}

pub const MODE: Wake = Wake::Awake;

pub fn patch_sit(raw: &[u8]) -> Res<Vec<u8>> {
    patch_sit_with(raw, MODE)
}

pub fn patch_sit_with(raw: &[u8], mode: Wake) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    let stacks: Vec<usize> = a.find_els(STACK);
    if stacks.len() != 1 || a.find_els(SPHERE).len() != 1 || a.find_els(BONE).len() != 1 {
        return Err(Error::Format("knight wake: the sit graph is not the one this edit was written for"));
    }
    let stack: usize = stacks[0];
    let key: Vec<u8> = a.pool.add(KEY).to_le_bytes().to_vec();
    let set: usize = a.add_node(&flag_query("ootrr knight wake set", key))?;
    let stand: usize = a.add_node(&Spec::node(WIN, "ootrr knight wake stand"))?;
    let off: usize = a.add_node(&Spec::node(HOLD, "ootrr knight wake stay off"))?;
    let on: usize = match mode {
        Wake::Awake => stand,
        Wake::SeatedUntilNear => {
            let spine: u32 = a.pool.add(SPINE);
            let mut bone_spec: Spec = Spec::node(BONE, "ootrr knight wake bone");
            bone_spec.role = QUERY;
            let bone: usize = a.add_node(
                &bone_spec
                    .prop(1, "IsIgnoreNotBoneExistsError", flag(false))
                    .prop(1, "OutputOnlyOnEnter", flag(false))
                    .prop(3, "BoneName", spine.to_le_bytes().to_vec())
                    .spare(1, "IsValid")
                    .out_ptr("Mat34F", MAT34),
            )?;
            let split: usize = a.add_node(&Spec::query(SPLIT, "ootrr knight wake split").linked_ptr("Matrix34f", MAT34, bone).out(4, "Translation").spare_ptr("Rotation", MAT33))?;
            feed(&mut a, bone, split, "Mat34F")?;
            let sphere: usize = a.add_node(
                &Spec::query(SPHERE, "ootrr knight wake sphere")
                    .idle(1, "IsUseGameBalanceClientRadius", flag(false))
                    .constant(2, "Radius", RADIUS.to_le_bytes().to_vec())
                    .linked(4, "Center", split, 0)
                    .out(1, "IsInside"),
            )?;
            feed(&mut a, split, sphere, "Translation")?;
            let stay: usize = a.add_node(&Spec::node(HOLD, "ootrr knight wake stay near"))?;
            a.add_node(&switch("ootrr knight wake near", sphere, stand, stay))?
        }
    };
    let gate: usize = a.add_node(&switch("ootrr knight wake gate", set, on, off))?;
    let name: u32 = a.pool.add("");
    a.params.get_mut(stack).ok_or(Error::Format("knight wake: no such node"))?.add_plug(2, 0, Plug { a: gate as u32, b: name, extra: Vec::new() })?;
    let bad: Vec<String> = audit(&a);
    if !bad.is_empty() {
        return Err(Error::Msg(format!("knight wake: {}", bad.join("; "))));
    }
    let out: Vec<u8> = a.write()?;
    let back: Ainb = Ainb::parse(&out)?;
    if back.write()? != out || back.params != a.params || back.ins != a.ins || back.outs != a.outs || back.qids != a.qids {
        return Err(Error::Format("knight wake: the written graph does not read back"));
    }
    Ok(out)
}

pub fn patch_table(raw: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    let list: &mut Vec<Node> = y.root.get_mut(BOOLS).and_then(Node::as_arr_mut).ok_or(Error::Format("knight wake: the knight blackboard has no bool list"))?;
    if list.iter().any(|r| r.get("BBKey").and_then(Node::as_str) == Some(KEY)) {
        return Err(Error::Format("knight wake: the knight blackboard already has the wake key"));
    }
    list.push(flag_row(KEY, false));
    let out: Vec<u8> = y.write()?;
    if Byml::parse(&out)?.root != y.root {
        return Err(Error::Format("knight wake: the blackboard does not read back"));
    }
    Ok(out)
}

pub fn patch_entries(es: &mut [Entry]) -> Res<()> {
    let mut done: usize = 0;
    for e in es.iter_mut() {
        if e.name == SIT {
            e.data = patch_sit(&e.data)?;
            done += 1;
        } else if e.name == TABLE {
            e.data = patch_table(&e.data)?;
            done += 1;
        }
    }
    if done != 2 {
        return Err(Error::Format("knight wake: the knight pack is missing the sit graph or its blackboard"));
    }
    Ok(())
}

pub fn stage(rf: &mut Romfs, have: crate::qol::Have) -> Res<Out> {
    let z: Vec<u8> = match have(PACK) {
        Some(b) => b,
        None => rf.read(PACK)?,
    };
    let raw: Vec<u8> = rf.zs.decompress(&z)?;
    let mut es: Vec<Entry> = sarc::parse(&raw)?;
    patch_entries(&mut es)?;
    let out: Vec<u8> = sarc::write(&es)?;
    Ok(Out { path: PACK.to_string(), old: raw.len(), new: out.len(), z: zs::compress_raw(&out, zs::dict_of(&z))? })
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

    fn kids(a: &Ainb, i: usize) -> Vec<usize> {
        a.params[i].group(2).iter().map(|p| p.a as usize).collect()
    }

    fn source(a: &Ainb, i: usize) -> usize {
        a.params[i].group(0)[0].a as usize
    }

    fn check_sit(old: &Ainb, new: &Ainb, mode: Wake) {
        assert!(audit(new).is_empty());
        let stack: usize = new.find_els(STACK)[0];
        let before: Vec<usize> = kids(old, old.find_els(STACK)[0]);
        let after: Vec<usize> = kids(new, stack);
        assert_eq!(after.len(), before.len() + 1);
        assert_eq!(&after[1..], &before[..]);
        let gate: usize = after[0];
        let flag_q: usize = source(new, gate);
        assert_eq!(new.el_name(flag_q), "QueryBlackboardGetBoolNoError");
        assert_eq!(new.prop_str(flag_q, "BlackboardKey"), Some(KEY));
        assert_eq!(new.el_name(kids(new, gate)[1]), HOLD);
        match mode {
            Wake::Awake => {
                assert_eq!(new.el_name(kids(new, gate)[0]), WIN);
                assert_eq!(new.find_els(SPHERE).len(), 1);
                assert_eq!(new.find_els(BONE).len(), 1);
            }
            Wake::SeatedUntilNear => {
                let near: usize = kids(new, gate)[0];
                let sphere: usize = source(new, near);
                assert_eq!(new.el_name(sphere), SPHERE);
                assert_eq!(new.el_name(kids(new, near)[0]), WIN);
                assert_eq!(new.el_name(kids(new, near)[1]), HOLD);
                assert_eq!(new.find_els(SPHERE).len(), 2);
                let radius = |i: usize| -> f32 {
                    let g: usize = new.input_ids(i, 2).into_iter().find(|g| new.input(2, *g).is_some_and(|x| new.s(x.name) == "Radius")).unwrap();
                    let v: Vec<u8> = new.input(2, g).unwrap().val;
                    f32::from_le_bytes([v[0], v[1], v[2], v[3]])
                };
                let radii: Vec<f32> = new.find_els(SPHERE).into_iter().map(radius).collect();
                assert_eq!(radii, vec![0.8, RADIUS]);
            }
        }
    }

    #[test]
    fn the_default_mode_is_awake() {
        assert_eq!(MODE, Wake::Awake);
    }

    #[test]
    fn the_sit_graph_gets_one_new_first_child_in_both_modes() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let was: Vec<Entry> = sarc::parse(&rf.read_zs(PACK).unwrap()).unwrap();
        let raw: Vec<u8> = sarc::get(&was, SIT).unwrap().data.clone();
        let old: Ainb = Ainb::parse(&raw).unwrap();
        for mode in [Wake::Awake, Wake::SeatedUntilNear] {
            let new: Ainb = Ainb::parse(&patch_sit_with(&raw, mode).unwrap()).unwrap();
            check_sit(&old, &new, mode);
        }
    }

    #[test]
    fn staging_changes_only_the_sit_graph_and_the_blackboard() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let was: Vec<Entry> = sarc::parse(&rf.read_zs(PACK).unwrap()).unwrap();
        let old: Ainb = Ainb::parse(&sarc::get(&was, SIT).unwrap().data).unwrap();
        let out: Out = stage(&mut rf, &none).unwrap();
        assert_eq!(out.path, PACK);
        let now: Vec<Entry> = sarc::parse(&rf.zs.decompress(&out.z).unwrap()).unwrap();
        let changed: Vec<&str> = now.iter().filter(|e| sarc::get(&was, &e.name).is_none_or(|w| w.data != e.data)).map(|e| e.name.as_str()).collect();
        assert_eq!(changed.len(), 2);
        assert!(changed.contains(&SIT) && changed.contains(&TABLE));
        assert_eq!(now.len(), was.len());
        let new: Ainb = Ainb::parse(&sarc::get(&now, SIT).unwrap().data).unwrap();
        check_sit(&old, &new, MODE);
        let t: Byml = Byml::parse(&sarc::get(&now, TABLE).unwrap().data).unwrap();
        let rows: &[Node] = t.root.get(BOOLS).unwrap().as_arr().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1], flag_row(KEY, false));
        assert_eq!(rows[1].get("IsInstanceParam"), Some(&Node::Bool(true)));
    }

    #[test]
    fn a_pack_that_was_already_patched_or_is_not_a_knight_pack_is_refused() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut es: Vec<Entry> = sarc::parse(&rf.read_zs(PACK).unwrap()).unwrap();
        patch_entries(&mut es).unwrap();
        assert!(patch_entries(&mut es).is_err());
        let mut other: Vec<Entry> = sarc::parse(&rf.read_zs("Pack/Actor/Enemy_Stalfos.pack.zs").unwrap()).unwrap();
        assert!(patch_entries(&mut other).is_err());
    }

    #[test]
    fn the_other_knights_are_not_touched() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let outs: Vec<Out> = crate::difficulty::encounters::stage(&mut rf, &none).unwrap();
        let packs: Vec<&str> = outs.iter().filter(|o| o.path.ends_with(".pack.zs")).map(|o| o.path.as_str()).collect();
        assert_eq!(packs, vec![PACK]);
    }
}
