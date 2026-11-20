use std::collections::HashSet as HS;
use std::sync::OnceLock;

use crate::error::{Error as E, Res};
use crate::json::{self, Json};

pub const SHOPS_JSON: &str = include_str!("../../../../data/shops.json");

#[derive(Debug, Clone, PartialEq)]
pub struct Shop {
    pub shop: String,
    pub files: Vec<String>,
    pub keepers: Vec<String>,
    pub bancs: Vec<String>,
    pub deal_order: Vec<usize>,
    pub deals: bool,
    pub cap: Option<i32>,
    pub once: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    pub name: String,
    pub shop: String,
    pub slot: usize,
    pub actor: String,
    pub item: String,
    pub price: i32,
    pub counters: Vec<String>,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewItem {
    pub name: String,
    pub actor: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub shops: Vec<Shop>,
    pub slots: Vec<Slot>,
    pub items: Vec<NewItem>,
    pub keep: Vec<String>,
    pub bottle: Vec<String>,
    pub bombs: Vec<String>,
    pub deal_items: Vec<String>,
    pub rupees: Vec<(String, u64)>,
    pub pin: Vec<(String, String)>,
    pub stack: Vec<(String, String)>,
    pub upgrades: Vec<String>,
    pub hearts: Vec<String>,
    pub song_models: Vec<String>,
    pub donors: Vec<(String, String)>,
    pub flex: Vec<String>,
    pub names: HS<String>,
}

fn text(j: &Json, k: &str) -> Res<String> {
    j.get(k).and_then(|v| v.as_str()).map(|s| s.to_string()).ok_or_else(|| E::Msg(format!("shops data: field {k} is missing or not text")))
}

fn arr<'a>(j: &'a Json, k: &str) -> Res<&'a [Json]> {
    j.get(k).and_then(|v| v.as_arr()).ok_or_else(|| E::Msg(format!("shops data: field {k} is missing or not a list")))
}

fn strs(j: &Json, k: &str) -> Res<Vec<String>> {
    arr(j, k)?.iter().map(|v| v.as_str().map(|s| s.to_string()).ok_or_else(|| E::Msg(format!("shops data: {k} must hold text")))).collect()
}

fn num(j: &Json, k: &str) -> Res<i64> {
    j.get(k).and_then(|v| v.as_i64()).ok_or_else(|| E::Msg(format!("shops data: field {k} is missing or not a number")))
}

fn pairs(j: &Json, k: &str) -> Res<Vec<(String, String)>> {
    let mut out: Vec<(String, String)> = Vec::new();
    for r in arr(j, k)? {
        match r.as_arr() {
            Some([a, b]) => out.push((a.as_str().unwrap_or("").to_string(), b.as_str().unwrap_or("").to_string())),
            _ => return Err(E::Msg(format!("shops data: an entry of {k} must be a pair"))),
        }
    }
    Ok(out)
}

pub fn parse(t: &str) -> Res<Table> {
    let j: Json = json::parse(t)?;
    if j.get("format").and_then(|v| v.as_i64()) != Some(1) {
        return Err(E::Format("shops data: unsupported format"));
    }
    let mut shops: Vec<Shop> = Vec::new();
    for s in arr(&j, "shops")? {
        let order: Vec<usize> = arr(s, "deal_order")?.iter().filter_map(|v| v.as_i64()).map(|v| v as usize).collect();
        shops.push(Shop {
            shop: text(s, "shop")?,
            files: strs(s, "files")?,
            keepers: strs(s, "keepers")?,
            bancs: strs(s, "bancs")?,
            deal_order: order,
            deals: s.get("deals").and_then(|v| v.as_bool()).unwrap_or(false),
            cap: s.get("price_cap").and_then(|v| v.as_i64()).map(|v| v as i32),
            once: s.get("once").and_then(|v| v.as_bool()).unwrap_or(false),
        });
    }
    let mut slots: Vec<Slot> = Vec::new();
    for s in arr(&j, "slots")? {
        slots.push(Slot {
            name: text(s, "name")?,
            shop: text(s, "shop")?,
            slot: num(s, "slot")? as usize,
            actor: text(s, "actor")?,
            item: text(s, "item")?,
            price: num(s, "price")? as i32,
            counters: strs(s, "counters")?,
            target: text(s, "target")?,
        });
    }
    let mut items: Vec<NewItem> = Vec::new();
    for s in arr(&j, "items")? {
        items.push(NewItem { name: text(s, "name")?, actor: text(s, "actor")? });
    }
    let mut rupees: Vec<(String, u64)> = Vec::new();
    for r in arr(&j, "rupees")? {
        let pair: &[Json] = r.as_arr().ok_or(E::Format("shops data: a rupee entry must be a pair"))?;
        match pair {
            [n, w] => rupees.push((n.as_str().unwrap_or("").to_string(), w.as_i64().unwrap_or(0).max(0) as u64)),
            _ => return Err(E::Format("shops data: a rupee entry must be a pair")),
        }
    }
    let mut pin: Vec<(String, String)> = Vec::new();
    for r in j.get("pin").and_then(|v| v.as_arr()).unwrap_or(&[]) {
        match r.as_arr() {
            Some([a, b]) => pin.push((a.as_str().unwrap_or("").to_string(), b.as_str().unwrap_or("").to_string())),
            _ => return Err(E::Format("shops data: a pin entry must be an item and a shop")),
        }
    }
    let stack: Vec<(String, String)> = pairs(&j, "stack")?;
    let names: HS<String> = slots.iter().map(|s| s.name.clone()).collect();
    let t: Table = Table {
        shops,
        slots,
        items,
        keep: strs(&j, "keep")?,
        bottle: strs(&j, "bottle")?,
        bombs: strs(&j, "bombs")?,
        deal_items: strs(&j, "deal_items")?,
        rupees,
        pin,
        stack,
        upgrades: strs(&j, "upgrades")?,
        hearts: strs(&j, "hearts")?,
        song_models: strs(&j, "song_models")?,
        donors: pairs(&j, "donors")?,
        flex: strs(&j, "flex")?,
        names,
    };
    t.check()?;
    Ok(t)
}

