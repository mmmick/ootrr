use crate::ainb::Ainb;
use crate::error::{Error, Res};
use crate::gerudo_card::{area_of, bool_node, multi_node, rewire, CREATE, FALSE_OUT, OR, STEP_CHECK};
use crate::qol::{edit_plain, Have};
use crate::romfs::Romfs;
use crate::staging::Out;
use crate::story_skip::MALON_GIVEN;

pub const MODULE: &str = "Logic/LonLonRanch_Child_NoTerrain_Prefab_5286.logic.module.ainb";
pub const AREA: &str = "Area_9a63";
pub const QUEST: &str = "Work/Challenge/Get_Epona.game__challenge__Challenge.gyml";
pub const STEP: &str = "ReadyForGetEponaSong";
pub const IS_EQUAL: i16 = 2;

pub fn step_gate(a: &Ainb) -> Option<(usize, usize, i16)> {
    let (i, g): (usize, usize) = area_of(a, AREA)?;
    let x = a.input(1, g)?;
    if x.src < 0 || x.plug != IS_EQUAL {
        return None;
    }
    let s: usize = x.src as usize;
    (a.el_name(s) == STEP_CHECK && a.in_str(s, "ChallengeName") == Some(QUEST) && a.in_str(s, "ChallengeStepName") == Some(STEP)).then_some((i, g, x.src))
}

pub fn patch(a: &mut Ainb) -> Res<usize> {
    let (i, g, src): (usize, usize, i16) = step_gate(a).ok_or(Error::Format("malon song: the song area is not made by the Epona quest step, the game files differ"))?;
    let flag: usize = bool_node(a, MALON_GIVEN, "ootrr malon song given read")?;
    let gate: usize = multi_node(a, OR, "ootrr malon song not given", &[(flag, 0)])?;
    let create: u32 = a.pool.add(CREATE);
    rewire(a, i, g, src, create, gate, FALSE_OUT as i16)?;
    Ok(gate)
}

pub fn patch_bytes(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    patch(&mut a)?;
    let out: Vec<u8> = a.write()?;
    let back: Ainb = Ainb::parse(&out)?;
    if back.write()? != out || !crate::ainb::audit(&back).is_empty() {
        return Err(Error::Format("malon song: the patched module does not read back"));
    }
    Ok(out)
}

pub fn stage(rf: &Romfs, have: Have) -> Res<Out> {
    edit_plain(rf, have, MODULE, patch_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gerudo_card::{queries, BOOL, GET_BOOL, MULTI, MULTI_COUNT};
    use std::path::PathBuf as PB;

    fn real() -> Option<PB> {
        let p: PB = ootrr_game::test_romfs::path();
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Some(p)
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    #[test]
    fn the_song_area_is_made_until_the_song_is_given_whatever_the_epona_quest_says() {
        let Some(root) = real() else {
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        let old: Ainb = Ainb::parse(&rf.read(MODULE).unwrap()).unwrap();
        assert!(step_gate(&old).is_some());
        let new: Ainb = Ainb::parse(&stage(&rf, &|_: &str| None).unwrap().z).unwrap();
        assert!(crate::ainb::audit(&new).is_empty());
        assert!(step_gate(&new).is_none());
        let (i, g) = area_of(&new, AREA).unwrap();
        let x = new.input(BOOL, g).unwrap();
        assert_eq!((new.el_name(x.src as usize), x.plug), (OR, FALSE_OUT as i16));
        let gate: usize = x.src as usize;
        let m = new.input(BOOL, new.find_in(gate, BOOL, MULTI).unwrap()).unwrap();
        assert_eq!(m.plug, 1);
        let start: usize = (-100 - i32::from(m.src)) as usize;
        let (flag, out) = crate::door_open::multi_entry(&new, start).unwrap();
        assert_eq!((new.el_name(flag as usize), out), (GET_BOOL, 0));
        assert_eq!(new.in_str(flag as usize, "GameDataNameBool"), Some(MALON_GIVEN));
        assert_eq!(new.els[gate][MULTI_COUNT], 1);
        assert!(queries(&new, i).contains(&(gate as u16)));
        for e in 0..old.els.len() {
            if e == i {
                continue;
            }
            for k in 0..6 {
                for (a, b) in old.input_ids(e, k).into_iter().zip(new.input_ids(e, k)) {
                    assert_eq!(old.input(k, a).map(|x| (x.src, x.plug)), new.input(k, b).map(|x| (x.src, x.plug)), "element {e}");
                }
            }
        }
    }

    #[test]
    fn the_malon_song_give_marks_the_song_as_given() {
        let Some(root) = real() else {
            return;
        };
        use crate::bfevfl::{Bf, Kind};
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let t = crate::story_skip::SONG_TRIGGERS.iter().find(|t| t.loc == "Song from Malon").unwrap();
        let writes = crate::story_skip::scene_writes(&mut rf, t.give).unwrap();
        let gives = vec![crate::story_skip::SceneGive { actor: "Item_Bottle".to_string(), family: None, key: None }];
        let mut b: Bf = Bf::parse(&rf.read_zs(&format!("Event/EventFlow/{}.bfevfl.zs", t.flow)).unwrap()).unwrap();
        crate::story_skip::patch_song_trigger(&mut b, t, &writes, &gives).unwrap();
        let f = b.flows.iter().find(|x| x.name == t.flow).unwrap();
        let marks: usize = f
            .events
            .iter()
            .filter(|e| matches!(&e.kind, Kind::Action { params: Some(c), .. } if c.get("GameDataName") == Some(&crate::bfevfl::Val::Str(MALON_GIVEN.to_string()))))
            .count();
        assert_eq!(marks, 1);
        for other in crate::story_skip::SONG_TRIGGERS.iter().filter(|x| x.loc != "Song from Malon") {
            assert_eq!(crate::story_skip::given_mark(other.loc), None);
        }
    }
}
