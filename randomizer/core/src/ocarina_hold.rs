//! Lets Link hold up the items that have no animation: the two Ocarinas and the songs.
//!
//! Every gift that has Link hold the item up (each Dm_SY_GetInEvent entry reaches :Item_Position)
//! shows the item with Player_Event.EventIgnitionShowItem, IsAsPlay true and ASCommand
//! Rotate_GetDemo, which plays that animation on the actor it was handed. The Ocarina item actors
//! (equipment accessories) and the song actors have no animation at all, so the game crashed.
//! Giving the Ocarina actors the item animation components still crashed in the game.
//!
//! The game's own hold up gifts also hand out pickup actors: the Death Mountain Crater scrub's
//! sale gives Obj_Heart_Piece. A pickup actor is built on Obj_Item_Base with the shared Item_Common
//! animation, and its PouchContentParam names the item it adds (PouchActor). So where Link holds
//! one of these items up, he is handed its pickup actor from the pickup catalog (Obj_Ocarina_Fairy,
//! Obj_Ocarina_TimeLand, Obj_Melody_* on Sheik's harp model). A progressive chain still asks about
//! the item actors and each branch hands out the matching pickup actor.
//!
//! Adding the pickup actor puts the item in the save, but the running game only sets an Ocarina up
//! for the D-pad after a reload. Saria's gift adds the Ocarina item itself, which sets it up at
//! once, so the gift then adds the item the same way (no window) and, for an Ocarina, writes the
//! equip slot as the starting items do ([`after`]).
//!
//! This covers the NPC gifts that hold the item up (slice::flow_rules::holds_up) and the reward
//! scenes the story skips turn into the standard item get (story_skip::item_get, Sheik's songs
//! included through the song triggers), for a placed item or the scene's own. The Great Fairies give
//! with their own routine and Requiem keeps its scene's window, so the NPC gift rule covers them. The seed build writes the pickup packs
//! all of those can hand out.

use std::sync::OnceLock;

use crate::bfevfl::{Cont, Flow, Kind, Val};
use crate::error::{Error, Res};
use crate::settings::seed::SeedFile;
use crate::slice::data::Data;
use crate::slice::key_spots::{pick, Pick};

/// The Ocarina item actors.
pub const OCARINAS: [&str; 2] = ["Item_Ocarina_Fairy", "Item_Ocarina_Time"];
const SONG: &str = "Item_Melody_";

/// The player's equipped Ocarina. Adding the pickup actor gives the Ocarina but leaves this empty,
/// so the D-pad cannot play it; the starting items write it the same way (start_items::give).
pub const EQUIP: &str = "PlayerStatus.EquipOcarina";

pub const NOTE: &str = "An Ocarina or a song is handed to Link with the item get that holds the item up, so it is handed out as its pickup actor (those items have no item animation), then added like Saria's gift, and its pickup pack is written into the mod. Experimental.";

/// Whether `actor` has no animation of its own, so a hold up cannot show it.
pub fn no_anim(actor: &str) -> bool {
    OCARINAS.contains(&actor) || actor.starts_with(SONG)
}

/// The game data the gift writes after handing out `tier`'s pickup actor, if any.
pub fn equip_key(tier: &str) -> Option<&'static str> {
    OCARINAS.contains(&tier).then_some(EQUIP)
}

/// The pickup actor a hold up hands out in place of `actor`, if `actor` cannot be shown.
pub fn shown(actor: &str) -> Option<String> {
    if !no_anim(actor) {
        return None;
    }
    pick(actor).map(|p| p.pack)
}

/// Every item actor with no animation that has a pickup pack, and that pack.
fn table() -> &'static Vec<(String, String)> {
    static T: OnceLock<Vec<(String, String)>> = OnceLock::new();
    T.get_or_init(|| {
        crate::pickup_pack::catalog::load()
            .unwrap_or_default()
            .into_iter()
            .filter(|e| no_anim(&e.item))
            .filter_map(|e| shown(&e.item).map(|p| (e.item, p)))
            .collect()
    })
}

/// The item actor whose pickup actor `form` is.
pub fn tier_of(form: &str) -> Option<&'static str> {
    table().iter().find(|(_, p)| p == form).map(|(i, _)| i.as_str())
}

/// Whether `actor` is one of the pickup actors handed out in place of such an item.
pub fn is_form(actor: &str) -> bool {
    tier_of(actor).is_some()
}

fn holds_up(d: &Data, loc: &str) -> bool {
    d.flow.by_name(loc).is_some_and(crate::slice::flow_rules::holds_up)
}

