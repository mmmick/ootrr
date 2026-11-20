//! Every Bow and Slingshot comes with its starting bag, wherever it is found.
//!
//! Only a chest hands the starting bag out with them: the chest module (crate::pregrant) adds the
//! small quiver and arrows, or the small bullet bag and seeds. An NPC, a gift scene or any other
//! give only adds the weapon, so the player got a Bow with no quiver (no arrows, and the shops
//! refuse arrows because they test IsGet.Item_Quiver_Small), and the same for the Slingshot.
//!
//! Every chest and gift item goes through GetPouchContent:GetPouchContent, whose one pouch add
//! adds the item, and every pickup on the ground (pots, crates, grass, rocks, beehives, freestanding
//! spots, skulltula drops) goes through Dm_SY_Get or Dm_SY_Get_WithoutPlayer, whose one pouch add
//! adds the pickup ([`ADDS`]). After each of those adds, the flow now asks about the player's state,
//! not about the item: a player who owns the weapon and none of its bags gets the small bag and its ammo, added
//! like Saria's gift (no window). So the first Bow or Slingshot from any of those gives brings its
//! bag, a chest's own pre grant is not repeated (the bag is already owned), a bigger bag is never
//! lowered, and a weapon that came some other way gets its bag at the next item get.
//!
//! In game (2026-11-19) the Slingshot from the lost dog lady still came with no bullet bag: right
//! after the pouch add of a held up gift the game does not report the weapon as owned yet, so that
//! question fails. So every give that hands out the Bow or Slingshot tier itself is also followed,
//! once its whole get event is over, by the bag adds ([`after`], [`follow`]): no bag of any size,
//! then the small bag and its ammo, added like Saria's gift, the way crate::ocarina_hold adds an
//! Ocarina after its gift. That question asks only about the bags, so it needs no ownership of
//! the weapon.

use crate::bfevfl::{Bf, Case, Cont, Event, Flow, Kind, Val};
use crate::error::{Error, Res};
use std::path::Path;

use crate::romfs::Romfs;
use crate::staging::Out;
use crate::start_items::table::{Bag, BOW, SLING};

pub const FLOW: &str = "GetPouchContent";
/// Each give flow and its pouch add: chests and gifts, then pickups.
pub const ADDS: [(&str, usize, &str); 3] = [(FLOW, 0, "Event0"), ("Dm_SY_Get", 7, "Event35"), ("Dm_SY_Get_WithoutPlayer", 0, "Event0")];
const PREFIX: &str = "OOTRRBag";
const GIVE_PREFIX: &str = "OOTRRGiveBag";
const HOLDER: &str = "GameDataActor";
const OWNS: &str = "EventQueryCheckIsGet";
const POUCH: &str = "PouchSystemActor";
const PUT: &str = "EventTriggerAddOrRemovePouchContent";

/// The weapon actor each kit asks about, with its bags and ammo.
pub const KITS: [(&str, Bag); 2] = [("Weapon_Range_Bow", BOW), ("Weapon_Range_SlingShot", SLING)];

fn push_as(f: &mut Flow, prefix: &str, kind: Kind) -> Res<u16> {
    let i: u16 = u16::try_from(f.events.len()).map_err(|_| Error::Format("starter bags: the flow has too many events"))?;
    f.events.push(Event { name: format!("{prefix}{i}"), kind });
    Ok(i)
}

fn owns(f: &mut Flow, prefix: &str, actor: &str, yes: u16, no: u16) -> Res<u16> {
    let (a, q) = crate::progressive::ensure(f, HOLDER, OWNS)?;
    let c: Cont = Cont { items: vec![("ActorName".to_string(), Val::Str(actor.to_string()))] };
    push_as(f, prefix, Kind::Switch { actor: a, query: q, cases: vec![Case { val: 0, ev: yes }, Case { val: 1, ev: no }], params: Some(c) })
}

fn add(f: &mut Flow, prefix: &str, actor: &str, num: i32, next: u16) -> Res<u16> {
    let (a, x) = crate::start_state::slot(f, POUCH, PUT)?;
    let items: Vec<(String, Val)> = vec![
        ("Num".to_string(), Val::Int(num)),
        ("IsEquipDIrect".to_string(), Val::Bool(0)),
        ("IsSetMessageTargetActorName".to_string(), Val::Bool(0)),
        ("Actor".to_string(), Val::Str(actor.to_string())),
        ("IsGetEvent".to_string(), Val::Bool(0)),
    ];
    push_as(f, prefix, Kind::Action { next, actor: a, act: x, params: Some(Cont { items }) })
}

