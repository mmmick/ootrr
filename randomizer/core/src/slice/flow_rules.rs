use crate::flow_locations::Loc;
use crate::progressive::family_of;
use crate::slice::data::{Data, Item};

pub fn grants_only(l: &Loc) -> bool {
    l.supported && !l.edits.is_empty() && l.edits.iter().all(|e| e.role == "grant")
}

pub fn tiered(l: &Loc, actor: &str) -> bool {
    l.edits.iter().any(|e| e.item != actor)
}

pub fn linked(i: &Item, actor: &str) -> bool {
    i.actor.as_deref() == Some(actor) || i.tiers.iter().any(|t| t == actor) || i.world.iter().any(|w| w == actor)
}

/// Saria's gift (ported from Ontico's REALAZK/ootrr pull request 11). Its vanilla Ocarina is
/// progressive, which pins a location whose flow also shows a get window (the window names one
/// tier). Saria can hold any item, progressive ones included: a placed progressive item wraps both
/// the window and the grant into tier checks (flow_wrap::grants takes every edit of the location),
/// and the window runs before the grant (crate::story_skip::patch_gift, and the scene's own C01
/// cut), so both name the same tier. The ocarina tutorial is turned off (crate::no_tutorials).
///
/// Saria also skips the flow_locations.json `accept` list, which only names actors that some
/// vanilla event hands out. Her gift uses the generic pouch add and
/// GetPouchContent:DisplayGetByActorName, which treasure chests use for every item, so any
/// pouch item fits (the Hookshot, the Gauntlets, the bags, the Bow and the Slingshot included).
pub const SARIA: &str = "LW Gift from Saria";

/// Darunia's Joy. Its vanilla Progressive Strength pinned it like Saria's Ocarina (no event hands
/// out the Silver or Golden Gauntlets). Its grant, Dm_Goron_0030's sub
/// Dm_SY_GetInEvent:GetInEvent_Goron, adds arg:ActorName to the pouch and opens the get window for
/// it, so it holds any pouch item Saria holds; a placed progressive item wraps the grant into tier
/// checks. It holds the item up, so the Ocarinas and songs go through crate::ocarina_hold.
pub const DARUNIA: &str = "GC Darunias Joy";

/// The Ocarina of Time Zelda throws in the moat. Its vanilla Ocarina is progressive, which pinned
/// it like Saria's gift. Its scene, Dm_MelodyGet_Time, is dropped by the song lesson skip
/// (crate::story_skip::SONG_TRIGGERS, Song from Ocarina of Time), whose item get hands out what
/// this location holds before the Song of Time, so like Saria it holds any pouch item; a placed
/// progressive item gets its tier check from that give (crate::slice::edit, give_for).
pub const OCARINA_OF_TIME: &str = "HF Ocarina of Time Item";

/// The NPC gifts that add any pouch item generically.
pub const ANY_POUCH: [&str; 3] = [SARIA, DARUNIA, OCARINA_OF_TIME];

/// Gift scenes: cutscenes, Great Fairies and the dungeon reward scenes. Every grant they run ends
/// in the game's generic pouch get, GetPouchContent:GetPouchContent, which shows the get window
/// (SetForGetWindow and OpenGetWindow, the same steps as GetPouchContent:DisplayGetByActorName)
/// and then adds the item (PouchSystemActor.EventTriggerAddOrRemovePouchContent, with the same
/// parameters as Saria's grant). GetPouchContentByActorName is that call, and every
/// Dm_SY_GetInEvent variant reaches it through GetPouchContentByActorName_ForGetInEvent after
/// its presentation. So like Saria they hand out any pouch item, not only the accept list.
pub const GIFT_CATS: [&str; 3] = ["cutscene_gift", "great_fairy", "dungeon_reward"];
const GENERIC_GRANTS: [&str; 3] = ["GetPouchContent:", "PouchSystemActor.EventTriggerAddOrRemovePouchContent", "Dm_SY_GetInEvent:"];

pub fn gift_scene(l: &Loc) -> bool {
    GIFT_CATS.contains(&l.cat.as_str())
        && l.edits.iter().any(|e| e.role == "grant")
        && l.edits.iter().filter(|e| e.role == "grant").all(|e| GENERIC_GRANTS.iter().any(|v| e.via.starts_with(v)))
}

