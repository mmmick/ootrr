#[cfg(test)]
mod tests {
    use crate::logic_items::ItemId;
    use crate::rules::Age;
    use crate::search::{explore, Search};
    use crate::slice::data::Data;
    use crate::slice::logic::{world, world_with};
    use crate::world::{Placement, World};

    const ROOM: &str = "Dodongos Cavern Upper Lizalfos";
    const BRIDGE: &str = "Dodongos Cavern Far Bridge";

    fn without_bow(w: &World, d: &Data) -> Placement {
        let mut p: Placement = crate::slice::fill::vanilla_place(w, d).unwrap();
        let bow: ItemId = w.item("Bow").unwrap();
        let f: ItemId = w.item("Rupees (5)").unwrap();
        for l in &w.real {
            if p.get(*l) == Some(bow) {
                p.put(*l, f);
            }
        }
        p
    }

    fn adult_at(w: &World, p: &Placement, more: &[&str], region: &str) -> bool {
        let extra: Vec<ItemId> = ["Hover Boots"].iter().chain(more.iter()).map(|n| w.item(n).unwrap()).collect();
        let s: Search = explore(w, p, &extra);
        s.region_reached(w.region(region).unwrap(), Some(Age::Adult))
    }

    #[test]
    fn the_upper_lizalfos_room_opens_only_from_the_eye_switch_door() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let p: Placement = without_bow(&w, &d);
        assert!(adult_at(&w, &p, &[], BRIDGE));
        assert!(!adult_at(&w, &p, &[], ROOM));
        assert!(adult_at(&w, &p, &["Bow"], ROOM));
    }

    fn stripped(w: &World, d: &Data, names: &[&str]) -> Placement {
        let mut p: Placement = crate::slice::fill::vanilla_place(w, d).unwrap();
        let f: ItemId = w.item("Rupees (5)").unwrap();
        let gone: Vec<ItemId> = names.iter().map(|n| w.item(n).unwrap()).collect();
        for l in &w.real {
            if p.get(*l).is_some_and(|i| gone.contains(&i)) {
                p.put(*l, f);
            }
        }
        p
    }

    #[test]
    fn an_adult_needs_a_bow_or_dins_fire_for_the_torch_room_switch() {
        let d: Data = Data::load().unwrap();
        let w: World = world_with(&d, &[("open_door_of_time", "open")]).unwrap();
        let p: Placement = stripped(&w, &d, &["Bow", "Dins Fire", "Bomb Bag", "Progressive Strength Upgrade"]);
        let torch: &str = "Dodongos Cavern Torch Room";
        assert!(adult_at(&w, &p, &["Megaton Hammer"], "Dodongos Cavern Lobby"));
        assert!(!adult_at(&w, &p, &["Megaton Hammer"], torch));
        assert!(adult_at(&w, &p, &["Megaton Hammer", "Bow"], torch));
        assert!(adult_at(&w, &p, &["Megaton Hammer", "Dins Fire", "Magic Meter"], torch));
    }

    #[test]
    fn the_hammer_opens_the_mouth_without_explosives() {
        let d: Data = Data::load().unwrap();
        let w: World = world_with(&d, &[("open_door_of_time", "open")]).unwrap();
        let p: Placement = stripped(&w, &d, &["Bomb Bag", "Megaton Hammer"]);
        let mouth: &str = "Dodongos Cavern Mouth";
        let base: [&str; 2] = ["Bow", "Progressive Strength Upgrade"];
        assert!(adult_at(&w, &p, &base, "Dodongos Cavern Far Bridge"));
        assert!(!adult_at(&w, &p, &base, mouth));
        assert!(adult_at(&w, &p, &["Bow", "Progressive Strength Upgrade", "Megaton Hammer"], mouth));
    }

    fn reached(w: &World, p: &Placement, more: &[&str], loc: &str) -> bool {
        let extra: Vec<ItemId> = more.iter().map(|n| w.item(n).unwrap()).collect();
        explore(w, p, &extra).loc_access(w.loc(loc).unwrap())
    }

    #[test]
    fn jabus_map_sits_before_the_boomerang_and_the_compass_after_it() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let p: Placement = stripped(&w, &d, &["Boomerang"]);
        assert!(reached(&w, &p, &[], "Jabu Jabus Belly Map Chest"));
        assert!(reached(&w, &p, &[], "Jabu Jabus Belly Deku Scrub"));
        assert!(!reached(&w, &p, &[], "Jabu Jabus Belly Compass Chest"));
        assert!(reached(&w, &p, &["Boomerang"], "Jabu Jabus Belly Compass Chest"));
    }

    #[test]
    fn child_bosses_want_the_kokiri_sword_and_not_sticks() {
        let d: Data = Data::load().unwrap();
        let w: World = world(&d).unwrap();
        let p: Placement = stripped(&w, &d, &["Kokiri Sword"]);
        for boss in ["Deku Tree Queen Gohma Heart", "Jabu Jabus Belly Barinade Heart"] {
            assert!(!reached(&w, &p, &["Deku Stick (1)"], boss), "{boss}");
            assert!(reached(&w, &p, &["Kokiri Sword"], boss), "{boss}");
        }
    }
}
