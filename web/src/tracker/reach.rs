use ootrr_core::logic_items::ItemId;
use ootrr_core::rules::Age;
use ootrr_core::search::Search;
use ootrr_core::slice::rebuild::Rebuilt;
use ootrr_core::world::{LocId, Placement};

pub fn open_now(r: &Rebuilt, checked: &[bool]) -> Vec<bool> {
    let mut p: Placement = r.got.clone();
    let mut have: Vec<ItemId> = Vec::new();
    for (k, l) in r.free.iter().enumerate() {
        if checked.get(k).copied().unwrap_or(false)
            && let Some(i) = p.get(*l)
        {
            have.push(i);
        }
        p.item[*l] = None;
    }
    let mut s: Search = Search::new(&r.w);
    s.add_all(&have);
    s.settle(&p, true);
    r.free.iter().map(|l: &LocId| s.loc_access(*l)).collect()
}

pub fn only_age(r: &Rebuilt) -> Vec<Option<Age>> {
    let mut s: Search = Search::new(&r.w);
    s.settle(&r.got, true);
    r.free
        .iter()
        .map(|l: &LocId| match (s.loc_access_as(*l, Age::Child), s.loc_access_as(*l, Age::Adult)) {
            (true, false) => Some(Age::Child),
            (false, true) => Some(Age::Adult),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ootrr_core::settings::Settings;
    use ootrr_core::slice::generate::generate;
    use ootrr_core::slice::rebuild::rebuild;
    use ootrr_core::spoiler::playthrough;

    #[test]
    fn checking_every_sphere_in_order_opens_the_whole_seed() {
        let s: Settings = Settings::default();
        let g = generate(&s, "reach").unwrap();
        let r: Rebuilt = rebuild(&s, "reach", &g.seed_file.placements).unwrap();
        let mut checked: Vec<bool> = vec![false; r.free.len()];
        let first: Vec<bool> = open_now(&r, &checked);
        let n0: usize = first.iter().filter(|x| **x).count();
        assert!(n0 > 0 && n0 < r.free.len());
        let sp = playthrough(&r.w, &r.got).unwrap();
        for sphere in &sp.spheres {
            let open: Vec<bool> = open_now(&r, &checked);
            for (l, _) in sphere {
                if let Some(k) = r.free.iter().position(|x| x == l) {
                    assert!(open[k], "{} should be open", r.w.locs[*l].name);
                    checked[k] = true;
                }
            }
        }
        let last: Vec<bool> = open_now(&r, &checked);
        assert!(last.iter().filter(|x| **x).count() > n0);
    }

    #[test]
    fn age_bound_spots_name_their_age_and_shared_ones_name_none() {
        let s: Settings = Settings::default();
        let g = generate(&s, "ages").unwrap();
        let r: Rebuilt = rebuild(&s, "ages", &g.seed_file.placements).unwrap();
        let ages: Vec<Option<Age>> = only_age(&r);
        let of = |n: &str| r.free.iter().position(|l| r.w.locs[*l].name == n).map(|k| ages[k]);
        assert_eq!(of("LH Adult Fishing"), Some(Some(Age::Adult)));
        assert_eq!(of("LH Child Fishing"), Some(Some(Age::Child)));
        assert!(ages.iter().any(|a| a.is_none()));
    }

    #[test]
    fn the_lake_poe_chest_waits_for_the_water_temple() {
        let s: Settings = Settings::default();
        let g = generate(&s, "lake").unwrap();
        let r: Rebuilt = rebuild(&s, "lake", &g.seed_file.placements).unwrap();
        let l: LocId = r.w.locs.iter().position(|x| x.name == "HF Lake Poe Chest").unwrap();
        assert_eq!(r.w.regions[r.w.locs[l].region].name, "Lake Hylia");
        if let Some(k) = r.free.iter().position(|x| *x == l) {
            assert!(!open_now(&r, &vec![false; r.free.len()])[k]);
        }
    }
}
