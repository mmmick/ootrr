use crate::bfevfl::Bf;
use crate::error::{Error, Res};
use crate::qol::{edit_zs, Have};
use crate::romfs::Romfs;
use crate::staging::Out;

use super::guard::{self, Run};

pub const FIRE_FILE: &str = "Event/EventFlow/Get_StoneOfFire.bfevfl.zs";
pub const FIRE_FLOW: &str = "Get_StoneOfFire";
pub const DANCE_FILE: &str = "Event/EventFlow/Dm_Goron_0030.bfevfl.zs";
pub const DANCE_FLOW: &str = "Dm_Goron_0030";
pub const FIRE: &str = "Get_StoneOfFire";
pub const SWITCH: &str = "Event39";
pub const OCARINA_SWITCH: &str = "Event82";
pub const AFTER_SARIA_EXP: &str = "Get_StoneOfFire_IsAfter_SariaMelodyPlay_Exp";
pub const PLAYED: &str = "HavePlayedEvent.Dm_Goron_0030";
pub const BOOL_QUERY: &str = "EventQueryIsGameDataBool";
pub const STEP: &str = "DodongoCavernOpne";
pub const PREFIX: &str = "OOTRRDanceGuard";
pub const LATE: [(&str, &str, &str); 4] = [
    ("DarniaRoomKokiriTalk", "Get_StoneOfFire_IsAfter_DarniaRoomKokiriTalk_Exp", "OOTRRFireRoom"),
    ("SariaMelodyPlay", "Get_StoneOfFire_IsAfter_SariaMelodyPlay_Exp", "OOTRRFireSong"),
    ("DeathMtTrailNpc", "Get_StoneOfFire_IsAfter_DeathMtTrailNpc_Exp", "OOTRRFireTrail"),
    ("DeathMtTrailIn", "Get_StoneOfFire_IsAfter_DeathMtTrailIn_Exp", "OOTRRFireLetter"),
];

fn flow_edit(raw: &[u8], f: impl FnOnce(&mut Bf) -> Res<()>) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    f(&mut b)?;
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(Error::Format("darunia: the edited flow does not parse back"));
    }
    Ok(out)
}

pub fn requery_bytes(raw: &[u8]) -> Res<Vec<u8>> {
    flow_edit(raw, |b| {
        guard::requery(b, FIRE_FLOW, SWITCH, AFTER_SARIA_EXP, BOOL_QUERY, PLAYED)?;
        guard::requery(b, FIRE_FLOW, OCARINA_SWITCH, AFTER_SARIA_EXP, BOOL_QUERY, PLAYED)?;
        for (step, exp, prefix) in LATE {
            guard::guard_steps(b, FIRE_FLOW, FIRE, &[step], exp, Run::WhenFalse, prefix)?;
        }
        Ok(())
    })
}

pub fn guard_bytes(raw: &[u8]) -> Res<Vec<u8>> {
    flow_edit(raw, |b| guard::guard_steps(b, DANCE_FLOW, FIRE, &[STEP], AFTER_SARIA_EXP, Run::WhenFalse, PREFIX).map(|_| ()))
}

pub fn stage_late(rf: &mut Romfs, late: &mut Vec<Out>, work: &std::path::Path) -> Res<()> {
    crate::story_skip::stage_late_file(rf, late, work, FIRE_FILE, requery_bytes)?;
    crate::story_skip::stage_late_file(rf, late, work, DANCE_FILE, guard_bytes)
}