/// The give routines of the other reward events (NPCs, minigames, Skulltula rewards). Each was
/// traced in the game's flows to the same generic pouch get: the NpcCommon handovers through
/// Dm_SY_GetInEvent:GetInEvent_NotTurn, the Gold Skulltula rewards' TakenReward_Common_Event*
/// through GetInEvent_NotTurn and GetInEvent_NotTurn_PosCam_InvNPC, the rolling Goron's handover
/// through GetInEvent_NotTurn_BombBag, and every Dm_SY_GetInEvent:GetInEvent* routine but the
/// bottle ones (those push into a bottle instead). Dungeon items run a window-only copy of the
/// routine (crate::item_window) and their credit (crate::gift_keys).
const ROUTINES: [&str; 4] = ["Dm_SY_GetInEvent:GetInEvent", "NpcCommon:NpcCommon_HandOver_NpcToPlayer_NoDisplay", ":TakenReward_Common_Event", ":NpcCommon_HandOver_NpcToPlayer_NoDisplay_BombBag"];
const BOTTLE_ROUTINES: [&str; 3] = ["Dm_SY_GetInEvent:GetInEventInBottle", "Dm_SY_GetInEvent:GetInEventInEquipedBottle", "Dm_SY_GetInEvent:GetInEventInBottle_BlueBottleShop"];

/// Reward events built by their own modules (clones, tiers of a chain, sales with prices).
fn own_module(l: &Loc) -> bool {
    crate::flow_extra::trade::is(&l.name)
        || crate::flow_extra::cows::is(&l.name)
        || crate::flow_extra::frogs::is(&l.name)
        || crate::flow_extra::potion::is(&l.name)
        || crate::scrubs_a::is_a(l)
        || crate::scrubs_b::is_b(l)
}

/// A reward event that hands out any item a chest can, dungeon items included: a gift scene, or an
/// NPC or minigame whose give routine ends in the generic pouch get.
pub fn any_item(l: &Loc) -> bool {
    if gift_scene(l) {
        return true;
    }
    let grants: Vec<&crate::flow_locations::Edit> = l.edits.iter().filter(|e| e.role == "grant").collect();
    !own_module(l)
        && !grants.is_empty()
        && grants.iter().all(|e| ROUTINES.iter().any(|v| e.via.starts_with(v)) && !BOTTLE_ROUTINES.contains(&e.via.as_str()))
}

/// Items a generic pouch get hands out. Treasure chests run the same routine
/// (GetPouchContent:GetPouchContentByActorNameForTreasureBox reaches GetPouchContent:GetPouchContent)
/// with their content actor, so every item a chest gives works: rupees and ammo (TBox_ and Tbox_
/// contents), shields, Heart Containers and Pieces of Heart, filled bottles (the get window has its
/// bottle branch), Gold Skulltula Tokens (and its token branch), Stones and Medallions. Dungeon
/// items need their dungeon credited, which crate::gift_keys does instead of the pouch add.
fn gift_form(a: &str) -> bool {
    !a.is_empty() && !a.starts_with("Item_Dungeon_")
}

/// An item a gift scene can hand out: one the game's events already hand out, or one whose every
/// form a chest can give.
pub fn gift_takes(d: &Data, i: &Item) -> bool {
    d.flow_accepts(i) || (i.actor.as_deref().is_some_and(gift_form) && i.tiers.iter().all(|t| gift_form(t)))
}

/// An item Saria can hand out: the same as any gift scene (her grant is the bare pouch add and her
/// window its own event, the two steps of the generic pouch get).
pub fn saria_takes(d: &Data, i: &Item) -> bool {
    gift_takes(d, i)
}

