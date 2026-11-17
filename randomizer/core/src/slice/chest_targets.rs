use std::collections::BTreeMap as BM;

use crate::dungeon_credit::key_dungeon;
use crate::flow_locations::Choice;
use crate::error::{Error as E, Res};
use crate::pickup_persist::Extra;
use crate::progressive::{guarded, needs_every_holder};
use crate::settings::seed::SeedFile;
use crate::special;
use crate::slice::data::{Data, Ev, State};
use crate::slice::flow_pool::{dest_form, pool_actor};
use crate::slice::flow_wrap::wraps as flow_wraps;
use crate::slice::pickups as pu;
use crate::slice::support::plan;


#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub name: String,
    pub hash: u64,
    pub files: Vec<String>,
    pub old_item: String,
    pub new_item: String,
    pub old_actor: String,
    pub new_actor: String,
    pub ev: Option<Ev>,
    pub family: Option<&'static str>,
    pub key: Option<&'static str>,
}

fn tier_rank(d: &Data, item: &str, actor: &str) -> usize {
    d.item(item).and_then(|i| i.tiers.iter().position(|t| t == actor)).unwrap_or(usize::MAX)
}

fn vanilla_actor(d: &Data, loc: &str) -> Res<String> {
    if let Some(a) = crate::slice::skull_edit::vanilla_actor(d, loc) {
        return Ok(a);
    }
    if let Some(a) = pu::pool_actor(d, loc) {
        return Ok(a);
    }
    if is_sale(d, loc) {
        if let Some(a) = d.flow.by_name(loc).and_then(|l| l.item.clone()) {
            return Ok(pool_actor(d, loc, &a));
        }
    }
    if let Some(l) = d.cat.by_name(loc) {
        return Ok(l.item.clone());
    }
    d.flow
        .by_name(loc)
        .and_then(|l| l.item.clone())
        .ok_or_else(|| E::Msg(format!("{loc} is not in the catalog")))
}

pub fn is_sale(d: &Data, loc: &str) -> bool {
    d.flow.by_name(loc).is_some() && d.cat.by_name(loc).is_none_or(|c| c.kind == "scrub" || (c.kind == "cow" && crate::flow_extra::cows::is(loc)))
}

pub(crate) fn actors_for(d: &Data, seed: &SeedFile) -> Res<BM<String, String>> {
    let mut pool: BM<String, Vec<String>> = BM::new();
    let mut dests: BM<String, Vec<String>> = BM::new();
    for (loc, item) in &seed.placements {
        if crate::pot_slice::targets::is_loc(d, loc) || crate::shops::seed::skip(loc) || crate::slice::silver::is_spot(loc) {
            continue;
        }
        let vanilla: &str = d.pool_item(loc).ok_or_else(|| E::Msg(format!("{loc} has no vanilla item")))?;
        let actor: String = match d.flow.by_name(loc).and_then(|l| l.item.clone()) {
            Some(a) if Some(vanilla) != d.vanilla_of(loc) => a,
            _ => vanilla_actor(d, loc)?,
        };
        pool.entry(vanilla.to_string()).or_default().push(actor);
        if !crate::slice::silver::is_item(item) {
            dests.entry(item.clone()).or_default().push(loc.clone());
        }
    }
    let pl: crate::slice::support::Plan = plan(&seed.settings()?.settings)?;
    let stripped: bool = crate::start_items::pool::active(&pl);
    let crow: bool = crate::slice::scarecrow::pooled(&pl);
    let mut out: BM<String, String> = BM::new();
    for (item, locs) in &dests {
        let mut acts: Vec<String> = pool.get(item).cloned().unwrap_or_default();
        crate::pot_slice::targets::balance(d, seed, item, &mut acts);
        crate::shops::seed::balance(d, seed, item, &mut acts);
        crate::slice::silver::balance(seed, item, &mut acts);
        if stripped {
            crate::start_items::pool::fit_actors(d, item, locs.len(), &mut acts);
        }
        if crow {
            crate::slice::scarecrow::balance(item, locs.len(), &mut acts);
        }
        if acts.len() != locs.len() {
            return Err(E::Msg(format!(
                "the seed places {} copies of {item} but its chests hold {}, this seed does not belong to these game files",
                locs.len(),
                acts.len()
            )));
        }
        acts.sort_by(|x, y| (tier_rank(d, item, x), x).cmp(&(tier_rank(d, item, y), y)));
        for (loc, actor) in locs.iter().zip(acts) {
            out.insert(loc.clone(), dest_form(d, item, loc, actor));
        }
    }
    Ok(out)
}

