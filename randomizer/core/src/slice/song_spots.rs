use crate::error::{Error as E, Res};
use crate::logic_items::ItemId;
use crate::rng::Rng;
use crate::search::Search;
use crate::slice::data::{Data, Item};
use crate::slice::fill::SHORT;
use crate::slice::fit::{Fit, Grid};
use crate::slice::generate::is_song;
use crate::slice::support::{Plan, Songs};
use crate::world::{LocId, Placement, World};

pub const KIND: &str = "Song";

pub fn is_song_item(i: &Item) -> bool {
    i.kind == KIND && i.actor.as_deref().is_some_and(is_song)
}

pub fn pickup_spot(d: &Data, name: &str) -> bool {
    crate::slice::skulls::is_skull(d, name) || d.pots.has(name) || d.cat.by_name(name).is_some_and(crate::slice::pickups::is_world) || name == crate::slice::treasure::LOC
}

pub fn gate(d: &Data, p: &Plan, name: &str, item: &str) -> Option<bool> {
    if p.songs == Songs::Anywhere || !d.item(item).is_some_and(is_song_item) {
        return None;
    }
    pickup_spot(d, name).then_some(false)
}

pub const TRIES: u32 = 8;
pub const QUICK_TRIES: u32 = 40;
pub const QUICK_SWITCH: u32 = 10;

pub fn first(d: &Data, w: &World, base: &Placement, free: &[LocId], plan: &[(LocId, ItemId)], ok: &dyn Fn(ItemId, LocId) -> bool, rng: &mut Rng) -> Res<Vec<(LocId, ItemId)>> {
    first_with(d, w, base, free, plan, ok, rng, false)
}

