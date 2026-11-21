#[cfg(test)]
mod tests {
    use crate::logic_items::ItemId;
    use crate::search::{explore, Search};
    use crate::settings::{Settings, Value};
    use crate::slice::data::Data;
    use crate::slice::dungeon_items::chest_ok;
    use crate::slice::fill::vanilla_place;
    use crate::slice::logic::{world, world_with};
    use crate::slice::pick::pick;
    use crate::slice::support::{plan, Plan};
    use crate::world::{Placement, World};

    const KEY: &str = "Small Key (Fire Temple)";

    fn open(d: &Data) -> World {
        world_with(d, &[("open_door_of_time", "open")]).unwrap()
    }

    fn stripped(w: &World, d: &Data, names: &[&str]) -> Placement {
        let mut p: Placement = vanilla_place(w, d).unwrap();
        let f: ItemId = w.item("Rupees (5)").unwrap();
        let gone: Vec<ItemId> = names.iter().map(|n| w.item(n).unwrap()).collect();
        for l in &w.real {
            if p.get(*l).is_some_and(|i| gone.contains(&i)) {
                p.put(*l, f);
            }
        }
        p
    }

    fn ids(w: &World, names: &[&str]) -> Vec<ItemId> {
        names.iter().map(|n| w.item(n).unwrap()).collect()
    }

    fn reached(w: &World, p: &Placement, more: &[&str], loc: &str) -> bool {
        explore(w, p, &ids(w, more)).loc_access(w.loc(loc).unwrap())
    }

    fn region(w: &World, p: &Placement, more: &[&str], at: &str) -> bool {
        explore(w, p, &ids(w, more)).region_reached(w.region(at).unwrap(), None)
    }

    #[test]
    fn the_boss_door_is_reached_from_the_hammer_floor_and_not_from_the_lava_room() {
        let d: Data = Data::load().unwrap();
        let w: World = open(&d);
        let p: Placement = stripped(&w, &d, &["Megaton Hammer"]);
        let all: [&str; 2] = ["Hover Boots", "Progressive Hookshot"];
        assert!(region(&w, &p, &all, "Fire Temple Upper"));
        assert!(!region(&w, &p, &all, "Fire Temple Before Boss"));
        assert!(region(&w, &p, &["Megaton Hammer"], "Fire Temple Before Boss"));
    }

    // Hover Boots stand in for the Hookshot on the way to the temple door, so
    // neither item that opens the cage is in hand.
    #[test]
    fn the_compass_cage_opens_for_the_hookshot_or_the_bow() {
        let d: Data = Data::load().unwrap();
        let w: World = open(&d);
        let p: Placement = stripped(&w, &d, &["Progressive Hookshot", "Bow"]);
        let cage: &str = "Fire Temple Map Chest";
        assert!(reached(&w, &p, &["Hover Boots"], "Fire Temple Boulder Maze Lower Chest"));
        assert!(reached(&w, &p, &["Hover Boots"], "Fire Temple Boulder Maze Upper Chest"));
        assert!(!reached(&w, &p, &["Hover Boots"], cage));
        assert!(reached(&w, &p, &["Hover Boots", "Progressive Hookshot"], cage));
        assert!(reached(&w, &p, &["Hover Boots", "Bow"], cage));
    }

    // The Bomb Bag stands in for the bracelet on the way to the temple.
    #[test]
    fn the_climb_to_the_boulder_maze_needs_no_bracelet() {
        let d: Data = Data::load().unwrap();
        let w: World = open(&d);
        let p: Placement = stripped(&w, &d, &["Progressive Strength Upgrade"]);
        assert!(reached(&w, &p, &["Bomb Bag"], "Fire Temple Boulder Maze Lower Chest"));
        assert!(reached(&w, &p, &["Bomb Bag"], "Fire Temple Megaton Hammer Chest"));
    }

    #[test]
    fn the_song_rooms_want_their_songs() {
        let d: Data = Data::load().unwrap();
        let w: World = open(&d);
        let p: Placement = stripped(&w, &d, &["Song of Time", "Scarecrow Song"]);
        for (song, loc) in [
            ("Song of Time", "Fire Temple GS Song of Time Room"),
            ("Song of Time", "Fire Temple Highest Goron Chest"),
            ("Scarecrow Song", "Fire Temple Scarecrow Chest"),
            ("Scarecrow Song", "Fire Temple Scarecrow Ledge Third Chest"),
            ("Scarecrow Song", "Fire Temple GS Scarecrow Top"),
        ] {
            assert!(!reached(&w, &p, &[], loc), "{loc}");
            assert!(reached(&w, &p, &[song], loc), "{loc}");
        }
    }