impl Table {
    fn check(&self) -> Res<()> {
        for sh in &self.shops {
            let mut got: Vec<usize> = self.slots.iter().filter(|s| s.shop == sh.shop).map(|s| s.slot).collect();
            got.sort_unstable();
            if got != (0..8).collect::<Vec<usize>>() {
                return Err(E::Msg(format!("shops data: {} does not have the eight slots 0 to 7", sh.shop)));
            }
            let mut order: Vec<usize> = sh.deal_order.clone();
            order.sort_unstable();
            if order != (0..8).collect::<Vec<usize>>() {
                return Err(E::Msg(format!("shops data: the deal order of {} is not a list of the eight slots", sh.shop)));
            }
        }
        if self.names.len() != self.slots.len() {
            return Err(E::Format("shops data: a location is listed twice"));
        }
        if self.rupees.is_empty() || self.rupees.iter().all(|r| r.1 == 0) {
            return Err(E::Format("shops data: the rupee mix is empty"));
        }
        Ok(())
    }

    pub fn shop(&self, name: &str) -> Option<&Shop> {
        self.shops.iter().find(|s| s.shop == name)
    }

    pub fn slot(&self, loc: &str) -> Option<&Slot> {
        self.slots.iter().find(|s| s.name == loc)
    }

    pub fn at(&self, shop: &str, slot: usize) -> Option<&Slot> {
        self.slots.iter().find(|s| s.shop == shop && s.slot == slot)
    }

    pub fn unit(&self, s: &Slot) -> String {
        let files: String = self.shop(&s.shop).map(|x| x.files.join("+")).unwrap_or_default();
        format!("{files}#{}", s.slot)
    }
}

static TABLE: OnceLock<Table> = OnceLock::new();

pub fn table() -> Res<&'static Table> {
    if let Some(t) = TABLE.get() {
        return Ok(t);
    }
    let t: Table = parse(SHOPS_JSON)?;
    Ok(TABLE.get_or_init(|| t))
}

pub fn is_loc(name: &str) -> bool {
    table().is_ok_and(|t| t.names.contains(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slice::data::Data;

    #[test]
    fn the_shipped_table_has_eight_shops_of_eight_slots() {
        let t: &Table = table().unwrap();
        assert_eq!(t.shops.len(), 8);
        assert_eq!(t.slots.len(), 64);
        assert!(is_loc("KF Shop Item 1"));
        assert!(is_loc("ZD Shop Item 8"));
        assert!(!is_loc("Market Mask Shop Item 1"));
        assert!(!is_loc("KF Shop Wonderitem"));
        let units: std::collections::BTreeSet<String> = t.slots.iter().map(|s| t.unit(s)).collect();
        assert_eq!(units.len(), 64, "every shop has a goods list of its own");
    }

    #[test]
    fn every_location_and_item_of_the_table_is_known() {
        let d: Data = Data::load().unwrap();
        let w = crate::slice::logic::world(&d).unwrap();
        let t: &Table = table().unwrap();
        for s in &t.slots {
            assert!(w.loc(&s.name).is_some(), "{}", s.name);
            let known: bool = d.item(&s.item).is_some() || t.items.iter().any(|i| i.name == s.item);
            assert!(known, "{}", s.item);
            assert!(s.price > 0 && s.price < 1000, "{}", s.name);
            assert!(!s.counters.is_empty() && s.counters.iter().all(|c| c.starts_with("Shop_SoldNum.")), "{}", s.name);
            assert!(s.target.starts_with("shop:"), "{}", s.name);
        }
        for n in t.keep.iter().chain(t.bottle.iter()).chain(t.bombs.iter()) {
            assert!(d.item(n).is_some() || t.items.iter().any(|i| &i.name == n), "{n}");
        }
        for n in &t.deal_items {
            let i = d.item(n).unwrap();
            assert!(i.actor.is_some() && i.tiers.is_empty() && i.kind == "Item", "{n}");
        }
        for (n, _) in &t.rupees {
            assert!(d.item(n).is_some_and(|i| i.actor.is_some()), "{n}");
        }
        for i in &t.items {
            assert!(d.item(&i.name).is_none(), "{} is already an item", i.name);
        }
    }

    #[test]
    fn potion_shops_stock_the_same_goods_on_their_own_lists() {
        let t: &Table = table().unwrap();
        for i in 0..8 {
            let a: &Slot = t.at("Market Potion Shop", i).unwrap();
            let b: &Slot = t.at("Kak Potion Shop", i).unwrap();
            assert_ne!(t.unit(a), t.unit(b), "shelf::split gives the Kakariko keeper a goods list of its own");
            assert_eq!(a.actor, b.actor);
        }
        for s in ["Market Potion Shop", "Kak Potion Shop"] {
            let sh: &Shop = t.shop(s).unwrap();
            assert!(sh.deals && sh.once && sh.keepers.len() == 1, "{s}");
        }
        assert_eq!(t.shop("Kak Potion Shop").unwrap().keepers[0], crate::shelf::split::KEEPER);
    }

    #[test]
    fn broken_tables_are_refused() {
        assert!(parse("{\"format\":2}").is_err());
        let short: String = SHOPS_JSON.replacen("\"deal_order\": [6, 0, 7, 2, 3, 1, 4, 5]", "\"deal_order\": [6, 0]", 1);
        assert!(parse(&short).is_err());
    }
}