pub fn ok(d: &Data, name: &str) -> bool {
    let Some(l) = d.flow.by_name(name) else {
        return false;
    };
    if crate::flow_extra::trade::is(name) {
        return l.gated && d.vanilla_of(name).is_some_and(|v| l.item.as_deref() != d.item(v).and_then(|i| i.actor.as_deref()));
    }
    if crate::flow_extra::cows::is(name) || crate::flow_extra::frogs::is(name) || crate::flow_extra::potion::is(name) {
        return l.gated && l.item.is_some() && d.vanilla_of(name).is_some();
    }
    if crate::scrubs_b::is_b(l) {
        return crate::scrubs_b::ok(d, l);
    }
    if !(l.supported || l.gated) || l.conf == "guess" || l.edits.is_empty() {
        return false;
    }
    let Some(actor) = l.item.as_deref() else {
        return false;
    };
    let Some(i) = d.vanilla_of(name).and_then(|v| d.item(v)) else {
        return false;
    };
    if family_of(&i.tiers).is_some() {
        if ANY_POUCH.contains(&name) {
            return linked(i, actor) && saria_takes(d, i);
        }
        return (grants_only(l) || crate::scrubs_a::grants_only(l)) && linked(i, actor) && d.flow_accepts(i);
    }
    if tiered(l, actor) {
        return grants_only(l);
    }
    linked(i, actor)
}

/// Gift routes that show the item in a get window and never hold it up: the pouch add of Saria's
/// gift and the Ocarina of Time scene, GetPouchContent (Sheik, Impa, Malon, the bosses) and the
/// cows' bottle hand over.
pub const WINDOW: [&str; 3] = ["PouchSystemActor.EventTriggerAddOrRemovePouchContent", "GetPouchContent:GetPouchContentByActorName", "Dm_SY_GetInEvent:GetInEventInBottle"];

/// Whether the location's gift has Link hold the item up (Dm_SY_GetInEvent :Item_Position plays
/// Rotate_GetDemo on the item). Items with no animation of their own (the Ocarinas and the
/// songs) are handed out there as their pickup actor (crate::ocarina_hold). The menu scrubs sell through the one
/// time scrub sale, which holds it up too.
pub fn holds_up(l: &Loc) -> bool {
    l.cat == "scrub_menu" || l.edits.iter().any(|e| e.role == "grant" && !WINDOW.contains(&e.via.as_str()))
}

