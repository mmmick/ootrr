use crate::ainb::Ainb;
use crate::error::{Error as E, Res};
use crate::romfs::Romfs;
use crate::staging::Out;

pub const NOTE: &str = "The jailed Gorons of the Fire Temple are gone. Their cells still open with their switches, and the chests in them open without talking to a Goron first.";

const BOOL: usize = 1;
const NPC: &str = "ActorLogicNpc";
const TBOX: &str = "ActorLogicTreasureBox";

pub const GORONS: [(&str, &str); 9] = [
    ("Room_02_Prefab_f1d5", "Npc_Goron_005_Adult_FireTemple_e40a"),
    ("Room_04_Prefab_c7fa", "Npc_Goron_002_Adult_FireTemple_6846"),
    ("Room_05_Prefab_812f", "Npc_Goron_008_Adult_FireTemple_26ba"),
    ("Room_10_Prefab_1fcc", "Npc_Goron_004_Adult_FireTemple_8eea"),
    ("Room_11_Prefab_6c9c", "Npc_Goron_009_Adult_FireTemple_127c"),
    ("Room_17_Prefab_68e8", "Npc_Goron_010_Adult_FireTemple_e8eb"),
    ("Room_20_Prefab_4850", "Npc_Goron_001_Adult_FireTemple_1f3b"),
    ("Room_22_Prefab_f820", "Npc_Goron_003_Adult_FireTemple_e347"),
    ("Room_23_Prefab_612a", "Npc_Goron_007_Adult_FireTemple_6279"),
];

pub const CHESTS: [(&str, &str); 8] = [
    ("Room_02_Prefab_3004", "Obj_TBox_A_01_435b"),
    ("Room_04_Prefab_d826", "Obj_TBox_A_01_d31e"),
    ("Room_05_Prefab_5b8f", "Obj_TBox_A_01_c516"),
    ("Room_10_Prefab_b3bc", "Obj_TBox_A_01_ade4"),
    ("Room_11_Prefab_c20f", "Obj_TBox_A_01_58b1"),
    ("Room_20_Prefab_7421", "Obj_TBox_A_01_ac82"),
    ("Room_22_Prefab_e0c9", "Obj_TBox_A_01_221a"),
    ("Room_23_Prefab_efdf", "Obj_TBox_A_01_e720"),
];

pub fn path(room: &str) -> String {
    format!("Logic/FireTemple_Level_{room}.logic.module.ainb")
}

fn named(a: &Ainb, kind: &str, instance: &str) -> Res<usize> {
    a.find_els(kind).into_iter().find(|&i| a.prop_str(i, "InstanceName") == Some(instance)).ok_or_else(|| E::Msg(format!("fire temple gorons: no {instance}, the game files differ")))
}

fn source(a: &Ainb, i: usize, name: &str) -> Res<i16> {
    let g: usize = a.find_in(i, BOOL, name).ok_or_else(|| E::Msg(format!("fire temple gorons: element {i} has no {name}")))?;
    let x = a.input(BOOL, g).ok_or(E::Format("fire temple gorons: input out of range"))?;
    if x.src < 0 {
        return Err(E::Msg(format!("fire temple gorons: {name} of element {i} is already a constant, the module is not the game's")));
    }
    Ok(x.src)
}

pub fn remove(a: &mut Ainb, goron: &str) -> Res<usize> {
    let i: usize = named(a, NPC, goron)?;
    let src: i16 = source(a, i, "Logic_Delete")?;
    let hit: Vec<usize> = (0..a.els.len())
        .filter(|&j| a.find_in(j, BOOL, "Logic_Delete").and_then(|g| a.input(BOOL, g)).is_some_and(|x| x.src == src))
        .collect();
    for &j in &hit {
        a.force_bool(j, "Logic_Delete", true)?;
    }
    Ok(hit.len())
}

pub fn unlock(a: &mut Ainb, chest: &str) -> Res<()> {
    let i: usize = named(a, TBOX, chest)?;
    source(a, i, "Unlock")?;
    a.force_bool(i, "Unlock", true)?;
    Ok(())
}

fn rewrite(rf: &mut Romfs, room: &str, f: impl FnOnce(&mut Ainb) -> Res<()>) -> Res<Out> {
    let p: String = path(room);
    let raw: Vec<u8> = rf.read(&p)?;
    let mut a: Ainb = Ainb::parse(&raw)?;
    f(&mut a)?;
    let out: Vec<u8> = a.write()?;
    if Ainb::parse(&out)?.write()? != out {
        return Err(E::Msg(format!("fire temple gorons: {p} does not round trip")));
    }
    Ok(Out { path: p, old: raw.len(), new: out.len(), z: out })
}

pub fn stage(rf: &mut Romfs) -> Res<Vec<Out>> {
    let mut out: Vec<Out> = Vec::new();
    for (room, goron) in GORONS {
        out.push(rewrite(rf, room, |a| remove(a, goron).map(|_| ()))?);
    }
    for (room, chest) in CHESTS {
        out.push(rewrite(rf, room, |a| unlock(a, chest))?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn romfs() -> Option<Romfs> {
        let p: std::path::PathBuf = ootrr_game::test_romfs::path();
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Romfs::open(&p).ok()
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    fn constant(a: &Ainb, kind: &str, instance: &str, name: &str) -> Option<bool> {
        let i: usize = named(a, kind, instance).ok()?;
        let x = a.input(BOOL, a.find_in(i, BOOL, name)?)?;
        (x.src < 0).then(|| x.val.iter().any(|&b| b != 0))
    }

    #[test]
    fn each_goron_and_its_wall_are_gone_and_each_chest_is_unlocked() {
        let Some(mut rf) = romfs() else {
            return;
        };
        let outs: Vec<Out> = stage(&mut rf).unwrap();
        assert_eq!(outs.len(), 17);
        for (room, goron) in GORONS {
            let mut a: Ainb = Ainb::parse(&rf.read(&path(room)).unwrap()).unwrap();
            assert_eq!(remove(&mut a, goron).unwrap(), 2, "{room}");
            assert_eq!(constant(&a, NPC, goron, "Logic_Delete"), Some(true), "{room}");
            let o: &Out = outs.iter().find(|o| o.path == path(room)).unwrap();
            let b: Ainb = Ainb::parse(&o.z).unwrap();
            assert_eq!(constant(&b, NPC, goron, "Logic_Delete"), Some(true), "{room}");
            assert!(remove(&mut a, goron).is_err(), "{room}");
        }
        for (room, chest) in CHESTS {
            let o: &Out = outs.iter().find(|o| o.path == path(room)).unwrap();
            let b: Ainb = Ainb::parse(&o.z).unwrap();
            assert_eq!(constant(&b, TBOX, chest, "Unlock"), Some(true), "{room}");
        }
    }
}
