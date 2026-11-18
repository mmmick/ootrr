use crate::error::{Error as E, Res};
use crate::shelf::{Job, What};
use crate::shops::data::{table, Shop, Slot, Table};
use crate::progressive::FAMILIES;
use crate::shops::layout::Layout;
use crate::shops::rules::{class, Class};
use crate::slice::data::Data;

pub fn job(t: &Table, s: &Slot, what: What) -> Res<Job> {
    let sh: &Shop = t.shop(&s.shop).ok_or_else(|| E::Msg(format!("{} has no shop", s.name)))?;
    Ok(Job { label: s.name.clone(), shop: s.shop.clone(), slot: s.slot, keepers: sh.keepers.clone(), bancs: sh.bancs.clone(), counters: s.counters.clone(), what, tiered: None })
}

pub fn deal(d: &Data, item: &str, tier: Option<&str>, price: i32) -> Res<What> {
    let t: &Table = table()?;
    let c: Class = class(d, item).ok_or_else(|| E::Msg(format!("{item} cannot be sold on a shelf by this build")))?;
    let actor = || d.item(item).and_then(|i| i.actor.clone()).ok_or_else(|| E::Msg(format!("{item} has no remake actor")));
    Ok(match c {
        Class::Stack => {
            let a: String = t.stack.iter().find(|(i, _)| i == item).map(|(_, a)| a.clone()).ok_or_else(|| E::Msg(format!("{item} has no shelf bundle")))?;
            What::Once { actor: a, price }
        }
        Class::Upgrade => {
            let a: &str = tier.ok_or_else(|| E::Msg(format!("{item} is an upgrade, the tier order names no tier for its shelf")))?;
            What::Deal { item: a.to_string(), price, model: donor(t, a) }
        }
        Class::Song => What::Deal { item: actor()?, price, model: t.song_models.first().cloned() },
        Class::Unique | Class::Heart => What::Deal { item: actor()?, price, model: None },
    })
}

pub fn donor(t: &Table, actor: &str) -> Option<String> {
    t.donors.iter().find(|(a, _)| a == actor).map(|(_, m)| m.clone())
}

/// The upgrade family a shelf deal of `item` sells tier by tier (shelf::hook), if it is one.
pub fn tiered_family(d: &Data, item: &str) -> Option<&'static str> {
    (class(d, item) == Some(Class::Upgrade)).then(|| d.family(item)).flatten()
}

/// One job per shelf slot of the seed's layout. A deal of an upgrade stands with the family's
/// first tier and is marked tiered: the shop names and gives the next tier at run time.
pub fn of(d: &Data, l: &Layout) -> Res<Vec<Job>> {
    let t: &Table = table()?;
    let mut out: Vec<Job> = Vec::new();
    for s in &t.slots {
        let p = l.get(&s.name).ok_or_else(|| E::Msg(format!("the shop layout has no entry for {}", s.name)))?;
        let tiered: Option<&'static str> = if p.deal { tiered_family(d, &p.item) } else { None };
        let what: What = match (p.deal, tiered) {
            (true, Some(f)) => deal(d, &p.item, crate::progressive::tiers(f)?.first().copied(), p.price)?,
            (true, None) => deal(d, &p.item, None, p.price)?,
            (false, _) => What::Goods { actor: p.actor.clone(), price: p.price },
        };
        let mut j: Job = job(t, s, what)?;
        j.tiered = tiered;
        out.push(j);
    }
    Ok(out)
}

