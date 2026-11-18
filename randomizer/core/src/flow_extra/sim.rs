use std::collections::{BTreeMap as BM, HashMap as HM};
use std::path::Path as P;

use crate::bfevfl::{Bf, Flow};
use crate::error::{Error as E, Res};
use crate::flow_extra::apply::{edit_flow, ops_for};
use crate::flow_extra::op::Op;
use crate::flow_locations::{plan, Change, Choice, Data, Opts as FlowOpts, Req};
use crate::flow_sim::db::Db;
use crate::flow_sim::load::{Disk, Src};
use crate::flow_sim::run::{lines, End, Env, Fin, Mach, Opts, Sim};
use crate::flow_sim::state::{State, V};
use crate::romfs::Romfs;

pub struct Over {
    pub disk: Disk,
    pub flows: HM<String, Flow>,
}

impl Src for Over {
    fn flow(&mut self, name: &str) -> Res<Option<Flow>> {
        match self.flows.get(name) {
            Some(f) => Ok(Some(f.clone())),
            None => self.disk.flow(name),
        }
    }

    fn event(&mut self, name: &str) -> Res<Option<(String, String)>> {
        self.disk.event(name)
    }

    fn cuts(&mut self, name: &str) -> Vec<String> {
        self.disk.cuts(name)
    }
}

pub fn stem(file: &str) -> String {
    file.trim_end_matches(".bfevfl").to_string()
}

pub fn edited(rf: &mut Romfs, location: &str, item: &str) -> Res<HM<String, Flow>> {
    edited_many(rf, &[Choice { location: location.to_string(), item: item.to_string() }])
}

pub fn edited_many(rf: &mut Romfs, cs: &[Choice]) -> Res<HM<String, Flow>> {
    let d: Data = Data::load()?;
    let (changes, _, _): (Vec<Change>, usize, Vec<String>) = plan(&d, cs, &FlowOpts::default())?;
    let mut rs: BM<String, Vec<Req>> = BM::new();
    for c in changes.iter().filter(|c| crate::flow_extra::trade::plain(c)) {
        rs.entry(c.file.clone()).or_default().push(Req {
            index: c.index,
            event: c.event.clone(),
            param: c.param.clone(),
            old: c.old.clone(),
            new: c.new.clone(),
        });
    }
    let os: BM<String, Vec<Op>> = ops_for(&changes)?;
    let mut files: Vec<&String> = rs.keys().chain(os.keys()).collect();
    files.sort();
    files.dedup();
    let mut out: HM<String, Flow> = HM::new();
    for file in files {
        let raw: Vec<u8> = crate::cow_clone::read(rf, file)?.0;
        let r: &[Req] = rs.get(file).map(|v| v.as_slice()).unwrap_or(&[]);
        let o: &[Op] = os.get(file).map(|v| v.as_slice()).unwrap_or(&[]);
        let b: Bf = edit_flow(&raw, r, o)?;
        let back: Bf = Bf::parse(&b.write()?)?;
        if back != b {
            return Err(E::Msg(format!("{file} does not survive a byte round trip after the edit")));
        }
        let mut b: Bf = back;
        let f: Flow = b.flows.drain(..).next().ok_or(E::Format("flow extra: the edited file holds no flow"))?;
        out.insert(stem(file), f);
    }
    Ok(out)
}

#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub flow: String,
    pub entry: String,
    pub set: Vec<(String, String)>,
    pub own: Vec<(String, i64)>,
}

pub fn sim(root: &P, flows: HM<String, Flow>) -> Res<Sim> {
    let mut rf: Romfs = Romfs::open(root)?;
    let db: Db = Db::load(&mut rf)?;
    let src: Over = Over { disk: Disk::new(Romfs::open(root)?, None, Vec::new()), flows };
    Ok(Sim::new(db, Box::new(src), Opts::default()))
}

pub fn state(s: &mut Sim, sc: &Scene) -> Res<State> {
    let mut st: State = State::new();
    for (k, v) in &sc.set {
        let h: u32 = s.db.reg(k);
        let x: V = s.db.parse(h, v)?;
        st.v.insert(h, x);
    }
    for (k, n) in &sc.own {
        s.own(&mut st, k, *n);
    }
    s.settle(&mut st);
    Ok(st)
}

pub fn run_from(s: &mut Sim, sc: &Scene, st: State) -> Res<Vec<Fin>> {
    let m: Mach = s.start(&sc.flow, &sc.entry, st, Env::new())?;
    Ok(s.explore(m))
}

