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

    const KEY: &str = "Small Key (Water Temple)";

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

    // The Dark Link room opens onto the river through bars the Longshot reaches past; the
    // N64 Song of Time block is gone.
    #[test]
    fn the_river_wants_the_longshot_and_not_the_song_of_time() {
        let d: Data = Data::load().unwrap();
        let w: World = open(&d);
        let p: Placement = stripped(&w, &d, &["Progressive Hookshot", "Song of Time"]);
        assert!(region(&w, &p, &["Progressive Hookshot"], "Water Temple Dark Link Region"));
        assert!(!region(&w, &p, &["Progressive Hookshot"], "Water Temple River"));
        assert!(region(&w, &p, &["Progressive Hookshot", "Progressive Hookshot"], "Water Temple River"));
        assert!(reached(&w, &p, &["Progressive Hookshot", "Progressive Hookshot"], "Water Temple GS River"));
    }

    #[test]
    fn the_compass_wants_explosives_and_the_torches_want_fire() {
        let d: Data = Data::load().unwrap();
        let w: World = open(&d);
        let p: Placement = stripped(&w, &d, &["Bomb Bag", "Bow", "Dins Fire", "Bombchus (5)", "Bombchus (10)", "Bombchus (20)"]);
        assert!(!reached(&w, &p, &[], "Water Temple Compass Chest"));
        assert!(reached(&w, &p, &["Bomb Bag"], "Water Temple Compass Chest"));
        assert!(!reached(&w, &p, &["Bomb Bag"], "Water Temple Torches Chest"));
        assert!(reached(&w, &p, &["Bomb Bag", "Bow"], "Water Temple Torches Chest"));
    }

    // The 2F rooms hang on the central pillar: no Lullaby, no water at 2F.
    #[test]
    fn the_second_floor_is_reached_through_the_central_pillar() {
        let d: Data = Data::load().unwrap();
        let w: World = open(&d);
        let p: Placement = stripped(&w, &d, &["Progressive Hookshot"]);
        let all: [&str; 2] = ["Progressive Hookshot", "Progressive Hookshot"];
        assert!(region(&w, &p, &all, "Water Temple Central Pillar"));
        assert!(reached(&w, &p, &all, "Water Temple GS Central Pillar"));
        assert!(reached(&w, &p, &all, "Water Temple Central Pillar Chest"));
        assert!(reached(&w, &p, &all, "Water Temple Compass Room Chest"));
        assert!(reached(&w, &p, &all, "Water Temple Central Bow Target Chest"));
        assert!(!reached(&w, &p, &["Progressive Hookshot"], "Water Temple Central Bow Target Chest"));
    }

    fn opens_with(d: &Data, lay: &str, n: usize, at: &str) -> bool {
        let w: World = world_with(d, &[("open_door_of_time", "open"), ("water_key_layout", lay)]).unwrap();
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

    // The four doors open in a fixed order. The 3F door can be reached on the first visit,
    // before the water is lowered, so roaming keys carry one key of slack until the last door.
    #[test]
    fn each_water_key_door_costs_its_place_and_one_key_of_slack_when_keys_roam() {
        let d: Data = Data::load().unwrap();
        let doors: [(usize, &str); 4] = [
            (1, "Water Temple Central Pillar"),
            (2, "Water Temple High Water Room"),
            (3, "Water Temple Falling Platform Room"),
            (4, "Water Temple North Basement"),
        ];
        for (n, at) in doors {
            let roam: usize = if n == 4 { n } else { n + 1 };
            assert!(!opens_with(&d, "vanilla", n - 1, at), "vanilla {at} opens with {}", n - 1);
            assert!(opens_with(&d, "vanilla", n, at), "vanilla {at} stays shut with {n}");
            assert!(!opens_with(&d, "shuffled", roam - 1, at), "shuffled {at} opens with {}", roam - 1);
            assert!(opens_with(&d, "shuffled", roam, at), "shuffled {at} stays shut with {roam}");
        }
    }

    // The video places the room 8 and room 20 key chests, so the N64 Dragon and Central Bow
    // Target names hold them.
    #[test]
    fn the_paired_key_chests_are_shuffled() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let mut s: Settings = Settings::default();
        s.set("shuffle_smallkeys", Value::s("keysanity")).unwrap();
        let p: Plan = plan(&s).unwrap();
        let free: Vec<String> = pick(&d, &w, &p).free.iter().map(|l| w.locs[*l].name.clone()).collect();
        for n in ["Water Temple Dragon Chest", "Water Temple Central Bow Target Chest", "Water Temple Torches Chest", "Water Temple Compass Room Chest"] {
            assert!(chest_ok(&d, n), "{n} is {:?}", d.state_of(n));
            assert_eq!(d.vanilla_of(n), Some(KEY), "{n}");
            assert!(free.iter().any(|f| f == n), "{n} is not shuffled");
        }
    }
}