/// No bag of `b` owned: the small bag and its ammo, ending at `then`. Returns its first event.
fn bags(f: &mut Flow, prefix: &str, b: &Bag, then: u16) -> Res<u16> {
    let ammo: u16 = add(f, prefix, b.ammo, b.caps[0], then)?;
    let mut at: u16 = add(f, prefix, b.tiers[0], 1, ammo)?;
    for t in b.tiers.iter().rev() {
        at = owns(f, prefix, t, then, at)?;
    }
    Ok(at)
}

/// The questions and adds of one kit, ending at `then`. Returns its first event.
fn kit(f: &mut Flow, weapon: &str, b: &Bag, then: u16) -> Res<u16> {
    let at: u16 = bags(f, PREFIX, b, then)?;
    owns(f, PREFIX, weapon, at, then)
}

/// The bags of the weapon a give of `tier` hands out: the first tier of the Bow or Slingshot.
pub fn bag_for(tier: &str) -> Option<&'static Bag> {
    match tier {
        "Item_Range_Bow" => Some(&BOW),
        "Item_Range_SlingShot" => Some(&SLING),
        _ => None,
    }
}

/// When `tier` is the Bow or the Slingshot, adds the bag adds for it ending at `then` and returns
/// their first event, for a give to run once it is over.
pub fn after(f: &mut Flow, tier: &str, then: u16) -> Res<Option<u16>> {
    let Some(b) = bag_for(tier) else {
        return Ok(None);
    };
    // No game flow lets a question end the flow, so a give that ends it ends on a short wait.
    let then: u16 = if then == crate::bfevfl::flow::NONE {
        let (a, x) = crate::start_state::slot(f, "EventSystemActor", "EventWait")?;
        let items: Vec<(String, Val)> = vec![("Frames".to_string(), Val::Int(1)), ("IsWaitEnd".to_string(), Val::Bool(1))];
        push_as(f, GIVE_PREFIX, Kind::Action { next: crate::bfevfl::flow::NONE, actor: a, act: x, params: Some(Cont { items }) })?
    } else {
        then
    };
    Ok(Some(bags(f, GIVE_PREFIX, b, then)?))
}

/// The small bag and its ammo, added one after the other with no question, ending at `then`, when
/// `tier` is the Bow or the Slingshot. For the first tier branch of a progressive chain, which only
/// runs for a player without the weapon (so without any of its bags). A straight line keeps the
/// chain's tier gives ending on one shared event for whoever walks it ([`skip`]).
pub fn line(f: &mut Flow, tier: &str, then: u16) -> Res<Option<u16>> {
    let Some(b) = bag_for(tier) else {
        return Ok(None);
    };
    let ammo: u16 = add(f, GIVE_PREFIX, b.ammo, b.caps[0], then)?;
    Ok(Some(add(f, GIVE_PREFIX, b.tiers[0], 1, ammo)?))
}

/// The last event of the straight bag line starting at `n`, if `n` starts one.
pub fn line_end(f: &Flow, n: u16) -> Option<usize> {
    let mut at: usize = usize::from(n);
    let mut last: Option<usize> = None;
    while let Some(Event { name, kind: Kind::Action { next, .. } }) = f.events.get(at) {
        if !name.starts_with(GIVE_PREFIX) {
            break;
        }
        last = Some(at);
        at = usize::from(*next);
    }
    last
}

/// Where `n` leads once past a straight bag line: `n` itself when it starts none.
pub fn skip(f: &Flow, n: u16) -> u16 {
    match line_end(f, n).and_then(|i| f.events.get(i)) {
        Some(Event { kind: Kind::Action { next, .. }, .. }) => *next,
        _ => n,
    }
}

/// The give at `index` hands out `tier`; when that is the Bow or the Slingshot, it now runs the
/// bag adds before going on. Returns whether it changed the flow.
pub fn follow(f: &mut Flow, index: usize, tier: &str) -> Res<bool> {
    let then: u16 = match f.events.get(index).map(|e| &e.kind) {
        Some(Kind::Action { next, .. } | Kind::Sub { next, .. }) => *next,
        _ => return Ok(false),
    };
    if f.events.get(usize::from(then)).is_some_and(|e| e.name.starts_with(GIVE_PREFIX)) {
        return Ok(false);
    }
    let Some(first) = after(f, tier, then)? else {
        return Ok(false);
    };
    match &mut f.events[index].kind {
        Kind::Action { next, .. } | Kind::Sub { next, .. } => *next = first,
        _ => {}
    }
    Ok(true)
}