pub fn parse(d: &Data, spec: &str) -> Res<Job> {
    let t: &Table = table()?;
    let (loc, rest) = spec.split_once('=').ok_or_else(|| E::Msg(format!("{spec} is not Location=Item@price")))?;
    let (item, price) = rest.rsplit_once('@').ok_or_else(|| E::Msg(format!("{spec} has no @price")))?;
    let price: i32 = price.trim().parse().map_err(|_| E::Msg(format!("{spec} has a price that is not a number")))?;
    if !(1..=999).contains(&price) {
        return Err(E::Msg(format!("{spec} has a price outside 1 to 999")));
    }
    let s: &Slot = t.slot(loc.trim()).ok_or_else(|| E::Msg(format!("{} is not a shelf slot, use a name like KF Shop Item 1", loc.trim())))?;
    let item: &str = crate::spoiler::unshown(item.trim());
    let tier: Option<(&str, &str)> = FAMILIES.iter().find(|(_, ts)| ts.contains(&item)).map(|(f, _)| (*f, item));
    let what: What = match (t.slots.iter().find(|x| x.item == item), tier) {
        (Some(g), _) => What::Goods { actor: g.actor.clone(), price },
        (None, Some((f, _))) if !t.upgrades.iter().any(|u| u == f) => return Err(E::Msg(format!("{item} cannot be sold on a shelf by this build"))),
        (None, Some((_, a))) => What::Deal { item: a.to_string(), price, model: donor(t, a) },
        (None, None) => deal(d, item, None, price)?,
    };
    job(t, s, what)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::shops::layout::{fill_deals, make};
    use crate::shops::plan::{Level, Opts, Prices};
    use std::collections::BTreeMap as BM;

    #[test]
    fn a_layout_becomes_one_job_per_slot() {
        let d: Data = Data::load().unwrap();
        let mut l: Layout = make(&Opts { level: Level::Deals(1), prices: Prices::Affordable, min: 10, max: 10, kinds: crate::shops::plan::Kinds::default() }, "jobs", crate::shops::layout::Room::any(64)).unwrap();
        let got: BM<String, String> = l.deals().map(|p| (p.loc.clone(), "Hover Boots".to_string())).collect();
        fill_deals(&mut l, &got).unwrap();
        let js: Vec<Job> = of(&d, &l).unwrap();
        assert_eq!(js.len(), 64);
        let deals: Vec<&Job> = js.iter().filter(|j| matches!(j.what, What::Deal { .. })).collect();
        assert_eq!(deals.len(), 6);
        assert!(deals.iter().all(|j| j.shelf_actor() == "Shop_OOTRR_Item_HoverBoot_10"));
        let kf: &Job = js.iter().find(|j| j.shop == "KF Shop" && j.slot == 6).unwrap();
        assert_eq!(kf.keepers.len(), 3);
        assert_eq!(kf.counters.len(), 3);
    }

    #[test]
    fn hand_written_slots_parse() {
        let d: Data = Data::load().unwrap();
        let j: Job = parse(&d, "KF Shop Item 1=Boomerang@20").unwrap();
        assert_eq!((j.shop.as_str(), j.slot), ("KF Shop", 6));
        assert_eq!(j.what, What::Deal { item: "Item_Boomerang".into(), price: 20, model: None });
        assert_eq!(parse(&d, "Market Bazaar Item 1=Bombs (10)@15").unwrap().what, What::Once { actor: "Shop_BombBundle_02".into(), price: 15 });
        assert_eq!(parse(&d, "GC Shop Item 5=Item_Longshot@90").unwrap().what, What::Deal { item: "Item_Longshot".into(), price: 90, model: None });
        assert_eq!(parse(&d, "GC Shop Item 5=Minuet of Forest@40").unwrap().what, What::Deal { item: "Item_Melody_ForestMinuet".into(), price: 40, model: Some("Dm_Item_Sheik_Harp".into()) });
        assert_eq!(parse(&d, "Kak Bazaar Item 1=Piece of Heart@30").unwrap().what, What::Deal { item: "Obj_Heart_Piece".into(), price: 30, model: None });
        assert_eq!(parse(&d, "GC Shop Item 6=Buy Heart@5").unwrap().what, What::Goods { actor: "Shop_Heart".into(), price: 5 });
        assert!(parse(&d, "GC Shop Item 6=Progressive Hookshot@50").is_err());
        for bad in ["Bow", "Item_Quiver_Large", "Item_BulletBag_Medium", "Magic Meter", "Bomb Bag"] {
            assert!(parse(&d, &format!("GC Shop Item 5={bad}@40")).is_err(), "{bad}");
        }
        assert_eq!(parse(&d, "GC Shop Item 5=Item_Gauntlet_Goron@40").unwrap().what, What::Deal { item: "Item_Gauntlet_Goron".into(), price: 40, model: None });
        assert_eq!(deal(&d, "Progressive Wallet", Some("Item_Wallet_Medium"), 25).unwrap(), What::Deal { item: "Item_Wallet_Medium".into(), price: 25, model: None });
        assert_eq!(deal(&d, "Progressive Strength Upgrade", Some("Item_Gauntlet_Silver"), 25).unwrap(), What::Deal { item: "Item_Gauntlet_Silver".into(), price: 25, model: None });
        assert!(deal(&d, "Progressive Strength Upgrade", None, 25).is_err());
        assert!(parse(&d, "GC Shop Item 6=Small Key (Forest Temple)@50").is_err());
        assert!(parse(&d, "Nowhere=Boomerang@5").is_err());
        assert!(parse(&d, "KF Shop Item 1=Boomerang@0").is_err());
    }
}
