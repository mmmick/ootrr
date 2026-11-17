use std::collections::BTreeMap as BM;

use crate::error::{Error as E, Res};
use crate::logic_items::ItemId;
use crate::settings::Settings;
use crate::slice::data::Data;
use crate::slice::fill::vanilla_place;
use crate::slice::logic::world_with;
use crate::slice::pick::{pick, Pick};
use crate::slice::support::{plan, Plan};
use crate::world::{LocId, Placement, World};

pub struct Rebuilt {
    pub w: World,
    pub got: Placement,
    pub free: Vec<LocId>,
    pub missing: Vec<String>,
}

pub fn rebuild_data(d: &Data, s: &Settings, seed: &str, placements: &BM<String, String>) -> Res<Rebuilt> {
    let p: Plan = plan(s)?;
    let mut lg: Vec<(&str, &str)> = p.logic();
    lg.push((crate::slice::tracker_fixes::KEY, crate::slice::tracker_fixes::ON));
    let room: crate::shops::layout::Room = crate::shops::rules::room(d, &p)?;
    let lay: Option<crate::shops::Layout> = crate::shops::layout::of(&p.shops, seed, room)?;
    let owned: Option<Data> = lay.as_ref().map(|l| crate::shops::logic::data_with(d, l)).transpose()?;
    let d: &Data = owned.as_ref().unwrap_or(d);
    let mut w: World = match &lay {
        Some(l) => crate::shops::logic::world(d, &lg, l)?,
        None => world_with(d, &lg)?,
    };
    crate::slice::starts::add(d, &p, &mut w, seed)?;
    let vanilla: Placement = vanilla_place(&w, d)?;
    let mut pk: Pick = pick(d, &w, &p);
    if let Some(l) = &lay {
        l.add_free(&w, &mut pk.free)?;
    }
    crate::entr::choose(s, &mut w, seed, &vanilla, &pk.free)?;
    let (mut got, _): (Placement, Vec<String>) = crate::start_items::pool::base(d, &w, &p, &vanilla, &pk.free)?;
    let mut missing: Vec<String> = Vec::new();
    for l in &pk.free {
        let name: &str = &w.locs[*l].name;
        match placements.get(name) {
            Some(it) if it == "(nothing)" => got.item[*l] = None,
            Some(it) => {
                let i: ItemId = w.item(it).ok_or_else(|| E::Msg(format!("seed: {name} holds {it}, an item this build does not know")))?;
                got.put(*l, i);
            }
            None => missing.push(name.to_string()),
        }
    }
    Ok(Rebuilt { w, got, free: pk.free, missing })
}

pub fn rebuild(s: &Settings, seed: &str, placements: &BM<String, String>) -> Res<Rebuilt> {
    rebuild_data(&Data::load()?, s, seed, placements)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slice::generate::generate;
    use crate::spoiler::playthrough;

    #[test]
    fn a_generated_seed_rebuilds_to_the_same_placements_and_is_beatable() {
        let s: Settings = Settings::default();
        let g = generate(&s, "rebuild").unwrap();
        let r: Rebuilt = rebuild(&s, "rebuild", &g.seed_file.placements).unwrap();
        assert!(r.missing.is_empty());
        assert_eq!(r.free.len(), g.seed_file.placements.len());
        for l in &r.free {
            let name: &str = &r.w.locs[*l].name;
            let it: String = r.got.get(*l).map(|i| r.w.items.names[i].clone()).unwrap_or_else(|| "(nothing)".to_string());
            assert_eq!(Some(&it), g.seed_file.placements.get(name));
        }
        assert!(playthrough(&r.w, &r.got).is_ok());
    }
}
