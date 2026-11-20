use std::path::Path as P;

use crate::bfevfl::{Bf, Kind, Val};
use crate::error::{Error, Res};
use crate::romfs::Romfs;
use crate::settings::Settings;
use crate::staging::Out;
use crate::start_defaults::Edit;

pub const SKIP: &str = "skip_ganon_trials";
pub const COUNT: &str = "trials";
pub const NAMES: [&str; 6] = ["Forest", "Fire", "Water", "Shadow", "Spirit", "Light"];
pub const LOGIC: [&str; 7] = ["0", "1", "2", "3", "4", "5", "6"];

pub const REMOVED: &str = "InsideGanonCastle_Dungeon_BarrierRemove_";
pub const PLAYED: &str = "HavePlayedEvent.InsideGanonCastle_Dungeon_BarrierRemove_";
pub const LEFT: &str = "InsideGanonCastle_Dungeon_Barrier_Count";
pub const GUIDE: &str = "InsideGanonCastle_Dungeon_SeeBarrierGuide";
pub const ALL_CLEAR: [&str; 3] = [
    "InsideGanonCastle_IsAllBarrierClear",
    "HavePlayedEvent.InsideGanonCastle_Dungeon_BarrierRemove_All",
    "HavePlayedEvent.Dm_InsideGanonCastle_0090",
];

pub const FLOW_FILE: &str = "Event/EventFlow/Defeat_Ganon.bfevfl.zs";
pub const FLOW: &str = "Defeat_Ganon";
pub const LAST_CHECK: &str = "Event606";
pub const BELOW: u32 = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trials {
    pub required: usize,
}

impl Default for Trials {
    fn default() -> Self {
        Trials { required: 6 }
    }
}

pub fn read(s: &Settings, bad: &mut Vec<String>) -> Trials {
    let skip: bool = s.flag(SKIP).unwrap_or_else(|e| {
        bad.push(e.to_string());
        false
    });
    let n: i64 = if skip {
        0
    } else {
        s.int(COUNT).unwrap_or_else(|e| {
            bad.push(e.to_string());
            6
        })
    };
    Trials { required: n.clamp(0, 6) as usize }
}

impl Trials {
    pub fn some(&self) -> bool {
        self.required > 0 && self.required < 6
    }

    pub fn logic(&self) -> Vec<(&'static str, &'static str)> {
        if self.required == 6 {
            return Vec::new();
        }
        vec![(COUNT, LOGIC[self.required])]
    }

    pub fn edits(&self) -> Vec<Edit> {
        let mut out: Vec<Edit> = Vec::new();
        if self.required == 6 {
            return out;
        }
        out.push(Edit::Int(LEFT.to_string(), self.required as i32));
        if self.required > 0 {
            return out;
        }
        for t in NAMES {
            out.push(Edit::Bool(format!("{REMOVED}{t}"), true));
            out.push(Edit::Bool(format!("{PLAYED}{t}"), true));
        }
        out.push(Edit::Bool(GUIDE.to_string(), true));
        out.extend(ALL_CLEAR.iter().map(|f| Edit::Bool(f.to_string(), true)));
        out
    }

    pub fn note(&self) -> Option<String> {
        match self.required {
            6 => None,
            0 => Some("Ganon's Trials are skipped: the barrier in Ganon's Castle starts dispelled and the tower is open.".to_string()),
            1 => Some("Ganon's Trials: clearing any one of the six trials dispels the barrier.".to_string()),
            n => Some(format!("Ganon's Trials: clearing any {n} of the six trials dispels the barrier.")),
        }
    }

    pub fn stage(&self, rf: &mut Romfs, more: &mut Vec<Out>, work: &P) -> Res<()> {
        if !self.some() {
            return Ok(());
        }
        let have = |p: &str| -> Option<Vec<u8>> { std::fs::read(p.split('/').fold(work.to_path_buf(), |a, s| a.join(s))).ok() };
        more.push(crate::qol::edit_zs(rf, &have, FLOW_FILE, flow_bytes)?);
        Ok(())
    }
}

pub fn patch_flow(b: &mut Bf) -> Res<()> {
    let f = b.flows.iter_mut().find(|f| f.name == FLOW).ok_or(Error::Format("ganon trials: Defeat_Ganon has no Defeat_Ganon flow"))?;
    let e = f.events.iter_mut().find(|e| e.name == LAST_CHECK).ok_or(Error::Format("ganon trials: Defeat_Ganon has no barrier count check"))?;
    match &mut e.kind {
        Kind::Switch { cases, params: Some(p), .. }
            if p.items.iter().any(|(k, v)| k == "GameDataName" && *v == Val::Str(LEFT.to_string()))
                && cases.len() == 2
                && cases.iter().any(|c| c.val == BELOW) =>
        {
            cases.retain(|c| c.val != BELOW);
            Ok(())
        }
        _ => Err(Error::Format("ganon trials: Event606 is not the barrier count check, the game files differ")),
    }
}