pub fn allows(d: &Data, name: &str, i: &Item) -> bool {
    let Some(l) = d.flow.by_name(name) else {
        return true;
    };
    if name == SARIA || ANY_POUCH.contains(&name) || gift_scene(l) {
        // An upgrade is wrapped into tier checks around every edit of the scene (flow_wrap::grants),
        // and the display events of these scenes run before the grant, so all name the same tier.
        return gift_takes(d, i) || d.vanilla_of(name) == Some(i.name.as_str());
    }
    if any_item(l) {
        // An NPC's display events may follow its grant, so an upgrade still wants grants only.
        let fits: bool = gift_takes(d, i) || d.vanilla_of(name) == Some(i.name.as_str());
        return fits && (family_of(&i.tiers).is_none() || grants_only(l) || d.vanilla_of(name) == Some(i.name.as_str()));
    }
    if family_of(&i.tiers).is_some() {
        return (grants_only(l) && l.cat != "scrub_sale" && d.flow_accepts(i)) || crate::scrubs_a::home(d, name, &i.name);
    }
    d.flow_accepts(i) || d.vanilla_of(name) == Some(i.name.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Settings, Value};
    use crate::slice::generate::{generate_data, GenOpts, Generated};

    const NEW: [&str; 19] = [
        "GC Darunias Joy",
        "Graveyard Dampe Gravedigging Tour",
        "Kak 40 Gold Skulltula Reward",
        "Kak 100 Gold Skulltula Reward",
        "LW Target in Woods",
        "Market Shooting Gallery Reward",
        "Kak Shooting Gallery Reward",
        "Market Bombchu Bowling First Prize",
        "GF HBA 1500 Points",
        "Deku Theater Skull Mask",
        "Deku Theater Mask of Truth",
        "LLR Talons Chickens",
        "Kak 10 Gold Skulltula Reward",
        "Kak 30 Gold Skulltula Reward",
        "DMT Great Fairy Reward",
        "DMC Great Fairy Reward",
        "ZD Diving Minigame",
        "LH Adult Fishing",
        "GC Rolling Goron as Child",
    ];

    const PINNED: [&str; 4] = [
        "DMT Biggoron",
        "GC Medigoron",
        "KF Links House Cow",
        "Kak 20 Gold Skulltula Reward",
    ];

    fn item<'a>(d: &'a Data, n: &str) -> &'a Item {
        d.item(n).unwrap()
    }

    #[test]
    fn the_ocarina_and_songs_go_to_gifts_that_hold_the_item_up() {
        let d: Data = Data::load().unwrap();
        let ocarina: &Item = item(&d, "Ocarina");
        let song: &Item = item(&d, "Minuet of Forest");
        for n in ["LW Skull Kid", "Kak Man on Roof", "HC Malon Egg", "ZD Diving Minigame", "Deku Theater Skull Mask", "Kak Anju as Child"] {
            assert!(holds_up(d.flow.by_name(n).unwrap()), "{n}");
            assert!(allows(&d, n, ocarina) && allows(&d, n, song), "{n}");
        }
        assert!(allows(&d, DARUNIA, ocarina) && allows(&d, SARIA, ocarina));
        assert!(!holds_up(d.flow.by_name(SARIA).unwrap()));
    }

    #[test]
    fn darunia_holds_any_pouch_item() {
        let d: Data = Data::load().unwrap();
        assert!(d.flow_ok(DARUNIA));
        for n in ["Progressive Strength Upgrade", "Bomb Bag", "Boomerang", "Progressive Hookshot", "Bow"] {
            assert!(allows(&d, DARUNIA, item(&d, n)), "{n}");
        }
        assert!(!allows(&d, DARUNIA, item(&d, "Small Key (Forest Temple)")));
    }

    #[test]
    fn saria_holds_any_item_progressive_ones_included() {
        let d: Data = Data::load().unwrap();
        assert!(d.flow_ok(SARIA));
        for n in ["Ocarina", "Progressive Wallet", "Progressive Scale", "Magic Meter", "Boomerang", "Progressive Hookshot", "Progressive Strength Upgrade", "Bomb Bag", "Bow", "Slingshot"] {
            assert!(allows(&d, SARIA, item(&d, n)), "{n}");
        }
        // A scrub sale keeps the accept list; the reward events hand out any pouch item (any_item).
        assert!(!allows(&d, "HF Deku Scrub Grotto", item(&d, "Progressive Hookshot")));
        assert!(allows(&d, "Kak 50 Gold Skulltula Reward", item(&d, "Progressive Hookshot")));
        assert!(d.flow_ok(OCARINA_OF_TIME), "the Ocarina of Time scene holds any item");
        for n in ["Ocarina", "Progressive Hookshot", "Bow", "Bombs (5)"] {
            assert!(allows(&d, OCARINA_OF_TIME, item(&d, n)), "{n}");
        }
        // Every item a chest gives works through the generic pouch get; dungeon items go through
        // crate::gift_keys instead.
        for n in ["Bottle with Milk", "Bombs (5)", "Gold Skulltula Token", "Rupees (20)", "Deku Shield", "Heart Container"] {
            assert!(saria_takes(&d, item(&d, n)), "{n}");
        }
        for n in ["Small Key (Forest Temple)", "Map (Deku Tree)"] {
            assert!(!saria_takes(&d, item(&d, n)), "{n}");
        }
    }

    #[test]
    fn the_new_locations_are_free_and_the_risky_ones_stay_pinned() {
        let d: Data = Data::load().unwrap();
        for n in NEW {
            assert!(d.flow_ok(n), "{n}");
        }
        // More NPC rewards is on by default and frees Biggoron, so it is turned off here.
        let mut quiet: Settings = Settings::default();
        quiet.set(crate::flow_extra::SETTING, Value::Bool(false)).unwrap();
        let off: crate::slice::support::Plan = crate::slice::support::plan(&quiet).unwrap();
        for n in PINNED {
            let open: bool = d.flow.by_name(n).is_some_and(|l| crate::flow_extra::open(l, &off));
            assert!(!(d.flow_ok(n) && open), "{n}");
        }
        assert!(d.flow.by_name("GC Rolling Goron as Child").is_some_and(|l| l.supported && grants_only(l) && crate::flow_extra::open(l, &off)));
        for n in ["Kak 20 Gold Skulltula Reward", "DMT Biggoron", "GC Medigoron"] {
            assert!(d.flow_ok(n) && d.flow.by_name(n).is_some_and(|l| l.gated), "{n}");
        }
    }

    fn goron(over: &[(&str, &str)], gone: &[&str]) -> bool {
        let d: Data = Data::load().unwrap();
        let w: crate::world::World = crate::slice::logic::world_with(&d, over).unwrap();
        let mut p: crate::world::Placement = crate::slice::fill::vanilla_place(&w, &d).unwrap();
        let g: Vec<crate::logic_items::ItemId> = gone.iter().map(|n| w.item(n).unwrap()).collect();
        let f: crate::logic_items::ItemId = w.item("Rupees (5)").unwrap();
        for l in &w.real {
            if p.get(*l).is_some_and(|i| g.contains(&i)) {
                p.put(*l, f);
            }
        }
        let s: crate::search::Search = crate::search::explore(&w, &p, &[]);
        s.loc_access(w.loc("GC Rolling Goron as Child").unwrap())
    }

    #[test]
    fn the_child_rolling_goron_needs_a_bomb_bag_and_the_gate_rule_of_goron_city_only() {
        let chus: (&str, &str) = (crate::slice::bombchus::LOGIC_KEY, "true");
        let open: (&str, &str) = ("open_kakariko", "open");
        let both: [&str; 2] = ["Bomb Bag", "Progressive Strength Upgrade"];
        assert!(goron(&[], &[]));
        assert!(!goron(&[], &both));
        assert!(!goron(&[chus], &both));
        assert!(!goron(&[("logic_child_rolling_with_strength", "true")], &both));
        assert!(!goron(&[], &["Zeldas Letter"]));
        assert!(goron(&[open], &["Zeldas Letter"]));
        assert!(!goron(&[open], &both));
        assert!(goron(&[], &["Bombchus (10)","Bombchus (5)", "Bombchus (20)"]));
        let line: &str = crate::slice::world_files::overworld_text().lines().find(|l| l.contains("\"GC Rolling Goron as Child\":")).unwrap();
        assert_eq!(line.trim(), "\"GC Rolling Goron as Child\": \"is_child and Bomb_Bag\",");
    }

    #[test]
    fn a_child_with_strength_and_no_bomb_bag_does_not_reach_the_rolling_goron() {
        assert!(!goron(&[], &["Bomb Bag"]));
        assert!(!goron(&[(crate::slice::bombchus::LOGIC_KEY, "true")], &["Bomb Bag"]));
    }

    #[test]
    fn the_adult_rolling_goron_stops_with_bomb_flowers_bombs_or_bombchus() {
        let rule: &str = crate::slice::world_files::overworld_text()
            .split("\"Stop GC Rolling Goron as Adult\": \"")
            .nth(1)
            .and_then(|r| r.split('"').next())
            .unwrap();
        assert_eq!(rule.split_whitespace().collect::<Vec<_>>().join(" "), "is_adult and (Progressive_Strength_Upgrade or has_explosives)");
    }

    #[test]
    fn the_two_fountains_follow_their_flows_and_the_remake_items() {
        let d: Data = Data::load().unwrap();
        for (loc, file, item, vanilla) in [
            ("DMT Great Fairy Reward", "Dm_FairyFountain_Din.bfevfl", "Item_DinFire", "Dins Fire"),
            ("HC Great Fairy Reward", "Dm_FairyFountain_Power.bfevfl", "Item_FairyBless_MP", "Magic Meter"),
        ] {
            let l: &Loc = d.flow.by_name(loc).unwrap();
            assert_eq!(l.edits.len(), 1, "{loc}");
            assert_eq!(l.edits[0].file, file, "{loc}");
            assert_eq!(l.item.as_deref(), Some(item), "{loc}");
            assert_eq!(l.vanilla, vanilla, "{loc}");
            assert_eq!(d.vanilla_of(loc), Some(vanilla), "{loc}");
            assert!(d.flow_ok(loc), "{loc}");
        }
        assert_eq!(d.vanilla_of("DMC Great Fairy Reward"), Some("Magic Meter"));
        let skull: &Loc = d.flow.by_name("Kak 40 Gold Skulltula Reward").unwrap();
        assert_eq!(skull.vanilla, "Bombchus (20)");
        assert_eq!(d.vanilla_of("Kak 40 Gold Skulltula Reward"), Some("Bombchus (20)"));
        assert_eq!(d.item("Bombchus (20)").and_then(|i| i.actor.as_deref()), skull.item.as_deref());
    }

    #[test]
    fn tiered_locations_name_the_lower_tier_and_hold_a_junk_vanilla_item() {
        let d: Data = Data::load().unwrap();
        for n in ["LW Target in Woods", "Market Shooting Gallery Reward", "Kak Shooting Gallery Reward", "Market Bombchu Bowling First Prize", "GF HBA 1500 Points"] {
            let l: &Loc = d.flow.by_name(n).unwrap();
            assert_eq!(l.edits.len(), 2, "{n}");
            assert!(tiered(l, l.item.as_deref().unwrap()), "{n}");
            assert_eq!(d.vanilla_of(n), Some("Rupees (5)"), "{n}");
        }
        assert_eq!(d.flow.by_name("LW Target in Woods").map(|l| l.conf.as_str()), Some("scene"));
    }

    #[test]
    fn family_items_go_only_to_grant_only_flow_locations_with_accepted_tiers() {
        let d: Data = Data::load().unwrap();
        let wallet: &Item = item(&d, "Progressive Wallet");
        assert!(allows(&d, "Kak 50 Gold Skulltula Reward", wallet));
        assert!(allows(&d, "LW Target in Woods", wallet));
        // The Light Arrows scene runs the generic pouch get like the other gift scenes.
        assert!(allows(&d, "ToT Light Arrows Cutscene", wallet));
        assert!(!allows(&d, "HF Deku Scrub Grotto", wallet));
        assert!(!allows(&d, "Song from Impa", wallet));
        assert!(allows(&d, "Kak 50 Gold Skulltula Reward", item(&d, "Progressive Strength Upgrade")));
        assert!(allows(&d, "Kak 50 Gold Skulltula Reward", item(&d, "Progressive Hookshot")));
        assert!(!allows(&d, "HF Deku Scrub Grotto", item(&d, "Progressive Hookshot")));
        assert!(allows(&d, "Some Chest", item(&d, "Progressive Hookshot")));
    }

    #[test]
    fn a_location_always_accepts_its_own_vanilla_item_but_the_item_goes_nowhere_else() {
        let d: Data = Data::load().unwrap();
        let rupees: &Item = item(&d, "Rupees (5)");
        // A location that keeps the accept list (a scrub sale) takes no one else's vanilla item;
        // the reward events take any pouch item (any_item).
        assert!(allows(&d, "LW Target in Woods", rupees));
        assert!(!allows(&d, "HF Deku Scrub Grotto", rupees));
        assert!(allows(&d, "Kak 50 Gold Skulltula Reward", rupees));
        let milk: &Item = item(&d, "Bottle with Milk");
        assert!(allows(&d, "LLR Talons Chickens", milk));
        assert!(!allows(&d, "HF Deku Scrub Grotto", milk));
        let gold: &Item = item(&d, "Rupees (200)");
        assert!(allows(&d, "Kak 100 Gold Skulltula Reward", gold));
        assert!(!allows(&d, "HF Deku Scrub Grotto", gold));
    }

    #[test]
    fn seeds_with_rewards_on_put_the_new_locations_in_the_pool_and_stay_valid() {
        let d: Data = Data::load().unwrap();
        let mut s: Settings = Settings::default();
        s.set("shuffle_chests", Value::s("all")).unwrap();
        s.set("shuffle_rewards", Value::s("supported")).unwrap();
        s.set(crate::flow_extra::SETTING, Value::Bool(false)).unwrap();
        for n in 0..2 {
            let g: Generated = generate_data(&d, &s, &format!("flow rules {n}"), &GenOpts::default()).unwrap();
            for l in NEW {
                assert!(g.seed_file.placements.contains_key(l), "{l}");
            }
            for l in PINNED {
                assert!(!g.seed_file.placements.contains_key(l), "{l}");
            }
        }
    }
}
