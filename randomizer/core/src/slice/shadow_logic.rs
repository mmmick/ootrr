#[cfg(test)]
mod tests {
    use crate::logic_items::ItemId;
    use crate::search::{explore, Search};
    use crate::slice::data::Data;
    use crate::slice::fill::vanilla_place;
    use crate::slice::logic::world_with;
    use crate::world::{Placement, World};

    const KEY: &str = "Small Key (Shadow Temple)";
    const BOMBS: [&str; 5] = ["Bomb Bag", "Bombchus", "Bombchus (5)", "Bombchus (10)", "Bombchus (20)"];

    // Every advancement item three times except the ones named in `skip` and the Shadow keys,
    // then `keys` Shadow keys. The vanilla Shadow keys and the skipped items leave the placement.
    fn reach(d: &Data, skip: &[&str], keys: usize, at: &str) -> bool {
        let w: World = world_with(d, &[("open_door_of_time", "open")]).unwrap();
        let k: ItemId = w.item(KEY).unwrap();
        let gone: Vec<ItemId> = skip.iter().map(|n| w.item(n).unwrap()).collect();
        let mut p: Placement = vanilla_place(&w, d).unwrap();
        for l in &w.real {
            if p.get(*l).is_some_and(|i| i == k || gone.contains(&i)) {
                p.item[*l] = None;
            }
        }
        let mut extra: Vec<ItemId> = Vec::new();
        for (i, name) in w.items.names.iter().enumerate() {
            if w.items.adv[i] && i != k && !gone.contains(&i) && !name.starts_with("Bombchu") {
                extra.extend([i; 3]);
            }
        }
        extra.extend(vec![k; keys]);
        let s: Search = explore(&w, &p, &extra);
        match w.loc(at) {
            Some(l) => s.loc_access(l),
            None => s.region_reached(w.region(at).unwrap(), None),
        }
    }

    #[test]
    fn the_five_doors_open_in_a_line() {
        let d: Data = Data::load().unwrap();
        let doors: [(usize, &str); 5] = [
            (1, "Shadow Temple Huge Pit"),
            (2, "Shadow Temple Invisible Spikes"),
            (3, "Shadow Temple Wind Tunnel"),
            (4, "Shadow Temple Boat"),
            (5, "Shadow Temple Boss Key Chest"),
        ];
        for (n, at) in doors {
            assert!(!reach(&d, &[], n - 1, at), "{at} opens with {}", n - 1);
            assert!(reach(&d, &[], n, at), "{at} stays shut with {n}");
        }
        assert!(!reach(&d, &[], 2, "Shadow Temple GS Single Giant Pot"));
        assert!(reach(&d, &[], 3, "Shadow Temple GS Single Giant Pot"));
    }

    // Bombs replace Din's Fire at the spike trap and the Bow at the statue bridge.
    #[test]
    fn bombs_open_the_boss_key_trap_and_drop_the_statue() {
        let d: Data = Data::load().unwrap();
        assert!(!reach(&d, &BOMBS, 5, "Shadow Temple Boss Key Chest"));
        // Without bombs the boss door is still reached the N64 way, through Pierre past the boat,
        // or by a Farore's Wind warp set there earlier.
        let no_pierre: [&str; 7] = ["Bomb Bag", "Bombchus", "Bombchus (5)", "Bombchus (10)", "Bombchus (20)", "Scarecrow Song", "Farores Wind"];
        assert!(reach(&d, &BOMBS, 5, "Shadow Temple Before Boss"));
        assert!(!reach(&d, &no_pierre, 5, "Shadow Temple Before Boss"));
        assert!(reach(&d, &["Bow"], 5, "Shadow Temple Before Boss"));
        assert!(reach(&d, &["Bow"], 5, "Shadow Temple Boss Key Chest"));
    }

    #[test]
    fn the_ship_skulltula_wants_pierre() {
        let d: Data = Data::load().unwrap();
        assert!(!reach(&d, &["Scarecrow Song"], 4, "Shadow Temple GS Near Ship"));
        assert!(reach(&d, &[], 4, "Shadow Temple GS Near Ship"));
    }
}


