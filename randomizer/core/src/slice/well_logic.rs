#[cfg(test)]
mod tests {
    use crate::logic_items::ItemId;
    use crate::search::{explore, Search};
    use crate::slice::data::Data;
    use crate::slice::fill::vanilla_place;
    use crate::slice::logic::world_with;
    use crate::world::{Placement, World};

    const KEY: &str = "Small Key (Bottom of the Well)";

    // Every advancement item three times except the ones named in `skip` and the well keys,
    // then `keys` well keys. The vanilla well keys are taken out of the placement.
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
            if w.items.adv[i] && i != k && !gone.contains(&i) {
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

    // The east rooms past the fake walls open without a key in the remake.
    #[test]
    fn the_deku_shield_chest_and_the_like_like_cage_want_the_lens_and_no_key() {
        let d: Data = Data::load().unwrap();
        for at in ["Bottom of the Well Fire Keese Chest", "Bottom of the Well GS Like Like Cage"] {
            assert!(reach(&d, &[], 0, at), "{at}");
            assert!(!reach(&d, &["Lens of Truth"], 2, at), "{at} without the Lens");
        }
    }

    // Both keys lie outside the two doors and either door can be opened first.
    #[test]
    fn each_inner_room_wants_both_keys() {
        let d: Data = Data::load().unwrap();
        for at in ["Bottom of the Well GS West Inner Room", "Bottom of the Well GS East Inner Room"] {
            assert!(!reach(&d, &[], 1, at), "{at} with one key");
            assert!(reach(&d, &[], 2, at), "{at} with two keys");
        }
    }

    #[test]
    fn the_dead_hand_side_wants_the_lullaby() {
        let d: Data = Data::load().unwrap();
        assert!(!reach(&d, &["Zeldas Lullaby"], 2, "Bottom of the Well Dead Hand Area"));
        assert!(reach(&d, &[], 0, "Bottom of the Well Lens of Truth Chest"));
        assert!(!reach(&d, &["Lens of Truth"], 2, "Bottom of the Well Invisible Chest"));
        assert!(!reach(&d, &["Kokiri Sword"], 2, "Bottom of the Well Lens of Truth Chest"));
    }
}