pub fn first_with(d: &Data, w: &World, base: &Placement, free: &[LocId], plan: &[(LocId, ItemId)], ok: &dyn Fn(ItemId, LocId) -> bool, rng: &mut Rng, fit: bool) -> Res<Vec<(LocId, ItemId)>> {
    let mut fixed: Placement = base.clone();
    let mut pool: Vec<ItemId> = free.iter().filter_map(|l| base.get(*l)).collect();
    for l in free {
        fixed.item[*l] = None;
    }
    for (l, i) in plan {
        if let Some(k) = pool.iter().position(|x| x == i) {
            pool.swap_remove(k);
        }
        fixed.put(*l, *i);
    }
    let mut songs: Vec<ItemId> = pool.iter().copied().filter(|i| d.item(&w.items.names[*i]).is_some_and(is_song_item)).collect();
    songs.sort();
    rng.shuffle(&mut songs);
    let mut open: Vec<LocId> = free.iter().copied().filter(|l| !plan.iter().any(|x| x.0 == *l)).collect();
    let spots: Vec<LocId> = open.clone();
    let none: [ItemId; 0] = [];
    let g: Grid = Grid::new(if fit { pool.as_slice() } else { &none }, &spots, ok);
    let mut f: Fit = Fit::new(&g);
    if fit && !f.full() {
        return Err(E::Msg(SHORT.to_string()));
    }
    let prog: Vec<ItemId> = pool.iter().copied().filter(|i| w.items.adv[*i]).collect();
    let mut assumed: Search = Search::new(w);
    assumed.settle(&fixed, true);
    assumed.add_all(&prog);
    let mut out: Vec<(LocId, ItemId)> = Vec::with_capacity(songs.len());
    for it in songs {
        assumed.remove(it);
        let mut mx: Search = assumed.clone();
        mx.settle(&fixed, true);
        let mut cands: Vec<LocId> = open.iter().copied().filter(|l| ok(it, *l) && mx.loc_access(*l)).collect();
        let p: Option<usize> = if fit { pool.iter().position(|x| *x == it) } else { None };
        let mut pick: Option<LocId> = None;
        while !cands.is_empty() {
            let k: usize = rng.below(cands.len() as u64) as usize;
            let l: LocId = cands[k];
            let kept: bool = match (p, spots.iter().position(|x| *x == l)) {
                (Some(p), Some(j)) => f.take(p, j),
                _ => true,
            };
            if kept {
                pick = Some(l);
                break;
            }
            cands.swap_remove(k);
        }
        let Some(l) = pick else {
            return Err(E::Msg(format!("no spot for {} when the songs are placed first", w.items.names[it])));
        };
        open.retain(|x| *x != l);
        fixed.put(l, it);
        out.push((l, it));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Settings, Value};
    use crate::slice::support::plan;

    fn with(songs: &str) -> Plan {
        let mut s: Settings = Settings::default();
        s.set("shuffle_songs", Value::s(songs)).unwrap();
        plan(&s).unwrap()
    }

    #[test]
    fn songs_reach_pickup_spots_only_in_anywhere_mode() {
        let d: Data = Data::load().unwrap();
        let free: &str = "LLR Freestanding PoH";
        let skull: &str = "KF GS Bean Patch";
        let pot: String = d.pots.state.keys().min().cloned().unwrap();
        for n in [free, skull, pot.as_str(), crate::slice::treasure::LOC] {
            assert!(pickup_spot(&d, n), "{n}");
            assert_eq!(gate(&d, &with("own"), n, "Suns Song"), Some(false), "{n}");
            assert_eq!(gate(&d, &with("off"), n, "Suns Song"), Some(false), "{n}");
            assert_eq!(gate(&d, &with("anywhere"), n, "Suns Song"), None, "{n}");
            assert_eq!(gate(&d, &with("own"), n, "Boomerang"), None, "{n}");
        }
        assert_eq!(gate(&d, &with("own"), "KF Midos Top Left Chest", "Suns Song"), None);
        assert!(d.item("Scarecrow Song").is_some_and(|i| !is_song_item(i)));
        assert_eq!(d.items.iter().filter(|i| is_song_item(i)).count(), 12);
    }

    #[test]
    fn placing_songs_first_reaches_pickup_spots_and_only_in_anywhere_mode() {
        use crate::slice::generate::{generate_data, GenOpts, Generated};
        let d: Data = Data::load().unwrap();
        let mut at_pickups: usize = 0;
        for mode in ["anywhere", "own"] {
            let mut s: Settings = Settings::default();
            for (k, v) in [("shuffle_songs", mode), ("shuffle_pots", "all"), ("shuffle_crates", "all"), ("shuffle_freestanding_items", "all")] {
                s.set(k, Value::s(v)).unwrap();
            }
            for n in 0..3 {
                let g: Generated = generate_data(&d, &s, &format!("songs first {n}"), &GenOpts::default()).unwrap();
                let here: usize = g.seed_file.placements.iter().filter(|(l, i)| pickup_spot(&d, l) && d.item(i).is_some_and(is_song_item)).count();
                if mode == "own" {
                    assert_eq!(here, 0);
                }
                at_pickups += here;
            }
        }
        assert!(at_pickups > 0);
    }

    #[test]
    fn songs_placed_first_leave_room_for_four_shelf_deals_per_shop() {
        use crate::slice::generate::{generate_data, GenOpts, Generated};
        let d: Data = Data::load().unwrap();
        let mut s: Settings = Settings::default();
        for (k, v) in [("shuffle_shops", "deals_4"), ("shuffle_chests", "all"), ("shuffle_songs", "anywhere")] {
            s.set(k, Value::s(v)).unwrap();
        }
        s.set("shopsanity_prices", Value::s("giant")).unwrap();
        for n in 0..2 {
            let g: Generated = generate_data(&d, &s, &format!("room for deals {n}"), &GenOpts::default()).unwrap_or_else(|e| panic!("{n}: {e}"));
            let songs: usize = g.seed_file.placements.values().filter(|i| d.item(i).is_some_and(is_song_item)).count();
            assert_eq!(songs, 12, "{n}");
            assert!(g.seed_file.shops.values().filter(|e| e.deal).count() > 0, "{n}");
        }
    }

    #[test]
    fn a_song_at_a_pickup_spot_needs_its_harp_pack_and_at_a_skulltula_its_kit() {
        use crate::settings::seed::SeedFile;
        use crate::slice::seed_packs::{need, Kind, Want};
        use std::collections::BTreeMap as BM;
        let d: Data = Data::load().unwrap();
        let pairs: [(&str, &str); 2] = [("LLR Freestanding PoH", "Suns Song"), ("KF GS Bean Patch", "Sarias Song")];
        let m: BM<String, String> = pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        let s: SeedFile = SeedFile::new("s", &Settings::default(), "g", m).unwrap();
        let w: BM<String, Want> = need(&d, &s, &BM::new()).unwrap();
        // Besides the placed songs' packs, every seed builds the song and Ocarina pickups the story
        // skips hold up (crate::ocarina_hold::wanted).
        let scene: Vec<String> = crate::ocarina_hold::wanted(&d, &s).into_iter().map(|(_, p)| p.pack).collect();
        for x in ["Obj_Melody_SariaSong_GH", "Obj_Melody_SunSong"] {
            assert!(w.contains_key(x), "{x}");
        }
        assert!(w.keys().all(|k| ["Obj_Melody_SariaSong_GH", "Obj_Melody_SunSong"].contains(&k.as_str()) || scene.contains(k)), "{:?}", w.keys());
        assert_eq!(w["Obj_Melody_SunSong"].kind, Kind::Free);
        for x in ["Obj_Melody_SariaSong_GH", "Obj_Melody_SunSong"].map(|k| &w[k]) {
            assert_eq!(x.spec().unwrap().1.model.as_deref(), Some(crate::pickup_pack::model::HARP));
        }
    }
}