pub fn patch(f: &mut Flow, i: usize, name: &str) -> Res<()> {
    let then: u16 = match f.events.get(i) {
        Some(Event { name: n, kind: Kind::Action { next, .. } }) if n == name => *next,
        _ => return Err(Error::Msg(format!("starter bags: event {i} of {} is not the pouch add {name}, the game files differ", f.name))),
    };
    if f.events.iter().any(|e| e.name.starts_with(PREFIX)) {
        return Ok(());
    }
    let mut at: u16 = then;
    for (weapon, b) in KITS.iter().rev() {
        at = kit(f, weapon, b, at)?;
    }
    if let Kind::Action { next, .. } = &mut f.events[i].kind {
        *next = at;
    }
    Ok(())
}

pub fn patch_bf(b: &mut Bf, flow: &str, i: usize, name: &str) -> Res<()> {
    let f: &mut Flow = b.flows.iter_mut().find(|x| x.name == flow).ok_or_else(|| Error::Msg(format!("starter bags: no {flow} flow")))?;
    patch(f, i, name)
}

/// Stages the three flows on top of what the build already wrote for them (`work`).
pub fn stage(rf: &mut Romfs, late: &mut Vec<Out>, work: &Path) -> Res<()> {
    for (flow, i, name) in ADDS {
        crate::story_skip::stage_late(rf, late, work, flow, |b| patch_bf(b, flow, i, name))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::flow::NONE;
    use crate::bfevfl::Actor;

    fn actor(name: &str, acts: &[&str], queries: &[&str]) -> Actor {
        Actor {
            name: name.to_string(),
            sub: String::new(),
            arg: String::new(),
            actions: acts.iter().map(|s| s.to_string()).collect(),
            queries: queries.iter().map(|s| s.to_string()).collect(),
            params: None,
            ep: 0,
            cut: 0,
        }
    }

    fn flow() -> Flow {
        let add: Cont = Cont { items: vec![("Num".to_string(), Val::Int(1)), ("Actor".to_string(), Val::Str("arg".to_string()))] };
        Flow {
            name: FLOW.to_string(),
            actors: vec![actor(POUCH, &[PUT], &[]), actor(HOLDER, &[], &["EventQueryIsBool"])],
            events: vec![
                Event { name: "Event0".to_string(), kind: Kind::Action { next: 1, actor: 0, act: 0, params: Some(add) } },
                Event { name: "Event83".to_string(), kind: Kind::Action { next: NONE, actor: 0, act: 0, params: None } },
            ],
            entries: Vec::new(),
        }
    }

    /// Walks the flow from the pouch add with `owned` and returns what it added before `stop`.
    fn walk(f: &Flow, owned: &[&str], stop: u16) -> Vec<(String, i64)> {
        walk_from(f, 0, owned, stop)
    }

    fn walk_from(f: &Flow, from: usize, owned: &[&str], stop: u16) -> Vec<(String, i64)> {
        let mut out: Vec<(String, i64)> = Vec::new();
        let (Kind::Action { next, .. } | Kind::Sub { next, .. }) = &f.events[from].kind else {
            panic!("no add");
        };
        let mut at: u16 = *next;
        for _ in 0..f.events.len() {
            if at == stop {
                return out;
            }
            match &f.events[at as usize].kind {
                Kind::Switch { cases, params, .. } => {
                    let who: Option<&Val> = params.as_ref().and_then(|c| c.get("ActorName"));
                    let has: bool = matches!(who, Some(Val::Str(s)) if owned.contains(&s.as_str()));
                    at = cases.iter().find(|c| c.val == if has { 0 } else { 1 }).unwrap().ev;
                }
                Kind::Action { next, params, .. } => {
                    let c: &Cont = params.as_ref().unwrap();
                    let (Some(Val::Str(a)), Some(Val::Int(n))) = (c.get("Actor"), c.get("Num")) else {
                        panic!("an add without an actor");
                    };
                    assert_eq!(c.get("IsGetEvent"), Some(&Val::Bool(0)));
                    out.push((a.clone(), i64::from(*n)));
                    at = *next;
                }
                _ => panic!("unexpected event"),
            }
        }
        panic!("the chain loops")
    }

    #[test]
    fn a_weapon_without_a_bag_gets_the_small_bag_and_its_ammo() {
        let mut f: Flow = flow();
        patch(&mut f, 0, "Event0").unwrap();
        let s = |a: &str, n: i64| (a.to_string(), n);
        let cases: [(&[&str], Vec<(String, i64)>); 7] = [
            (&[], vec![]),
            (&["Weapon_Range_Bow"], vec![s("Item_Quiver_Small", 1), s("Item_Arrow_Normal", 30)]),
            (&["Weapon_Range_Bow", "Item_Quiver_Small"], vec![]),
            (&["Weapon_Range_Bow", "Item_Quiver_Large"], vec![]),
            (&["Weapon_Range_SlingShot"], vec![s("Item_BulletBag_Small", 1), s("Item_DekuSeed", 30)]),
            (&["Weapon_Range_SlingShot", "Item_BulletBag_Medium"], vec![]),
            (&["Weapon_Range_Bow", "Weapon_Range_SlingShot"], vec![s("Item_Quiver_Small", 1), s("Item_Arrow_Normal", 30), s("Item_BulletBag_Small", 1), s("Item_DekuSeed", 30)]),
        ];
        for (owned, want) in cases {
            assert_eq!(walk(&f, owned, 1), want, "{owned:?}");
        }
        assert_eq!(f.actors[1].queries, vec!["EventQueryIsBool".to_string(), OWNS.to_string()]);
        let before: Flow = f.clone();
        patch(&mut f, 0, "Event0").unwrap();
        assert_eq!(f, before);
    }

    fn give(next: u16, item: &str) -> Flow {
        Flow {
            name: "SubEvent_FindRichard".to_string(),
            actors: vec![actor("GameSystemActor", &[], &[])],
            events: vec![Event {
                name: "Event16".to_string(),
                kind: Kind::Sub { next, params: Some(Cont { items: vec![("ReveiveItem".to_string(), Val::Str(item.to_string()))] }), flow: "NpcCommon".to_string(), entry: "NpcCommon_HandOver_NpcToPlayer_NoDisplay".to_string() },
            }],
            entries: Vec::new(),
        }
    }

    #[test]
    fn a_gift_of_the_weapon_then_adds_its_bag_unless_a_bag_is_owned() {
        let mut f: Flow = give(7, "Item_Range_SlingShot");
        assert!(follow(&mut f, 0, "Item_Range_SlingShot").unwrap());
        let s = |a: &str, n: i64| (a.to_string(), n);
        assert_eq!(walk(&f, &[], 7), vec![s("Item_BulletBag_Small", 1), s("Item_DekuSeed", 30)]);
        assert_eq!(walk(&f, &["Item_BulletBag_Medium"], 7), vec![]);
        // Whether the game already counts the weapon as owned does not matter.
        assert_eq!(walk(&f, &["Weapon_Range_SlingShot"], 7), vec![s("Item_BulletBag_Small", 1), s("Item_DekuSeed", 30)]);
        let before: Flow = f.clone();
        assert!(!follow(&mut f, 0, "Item_Range_SlingShot").unwrap());
        assert_eq!(f, before);
        let mut g: Flow = give(7, "Item_Hookshot");
        assert!(!follow(&mut g, 0, "Item_Hookshot").unwrap());
        assert_eq!(g.events.len(), 1);
    }

    #[test]
    fn a_gift_that_ends_its_flow_ends_on_a_wait_and_no_question_ends_the_flow() {
        let mut f: Flow = give(NONE, "Item_Range_Bow");
        assert!(follow(&mut f, 0, "Item_Range_Bow").unwrap());
        for e in &f.events {
            if let Kind::Switch { cases, .. } = &e.kind {
                assert!(cases.iter().all(|c| c.ev != NONE), "{}", e.name);
            }
        }
        let ends: Vec<&Event> = f.events.iter().filter(|e| matches!(e.kind, Kind::Action { next: NONE, .. })).collect();
        assert_eq!(ends.len(), 1);
        assert_eq!(ends[0].kind.params().and_then(|c| c.get("Frames")), Some(&Val::Int(1)));
    }

    #[test]
    fn the_slingshot_from_the_lost_dog_lady_brings_its_bullet_bag() {
        let root: std::path::PathBuf = ootrr_game::test_romfs::path();
        if !root.join("Pack").join("ZsDic.pack.zs").is_file() {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", root.display());
            return;
        }
        let d: crate::slice::data::Data = crate::slice::data::Data::load().unwrap();
        let mut st: crate::settings::Settings = crate::settings::Settings::default();
        crate::slice::pick::shuffle_both(&mut st);
        let mut g = crate::slice::generate::generate_data(&d, &st, "lost dog slingshot", &crate::slice::GenOpts::default()).unwrap();
        let spot: &str = "Market Lost Dog";
        let at: String = g.seed_file.placements.iter().find(|(_, i)| i.as_str() == "Slingshot").map(|(l, _)| l.clone()).unwrap();
        let there: String = g.seed_file.placements[spot].clone();
        g.seed_file.placements.insert(at, there);
        g.seed_file.placements.insert(spot.to_string(), "Slingshot".to_string());
        let load: std::path::PathBuf = std::env::temp_dir().join(format!("ootrr_lost_dog_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&load);
        let b = crate::slice::edit::build(&g.seed_file, &root, &load, "lost dog").unwrap();
        let file: std::path::PathBuf = b.root.join("romfs").join("Event").join("EventFlow").join("SubEvent_FindRichard.bfevfl.zs");
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let bf: Bf = Bf::parse(&rf.zs.decompress(&std::fs::read(file).unwrap()).unwrap()).unwrap();
        let f: &Flow = &bf.flows[0];
        // The Slingshot tier's give runs on into the bullet bag and the seeds.
        let give: &Event = f.events.iter().find(|e| !matches!(e.kind, Kind::Switch { .. }) && e.kind.params().and_then(|c| c.get("ReveiveItem")) == Some(&Val::Str("Item_Range_SlingShot".to_string()))).expect("no Slingshot give");
        let Kind::Sub { next, .. } = &give.kind else {
            panic!("the give is not a sub");
        };
        let mut got: Vec<String> = Vec::new();
        let mut at: u16 = *next;
        while let Some(Event { name, kind: Kind::Action { next, params, .. } }) = f.events.get(usize::from(at)) {
            if !name.starts_with(GIVE_PREFIX) {
                break;
            }
            if let Some(Val::Str(a)) = params.as_ref().and_then(|c| c.get("Actor")) {
                got.push(a.clone());
            }
            at = *next;
        }
        assert_eq!(got, vec!["Item_BulletBag_Small".to_string(), "Item_DekuSeed".to_string()]);
        let _ = std::fs::remove_dir_all(&load);
    }

    #[test]
    fn another_first_event_is_refused() {
        let mut f: Flow = flow();
        f.events[0].name = "Other".to_string();
        assert!(patch(&mut f, 0, "Event0").is_err());
    }

    #[test]
    fn the_real_flow_takes_the_hook() {
        let p: std::path::PathBuf = ootrr_game::test_romfs::path();
        if !p.join("Pack").join("ZsDic.pack.zs").is_file() {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            return;
        }
        let mut rf: Romfs = Romfs::open(&p).unwrap();
        for (flow, i, name) in ADDS {
            let raw: Vec<u8> = rf.read_zs(&format!("Event/EventFlow/{flow}.bfevfl.zs")).unwrap();
            let mut bf: Bf = Bf::parse(&raw).unwrap();
            let then: u16 = {
                let f: &Flow = bf.flows.iter().find(|x| x.name == flow).unwrap();
                let Kind::Action { next, .. } = &f.events[i].kind else {
                    panic!("{flow}: event {i} is not an action");
                };
                *next
            };
            patch_bf(&mut bf, flow, i, name).unwrap();
            let f: &Flow = bf.flows.iter().find(|x| x.name == flow).unwrap();
            assert_eq!(walk_from(f, i, &["Weapon_Range_Bow"], then), vec![("Item_Quiver_Small".to_string(), 1), ("Item_Arrow_Normal".to_string(), 30)], "{flow}");
            let want: Flow = f.clone();
            let back: Bf = Bf::parse(&bf.write().unwrap()).unwrap();
            assert_eq!(back.flows.iter().find(|x| x.name == flow).unwrap(), &want, "{flow}");
        }
    }
}