pub fn flow_bytes(raw: &[u8]) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    patch_flow(&mut b)?;
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(Error::Format("ganon trials: the patched flow does not parse back"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Value;

    fn with(skip: bool, n: i64) -> Trials {
        let mut s: Settings = Settings::default();
        s.set(SKIP, Value::Bool(skip)).unwrap();
        s.set(COUNT, Value::Int(n)).unwrap();
        let mut bad: Vec<String> = Vec::new();
        let t: Trials = read(&s, &mut bad);
        assert!(bad.is_empty(), "{bad:?}");
        t
    }

    #[test]
    fn the_default_needs_all_six_and_changes_nothing() {
        let mut bad: Vec<String> = Vec::new();
        let t: Trials = read(&Settings::default(), &mut bad);
        assert!(bad.is_empty());
        assert_eq!(t, Trials::default());
        assert!(!t.some() && t.logic().is_empty() && t.edits().is_empty() && t.note().is_none());
    }

    #[test]
    fn skip_clears_every_trial_whatever_the_count() {
        let t: Trials = with(true, 4);
        assert_eq!(t.required, 0);
        assert_eq!(t.logic(), vec![(COUNT, "0")]);
        let e: Vec<Edit> = t.edits();
        assert!(e.contains(&Edit::Int(LEFT.to_string(), 0)));
        assert!(e.contains(&Edit::Bool(ALL_CLEAR[0].to_string(), true)));
        assert_eq!(e.len(), 1 + 12 + 1 + 3);
        assert_eq!(with(false, 0), t);
        assert!(!t.some());
    }

    #[test]
    fn a_set_number_only_lowers_the_barrier_count() {
        let t: Trials = with(false, 2);
        assert!(t.some());
        assert_eq!(t.logic(), vec![(COUNT, "2")]);
        assert_eq!(t.edits(), vec![Edit::Int(LEFT.to_string(), 2)]);
        assert!(t.note().unwrap().contains("any 2"));
    }

    fn tower(t: &Trials, gone: &[&str]) -> bool {
        let d: crate::slice::data::Data = crate::slice::data::Data::load().unwrap();
        let w: crate::world::World = crate::slice::logic::world_with(&d, &t.logic()).unwrap();
        let mut p: crate::world::Placement = crate::slice::fill::vanilla_place(&w, &d).unwrap();
        let junk: crate::logic_items::ItemId = w.item("Rupees (5)").unwrap();
        for l in w.real.clone() {
            if gone.contains(&w.locs[l].name.as_str()) || p.get(l).is_some_and(|i| gone.contains(&w.items.names[i].as_str())) {
                p.put(l, junk);
            }
        }
        crate::search::explore(&w, &p, &[]).region_reached(w.region("Ganons Castle Tower").unwrap(), None)
    }

    #[test]
    fn the_tower_needs_any_n_trials() {
        assert!(tower(&Trials::default(), &[]));
        assert!(!tower(&Trials::default(), &["Light Arrows"]));
        assert!(tower(&with(true, 6), &["Light Arrows"]));
        let no_gold: &[&str] = &["Ganons Castle Shadow Trial Golden Gauntlets Chest"];
        assert!(!tower(&Trials::default(), no_gold));
        assert!(!tower(&with(false, 5), no_gold));
        assert!(tower(&with(false, 4), no_gold));
        assert!(!tower(&with(false, 1), &["Light Arrows"]));
    }

    #[test]
    fn the_flow_patch_drops_only_the_below_zero_case() {
        let root: std::path::PathBuf = ootrr_game::test_romfs::path();
        if !root.join("Pack").join("ZsDic.pack.zs").is_file() {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", root.display());
            return;
        }
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read_zs(FLOW_FILE).unwrap();
        let out: Vec<u8> = flow_bytes(&raw).unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        let f = b.flows.iter().find(|f| f.name == FLOW).unwrap();
        let e = f.events.iter().find(|e| e.name == LAST_CHECK).unwrap();
        let Kind::Switch { cases, .. } = &e.kind else { panic!("not a switch") };
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].val, 1);
        assert!(flow_bytes(&out).is_err());
    }
}
