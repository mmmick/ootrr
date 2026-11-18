use crate::ainb::Ainb;
use crate::error::{Error, Res};
use crate::romfs::Romfs;
use crate::staging::Out;

pub const MODULE: &str = "Logic/KokiriForest_Common_Level_All_01_6aaf.logic.module.ainb";
pub const AGE: &str = "HoldIsChildhoodOrAdulthood";
pub const FLAG_NODE: &str = "HoldGameDataGetBoolSimple";
pub const FLAG: &str = "SageOfForest_IsBefore_Step08";
pub const AND: &str = "OperateAND";
pub const MULTI: &str = "BoolMulti";
pub const ADULT_OUT: u16 = 0;
const BOOL: usize = 1;
const BOOL_IN: usize = 16;
const MULTI_COUNT: usize = 0x1c;

fn entry(a: &Ainb, j: usize) -> Option<(u16, u16)> {
    let e: &[u8] = a.multi.get(8 * j..8 * j + 4)?;
    Some((u16::from_le_bytes([e[0], e[1]]), u16::from_le_bytes([e[2], e[3]])))
}

pub fn sources(a: &Ainb, i: usize) -> Option<Vec<(u16, u16)>> {
    let g: usize = a.find_in(i, BOOL, MULTI)?;
    let x = a.input(BOOL, g)?;
    if x.src > -100 {
        return None;
    }
    let start: usize = (-100 - i32::from(x.src)) as usize;
    (start..start + x.plug as usize).map(|j| entry(a, j)).collect()
}

pub fn gate(a: &Ainb) -> Option<usize> {
    a.find_els(AND).into_iter().find(|&i| {
        sources(a, i).is_some_and(|s| {
            s.len() == 2 && a.el_name(s[0].0 as usize) == AGE && s[0].1 == ADULT_OUT && a.el_name(s[1].0 as usize) == FLAG_NODE && a.in_str(s[1].0 as usize, "GameDataNameBool") == Some(FLAG)
        })
    })
}

pub fn patch(a: &mut Ainb) -> Res<usize> {
    let i: usize = gate(a).ok_or(Error::Format("kokiri ring: the adult and forest gate of the rock ring is missing, the game files differ"))?;
    let g: usize = a.find_in(i, BOOL, MULTI).ok_or(Error::Format("kokiri ring: the gate has no multi input"))?;
    let at: usize = g * BOOL_IN + 6;
    a.ins[BOOL][at..at + 2].copy_from_slice(&1i16.to_le_bytes());
    a.els[i][MULTI_COUNT..MULTI_COUNT + 4].copy_from_slice(&1u32.to_le_bytes());
    Ok(i)
}

pub fn patch_bytes(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    patch(&mut a)?;
    let out: Vec<u8> = a.write()?;
    let back: Ainb = Ainb::parse(&out)?;
    if back.write()? != out || !crate::ainb::audit(&back).is_empty() {
        return Err(Error::Format("kokiri ring: the patched module does not read back"));
    }
    Ok(out)
}

pub fn stage(rf: &Romfs) -> Res<Out> {
    let raw: Vec<u8> = rf.read(MODULE)?;
    let out: Vec<u8> = patch_bytes(&raw)?;
    Ok(Out { path: MODULE.to_string(), old: raw.len(), new: out.len(), z: out })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf as PB;

    const RING: [&str; 8] = ["4079", "4151", "6b1c", "79ff", "ab49", "b129", "e464", "f107"];
    const KEPT: [&str; 4] = ["1345", "39ec", "b2db", "f726"];

    fn real() -> Option<PB> {
        let p: PB = ootrr_game::test_romfs::path();
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Some(p)
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    fn rock(a: &Ainb, tail: &str) -> usize {
        let name: String = format!("Obj_LiftRock_A_01_{tail}");
        a.find_els("ActorLogicLiftableActor").into_iter().find(|&i| a.prop_str(i, "InstanceName") == Some(name.as_str())).unwrap()
    }

    fn read_by(a: &Ainb, i: usize, input: &str) -> (i16, i16) {
        let x = a.input(BOOL, a.find_in(i, BOOL, input).unwrap()).unwrap();
        (x.src, x.plug)
    }

    #[test]
    fn the_ring_follows_the_age_alone() {
        let Some(root) = real() else {
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        let old: Ainb = Ainb::parse(&rf.read(MODULE).unwrap()).unwrap();
        let g: usize = gate(&old).unwrap();
        for t in RING {
            assert_eq!(read_by(&old, rock(&old, t), "Logic_Delete"), (g as i16, 1), "{t}");
        }
        for t in KEPT {
            assert_eq!(read_by(&old, rock(&old, t), "Logic_Create"), (g as i16, 1), "{t}");
        }
        assert_eq!(old.els[g][MULTI_COUNT], 2);
        let o: Out = stage(&rf).unwrap();
        let new: Ainb = Ainb::parse(&o.z).unwrap();
        assert!(crate::ainb::audit(&new).is_empty());
        assert!(gate(&new).is_none());
        let s: Vec<(u16, u16)> = sources(&new, g).unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!((new.el_name(s[0].0 as usize), s[0].1), (AGE, ADULT_OUT));
        assert_eq!(new.els[g][MULTI_COUNT], 1);
        for t in RING {
            assert_eq!(read_by(&new, rock(&new, t), "Logic_Delete"), (g as i16, 1), "{t}");
        }
        for t in KEPT {
            assert_eq!(read_by(&new, rock(&new, t), "Logic_Create"), (g as i16, 1), "{t}");
        }
        assert_eq!(new.els.len(), old.els.len());
        for i in (0..old.els.len()).filter(|&i| i != g) {
            assert_eq!(old.els[i], new.els[i], "element {i}");
        }
    }
}
