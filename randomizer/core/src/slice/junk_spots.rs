use std::collections::BTreeMap as BM;

use crate::logic_items::ItemId;
use crate::slice::data::Data;
use crate::slice::skull_names::{shared, Grade};
use crate::slice::skulls::{Mode, TOKEN};
use crate::slice::support::Plan;
use crate::world::{LocId, World};

pub const SHORT: &str = "too few junk items are shuffled to fill the unsure skulltulas";
pub const SLACK: usize = 2;

pub fn only(p: &Plan, name: &str) -> bool {
    p.skulls == Mode::Matched && shared().and_then(|t| t.spot(name)).is_some_and(|s| s.grade != Grade::Release)
}

pub fn is_junk(d: &Data, item: &str) -> bool {
    item != TOKEN && d.item(item).is_some_and(|i| !i.adv)
}

pub fn refuses(d: &Data, w: &World, p: &Plan, it: ItemId, l: LocId) -> bool {
    only(p, &w.locs[l].name) && !is_junk(d, &w.items.names[it])
}

fn fits(d: &Data, item: &str) -> bool {
    is_junk(d, item) && crate::slice::skulls::shared().is_some_and(|t| crate::slice::skulls::pickup(t, d, item).is_some())
}

pub fn trim(d: &Data, w: &World, p: &Plan, free: &mut Vec<LocId>, left: &mut BM<String, usize>) {
    let spots: Vec<LocId> = free.iter().copied().filter(|l| only(p, &w.locs[*l].name)).collect();
    if spots.is_empty() {
        return;
    }
    let have: usize = free.iter().filter(|l| !spots.contains(l) && d.vanilla_of(&w.locs[**l].name).is_some_and(|i| fits(d, i))).count();
    if have >= SLACK * spots.len() {
        return;
    }
    free.retain(|l| !spots.contains(l));
    *left.entry(SHORT.to_string()).or_insert(0) += spots.len();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Settings, Value};
    use crate::slice::generate::{generate_data, GenOpts, Generated};
    use crate::slice::logic::world_with;
    use crate::slice::pick::{pick, Pick};
    use crate::slice::support::plan;

    fn with(pairs: &[(&str, &str)]) -> Settings {
        let mut s: Settings = Settings::default();
        for (k, v) in pairs {
            s.set(k, Value::s(v)).unwrap();
        }
        s
    }

    #[test]
    fn junk_is_never_progression_or_a_token() {
        let d: Data = Data::load().unwrap();
        assert!(is_junk(&d, "Rupees (20)"));
        assert!(is_junk(&d, "Recovery Heart"));
        assert!(!is_junk(&d, TOKEN));
        assert!(!is_junk(&d, "Progressive Hookshot"));
        assert!(!is_junk(&d, "Piece of Heart"));
        assert!(!is_junk(&d, "Small Key (Forest Temple)"));
    }

    #[test]
    fn only_unsure_skulltulas_in_matched_mode_are_junk_spots() {
        let m: Plan = plan(&with(&[("shuffle_skulltulas", "matched")])).unwrap();
        let p: Plan = plan(&with(&[("shuffle_skulltulas", "pinned")])).unwrap();
        assert!(only(&m, "Colossus GS Tree"));
        assert!(only(&m, "HF GS Near Kak Grotto"));
        assert!(!only(&m, "Colossus GS Hill"));
        assert!(!only(&m, "KF GS Bean Patch"));
        assert!(!only(&p, "Colossus GS Tree"));
    }

    #[test]
    fn matched_shuffles_all_hundred_and_drops_the_junk_spots_without_junk() {
        let d: Data = Data::load().unwrap();
        let s: Settings = with(&[("shuffle_skulltulas", "matched"), ("shuffle_pots", "all"), ("shuffle_crates", "all")]);
        let p: Plan = plan(&s).unwrap();
        let w: World = world_with(&d, &p.logic()).unwrap();
        let pk: Pick = pick(&d, &w, &p);
        let skulls: usize = pk.free.iter().filter(|l| crate::slice::skulls::is_skull(&d, &w.locs[**l].name)).count();
        assert_eq!(skulls, 100);
        let bare: Settings = with(&[("shuffle_skulltulas", "matched"), ("shuffle_chests", "off"), ("shuffle_rewards", "off")]);
        let q: Plan = plan(&bare).unwrap();
        let v: World = world_with(&d, &q.logic()).unwrap();
        let pq: Pick = pick(&d, &v, &q);
        assert!(!pq.free.iter().any(|l| only(&q, &v.locs[*l].name)));
        assert_eq!(pq.left.get(SHORT).copied(), Some(18), "{:?}", pq.left);
    }

    #[test]
    fn junk_spots_only_hold_junk_in_real_seeds() {
        let d: Data = Data::load().unwrap();
        let s: Settings = with(&[("shuffle_skulltulas", "matched"), ("shuffle_pots", "all"), ("shuffle_crates", "all")]);
        let p: Plan = plan(&s).unwrap();
        for n in 0..3 {
            let g: Generated = generate_data(&d, &s, &format!("junk spots {n}"), &GenOpts::default()).unwrap();
            let mut seen: usize = 0;
            for (loc, item) in &g.seed_file.placements {
                if only(&p, loc) {
                    seen += 1;
                    assert!(is_junk(&d, item), "{loc} holds {item}");
                }
            }
            assert_eq!(seen, 18);
            assert_eq!(g.seed_file.placements.values().filter(|i| i.as_str() == TOKEN).count(), 100);
        }
    }
}