/// The actor `loc` hands out for `actor`: its pickup actor at a hold up gift.
pub fn shown_at(d: &Data, loc: &str, actor: &str) -> Option<String> {
    if !holds_up(d, loc) {
        return None;
    }
    shown(actor)
}

/// The actors each tier's branch of a progressive chain that holds the item up hands out, or
/// nothing when the tiers themselves can be shown.
pub fn forms_of(family: &str) -> Vec<String> {
    let Ok(t) = crate::progressive::tiers(family) else {
        return Vec::new();
    };
    if !t.iter().any(|a| no_anim(a)) {
        return Vec::new();
    }
    t.iter().map(|a| shown(a).unwrap_or_else(|| a.to_string())).collect()
}

/// forms_of, for the chain of an NPC gift at `loc` (nothing where the gift shows a window).
pub fn forms(d: &Data, loc: &str, family: &str) -> Vec<String> {
    if !holds_up(d, loc) {
        return Vec::new();
    }
    forms_of(family)
}

/// Adds, after the gift handed out the pickup actor of `tier`, the tier item like Saria's gift
/// (no window, no equip) and, for an Ocarina, the equip slot. Returns the first added event,
/// whose chain ends at `then`.
pub fn after(f: &mut Flow, tier: &str, then: u16) -> Res<u16> {
    let push = |f: &mut Flow, k: Kind| -> Res<u16> {
        let i: u16 = u16::try_from(f.events.len()).map_err(|_| Error::Format("ocarina: the flow has too many events"))?;
        f.events.push(crate::bfevfl::Event { name: format!("OOTRRHeldAdd{i}"), kind: k });
        Ok(i)
    };
    let mut next: u16 = then;
    if let Some(key) = equip_key(tier) {
        let (da, dx): (u16, u16) = crate::start_state::slot(f, "GameDataActor", "EventTriggerSetGameDataString64")?;
        let items: Vec<(String, Val)> = vec![("GameDataName".to_string(), Val::Str(key.to_string())), ("Value".to_string(), Val::Str(tier.to_string())), ("Index".to_string(), Val::Int(-1))];
        next = push(f, Kind::Action { next, actor: da, act: dx, params: Some(Cont { items }) })?;
    }
    let (pa, px): (u16, u16) = crate::start_state::slot(f, "PouchSystemActor", "EventTriggerAddOrRemovePouchContent")?;
    let add: Vec<(String, Val)> = vec![
        ("Num".to_string(), Val::Int(1)),
        ("IsEquipDIrect".to_string(), Val::Bool(0)),
        ("IsSetMessageTargetActorName".to_string(), Val::Bool(0)),
        ("Actor".to_string(), Val::Str(tier.to_string())),
        ("IsGetEvent".to_string(), Val::Bool(0)),
    ];
    push(f, Kind::Action { next, actor: pa, act: px, params: Some(Cont { items: add }) })
}

fn forms_in(d: &Data, item: &str) -> Vec<String> {
    d.item(item).map(|i| i.actor.iter().chain(i.tiers.iter()).filter(|a| no_anim(a)).cloned().collect()).unwrap_or_default()
}

/// The reward scenes the story skips turn into the standard item get (story_skip), which holds the
/// item up whatever the check holds.
pub fn scene_locs() -> Vec<String> {
    let mut out: Vec<String> = vec![crate::lacs_key::LOC.to_string(), crate::slice::flow_rules::SARIA.to_string(), crate::story_skip::RAURU_LOC.to_string(), crate::story_skip::REQUIEM_LOC.to_string()];
    out.extend(crate::story_skip::REWARD_SCENES.iter().map(|x| x.0.to_string()));
    for t in &crate::story_skip::SONG_TRIGGERS {
        out.push(t.loc.to_string());
        out.extend(t.also.iter().map(|x| x.to_string()));
    }
    out.sort();
    out.dedup();
    out
}

/// The locations that hand out an item with no animation by holding it up, with the item actors
/// they can hand out: placed items at NPC gifts that hold them up, and placed or vanilla items at
/// the reward scenes turned into the item get.
pub fn spots(d: &Data, seed: &SeedFile) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for (loc, item) in &seed.placements {
        if holds_up(d, loc) && d.vanilla_of(loc) != Some(item.as_str()) {
            let fs: Vec<String> = forms_in(d, item);
            if !fs.is_empty() {
                out.push((loc.clone(), fs));
            }
        }
    }
    for loc in scene_locs() {
        if out.iter().any(|x| x.0 == loc) {
            continue;
        }
        let Some(item) = seed.placements.get(&loc).map(|s| s.as_str()).or_else(|| d.vanilla_of(&loc)) else {
            continue;
        };
        let fs: Vec<String> = forms_in(d, item);
        if !fs.is_empty() {
            out.push((loc, fs));
        }
    }
    out
}