/// The targets of a seed as the build sees them once slice::seed_packs has written the
/// pickup packs its freestanding spots need (Pieces of Heart on the ground are always shuffled,
/// so most seeds need some).
#[cfg(test)]
pub(crate) fn targets(d: &Data, seed: &SeedFile) -> Res<(Vec<Target>, usize, Vec<Choice>)> {
    let v: crate::slice::tier_order::Verdict = crate::slice::tier_order::fixed_actors(d, seed)?;
    let mut extra: Extra = Extra::default();
    for w in crate::slice::seed_packs::need(d, seed, &v.assign)?.into_values() {
        extra.packs.insert(w.name, Vec::new());
    }
    targets_with(d, seed, false, &v.assign, &extra)
}

pub(crate) fn targets_with(d: &Data, seed: &SeedFile, roam: bool, fixed: &BM<String, String>, extra: &Extra) -> Res<(Vec<Target>, usize, Vec<Choice>)> {
    let mut out: Vec<Target> = Vec::new();
    let mut flows: Vec<Choice> = Vec::new();
    let mut same: usize = 0;
    let given: BM<String, String> = actors_for(d, seed)?;
    let wrapped: BM<String, &'static str> = flow_wraps(d, seed);
    for (loc, item) in &seed.placements {
        if crate::pot_slice::targets::is_loc(d, loc) || crate::slice::skulls::is_skull(d, loc) || crate::shops::seed::skip(loc) || crate::slice::silver::is_item(item) {
            continue;
        }
        if let Some(t) = crate::slice::silver::targets(loc, item)? {
            out.extend(t);
            continue;
        }
        if is_sale(d, loc) {
            if !d.flow_ok(loc) && !d.song_ok(loc) {
                return Err(E::Msg(format!("{loc} cannot be placed, its reward event is not supported")));
            }
            if crate::scrubs_a::home(d, loc, item) {
                same += 1;
                continue;
            }
            if wrapped.contains_key(loc) {
                continue;
            }
            let actor: String = fixed.get(loc).or_else(|| given.get(loc)).cloned().ok_or_else(|| E::Msg(format!("{loc} was given no item actor")))?;
            if Some(actor.as_str()) == d.flow.by_name(loc).and_then(|l| l.item.as_deref()) {
                same += 1;
            } else {
                flows.push(Choice { location: loc.clone(), item: actor });
            }
            continue;
        }
        if let Some(l) = d.cat.by_name(loc).filter(|l| pu::is_world(l)) {
            let t: Vec<Target> = pu::targets_with(d, l, item, extra, fixed.get(loc).map(|a| a.as_str()))?;
            if t.is_empty() {
                same += 1;
            } else {
                out.extend(t);
            }
            continue;
        }
        if let Some(l) = d.cat.by_name(loc).filter(|l| l.kind == "bossheart") {
            let new: &crate::slice::data::Item = d.item(item).ok_or_else(|| E::Msg(format!("unknown item {item}")))?;
            if new.actor.is_none() {
                return Err(E::Msg(format!("{item} has no remake item actor and cannot be placed")));
            }
            let new_actor: String = given.get(loc).cloned().ok_or_else(|| E::Msg(format!("{loc} was given no item actor")))?;
            if new_actor == l.item {
                same += 1;
                continue;
            }
            let vanilla: &str = d.vanilla_of(loc).ok_or_else(|| E::Msg(format!("{loc} has no vanilla item")))?;
            for r in &l.refs {
                out.push(Target {
                    name: loc.clone(),
                    hash: r.hash,
                    files: r.files.clone(),
                    old_item: vanilla.to_string(),
                    new_item: item.clone(),
                    old_actor: l.item.clone(),
                    new_actor: new_actor.clone(),
                    ev: None,
                    family: None,
                    key: None,
                });
            }
            continue;
        }
        match d.state_of(loc) {
            Some(State::Eligible) => {}
            Some(State::Event) if d.event_ok(loc) => {}
            None if pu::is_held(loc) => {}
            Some(s) => {
                return Err(E::Msg(format!("{loc} cannot be placed, it stays vanilla because {}", s.why())));
            }
            None => return Err(E::Msg(format!("{loc} is not a chest this build knows"))),
        }
        let vanilla: &str = d.vanilla_of(loc).ok_or_else(|| E::Msg(format!("{loc} has no vanilla item")))?;
        let new: &crate::slice::data::Item = d.item(item).ok_or_else(|| E::Msg(format!("unknown item {item}")))?;
        if new.actor.is_none() {
            return Err(E::Msg(format!("{item} has no remake item actor and cannot be placed")));
        }
        let family: Option<&'static str> = d
            .family(item)
            .filter(|f| !needs_every_holder(f) || d.holders(f).iter().all(|h| seed.placements.contains_key(*h) || d.wrappable(h)));
        let new_actor: String = match family {
            Some(f) => crate::pregrant::chest_drop(f)?.to_string(),
            None => fixed.get(loc).or_else(|| given.get(loc)).cloned().ok_or_else(|| E::Msg(format!("{loc} was given no item actor")))?,
        };
        let key: Option<&'static str> = if roam && new.kind == "SmallKey" {
            let name: &str = new.dungeon.as_deref().unwrap_or("");
            Some(key_dungeon(name).ok_or_else(|| E::Msg(format!("{item} cannot be placed in a chest, no key count belongs to it")))?)
        } else {
            None
        };
        let l: &crate::catalog::Loc = d.cat.by_name(loc).ok_or_else(|| E::Msg(format!("{loc} is not in the catalog")))?;
        // A bomb item in a chest that already held bombs still needs the bomb bag guard, or the
        // vanilla get event hands out a usable bomb bag along with the bombs.
        if family.is_none() && key.is_none() && new_actor == l.item && !guarded(&new_actor) {
            same += 1;
            continue;
        }
        for r in &l.refs {
            out.push(Target {
                name: loc.clone(),
                hash: r.hash,
                files: r.files.clone(),
                old_item: vanilla.to_string(),
                new_item: item.clone(),
                old_actor: l.item.clone(),
                new_actor: new_actor.clone(),
                ev: d.event_mode(r.hash),
                family,
                key,
            });
        }
    }
    out.extend(unshuffled_bomb_chests(d, seed));
    Ok((out, same, flows))
}

/// Chests that keep their vanilla bombs because this seed does not shuffle them. Opened before
/// the bomb bag is found, their vanilla get event would hand out a usable bag with the bombs, so
/// they take the same bomb bag guard as a shuffled bomb chest.
fn unshuffled_bomb_chests(d: &Data, seed: &SeedFile) -> Vec<Target> {
    let mut out: Vec<Target> = Vec::new();
    for l in d.cat.locs.iter().filter(|l| l.kind == "chest" && guarded(&l.item) && !seed.placements.contains_key(&l.name)) {
        if d.state_of(&l.name) != Some(State::Eligible) || d.keeps_event(&l.name) || special::sized(l.refs.first().map(|r| r.hash).unwrap_or(0)).is_some() {
            continue;
        }
        let vanilla: String = d.vanilla_of(&l.name).unwrap_or(&l.vanilla).to_string();
        for r in &l.refs {
            out.push(Target {
                name: l.name.clone(),
                hash: r.hash,
                files: r.files.clone(),
                old_item: vanilla.clone(),
                new_item: vanilla.clone(),
                old_actor: l.item.clone(),
                new_actor: l.item.clone(),
                ev: d.event_mode(r.hash),
                family: None,
                key: None,
            });
        }
    }
    out
}
