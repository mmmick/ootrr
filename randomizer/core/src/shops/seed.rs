use std::collections::BTreeMap as BM;

use crate::error::{Error as E, Res};
use crate::settings::seed::{SeedFile, ShopEntry};
use crate::shops::data::{is_loc, table, Table};
use crate::shops::layout::{Layout, Place};
use crate::shops::plan::Opts;
use crate::slice::data::Data;

pub fn entries(l: &Layout) -> BM<String, ShopEntry> {
    l.places
        .iter()
        .map(|p| (p.loc.clone(), ShopEntry { item: p.item.clone(), price: p.price as i64, deal: p.deal, pool: if p.deal { p.pool.clone() } else { String::new() } }))
        .collect()
}

pub fn layout(seed: &SeedFile) -> Res<Option<Layout>> {
    if seed.shops.is_empty() {
        return Ok(None);
    }
    let t: &Table = table()?;
    let mut places: Vec<Place> = Vec::new();
    for s in &t.slots {
        let e: &ShopEntry = seed.shops.get(&s.name).ok_or_else(|| E::Msg(format!("the seed file has no shop entry for {}", s.name)))?;
        let actor: String = if e.deal { String::new() } else { t.slots.iter().find(|x| x.item == e.item).map(|x| x.actor.clone()).ok_or_else(|| E::Msg(format!("{} at {} is not a shop good", e.item, s.name)))? };
        if e.deal && seed.placements.get(&s.name) != Some(&e.item) {
            return Err(E::Msg(format!("the shop deal {} does not match its placement", s.name)));
        }
        places.push(Place { loc: s.name.clone(), item: e.item.clone(), actor, price: e.price as i32, deal: e.deal, pool: e.pool.clone() });
    }
    if seed.shops.len() != t.slots.len() {
        return Err(E::Format("the seed file lists a shop slot this build does not know"));
    }
    Ok(Some(Layout { places, cut: 0 }))
}

pub fn check(o: &Opts, seed: &SeedFile) -> Res<()> {
    match (o.on(), seed.shops.is_empty()) {
        (true, true) => Err(E::Format("this seed turns shop shuffle on but its file has no shop table, generate it again with this build")),
        (false, false) => Err(E::Format("this seed file has a shop table but its settings leave the shops vanilla")),
        _ => Ok(()),
    }
}

pub fn delta(seed: &SeedFile, item: &str) -> isize {
    let mut here: isize = 0;
    for (loc, e) in &seed.shops {
        if !e.deal {
            continue;
        }
        if e.pool == item {
            here += 1;
        }
        if seed.placements.get(loc).is_some_and(|g| g == item) {
            here -= 1;
        }
    }
    here
}

pub fn balance(d: &Data, seed: &SeedFile, item: &str, acts: &mut Vec<String>) {
    crate::slice::chest_targets::adjust(d, item, delta(seed, item), acts);
}

pub fn skip(loc: &str) -> bool {
    is_loc(loc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use crate::shops::layout::{fill_deals, make};
    use crate::shops::plan::{Level, Prices};

    #[test]
    fn a_layout_survives_the_seed_file() {
        let o: Opts = Opts { level: Level::Deals(1), prices: Prices::Weighted, min: 20, max: 400, kinds: crate::shops::plan::Kinds::default() };
        let mut l: Layout = make(&o, "seed rt", crate::shops::layout::Room::any(64)).unwrap();
        let got: BM<String, String> = l.deals().map(|p| (p.loc.clone(), "Boomerang".to_string())).collect();
        fill_deals(&mut l, &got).unwrap();
        let mut f: SeedFile = SeedFile::new("seed rt", &Settings::default(), "g", got.clone()).unwrap();
        f.shops = entries(&l);
        let back: SeedFile = SeedFile::parse(&f.to_text()).unwrap();
        assert_eq!(layout(&back).unwrap().unwrap(), l);
        let d: Data = Data::load().unwrap();
        let mut acts: Vec<String> = Vec::new();
        let rupee: String = l.deals().next().unwrap().pool.clone();
        balance(&d, &back, &rupee, &mut acts);
        assert!(!acts.is_empty());
        let mut boom: Vec<String> = vec!["Item_Boomerang".into(); 7];
        balance(&d, &back, "Boomerang", &mut boom);
        assert_eq!(boom.len(), 7 - l.deal_count());
        assert!(check(&o, &back).is_ok());
        assert!(check(&Opts::default(), &back).is_err());
        assert!(check(&o, &SeedFile::new("x", &Settings::default(), "g", BM::new()).unwrap()).is_err());
    }
}