/// Whether the seed hands out such an item by holding it up anywhere.
pub fn needed(d: &Data, seed: &SeedFile) -> bool {
    !spots(d, seed).is_empty()
}

/// The pickup packs the seed build writes for those locations.
pub fn wanted(d: &Data, seed: &SeedFile) -> Vec<(String, Pick)> {
    let mut out: Vec<(String, Pick)> = Vec::new();
    for (loc, actors) in spots(d, seed) {
        for a in actors {
            if let Some(p) = pick(&a) {
                out.push((loc.clone(), p));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hold_up_gift_hands_out_the_ocarina_pickup_actors() {
        let d: Data = Data::load().unwrap();
        assert_eq!(shown("Item_Ocarina_Fairy").as_deref(), Some("Obj_Ocarina_Fairy"));
        assert_eq!(shown("Item_Ocarina_Time").as_deref(), Some("Obj_Ocarina_TimeLand"));
        assert_eq!(shown("Item_Hookshot"), None);
        assert!(is_form("Obj_Ocarina_TimeLand") && !is_form("Obj_Heart_Piece"));
        assert_eq!(forms(&d, "Kak Man on Roof", "Ocarina"), vec!["Obj_Ocarina_Fairy".to_string(), "Obj_Ocarina_TimeLand".to_string()]);
        assert!(forms(&d, "Kak Man on Roof", "Hookshot").is_empty());
        // Saria's gift shows a get window, no hold up, until the story skips turn it into the item get.
        assert!(forms(&d, "LW Gift from Saria", "Ocarina").is_empty());
        assert_eq!(forms_of("Ocarina"), vec!["Obj_Ocarina_Fairy".to_string(), "Obj_Ocarina_TimeLand".to_string()]);
        assert_eq!(shown("Item_Melody_ForestMinuet").as_deref(), Some("Obj_Melody_ForestMinuet"));
        assert_eq!(tier_of("Obj_Melody_ForestMinuet"), Some("Item_Melody_ForestMinuet"));
        assert_eq!(tier_of("Obj_Ocarina_TimeLand"), Some("Item_Ocarina_Time"));
        assert!(equip_key("Item_Melody_ForestMinuet").is_none() && equip_key("Item_Ocarina_Time") == Some(EQUIP));
        let scenes: Vec<String> = scene_locs();
        for l in ["LW Gift from Saria", "Song from Impa", "ToT Reward from Rauru", "Sheik in Forest", "Queen Gohma"] {
            assert!(scenes.iter().any(|x| x == l), "{l}");
        }
        assert!(scenes.iter().any(|l| l.starts_with("Sheik")), "{scenes:?}");
        assert_eq!(crate::slice::flow_pool::dest_form(&d, "Ocarina", "Kak Anju as Child", "Item_Ocarina_Fairy".to_string()), "Obj_Ocarina_Fairy");
    }

    #[test]
    fn a_seed_with_the_ocarina_at_the_man_on_the_roof_builds_both_pickup_packs_and_hands_them_out() {
        let root: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("D:/Oot/romfs"));
        if !root.join("Pack").join("ZsDic.pack.zs").is_file() {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", root.display());
            return;
        }
        let d: Data = Data::load().unwrap();
        let mut g = crate::slice::generate::generate_data(&d, &crate::settings::Settings::default(), "ocarina held", &crate::slice::GenOpts::default()).unwrap();
        // Trade one Ocarina copy with the man on the roof's item; only the build is tested here.
        let roof: &str = "Kak Man on Roof";
        let at: String = g.seed_file.placements.iter().find(|(_, i)| i.as_str() == "Ocarina").map(|(l, _)| l.clone()).unwrap();
        let there: String = g.seed_file.placements[roof].clone();
        g.seed_file.placements.insert(at, there);
        g.seed_file.placements.insert(roof.to_string(), "Ocarina".to_string());
        assert!(needed(&d, &g.seed_file));
        let load: std::path::PathBuf = std::env::temp_dir().join(format!("ootrr_ocarina_hold_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&load);
        let b = crate::slice::edit::build(&g.seed_file, &root, &load, "ocarina").unwrap();
        let romfs: std::path::PathBuf = b.root.join("romfs");
        for a in OCARINAS {
            let p: std::path::PathBuf = format!("Pack/Actor/{}.pack.zs", shown(a).unwrap()).split('/').fold(romfs.clone(), |x, s| x.join(s));
            assert!(p.is_file(), "{a}'s pickup pack not written");
            let item: std::path::PathBuf = format!("Pack/Actor/{a}.pack.zs").split('/').fold(romfs.clone(), |x, s| x.join(s));
            assert!(!item.is_file(), "{a}'s own pack stays vanilla");
        }
        // The man on the roof's gift hands out the pickup actors.
        let l = d.flow.by_name(roof).unwrap();
        let file: String = crate::flow_locations::path_of(&l.edits[0].file);
        let mut rf: crate::romfs::Romfs = crate::romfs::Romfs::open(&root).unwrap();
        let z: Vec<u8> = std::fs::read(file.split('/').fold(romfs.clone(), |x, s| x.join(s))).unwrap();
        let raw: Vec<u8> = rf.zs.decompress(&z).unwrap();
        let bf = crate::bfevfl::Bf::parse(&raw).unwrap();
        // Hand out events only: the chain's ownership questions still ask about the item actors.
        let named = |n: &str| {
            bf.flows.iter().flat_map(|f| f.events.iter()).filter(|e| !matches!(e.kind, crate::bfevfl::Kind::Switch { .. })).any(|e| e.kind.params().and_then(|c| c.get(&l.edits[0].param)) == Some(&crate::bfevfl::Val::Str(n.to_string())))
        };
        assert!(named("Obj_Ocarina_Fairy") && named("Obj_Ocarina_TimeLand"));
        // Each pickup branch then adds its Ocarina tier like Saria's gift and equips it.
        for t in OCARINAS {
            let adds = bf.flows.iter().flat_map(|f| f.events.iter()).any(|e| e.kind.params().is_some_and(|c| c.get("Actor") == Some(&crate::bfevfl::Val::Str(t.to_string())) && c.get("IsGetEvent") == Some(&crate::bfevfl::Val::Bool(0))));
            assert!(adds, "{t} is not added after its pickup actor");
        }
        for t in OCARINAS {
            let equips = bf.flows.iter().flat_map(|f| f.events.iter()).any(|e| {
                e.kind.params().is_some_and(|c| c.get("GameDataName") == Some(&crate::bfevfl::Val::Str(EQUIP.to_string())) && c.get("Value") == Some(&crate::bfevfl::Val::Str(t.to_string())))
            });
            assert!(equips, "{t} is not equipped after its pickup actor");
        }
        assert!(!named("Item_Ocarina_Fairy"));
        let _ = std::fs::remove_dir_all(&load);
    }
}

#[cfg(test)]
mod pickup_tests {
    use crate::settings::{Settings, Value};
    use crate::slice::data::Data;

    #[test]
    fn a_generated_seed_with_an_ocarina_at_a_pickup_spot_builds_its_pickup_pack() {
        let root: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("D:/Oot/romfs"));
        if !root.join("Pack").join("ZsDic.pack.zs").is_file() {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", root.display());
            return;
        }
        let d: Data = Data::load().unwrap();
        let mut s: Settings = Settings::default();
        s.set("shuffle_freestanding_items", Value::s("all")).unwrap();
        s.set("shuffle_pots", Value::s("all")).unwrap();
        let spot = |loc: &str| d.pots.has(loc) || d.cat.by_name(loc).is_some_and(crate::slice::pickups::is_world);
        let mut found = None;
        for n in 0..30 {
            let g = crate::slice::generate::generate_data(&d, &s, &format!("ocarina spot {n}"), &crate::slice::GenOpts::default()).unwrap();
            if let Some((l, _)) = g.seed_file.placements.iter().find(|(l, i)| i.as_str() == "Ocarina" && spot(l)) {
                found = Some((l.clone(), g));
                break;
            }
        }
        let Some((loc, g)) = found else {
            panic!("no seed put an Ocarina at a freestanding spot or a pot in 30 tries");
        };
        let load: std::path::PathBuf = std::env::temp_dir().join(format!("ootrr_ocarina_spot_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&load);
        let b = crate::slice::edit::build(&g.seed_file, &root, &load, "ocarina spot").unwrap_or_else(|e| panic!("{loc}: {e}"));
        let actor = b.root.join("romfs").join("Pack").join("Actor");
        assert!(actor.join("Obj_Ocarina_Fairy.pack.zs").is_file() || actor.join("Obj_Ocarina_TimeLand.pack.zs").is_file(), "{loc}");
        let _ = std::fs::remove_dir_all(&load);
    }
}