pub fn stage(rf: &mut Romfs, have: Have) -> Res<Vec<Out>> {
    Ok(vec![edit_zs(rf, have, FIRE_FILE, requery_bytes)?, edit_zs(rf, have, DANCE_FILE, guard_bytes)?])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::{Flow, Kind};
    use std::fs;
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

    fn flow_of<'a>(b: &'a Bf, n: &str) -> &'a Flow {
        b.flows.iter().find(|f| f.name == n).unwrap()
    }

    fn cases(f: &Flow, n: &str) -> Vec<(u32, String)> {
        let Kind::Switch { cases, .. } = &f.events.iter().find(|e| e.name == n).unwrap().kind else { panic!() };
        cases.iter().map(|c| (c.val, f.events[c.ev as usize].name.clone())).collect()
    }

    #[test]
    fn the_dance_follows_its_own_flag_and_only_writes_the_step_before_the_song_is_learned() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let outs: Vec<Out> = stage(&mut rf, &|_: &str| None).unwrap();
        assert_eq!(outs.iter().map(|o| o.path.as_str()).collect::<Vec<&str>>(), vec![FIRE_FILE, DANCE_FILE]);
        let fire_old: Bf = Bf::parse(&rf.read_zs(FIRE_FILE).unwrap()).unwrap();
        let fire_new: Bf = Bf::parse(&rf.zs.decompress(&outs[0].z).unwrap()).unwrap();
        let (a, b): (&Flow, &Flow) = (flow_of(&fire_old, FIRE_FLOW), flow_of(&fire_new, FIRE_FLOW));
        for n in [SWITCH, OCARINA_SWITCH] {
            assert_eq!(cases(a, n), cases(b, n));
            let e = b.events.iter().find(|e| e.name == n).unwrap();
            assert_eq!(guard::text(&e.kind, "GameDataName"), Some(PLAYED));
            let Kind::Switch { actor, query, .. } = &e.kind else { panic!() };
            assert_eq!(b.actors[*actor as usize].queries[*query as usize], BOOL_QUERY);
        }
        assert_eq!(b.events.len(), a.events.len() + LATE.len());
        let added: usize = a.events.len();
        let into_guard = |k: &Kind| -> bool {
            match k {
                Kind::Action { next, .. } | Kind::Sub { next, .. } | Kind::Join { next } => usize::from(*next) >= added,
                Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => cases.iter().any(|c| usize::from(c.ev) >= added),
                Kind::Fork { forks, .. } => forks.iter().any(|x| usize::from(*x) >= added),
                Kind::Label { .. } => false,
            }
        };
        for (x, y) in a.events.iter().zip(b.events.iter()).filter(|(x, y)| x != y) {
            assert!(x.name == SWITCH || x.name == OCARINA_SWITCH || into_guard(&y.kind), "{}", x.name);
        }
        for (step, exp, prefix) in LATE {
            let g: &crate::bfevfl::Event = b.events.iter().find(|e| e.name == format!("{prefix}0")).unwrap();
            assert_eq!(guard::text(&g.kind, "GameDataName"), Some(exp));
            let Kind::Switch { cases, .. } = &g.kind else { panic!() };
            let w: &crate::bfevfl::Event = &b.events[cases.iter().find(|c| c.val == 1).unwrap().ev as usize];
            assert_eq!(guard::step_of(b, w, FIRE), Some(step));
        }
        let dance_old: Bf = Bf::parse(&rf.read_zs(DANCE_FILE).unwrap()).unwrap();
        let dance_new: Bf = Bf::parse(&rf.zs.decompress(&outs[1].z).unwrap()).unwrap();
        let (c, d): (&Flow, &Flow) = (flow_of(&dance_old, DANCE_FLOW), flow_of(&dance_new, DANCE_FLOW));
        assert_eq!(d.events.len(), c.events.len() + 1);
        assert_eq!(d.actors.len(), c.actors.len() + 1);
        assert_eq!(cases(d, "OOTRRDanceGuard0"), vec![(0, "Event128".to_string()), (1, "Event121".to_string())]);
        let steps = |f: &Flow| -> Vec<String> { f.events.iter().filter_map(|e| guard::step_of(f, e, FIRE).map(|s| s.to_string())).collect() };
        assert_eq!(steps(c), steps(d));
        let g: &crate::bfevfl::Event = d.events.last().unwrap();
        assert_eq!(guard::text(&g.kind, "GameDataName"), Some(AFTER_SARIA_EXP));
        let out: PB = PB::from(env!("CARGO_MANIFEST_DIR")).join("../../target/darunia_flow/Event/EventFlow");
        fs::create_dir_all(&out).unwrap();
        for o in &outs {
            fs::write(out.join(o.path.rsplit('/').next().unwrap()), &o.z).unwrap();
        }
        assert!(stage(&mut rf, &|p: &str| outs.iter().find(|o| o.path == p).map(|o| o.z.clone())).is_err());
    }

    fn edited_flows(root: &PB) -> std::collections::HashMap<String, Flow> {
        let mut rf: Romfs = Romfs::open(root).unwrap();
        let outs: Vec<Out> = stage(&mut rf, &|_: &str| None).unwrap();
        outs.iter()
            .flat_map(|o| Bf::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap().flows)
            .map(|f| (f.name.clone(), f))
            .collect()
    }

    fn scene(entry: &str, set: &[(&str, &str)]) -> crate::flow_extra::sim::Scene {
        crate::flow_extra::sim::Scene {
            flow: FIRE_FLOW.to_string(),
            entry: entry.to_string(),
            set: set.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            own: Vec::new(),
        }
    }

    fn runs(root: &PB, flows: &std::collections::HashMap<String, Flow>, sc: &crate::flow_extra::sim::Scene) -> Vec<crate::flow_sim::run::Fin> {
        let mut s = crate::flow_extra::sim::sim(root, flows.clone()).unwrap();
        crate::flow_extra::sim::run(&mut s, sc).unwrap()
    }

    #[test]
    fn darunia_dances_after_dodongos_cavern_and_late_events_never_move_the_fire_quest_back() {
        let Some(root) = real() else {
            return;
        };
        let edited = edited_flows(&root);
        let vanilla = std::collections::HashMap::new();
        let dances = |fins: &[crate::flow_sim::run::Fin]| fins.iter().any(|f| f.m.trans.iter().any(|t| t == DANCE_FLOW));
        for step in ["GoronCityDarniaTalk", "DodongoCavernOpne", "DungeonStart", "BossBattle", "EscapeDungeon"] {
            let fresh = scene("Get_StoneOfFire_PlayOcarina", &[("Step_Get_StoneOfFire", step), (PLAYED, "false")]);
            assert!(dances(&runs(&root, &edited, &fresh)), "{step}");
            assert_eq!(dances(&runs(&root, &vanilla, &fresh)), step == "GoronCityDarniaTalk", "{step}");
            let done = scene("Get_StoneOfFire_PlayOcarina", &[("Step_Get_StoneOfFire", step), (PLAYED, "true")]);
            assert!(!dances(&runs(&root, &edited, &done)), "{step}");
        }
        let moved = |fins: &[crate::flow_sim::run::Fin]| fins.iter().any(|f| f.m.writes.contains_key("Step_Get_StoneOfFire"));
        for entry in ["Get_StoneOfFire_MelodyGet_Saria", "Get_StoneOfFire_DeathMtTrailIn", "Get_StoneOfFire_KakarikoVillageLetter", "Get_StoneOfFire_MeetDarnia"] {
            let late = scene(entry, &[("Step_Get_StoneOfFire", "DungeonStart"), ("Sequence_IsChildhood", "true")]);
            assert!(moved(&runs(&root, &vanilla, &late)), "{entry}");
            assert!(!moved(&runs(&root, &edited, &late)), "{entry}");
            let early = scene(entry, &[("Step_Get_StoneOfFire", "Ready"), ("Sequence_IsChildhood", "true")]);
            assert_eq!(moved(&runs(&root, &edited, &early)), moved(&runs(&root, &vanilla, &early)), "{entry}");
        }
    }
}
