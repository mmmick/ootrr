pub mod actor;
pub mod audit;
pub mod banc;
pub mod counters;
pub mod goods;
pub mod hook;
pub mod inner;
pub mod seed;
pub mod stage;
pub mod text;

pub const BASE: &str = "Shop_Item_Base";
pub const TEMPLATE: &str = "Shop_Armor_Goron";
pub const PREFIX: &str = "Shop_OOTRR_";

pub const KEEPER_PACKS: [&str; 12] = [
    "Npc_Kokiri_001_Shop_Child",
    "Npc_Kokiri_001_Shop_Child_ShortStock",
    "Npc_Kokiri_001_Shop_Adult",
    "Npc_Hylia_018_Shop_Child",
    "Npc_Hylia_018_Shop_Adult",
    "Npc_Hylia_020_Shop_Child",
    "Npc_Hylia_020_Shop_Adult",
    "Npc_Hylia_021_Shop_Child",
    "Npc_Goron_011_Shop_Child",
    "Npc_Goron_011_Shop_Adult",
    "Npc_Zora_011_Shop_Child",
    "Npc_Zora_011_Shop_Adult",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum What {
    Goods { actor: String, price: i32 },
    Once { actor: String, price: i32 },
    Deal { item: String, price: i32, model: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub label: String,
    pub shop: String,
    pub slot: usize,
    pub keepers: Vec<String>,
    pub bancs: Vec<String>,
    pub counters: Vec<String>,
    pub what: What,
    /// The upgrade family a seed deal sells tier by tier at run time (shelf::hook): the shelf
    /// stands with the first tier. None for every other slot and for hand made deals.
    pub tiered: Option<&'static str>,
}

impl Job {
    pub fn price(&self) -> i32 {
        match &self.what {
            What::Goods { price, .. } | What::Once { price, .. } | What::Deal { price, .. } => *price,
        }
    }

    pub fn once(&self) -> bool {
        !matches!(self.what, What::Goods { .. })
    }

    pub fn shelf_actor(&self) -> String {
        match &self.what {
            What::Goods { actor, .. } | What::Once { actor, .. } => actor.clone(),
            What::Deal { item, price, .. } => shelf_name(item, *price),
        }
    }
}

pub fn shelf_name(item: &str, price: i32) -> String {
    format!("{PREFIX}{item}_{price}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_shelf_names_carry_the_item_and_the_price() {
        assert_eq!(shelf_name("Item_Boomerang", 20), "Shop_OOTRR_Item_Boomerang_20");
        assert!(crate::pickup_pack::is_actor_name(&shelf_name("Item_FairyBless_DefenseDouble", 995)));
        let j: Job = Job { label: "x".into(), shop: "KF Shop".into(), slot: 6, keepers: Vec::new(), bancs: Vec::new(), counters: Vec::new(), what: What::Goods { actor: "Shop_Heart".into(), price: 5 }, tiered: None };
        assert_eq!((j.shelf_actor().as_str(), j.price()), ("Shop_Heart", 5));
    }
}