    fn opens_with(d: &Data, lay: &str, n: usize, at: &str) -> bool {
        let w: World = world_with(d, &[("open_door_of_time", "open"), ("fire_key_layout", lay)]).unwrap();
        let k: ItemId = w.item(KEY).unwrap();
        let mut p: Placement = vanilla_place(&w, d).unwrap();
        for l in &w.real {
            if p.get(*l) == Some(k) {
                p.item[*l] = None;
            }
        }
        let mut extra: Vec<ItemId> = Vec::new();
        for (i, name) in w.items.names.iter().enumerate() {
            if w.items.adv[i] && !name.starts_with(KEY) {
                extra.extend([i; 3]);
            }
        }
        extra.extend(vec![k; n]);
        let s: Search = explore(&w, &p, &extra);
        match w.loc(at) {
            Some(l) => s.loc_access(l),
            None => s.region_reached(w.region(at).unwrap(), None),
        }
    }

    #[test]
    fn each_fire_key_door_costs_its_place_and_one_key_of_slack_when_keys_roam() {
        let d: Data = Data::load().unwrap();
        let doors: [(usize, &str); 7] = [
            (1, "Fire Temple Big Lava Room Lower Open Door Chest"),
            (3, "Fire Temple Elevator Room"),
            (4, "Fire Temple Map Chest"),
            (5, "Fire Temple Compass Chest"),
            (6, "Fire Temple Flame Maze Far Side"),
            (7, "Fire Temple Megaton Hammer Chest"),
            (8, "Fire Temple Boss Key Chest"),
        ];
        for (n, at) in doors {
            let roam: usize = if n == 8 { n } else { n + 1 };
            assert!(!opens_with(&d, "vanilla", n - 1, at), "vanilla {at} opens with {}", n - 1);
            assert!(opens_with(&d, "vanilla", n, at), "vanilla {at} stays shut with {n}");
            assert!(!opens_with(&d, "shuffled", roam - 1, at), "shuffled {at} opens with {}", roam - 1);
            assert!(opens_with(&d, "shuffled", roam, at), "shuffled {at} stays shut with {roam}");
        }
    }

    // The video places the three Goron key chests of rooms 1, 2 and 5, so they left
    // their pool. The Map big chest in the lower boulder maze carries the N64 Side Room
    // name, which had no remake chest. The flame maze Goron chest keeps the N64 Compass
    // name but holds the eighth key, the one the N64 Side Room chest held.
    const OPENED: [(&str, &str); 5] = [
        ("Fire Temple Near Boss Chest", "Small Key (Fire Temple)"),
        ("Fire Temple Big Lava Room Lower Open Door Chest", "Small Key (Fire Temple)"),
        ("Fire Temple Boulder Maze Lower Chest", "Small Key (Fire Temple)"),
        ("Fire Temple Boulder Maze Side Room Chest", "Map (Fire Temple)"),
        ("Fire Temple Compass Chest", "Small Key (Fire Temple)"),
    ];

    #[test]
    fn the_confirmed_key_chests_and_the_map_big_chest_are_shuffled() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        assert!(d.cat.locs.iter().all(|l| l.pool.as_deref() != Some("pool-Fire_Temple_Near_Boss_Chest-8")));
        let mut s: Settings = Settings::default();
        s.set("shuffle_smallkeys", Value::s("keysanity")).unwrap();
        s.set("shuffle_mapcompass", Value::s("keysanity")).unwrap();
        let p: Plan = plan(&s).unwrap();
        let free: Vec<String> = pick(&d, &w, &p).free.iter().map(|l| w.locs[*l].name.clone()).collect();
        for (n, item) in OPENED {
            assert!(chest_ok(&d, n), "{n} is {:?}", d.state_of(n));
            assert_eq!(d.vanilla_of(n), Some(item), "{n}");
            let actor: Option<&str> = d.item(item).and_then(|i| i.actor.as_deref());
            assert_eq!(actor, d.cat.by_name(n).map(|c| c.item.as_str()), "{n}");
            assert!(free.iter().any(|f| f == n), "{n} is not shuffled");
        }
    }
}