pub fn run(s: &mut Sim, sc: &Scene) -> Res<Vec<Fin>> {
    let st: State = state(s, sc)?;
    run_from(s, sc, st)
}

pub fn run_args(s: &mut Sim, sc: &Scene, args: &[(&str, &str)]) -> Res<Vec<Fin>> {
    let st: State = state(s, sc)?;
    let mut env: Env = Env::new();
    for (k, v) in args {
        env.insert(k.to_string(), crate::bfevfl::Val::Str(v.to_string()));
    }
    let m: Mach = s.start(&sc.flow, &sc.entry, st, env)?;
    Ok(s.explore(m))
}

pub fn beyond_always(fl: &HM<String, Flow>) -> Vec<String> {
    let own: Vec<String> = crate::flow_extra::always_ops().map(|m| m.keys().map(|k| stem(k)).collect()).unwrap_or_default();
    fl.keys().filter(|k| !own.contains(k)).cloned().collect()
}

pub fn granted(f: &Fin, item: &str) -> i64 {
    f.m.gr.get(item).copied().unwrap_or(0)
}

pub fn called(f: &Fin, what: &str) -> bool {
    lines(&f.m.log).iter().any(|(_, t)| t.contains(&format!("call {what}")))
}

pub fn broken(fins: &[Fin]) -> usize {
    fins.iter().filter(|f| matches!(f.end, End::Loop(_) | End::Fail(_) | End::Limit)).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf as PB;

    const NEW: &str = "Item_Longshot";

    fn root() -> Option<PB> {
        let p: PB = ootrr_game::test_romfs::path();
        if p.join("Event/EventFlow").is_dir() {
            Some(p)
        } else {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            None
        }
    }

    fn scene(flow: &str, entry: &str, set: &[(&str, &str)], own: &[(&str, i64)]) -> Scene {
        Scene {
            flow: flow.to_string(),
            entry: entry.to_string(),
            set: set.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            own: own.iter().map(|(k, n)| (k.to_string(), *n)).collect(),
        }
    }

    fn both(r: &P, loc: &str, sc: &Scene) -> (Vec<Fin>, Vec<Fin>) {
        let mut rf: Romfs = Romfs::open(r).unwrap();
        let fl: HM<String, Flow> = edited(&mut rf, loc, NEW).unwrap();
        assert!(!fl.is_empty(), "{loc}");
        let mut a: Sim = sim(r, HM::new()).unwrap();
        let mut b: Sim = sim(r, fl).unwrap();
        (run(&mut a, sc).unwrap(), run(&mut b, sc).unwrap())
    }

    fn max(fins: &[Fin], item: &str) -> i64 {
        fins.iter().filter(|f| f.end == End::Done).map(|f| granted(f, item)).max().unwrap_or(0)
    }

    fn min(fins: &[Fin], item: &str) -> i64 {
        fins.iter().map(|f| granted(f, item)).min().unwrap_or(0)
    }

    #[test]
    fn the_vanilla_item_leaves_every_gated_flow_untouched() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        for (loc, actor) in [
            ("Kak 20 Gold Skulltula Reward", "Item_AgonyStone"),
            ("DMT Biggoron", "Item_Sword_Biggoron"),
            ("GC Medigoron", "Item_Sword_Giant"),
        ] {
            assert!(beyond_always(&edited(&mut rf, loc, actor).unwrap()).is_empty(), "{loc}");
        }
        let tiered: HM<String, Flow> = edited(&mut rf, "GC Rolling Goron as Child", "Item_BombBag_Medium").unwrap();
        assert_eq!(beyond_always(&tiered).len(), 1);
    }

    #[test]
    fn every_gated_edit_applies_to_the_real_files_and_round_trips() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let x: crate::flow_extra::Extra = crate::flow_extra::Extra::load().unwrap();
        for s in &x.specs {
            let fl: HM<String, Flow> = edited(&mut rf, &s.name, NEW).unwrap();
            for o in &s.ops {
                assert!(fl.contains_key(&stem(&o.file)), "{} {}", s.name, o.file);
            }
        }
    }

    #[test]
    fn kak_20_gives_the_new_item_once_and_skips_the_agony_guide() {
        let Some(r) = root() else { return };
        let sc: Scene = scene(
            "GatheringTokenGold",
            "Reward20",
            &[("KakarikoVillage_GatheringTokenGold_GetReward10", "true"), ("KakarikoVillage_GatheringTokenGold_GetReward20", "false")],
            &[("Item_Token_Gold", 20)],
        );
        let (a, b): (Vec<Fin>, Vec<Fin>) = both(&r, "Kak 20 Gold Skulltula Reward", &sc);
        assert_eq!(max(&a, "Item_AgonyStone"), 1);
        assert!(a.iter().any(|f| called(f, "ActionGuide:OpenActionGuideSceen")));
        assert_eq!(max(&b, NEW), 1);
        assert_eq!(max(&b, "Item_AgonyStone"), 0);
        assert!(!b.iter().any(|f| called(f, "ActionGuide:OpenActionGuideSceen")));
        assert!(b.iter().filter(|f| granted(f, NEW) == 1).all(|f| f.m.writes.get("KakarikoVillage_GatheringTokenGold_GetReward20").map(|s| s.as_str()) == Some("true")));
        assert_eq!(broken(&b), broken(&a));
        let done: Scene = scene("GatheringTokenGold", "Reward20", &[("KakarikoVillage_GatheringTokenGold_GetReward20", "true")], &[("Item_Token_Gold", 20)]);
        let (_, again): (Vec<Fin>, Vec<Fin>) = both(&r, "Kak 20 Gold Skulltula Reward", &done);
        assert_eq!(max(&again, NEW), 0);
        let hundred: Scene = scene("GatheringTokenGold", "Reward100", &[], &[("Item_Token_Gold", 100)]);
        let (va, vb): (Vec<Fin>, Vec<Fin>) = both(&r, "Kak 20 Gold Skulltula Reward", &hundred);
        assert!(va.iter().any(|f| called(f, "ActionGuide:OpenActionGuideSceen")));
        assert!(!vb.iter().any(|f| called(f, "ActionGuide:OpenActionGuideSceen")));
    }

    #[test]
    fn the_fortress_gatekeeper_opens_the_gate_without_moving_the_spirit_quest() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let mut fl: HM<String, Flow> = edited(&mut rf, "Kak 20 Gold Skulltula Reward", NEW).unwrap();
        fl.retain(|k, _| k == "Npc_Gerudo_006");
        assert_eq!(fl.len(), 1);
        let sc: Scene = scene("Npc_Gerudo_006", "Talk", &[("Step_SageOfSpirit", "Step01")], &[("Item_Token_Gerudo", 1)]);
        let text = |fins: &[Fin]| -> Vec<String> { fins.iter().filter(|f| f.end == End::Done).map(|f| lines(&f.m.log).into_iter().map(|(_, t)| t).collect::<Vec<_>>().join("\n")).collect() };
        let a: Vec<String> = text(&run(&mut sim(&r, HM::new()).unwrap(), &sc).unwrap());
        let b: Vec<String> = text(&run(&mut sim(&r, fl).unwrap(), &sc).unwrap());
        assert!(a.iter().any(|t| t.contains("request SageOfSpirit")), "{a:#?}");
        assert!(!b.is_empty());
        assert!(b.iter().all(|t| !t.contains("request SageOfSpirit")), "{b:#?}");
        assert!(b.iter().any(|t| t.contains("transition to event Dm_Gerudo_0030")), "{b:#?}");
    }

    #[test]
    fn rolling_goron_gives_the_item_without_a_bomb_bag() {
        let Some(r) = root() else { return };
        let sc: Scene = scene("Npc_Goron_006_Child", "Talk", &[("GoronCity_StopGoron006", "true"), ("GoronCity_GetBombBag_fromGoron006", "false")], &[]);
        let (a, b): (Vec<Fin>, Vec<Fin>) = both(&r, "GC Rolling Goron as Child", &sc);
        assert_eq!(max(&a, "Item_BombBag_Medium") + max(&a, "Item_BombBag_Large"), 0);
        assert_eq!(max(&b, NEW), 1);
        assert!(b.iter().filter(|f| granted(f, NEW) == 1).all(|f| f.m.writes.contains_key("GoronCity_GetBombBag_fromGoron006")));
        let bag: Scene = scene("Npc_Goron_006_Child", "Talk", &[("GoronCity_StopGoron006", "true")], &[("Item_BombBag_Small", 1)]);
        let (a, b): (Vec<Fin>, Vec<Fin>) = both(&r, "GC Rolling Goron as Child", &bag);
        assert_eq!(max(&a, "Item_BombBag_Medium"), 1);
        assert_eq!(max(&b, NEW), 1);
        assert_eq!(max(&b, "Item_BombBag_Medium"), 0);
        assert_eq!(broken(&b), broken(&a));
    }

    #[test]
    fn biggoron_keeps_the_knife_and_skips_the_sword_equip_loop() {
        let Some(r) = root() else { return };
        let sc: Scene = scene("Npc_Goron_Biggoron_Adult", "SubEvent_GetSwordBigGoron_Step11", &[("Step_SubEvent_GetSwordBigGoron", "Step11")], &[("Weapon_Sword_Giant", 1)]);
        let (a, b): (Vec<Fin>, Vec<Fin>) = both(&r, "DMT Biggoron", &sc);
        assert_eq!(min(&a, "Weapon_Sword_Giant"), -1);
        assert!(a.iter().any(|f| called(f, "Dm_SY_GetInEvent:GetInEventItemSwordBiggoron")));
        assert_eq!(min(&b, "Weapon_Sword_Giant"), 0);
        assert_eq!(max(&b, NEW), 1);
        assert_eq!(max(&b, "Item_Sword_Biggoron"), 0);
        assert!(b.iter().any(|f| called(f, "Dm_SY_GetInEvent:GetInEvent_NotTurnForPosCamera_BigGoron")));
        assert!(!b.iter().any(|f| called(f, "Dm_SY_GetInEvent:GetInEventItemSwordBiggoron")));
        assert!(b.iter().filter(|f| granted(f, NEW) == 1).all(|f| f.m.reqs.iter().any(|q| q.starts_with("SubEvent_GetSwordBigGoron -> Complete"))));
        assert!(broken(&b) <= broken(&a));
        let broken_knife: Scene = scene("Npc_Goron_Biggoron_Adult", "SubEvent_GetSwordBigGoron_Step11", &[("Step_SubEvent_GetSwordBigGoron", "Step11")], &[("Weapon_Sword_BrokenGiant", 1)]);
        let (_, b): (Vec<Fin>, Vec<Fin>) = both(&r, "DMT Biggoron", &broken_knife);
        assert_eq!(min(&b, "Weapon_Sword_BrokenGiant"), 0);
        assert_eq!(max(&b, NEW), 1);
    }

    #[test]
    fn the_din_fountain_never_waits_on_a_magic_gauge_for_another_item() {
        let Some(r) = root() else { return };
        let flag: &str = "GreatFairy_HasNoMPOnDinDemo";
        let gauge = |s: &Sim| s.assumed.keys().any(|k| k.contains("MPGauge"));
        for mp in ["0", "240"] {
            let sc: Scene = scene("Dm_FairyFountain_Din", "FirstTime_Din", &[("PlayerStatus.CurrentRupee", "100"), ("PlayerStatus.MPMax", mp)], &[]);
            let mut rf: Romfs = Romfs::open(&r).unwrap();
            let fl: HM<String, Flow> = edited(&mut rf, "DMT Great Fairy Reward", NEW).unwrap();
            let mut sa: Sim = sim(&r, HM::new()).unwrap();
            let mut sb: Sim = sim(&r, fl).unwrap();
            let a: Vec<Fin> = run(&mut sa, &sc).unwrap();
            let b: Vec<Fin> = run(&mut sb, &sc).unwrap();
            let gave = |v: &[Fin]| -> Vec<usize> { v.iter().enumerate().filter(|(_, f)| f.end == End::Done && granted(f, "Item_DinFire") + granted(f, NEW) > 0).map(|(i, _)| i).collect() };
            assert!(!gave(&a).is_empty() && !gave(&b).is_empty(), "{mp}");
            assert!(gave(&a).iter().all(|i| granted(&a[*i], "Item_DinFire") == 1 && granted(&a[*i], NEW) == 0), "{mp}");
            assert!(gave(&b).iter().all(|i| granted(&b[*i], NEW) == 1 && granted(&b[*i], "Item_DinFire") == 0), "{mp}");
            assert_eq!(a.iter().any(|f| f.m.writes.get(flag).map(|s| s.as_str()) == Some("true")), mp == "0", "{mp}");
            assert_eq!(gauge(&sa), mp == "0", "{mp}");
            assert!(b.iter().all(|f| !f.m.writes.contains_key(flag)), "{mp}");
            assert!(!gauge(&sb), "{mp}");
            assert_eq!(broken(&b), broken(&a), "{mp}");
        }
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let vanilla: HM<String, Flow> = edited(&mut rf, "DMT Great Fairy Reward", "Item_DinFire").unwrap();
        assert!(beyond_always(&vanilla).is_empty());
        assert!(vanilla.contains_key("Dm_FairyFountain_Din"));
        for mp in ["0", "240"] {
            let sc: Scene = scene("Dm_FairyFountain_Din", "FirstTime_Din", &[("PlayerStatus.CurrentRupee", "100"), ("PlayerStatus.MPMax", mp)], &[]);
            let mut s: Sim = sim(&r, vanilla.clone()).unwrap();
            let fins: Vec<Fin> = run(&mut s, &sc).unwrap();
            let gave: Vec<&Fin> = fins.iter().filter(|f| f.end == End::Done && granted(f, "Item_DinFire") == 1).collect();
            assert!(!gave.is_empty(), "{mp}");
            assert!(fins.iter().all(|f| !f.m.writes.contains_key(flag)), "{mp}");
            assert!(!gauge(&s), "{mp}");
            let mut base: Sim = sim(&r, HM::new()).unwrap();
            assert_eq!(broken(&fins), broken(&run(&mut base, &sc).unwrap()), "{mp}");
        }
    }

    const SEVEN: [&str; 7] = ["Item_GlassTruth", "Item_DinFire", "Item_FaroreWind", "Item_NayruLove", "Item_Arrow_Fire", "Item_Arrow_Ice", "Item_Arrow_Light"];

    fn gauge_of(s: &Sim) -> bool {
        s.assumed.keys().any(|k| k.contains("MPGauge"))
    }

    #[test]
    fn the_wisdom_fountain_hands_over_any_item_without_a_magic_gauge() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        for entry in ["C15-01"] {
            let sc: Scene = scene("Dm_FairyFountain_Wisdom", entry, &[("PlayerStatus.CurrentRupee", "100"), ("PlayerStatus.MPMax", "0")], &[]);
            let mut base: Sim = sim(&r, HM::new()).unwrap();
            let a: Vec<Fin> = run(&mut base, &sc).unwrap();
            assert!(gauge_of(&base), "{entry}");
            for item in SEVEN {
                let fl: HM<String, Flow> = edited(&mut rf, "DMC Great Fairy Reward", item).unwrap();
                assert!(fl.contains_key("Dm_FairyFountain_Wisdom"), "{item}");
                let mut s: Sim = sim(&r, fl).unwrap();
                let b: Vec<Fin> = run(&mut s, &sc).unwrap();
                let gave: Vec<&Fin> = b.iter().filter(|f| f.end == End::Done && granted(f, item) == 1).collect();
                assert!(!gave.is_empty(), "{entry} {item}");
                assert!(b.iter().all(|f| granted(f, "Item_FairyBless_MPDouble") == 0), "{entry} {item}");
                assert!(!gauge_of(&s), "{entry} {item}");
                assert!(broken(&b) <= broken(&a), "{entry} {item}");
            }
        }
        let own: HM<String, Flow> = edited(&mut rf, "DMC Great Fairy Reward", "Item_FairyBless_MPDouble").unwrap();
        assert!(!own.contains_key("Dm_FairyFountain_Wisdom"));
    }

    #[test]
    fn the_pickup_flow_never_builds_a_gauge_that_no_meter_item_can_create() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let fl: HM<String, Flow> = edited(&mut rf, "DMT Great Fairy Reward", "Item_DinFire").unwrap();
        assert!(fl.contains_key("Dm_SY_Get"));
        for entry in ["Dm_SY_Get", "Default_Get", "PouchAddItem", "MPOpen"] {
            let sc: Scene = scene("Dm_SY_Get", entry, &[("PlayerStatus.MPMax", "0")], &[]);
            let mut base: Sim = sim(&r, HM::new()).unwrap();
            let a: Vec<Fin> = run_args(&mut base, &sc, &[("ActorName", "Obj_Arrow_Fire")]).unwrap();
            let mut s: Sim = sim(&r, fl.clone()).unwrap();
            for item in SEVEN.iter().copied().chain(["Obj_Arrow_Fire", "Armor_GlassTruth"]) {
                let b: Vec<Fin> = run_args(&mut s, &sc, &[("ActorName", item)]).unwrap();
                assert!(!b.is_empty(), "{entry} {item}");
                assert!(broken(&b) <= broken(&a), "{entry} {item}");
            }
            assert!(!gauge_of(&s), "{entry}");
            if entry == "MPOpen" {
                assert!(gauge_of(&base));
            }
        }
    }

    #[test]
    fn a_pickup_hands_over_the_improved_spin_to_a_player_who_holds_only_double_magic() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let fl: HM<String, Flow> = edited(&mut rf, "DMT Great Fairy Reward", "Item_DinFire").unwrap();
        assert!(fl.contains_key("Dm_SY_Get"));
        let cases: [(&[(&str, &str)], i64); 4] = [
            (&[("IsGet.Item_FairyBless_MPDouble", "true"), ("IsGet.Item_FairyBless_MP", "false")], 1),
            (&[("IsGet.Item_FairyBless_MPDouble", "true"), ("IsGet.Item_FairyBless_MP", "true")], 0),
            (&[("IsGet.Item_FairyBless_MPDouble", "false"), ("IsGet.Item_FairyBless_MP", "true")], 0),
            (&[("IsGet.Item_FairyBless_MPDouble", "false"), ("IsGet.Item_FairyBless_MP", "false")], 0),
        ];
        for (set, want) in cases {
            let sc: Scene = scene("Dm_SY_Get", "PouchAddItem", set, &[]);
            let mut base: Sim = sim(&r, HM::new()).unwrap();
            let a: Vec<Fin> = run_args(&mut base, &sc, &[("ActorName", "Obj_FairyBless_MPDouble")]).unwrap();
            let mut s: Sim = sim(&r, fl.clone()).unwrap();
            let b: Vec<Fin> = run_args(&mut s, &sc, &[("ActorName", "Obj_FairyBless_MPDouble")]).unwrap();
            assert!(!b.is_empty(), "{set:?}");
            assert_eq!(broken(&b), broken(&a), "{set:?}");
            assert!(b.iter().all(|f| granted(f, "Item_FairyBless_MP") == want), "{set:?}");
            assert!(b.iter().all(|f| granted(f, "Item_FairyBless_MPDouble") == want), "{set:?}");
            assert!(a.iter().all(|f| granted(f, "Item_FairyBless_MP") == 0), "{set:?}");
        }
    }

    const FOUNTAINS: [(&str, &str); 6] = [
        ("HC Great Fairy Reward", "Dm_FairyFountain_Power"),
        ("ZF Great Fairy Reward", "Dm_FairyFountain_Frore"),
        ("DMC Great Fairy Reward", "Dm_FairyFountain_Wisdom"),
        ("Colossus Great Fairy Reward", "Dm_FairyFountain_Nayru"),
        ("OGC Great Fairy Reward", "Dm_FairyFountain_Courage"),
        ("DMT Great Fairy Reward", "Dm_FairyFountain_Din"),
    ];

    const OWNS: [&[(&str, i64)]; 2] = [&[], &[("Item_DinFire", 1)]];

    fn asked(f: &Fin, event: &str, to: u16) -> bool {
        lines(&f.m.log).iter().any(|(_, t)| t.contains(&format!("{event} switch")) && t.contains(&format!("IsGet.Item_DinFire = false -> event {to}")))
    }

    #[test]
    fn every_fountain_opens_and_the_owl_offers_its_ride_without_dins_fire() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let fl: HM<String, Flow> = edited(&mut rf, "Kak 20 Gold Skulltula Reward", "Item_AgonyStone").unwrap();
        assert!(fl.contains_key("TriForcePoint") && fl.contains_key("Npc_Kaepora_DeathMountainTrail"));
        for child in ["true", "false"] {
            for (_, fountain) in FOUNTAINS {
                let entry: String = format!("Demo_{}", fountain.trim_start_matches("Dm_"));
                for own in OWNS {
                    let sc: Scene = scene("TriForcePoint", &entry, &[("Sequence_IsChildhood", child)], own);
                    let mut base: Sim = sim(&r, HM::new()).unwrap();
                    let a: Vec<Fin> = run(&mut base, &sc).unwrap();
                    let mut s: Sim = sim(&r, fl.clone()).unwrap();
                    let b: Vec<Fin> = run(&mut s, &sc).unwrap();
                    let opens = |f: &Fin| f.m.trans.iter().any(|t| t == fountain);
                    assert_eq!(broken(&b), broken(&a), "{fountain} {child} {own:?}");
                    if own.is_empty() {
                        assert!(a.iter().any(|f| asked(f, "Event8", 46)), "{fountain} {child}");
                        assert!(a.iter().filter(|f| asked(f, "Event8", 46)).all(|f| !opens(f)), "{fountain} {child}");
                        assert!(b.iter().all(|f| !asked(f, "Event8", 46)), "{fountain} {child}");
                        assert!(b.iter().any(|f| asked(f, "Event8", 57)), "{fountain} {child}");
                        assert!(b.iter().filter(|f| asked(f, "Event8", 57)).all(|f| opens(f)), "{fountain} {child}");
                    } else {
                        assert_eq!(a.len(), b.len(), "{fountain} {child}");
                        assert_eq!(a.iter().filter(|f| opens(f)).count(), b.iter().filter(|f| opens(f)).count(), "{fountain} {child}");
                    }
                }
            }
        }
        let ride = |f: &Fin| f.m.trans.iter().any(|t| t == "Dm_DeathMtTrail_0030");
        for own in OWNS {
            let sc: Scene = scene("Npc_Kaepora_DeathMountainTrail", "Talk", &[], own);
            let mut base: Sim = sim(&r, HM::new()).unwrap();
            let a: Vec<Fin> = run(&mut base, &sc).unwrap();
            let mut s: Sim = sim(&r, fl.clone()).unwrap();
            let b: Vec<Fin> = run(&mut s, &sc).unwrap();
            assert_eq!(broken(&b), broken(&a), "{own:?}");
            assert!(b.iter().any(|f| ride(f)), "{own:?}");
            assert_eq!(a.iter().any(|f| ride(f)), !own.is_empty(), "{own:?}");
            assert!(b.iter().all(|f| !asked(f, "Event4", 16)), "{own:?}");
        }
    }

    fn fountain_hands_over_any_reward(part: usize) {
        let Some(r) = root() else { return };
        let d: Data = Data::load().unwrap();
        let meter: [&str; 2] = ["Item_FairyBless_MP", "Item_FairyBless_MPDouble"];
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let mut report: Vec<String> = Vec::new();
        for (loc, fountain) in FOUNTAINS.iter().skip(part).take(1) {
            for met in ["false", "true"] {
                let sc: Scene = scene(fountain, "EntryPoint0", &[("PlayerStatus.CurrentRupee", "500"), ("PlayerStatus.MPMax", "0"), ("GreatFairy_MeetFirst", met)], &[]);
                let mut base: Sim = sim(&r, HM::new()).unwrap();
                let a: Vec<Fin> = run(&mut base, &sc).unwrap();
                for item in &d.accept {
                    let Ok(fl) = edited(&mut rf, loc, item) else {
                        report.push(format!("{loc} met={met} {item}: refused by the plan"));
                        continue;
                    };
                    let mut s: Sim = sim(&r, fl).unwrap();
                    let b: Vec<Fin> = run(&mut s, &sc).unwrap();
                    let gave: usize = b.iter().filter(|f| f.end == End::Done && granted(f, item) == 1).count();
                    report.push(format!("{loc} met={met} {item}: gave {gave} broken {} vanilla {} gauge {}", broken(&b), broken(&a), gauge_of(&s)));
                    assert!(gave > 0, "{loc} {met} {item}");
                    assert!(b.iter().all(|f| granted(f, item) <= 1), "{loc} {met} {item}");
                    assert_eq!(broken(&b), broken(&a), "{loc} {met} {item}");
                    if !meter.contains(&item.as_str()) {
                        assert!(!gauge_of(&s), "{loc} {met} {item}");
                    }
                }
            }
        }
        if let Ok(p) = std::env::var("OOTRR_FAIRY_OUT") {
            std::fs::write(format!("{p}.{part}"), report.join("\n")).unwrap();
        }
    }

    parts!(
        fountain_hands_over_any_reward,
        every_fountain_hands_over_any_reward_to_a_player_who_owns_nothing_part_1 = 0,
        every_fountain_hands_over_any_reward_to_a_player_who_owns_nothing_part_2 = 1,
        every_fountain_hands_over_any_reward_to_a_player_who_owns_nothing_part_3 = 2,
        every_fountain_hands_over_any_reward_to_a_player_who_owns_nothing_part_4 = 3,
        every_fountain_hands_over_any_reward_to_a_player_who_owns_nothing_part_5 = 4,
        every_fountain_hands_over_any_reward_to_a_player_who_owns_nothing_part_6 = 5,
    );

    #[test]
    fn every_fountain_gives_its_shuffled_reward_once() {
        let Some(r) = root() else { return };
        let x: crate::flow_extra::Extra = crate::flow_extra::Extra::load().unwrap();
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let mut flags: Vec<String> = Vec::new();
        for (loc, fountain) in FOUNTAINS {
            let flag: String = x
                .by_name(loc)
                .unwrap()
                .ops
                .iter()
                .find_map(|o| match &o.how {
                    crate::flow_extra::op::How::Insert { flag } => Some(flag.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{loc} has no once flag"));
            flags.push(flag.clone());
            let sc: Scene = scene(fountain, "EntryPoint0", &[("PlayerStatus.CurrentRupee", "500"), ("PlayerStatus.MPMax", "240"), ("GreatFairy_MeetFirst", "true")], &[]);
            let mut base: Sim = sim(&r, HM::new()).unwrap();
            let a: Vec<Fin> = run(&mut base, &sc).unwrap();
            let mut s: Sim = sim(&r, edited(&mut rf, loc, NEW).unwrap()).unwrap();
            let b: Vec<Fin> = run(&mut s, &sc).unwrap();
            let gave: Vec<&Fin> = b.iter().filter(|f| f.end == End::Done && granted(f, NEW) == 1).collect();
            assert!(!gave.is_empty(), "{loc}");
            assert!(gave.iter().all(|f| f.m.writes.get(&flag).map(|v| v.as_str()) == Some("true")), "{loc}");
            assert_eq!(broken(&b), broken(&a), "{loc}");
            let played: String = format!("HavePlayedEvent.{fountain}");
            let visit = |given: &str| scene(fountain, "EntryPoint0", &[("PlayerStatus.CurrentRupee", "500"), ("PlayerStatus.MPMax", "240"), ("GreatFairy_MeetFirst", "true"), (&played, "true"), (&flag, given)], &[]);
            let before: Vec<Fin> = run(&mut s, &visit("false")).unwrap();
            assert_eq!(max(&before, NEW), 1, "{loc}");
            let mut vanilla: Sim = sim(&r, HM::new()).unwrap();
            assert!(run(&mut vanilla, &visit("false")).unwrap().iter().any(|f| !f.m.gr.is_empty()), "{loc}");
            let again: Vec<Fin> = run(&mut s, &visit("true")).unwrap();
            assert!(!again.is_empty(), "{loc}");
            assert!(again.iter().all(|f| f.m.gr.is_empty()), "{loc}");
            assert_eq!(broken(&again), broken(&before), "{loc}");
        }
        let distinct: std::collections::BTreeSet<&String> = flags.iter().collect();
        assert_eq!(distinct.len(), FOUNTAINS.len());
        let rows: &str = include_str!("../../../../data/flags.json");
        let extra: &str = include_str!("../../../../data/flow_extra.json");
        let names: Vec<String> = flags.iter().map(|f| f.trim_start_matches("HavePlayedEvent.").to_string()).collect();
        for f in &flags {
            let row: &str = rows.lines().find(|l| l.contains(&format!("\"name\":\"{f}\""))).unwrap_or_else(|| panic!("{f} is not in flags.json"));
            assert!(row.contains("\"type\":\"Bool\"") && row.contains("\"default\":false") && row.contains("\"reset\":8") && row.contains("\"save\":0"), "{f}");
            assert_eq!(extra.matches(&format!("\"{f}\"")).count(), 2, "{f}");
            assert!(!crate::slice::silver::spare_flags().contains(&f.as_str()), "{f}");
        }
        assert!(crate::slice::silver::refs(&mut rf, &names).unwrap().is_empty());
    }

    #[test]
    fn medigoron_sells_once_even_with_the_biggoron_sword() {
        let Some(r) = root() else { return };
        let flag: &str = "HaveTalked_Special.Npc_Goron_Medigoron_Adult_Talk_Event26";
        let sc: Scene = scene("Npc_Goron_Medigoron_Adult", "Talk", &[("PlayerStatus.CurrentRupee", "300")], &[("Weapon_Sword_Biggoron", 1), ("Weapon_Sword_BrokenGiant", 1)]);
        let (a, b): (Vec<Fin>, Vec<Fin>) = both(&r, "GC Medigoron", &sc);
        assert!(a.iter().all(|f| f.m.gr.is_empty()));
        assert_eq!(max(&b, NEW), 1);
        assert_eq!(min(&b, "Weapon_Sword_BrokenGiant"), 0);
        let sold: Vec<&Fin> = b.iter().filter(|f| granted(f, NEW) == 1).collect();
        assert!(!sold.is_empty());
        assert!(sold.iter().all(|f| f.m.writes.get(flag).map(|s| s.as_str()) == Some("true")));
        assert!(b.iter().filter(|f| granted(f, NEW) == 0).all(|f| !f.m.writes.contains_key(flag)));
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let mut s: Sim = sim(&r, edited(&mut rf, "GC Medigoron", NEW).unwrap()).unwrap();
        let after: State = sold[0].m.st.clone();
        let again: Vec<Fin> = run_from(&mut s, &sc, after).unwrap();
        assert_eq!(max(&again, NEW), 0);
        let plain: Scene = scene("Npc_Goron_Medigoron_Adult", "Talk", &[("PlayerStatus.CurrentRupee", "300")], &[]);
        let (a, b): (Vec<Fin>, Vec<Fin>) = both(&r, "GC Medigoron", &plain);
        assert!(a.iter().any(|f| !f.m.gr.is_empty()));
        assert_eq!(max(&b, NEW), 1);
        assert_eq!(broken(&b), broken(&a));
    }
}
