use std::collections::{BTreeMap as BM, BTreeSet as BS};

use crate::error::{Error as E, Res};
use crate::rng::Rng;
use crate::shops::data::{table, Slot, Table};
use crate::shops::plan::{Kinds, Level, Opts};
use crate::shops::price;
use crate::world::{LocId, World};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub loc: String,
    pub item: String,
    pub actor: String,
    pub price: i32,
    pub deal: bool,
    pub pool: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Room {
    pub any: usize,
    pub once: usize,
}

impl Room {
    pub fn any(n: usize) -> Room {
        Room { any: n, once: 0 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Layout {
    pub places: Vec<Place>,
    pub cut: usize,
}

#[derive(Debug, Clone)]
struct Goods {
    item: String,
    actor: String,
    price: i32,
}

impl Layout {
    pub fn get(&self, loc: &str) -> Option<&Place> {
        self.places.iter().find(|p| p.loc == loc)
    }

    pub fn deals(&self) -> impl Iterator<Item = &Place> {
        self.places.iter().filter(|p| p.deal)
    }

    pub fn deal_count(&self) -> usize {
        self.deals().count()
    }

    pub fn logic_item(&self, loc: &str) -> Option<&str> {
        self.get(loc).map(|p| if p.deal { p.pool.as_str() } else { p.item.as_str() })
    }

    pub fn add_free(&self, w: &World, free: &mut Vec<LocId>) -> Res<()> {
        for p in self.deals() {
            let l: LocId = w.loc(&p.loc).ok_or_else(|| E::Msg(format!("{} is not in the logic world", p.loc)))?;
            free.push(l);
        }
        free.sort_unstable();
        free.dedup();
        Ok(())
    }
}

fn units(t: &Table) -> Vec<(String, Vec<&Slot>)> {
    let mut out: Vec<(String, Vec<&Slot>)> = Vec::new();
    for s in &t.slots {
        let k: String = t.unit(s);
        match out.iter_mut().find(|u| u.0 == k) {
            Some(u) => u.1.push(s),
            None => out.push((k, vec![s])),
        }
    }
    out
}

fn deal_slots(t: &Table, o: &Opts, room: Room, rng: &mut Rng) -> (BS<String>, usize) {
    let mut all: Vec<(usize, usize, String)> = Vec::new();
    for (si, sh) in t.shops.iter().enumerate().filter(|(_, s)| s.deals) {
        let n: usize = match o.level {
            Level::Deals(n) => n as usize,
            Level::Random if si == 0 => 1 + rng.below(4) as usize,
            Level::Random => rng.below(5) as usize,
            _ => 0,
        };
        for (k, i) in sh.deal_order.iter().take(n.min(8)).enumerate() {
            if let Some(s) = t.at(&sh.shop, *i) {
                all.push((k, si, s.name.clone()));
            }
        }
    }
    all.sort();
    let (mut any, mut once): (usize, usize) = (room.any, room.once);
    let mut out: BS<String> = BS::new();
    let mut cut: usize = 0;
    for (_, si, name) in all {
        if t.shops[si].once && once > 0 {
            once -= 1;
        } else if any > 0 {
            any -= 1;
        } else {
            cut += 1;
            continue;
        }
        out.insert(name);
    }
    (out, cut)
}

fn rupee(t: &Table, rng: &mut Rng) -> String {
    let total: u64 = t.rupees.iter().map(|r| r.1).sum();
    let mut x: u64 = rng.below(total);
    for (n, w) in &t.rupees {
        if x < *w {
            return n.clone();
        }
        x -= w;
    }
    t.rupees[0].0.clone()
}

fn survivors(t: &Table, mut goods: Vec<Goods>, room: usize, bottles: usize) -> Res<Vec<Goods>> {
    let mut kept: Vec<Goods> = Vec::new();
    for k in &t.keep {
        if kept.len() >= room {
            break;
        }
        if let Some(i) = goods.iter().position(|g| &g.item == k) {
            kept.push(goods.remove(i));
        }
    }
    while kept.iter().filter(|g| t.bottle.contains(&g.item)).count() < bottles && kept.len() < room {
        match goods.iter().position(|g| t.bottle.contains(&g.item)) {
            Some(i) => kept.push(goods.remove(i)),
            None => break,
        }
    }
    let rest: usize = room.saturating_sub(kept.len());
    kept.extend(goods.into_iter().take(rest));
    if kept.len() != room {
        return Err(E::Msg(format!("shops: {} shop goods for {room} shelf slots", kept.len())));
    }
    Ok(kept)
}

/// Shop goods a child can buy in any shop (shelf::hook::child_tunics). Moved out of the shop that
/// sells them in the game, they cost a random shopsanity price instead of their own.
pub const TUNICS: [&str; 2] = ["Buy Goron Tunic", "Buy Zora Tunic"];

/// Whether `item` is a tunic standing in a shop other than its own.
pub fn moved_tunic(t: &Table, item: &str, shop: &str) -> bool {
    TUNICS.contains(&item) && !t.slots.iter().any(|x| x.item == item && x.shop == shop)
}

pub fn make(o: &Opts, seed: &str, room: Room) -> Res<Layout> {
    let t: &Table = table()?;
    let mut rng: Rng = Rng::from_text(&format!("{seed}/shops"));
    let (deals, cut): (BS<String>, usize) = deal_slots(t, o, room, &mut rng);
    let us: Vec<(String, Vec<&Slot>)> = units(t);
    let mut goods: Vec<Goods> = us.iter().map(|(_, ss)| Goods { item: ss[0].item.clone(), actor: ss[0].actor.clone(), price: ss[0].price }).collect();
    rng.shuffle(&mut goods);
    let open: Vec<&(String, Vec<&Slot>)> = us.iter().filter(|(_, ss)| !ss.iter().any(|s| deals.contains(&s.name))).collect();
    let potion = |ss: &[&Slot]| ss.len() > 1;
    let bottles: usize = open.iter().filter(|(_, ss)| potion(ss)).count();
    let mut kept: Vec<Goods> = survivors(t, goods, open.len(), bottles)?;
    rng.shuffle(&mut kept);
    let mut given: BM<String, Goods> = BM::new();
    for (item, shop) in &t.pin {
        let Some(i) = kept.iter().position(|g| &g.item == item) else {
            continue;
        };
        let homes: Vec<String> = open.iter().filter(|(k, ss)| !potion(ss) && ss[0].shop == *shop && !given.contains_key(k)).map(|(k, _)| k.clone()).collect();
        if !homes.is_empty() {
            let k: String = homes[rng.below(homes.len() as u64) as usize].clone();
            given.insert(k, kept.remove(i));
        }
    }
    for (k, ss) in open.iter().filter(|(_, ss)| potion(ss)) {
        let i: usize = kept.iter().position(|g| t.bottle.contains(&g.item)).ok_or_else(|| E::Msg(format!("shops: no bottle goods left for the shared Potion Shop slot {}", ss[0].slot)))?;
        given.insert(k.clone(), kept.remove(i));
    }
    let left: Vec<String> = open.iter().filter(|(k, ss)| !potion(ss) && !given.contains_key(k)).map(|(k, _)| k.clone()).collect();
    for k in left {
        given.insert(k, kept.remove(0));
    }
    let mut places: Vec<Place> = Vec::new();
    for s in &t.slots {
        if deals.contains(&s.name) {
            let cap: i32 = t.shop(&s.shop).and_then(|x| x.cap).unwrap_or(i32::MAX);
            let p: i32 = price::draw(o, &mut rng).min(cap);
            let pool: String = if o.kinds != Kinds::default() && !t.flex.is_empty() { t.flex[rng.below(t.flex.len() as u64) as usize].clone() } else { rupee(t, &mut rng) };
            places.push(Place { loc: s.name.clone(), item: String::new(), actor: String::new(), price: p, deal: true, pool });
            continue;
        }
        let g: &Goods = given.get(&t.unit(s)).ok_or_else(|| E::Msg(format!("shops: {} got no goods", s.name)))?;
        let price: i32 = if moved_tunic(t, &g.item, &s.shop) {
            let cap: i32 = t.shop(&s.shop).and_then(|x| x.cap).unwrap_or(i32::MAX);
            price::draw(o, &mut rng).min(cap)
        } else {
            g.price
        };
        places.push(Place { loc: s.name.clone(), item: g.item.clone(), actor: g.actor.clone(), price, deal: false, pool: String::new() });
    }
    Ok(Layout { places, cut })
}

pub fn home() -> Res<Layout> {
    let t: &Table = table()?;
    let places: Vec<Place> = t.slots.iter().map(|s| Place { loc: s.name.clone(), item: s.item.clone(), actor: s.actor.clone(), price: s.price, deal: false, pool: String::new() }).collect();
    Ok(Layout { places, cut: 0 })
}

pub fn of(o: &Opts, seed: &str, room: Room) -> Res<Option<Layout>> {
    if !o.on() {
        return Ok(None);
    }
    make(o, seed, room).map(Some)
}

pub fn fill_deals(l: &mut Layout, got: &BM<String, String>) -> Res<()> {
    for p in l.places.iter_mut().filter(|p| p.deal) {
        p.item = got.get(&p.loc).cloned().ok_or_else(|| E::Msg(format!("the shop deal {} was not filled", p.loc)))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shops::plan::Prices;

    fn opts(l: Level) -> Opts {
        Opts { level: l, prices: Prices::Weighted, min: 10, max: 300, kinds: crate::shops::plan::Kinds::default() }
    }

    fn count(l: &Layout, item: &str) -> usize {
        l.places.iter().filter(|p| !p.deal && p.item == item).count()
    }

    #[test]
    fn shuffle_keeps_every_goods_unit_and_moves_them() {
        let t: &Table = table().unwrap();
        let a: Layout = make(&opts(Level::Shuffle), "s1", Room::any(64)).unwrap();
        assert_eq!(a, make(&opts(Level::Shuffle), "s1", Room::any(64)).unwrap());
        assert_ne!(a, make(&opts(Level::Shuffle), "s2", Room::any(64)).unwrap());
        assert_eq!(a.places.len(), 64);
        assert_eq!(a.deal_count(), 0);
        let mut moved: usize = 0;
        for p in &a.places {
            let s: &Slot = t.slot(&p.loc).unwrap();
            if p.actor != s.actor {
                moved += 1;
            }
        }
        assert!(moved > 30, "{moved}");
        for p in &a.places {
            let s: &Slot = t.slot(&p.loc).unwrap();
            if moved_tunic(t, &p.item, &s.shop) {
                assert!((10..=300).contains(&p.price) && p.price % 5 == 0, "{} {}", p.loc, p.price);
            } else {
                assert!(t.slots.iter().any(|x| x.item == p.item && x.price == p.price), "{} keeps the price of {}", p.loc, p.item);
            }
        }
        let differ: bool = (0..20).any(|n| {
            let l: Layout = make(&opts(Level::Shuffle), &format!("p{n}"), Room::any(64)).unwrap();
            (0..8).any(|i| l.get(&t.at("Market Potion Shop", i).unwrap().name).unwrap().item != l.get(&t.at("Kak Potion Shop", i).unwrap().name).unwrap().item)
        });
        assert!(differ, "the two Potion Shops stock their shelves apart");
    }

    #[test]
    fn a_tunic_out_of_its_own_shop_costs_a_random_price() {
        let t: &Table = table().unwrap();
        assert!(!moved_tunic(t, "Buy Goron Tunic", "GC Shop"));
        assert!(moved_tunic(t, "Buy Goron Tunic", "KF Shop"));
        assert!(moved_tunic(t, "Buy Zora Tunic", "GC Shop"));
        assert!(!moved_tunic(t, "Buy Deku Shield", "GC Shop"));
        let mut seen: usize = 0;
        for n in 0..40 {
            let l: Layout = make(&Opts { max: 500, ..opts(Level::Deals(2)) }, &format!("t{n}"), Room::any(64)).unwrap();
            for p in l.places.iter().filter(|p| !p.deal && TUNICS.contains(&p.item.as_str())) {
                let shop: &str = &t.slot(&p.loc).unwrap().shop;
                if moved_tunic(t, &p.item, shop) {
                    seen += 1;
                    let cap: i32 = t.shop(shop).unwrap().cap.unwrap_or(500);
                    assert!((10..=cap.min(500)).contains(&p.price), "{} {}", p.loc, p.price);
                }
            }
        }
        assert!(seen > 10, "{seen}");
    }

    #[test]
    fn deals_take_the_first_slots_of_each_shop_and_keep_the_needed_goods() {
        let t: &Table = table().unwrap();
        for n in 1..=4u8 {
            for s in 0..6 {
                let l: Layout = make(&opts(Level::Deals(n)), &format!("d{n} {s}"), Room::any(64)).unwrap();
                assert_eq!(l.deal_count(), 8 * n as usize);
                for sh in t.shops.iter() {
                    for (k, i) in sh.deal_order.iter().enumerate() {
                        let p: &Place = l.get(&t.at(&sh.shop, *i).unwrap().name).unwrap();
                        assert_eq!(p.deal, sh.deals && k < n as usize, "{} {i}", sh.shop);
                        if p.deal {
                            assert!(t.rupees.iter().any(|r| r.0 == p.pool));
                            assert!((10..=300).contains(&p.price) && p.price % 5 == 0);
                        }
                    }
                }
                for k in &t.keep {
                    assert!(count(&l, k) >= 1, "{k} lost at deals {n}");
                }
                for (item, shop) in &t.pin {
                    assert!(l.places.iter().any(|p| &p.item == item && t.slot(&p.loc).unwrap().shop == *shop), "{item} left {shop}");
                }
            }
        }
        let r: Layout = make(&opts(Level::Random), "r", Room::any(64)).unwrap();
        assert!(r.deal_count() <= 32);
        let few: Layout = make(&opts(Level::Deals(4)), "few", Room::any(3)).unwrap();
        assert_eq!((few.deal_count(), few.cut), (3, 29));
        for sh in t.shops.iter().filter(|s| s.deals).take(3) {
            assert!(few.get(&t.at(&sh.shop, sh.deal_order[0]).unwrap().name).unwrap().deal, "{}", sh.shop);
        }
    }
}
