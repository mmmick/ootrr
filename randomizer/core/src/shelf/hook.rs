//! Shelf deals of progressive upgrades sell the next tier the player lacks, once per shelf.
//!
//! A seed shelf that holds an upgrade family copy (shops::jobs::of marks its job `tiered`) stands
//! with the family's first tier. The shared shelf shop flow DisplayShop gets two entries:
//!
//! - [`PICK`] runs right after the player says yes to a purchase, before the shop asks whether the
//!   item can be carried (event 280). When Shop_SelectItemName names the first tier of such a
//!   family, it adds up the sold counters of the family's shelves (Shop_SoldNum, kept across saves
//!   by shelf::counters). Once those shelves sold as many times as there are shelves, it leaves the
//!   first tier, which the player then owns, so the shop refuses the sale as for any owned item:
//!   each shelf sells one copy, wherever the player buys it. Otherwise it writes the next tier the
//!   player lacks (the chest chain's question, progressive::chain) into Shop_SelectItemName, which
//!   the fit check, the get scene, its fanfare and the get window all read. A player who owns the
//!   top tier gets the top tier named, and the shop refuses it.
//! - [`GIVE`] runs after both grants (the first purchase and the repeat purchase paths). It adds
//!   the tier Shop_SelectItemName names like Saria's gift, and for an Ocarina writes the equip slot
//!   (crate::ocarina_hold::after), so the player gets that tier whatever the game itself added for
//!   the shelf. The tier comes last, so a first tier the game added cannot leave a higher one lower.
//!
//! Untested in the game: whether the game's own add reads Shop_SelectItemName or the shelf.

use std::collections::{BTreeMap as BM, BTreeSet as BS, HashMap as HM};

use crate::bfevfl::flow::NONE;
use crate::bfevfl::{Bf, Case, Cont, Entry, Event, Flow, Kind, Val};
use crate::dungeon_credit::sound;
use crate::error::{Error as E, Res};
use crate::progressive::{owners, tiers};
use crate::shelf::{Job, What};

pub const FLOW: &str = "Event/EventFlow/DisplayShop.bfevfl.zs";
pub const PICK: &str = "OOTRR_ShelfTier";
pub const GIVE: &str = "OOTRR_ShelfGive";
pub const NAME: &str = "Shop_SelectItemName";
/// The shop's item category question right after "buy it?", which every purchase passes.
pub const CHECK: (usize, &str) = (280, "Event27");
/// The two grants and the events they lead to.
pub const GRANTS: [(usize, &str, u16); 2] = [(156, "Event281", 277), (461, "Event543", 430)];

const GRANT: &str = "EventTriggerGetItemAtShop";
const STARTER: &str = "Npc_EventStarter";
const SORT: &str = "EventQueryCheckShopSelectItemName";
const DATA: &str = "GameDataActor";
const ASK: &str = "EventQueryIsGameDataString64";
const COUNT: &str = "EventQueryIsGameDataInt";
const OWNS: &str = "EventQueryCheckIsGet";
const SET: &str = "EventTriggerSetGameDataString64";
const PREFIX: &str = "OOTRR_Tier";

/// A family sold tier by tier: its shelves' sold counters and how many shelves hold it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sold {
    pub family: &'static str,
    pub counters: Vec<String>,
    pub copies: usize,
}

/// The families the seed's tiered shelves sell, in name order.
pub fn plan(jobs: &[Job]) -> Vec<Sold> {
    let mut by: BM<&'static str, Sold> = BM::new();
    for j in jobs {
        let (Some(f), What::Deal { .. }) = (j.tiered, &j.what) else {
            continue;
        };
        let s: &mut Sold = by.entry(f).or_insert_with(|| Sold { family: f, counters: Vec::new(), copies: 0 });
        s.copies += 1;
        for c in &j.counters {
            if !s.counters.contains(c) {
                s.counters.push(c.clone());
            }
        }
    }
    by.into_values().collect()
}

fn idx(n: usize) -> Res<u16> {
    u16::try_from(n).map_err(|_| E::Format("shelf tiers: index does not fit in 16 bits"))
}

fn push(f: &mut Flow, kind: Kind) -> Res<u16> {
    let i: u16 = idx(f.events.len())?;
    f.events.push(Event { name: format!("{PREFIX}{i}"), kind });
    Ok(i)
}

fn cont(items: &[(&str, Val)]) -> Cont {
    Cont { items: items.iter().map(|(k, v)| (k.to_string(), v.clone())).collect() }
}

fn holder(f: &Flow, who: &str) -> Res<usize> {
    f.actors.iter().position(|a| a.name == who && a.sub.is_empty()).ok_or_else(|| E::Msg(format!("shelf tiers: the shop flow has no {who}, the game files differ")))
}

fn query(f: &mut Flow, who: &str, q: &str) -> Res<(u16, u16)> {
    let i: usize = holder(f, who)?;
    let j: usize = match f.actors[i].queries.iter().position(|x| x == q) {
        Some(j) => j,
        None => {
            f.actors[i].queries.push(q.to_string());
            f.actors[i].queries.len() - 1
        }
    };
    Ok((idx(i)?, idx(j)?))
}

fn targets(k: &Kind) -> Vec<u16> {
    match k {
        Kind::Action { next, .. } | Kind::Join { next } | Kind::Sub { next, .. } => vec![*next],
        Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => cases.iter().map(|c| c.ev).collect(),
        Kind::Fork { join, forks } => forks.iter().copied().chain([*join]).collect(),
        Kind::Label { .. } => Vec::new(),
    }
}

fn retarget(k: &mut Kind, from: u16, to: u16) -> usize {
    let mut n: usize = 0;
    let mut swap = |t: &mut u16| {
        if *t == from {
            *t = to;
            n += 1;
        }
    };
    match k {
        Kind::Action { next, .. } | Kind::Join { next } | Kind::Sub { next, .. } => swap(next),
        Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => cases.iter_mut().for_each(|c| swap(&mut c.ev)),
        Kind::Fork { join, forks } => {
            swap(join);
            forks.iter_mut().for_each(swap);
        }
        Kind::Label { .. } => {}
    }
    n
}

fn reach(f: &Flow, from: u16) -> BS<u16> {
    let mut seen: BS<u16> = BS::new();
    let mut todo: Vec<u16> = vec![from];
    while let Some(i) = todo.pop() {
        if i == NONE || !seen.insert(i) {
            continue;
        }
        if let Some(e) = f.events.get(i as usize) {
            todo.extend(targets(&e.kind));
        }
    }
    seen
}

fn action_name(f: &Flow, actor: u16, act: u16) -> Option<(&str, &str)> {
    let a = f.actors.get(actor as usize)?;
    Some((a.name.as_str(), a.actions.get(act as usize)?.as_str()))
}

fn grant_ok(f: &Flow, i: usize, name: &str) -> Res<u16> {
    let e: &Event = f.events.get(i).ok_or_else(|| E::Msg(format!("shelf tiers: the shop flow has no event {i}, the game files differ")))?;
    let Kind::Action { next, actor, act, .. } = &e.kind else {
        return Err(E::Msg(format!("shelf tiers: event {i} is not an action, the game files differ")));
    };
    if e.name != name || action_name(f, *actor, *act) != Some((STARTER, GRANT)) {
        return Err(E::Msg(format!("shelf tiers: event {i} is not the shop grant {name}, the game files differ")));
    }
    Ok(*next)
}

fn check_ok(f: &Flow) -> Res<()> {
    let (i, name): (usize, &str) = CHECK;
    let ok: bool = f.events.get(i).is_some_and(|e| {
        e.name == name && matches!(&e.kind, Kind::Switch { actor, query, .. } if f.actors.get(*actor as usize).is_some_and(|a| a.name == STARTER && a.queries.get(*query as usize).is_some_and(|q| q == SORT)))
    });
    if !ok {
        return Err(E::Msg(format!("shelf tiers: event {i} is not the purchase question {name}, the game files differ")));
    }
    Ok(())
}

fn is_name(f: &mut Flow, value: &str, hit: u16, miss: u16) -> Res<u16> {
    let (a, q): (u16, u16) = query(f, DATA, ASK)?;
    let c: Cont = cont(&[("GameDataName", Val::Str(NAME.to_string())), ("Index", Val::Int(-1)), ("Value", Val::Str(value.to_string()))]);
    push(f, Kind::Switch { actor: a, query: q, cases: vec![Case { val: 0, ev: hit }, Case { val: 1, ev: miss }], params: Some(c) })
}

fn set_name(f: &mut Flow, value: &str) -> Res<u16> {
    let (a, x): (u16, u16) = crate::start_state::slot(f, DATA, SET)?;
    let c: Cont = cont(&[("GameDataName", Val::Str(NAME.to_string())), ("Value", Val::Str(value.to_string())), ("Index", Val::Int(-1))]);
    push(f, Kind::Action { next: NONE, actor: a, act: x, params: Some(c) })
}

/// The event that goes to `sold` once the counters add up to `copies`, and to `open` before.
/// Counter `i` is asked "more than v?" for v = 0, 1, ...; the game answers 2 for more.
fn sold_tree(f: &mut Flow, counters: &[String], copies: usize, sold: u16, open: u16) -> Res<u16> {
    let mut memo: HM<(usize, usize), u16> = HM::new();
    tree_at(f, counters, copies, sold, open, 0, 0, &mut memo)
}

#[allow(clippy::too_many_arguments)]
fn tree_at(f: &mut Flow, cs: &[String], copies: usize, sold: u16, open: u16, i: usize, acc: usize, memo: &mut HM<(usize, usize), u16>) -> Res<u16> {
    if acc >= copies {
        return Ok(sold);
    }
    if i == cs.len() {
        return Ok(open);
    }
    if let Some(e) = memo.get(&(i, acc)) {
        return Ok(*e);
    }
    let (a, q): (u16, u16) = query(f, DATA, COUNT)?;
    // From the last question back: "more than v" sends on to v + 1, otherwise counter i added v.
    let mut more: u16 = sold;
    for v in (0..copies - acc).rev() {
        let rest: u16 = tree_at(f, cs, copies, sold, open, i + 1, acc + v, memo)?;
        let c: Cont = cont(&[("GameDataName", Val::Str(cs[i].clone())), ("Index", Val::Int(-1)), ("Value", Val::Int(i32::try_from(v).map_err(|_| E::Format("shelf tiers: too many copies"))?))]);
        more = push(f, Kind::Switch { actor: a, query: q, cases: vec![Case { val: 0, ev: rest }, Case { val: 1, ev: rest }, Case { val: 2, ev: more }], params: Some(c) })?;
    }
    memo.insert((i, acc), more);
    Ok(more)
}

/// The question chain that names the next tier of `s` the player lacks.
fn next_tier(f: &mut Flow, family: &str) -> Res<u16> {
    let o: &[&str] = owners(family)?;
    let mut gives: Vec<u16> = Vec::new();
    for x in o {
        gives.push(set_name(f, x)?);
    }
    let (a, q): (u16, u16) = crate::progressive::ensure(f, DATA, OWNS)?;
    let root: Kind = crate::progressive::chain(f, a, q, family, o, &gives)?;
    push(f, root)
}

fn pick_entry(f: &mut Flow, sold: &[Sold]) -> Res<u16> {
    let mut miss: u16 = NONE;
    for s in sold.iter().rev() {
        let first: &str = owners(s.family)?.first().copied().ok_or(E::Format("shelf tiers: a family has no tier"))?;
        let open: u16 = next_tier(f, s.family)?;
        let gate: u16 = sold_tree(f, &s.counters, s.copies, NONE, open)?;
        miss = is_name(f, first, gate, miss)?;
    }
    Ok(miss)
}

fn give_entry(f: &mut Flow, sold: &[Sold]) -> Res<u16> {
    let mut miss: u16 = NONE;
    for s in sold.iter().rev() {
        let (t, o): (&[&str], &[&str]) = (tiers(s.family)?, owners(s.family)?);
        for (tier, owner) in t.iter().zip(o).rev() {
            let add: u16 = crate::ocarina_hold::after(f, tier, NONE)?;
            miss = is_name(f, owner, add, miss)?;
        }
    }
    Ok(miss)
}

fn call(f: &mut Flow, entry: &str, next: u16) -> Res<u16> {
    push(f, Kind::Sub { next, params: None, flow: String::new(), entry: entry.to_string() })
}

/// Adds both entries to the shop flow `f` and calls them before the purchase question and after
/// both grants. Every entry that reaches a new call preloads it.
pub fn apply(f: &mut Flow, sold: &[Sold]) -> Res<()> {
    if sold.is_empty() {
        return Err(E::Format("shelf tiers: no family is sold tier by tier"));
    }
    if f.entries.iter().any(|e| e.name == PICK || e.name == GIVE) {
        return Err(E::Format("shelf tiers: the shop flow already holds the tier entries"));
    }
    check_ok(f)?;
    for (i, name, next) in GRANTS {
        if grant_ok(f, i, name)? != next {
            return Err(E::Msg(format!("shelf tiers: the grant {name} does not lead to event {next}, the game files differ")));
        }
    }
    let pad: u16 = f.entries.first().map(|e| e.pad).ok_or(E::Format("shelf tiers: the shop flow has no entries"))?;
    let n: usize = f.events.len();
    let pick: u16 = pick_entry(f, sold)?;
    let give: u16 = give_entry(f, sold)?;
    let at: u16 = idx(CHECK.0)?;
    let before: u16 = call(f, PICK, at)?;
    let mut moved: usize = 0;
    for e in f.events.iter_mut().take(n) {
        moved += retarget(&mut e.kind, at, before);
    }
    if moved == 0 {
        return Err(E::Format("shelf tiers: nothing leads to the purchase question"));
    }
    let mut calls: Vec<u16> = vec![before];
    for (g, _, next) in GRANTS {
        let s: u16 = call(f, GIVE, next)?;
        if let Kind::Action { next: x, .. } = &mut f.events[g].kind {
            *x = s;
        }
        calls.push(s);
    }
    for k in 0..f.entries.len() {
        let r: BS<u16> = reach(f, f.entries[k].main);
        for c in &calls {
            if r.contains(c) && !f.entries[k].subs.contains(c) {
                f.entries[k].subs.push(*c);
            }
        }
    }
    f.entries.push(Entry { name: PICK.to_string(), subs: Vec::new(), vars: Vec::new(), main: pick, pad });
    f.entries.push(Entry { name: GIVE.to_string(), subs: Vec::new(), vars: Vec::new(), main: give, pad });
    Ok(())
}

/// What may differ from the game's flow: new events past its own, the retargeted way into the
/// purchase question and the two grants' nexts, new sub calls in the entries' preload lists, new
/// actions and questions on the game's own actors, and the two entries.
pub fn audit(a: &Flow, b: &Flow) -> Vec<String> {
    let mut bad: Vec<String> = sound(b);
    if a.name != b.name || b.actors.len() != a.actors.len() || b.events.len() <= a.events.len() || b.entries.len() != a.entries.len() + 2 {
        bad.push("the flow lost or gained actors, events or entries it should not".to_string());
        return bad;
    }
    for (i, (x, y)) in a.actors.iter().zip(&b.actors).enumerate() {
        let grown: bool = y.actions.starts_with(&x.actions) && y.queries.starts_with(&x.queries) && {
            let mut z = y.clone();
            z.actions.truncate(x.actions.len());
            z.queries.truncate(x.queries.len());
            z == *x
        };
        if !grown {
            bad.push(format!("actor {i} {} changed in more than new actions and questions", x.name));
        }
    }
    let new = |t: u16| (t as usize) >= a.events.len() && t != NONE;
    for (i, (x, y)) in a.events.iter().zip(&b.events).enumerate() {
        if x == y {
            continue;
        }
        let (tx, ty): (Vec<u16>, Vec<u16>) = (targets(&x.kind), targets(&y.kind));
        let same_but_targets: bool = x.name == y.name && tx.len() == ty.len() && {
            let mut z: Kind = y.kind.clone();
            for (p, q) in tx.iter().zip(&ty) {
                if p != q {
                    retarget(&mut z, *q, *p);
                }
            }
            z == x.kind
        };
        let moved: bool = tx.iter().zip(&ty).all(|(p, q)| p == q || (*p as usize == CHECK.0 || GRANTS.iter().any(|g| g.0 == i)) && new(*q));
        if !same_but_targets || !moved {
            bad.push(format!("event {i} {} changed", x.name));
        }
    }
    for (x, y) in a.entries.iter().zip(&b.entries) {
        let mut z: Entry = y.clone();
        z.subs.truncate(x.subs.len());
        let added: bool = y.subs.len() >= x.subs.len() && y.subs[x.subs.len()..].iter().all(|s| matches!(b.events.get(*s as usize).map(|e| &e.kind), Some(Kind::Sub { entry, flow, .. }) if flow.is_empty() && (entry == PICK || entry == GIVE)));
        if z != *x || !added {
            bad.push(format!("entry {} changed", x.name));
        }
    }
    let names: Vec<&str> = b.entries.iter().skip(a.entries.len()).map(|e| e.name.as_str()).collect();
    if names != [PICK, GIVE] {
        bad.push(format!("the new entries are {names:?}"));
    }
    // Every sub call an entry reaches is in its preload list (a missing one crashed the game).
    for e in &b.entries {
        for i in reach(b, e.main) {
            if (i as usize) >= a.events.len() && matches!(b.events.get(i as usize).map(|x| &x.kind), Some(Kind::Sub { .. })) && !e.subs.contains(&i) {
                bad.push(format!("entry {} reaches the call {i} without preloading it", e.name));
            }
        }
    }
    bad
}

pub fn edit(raw: &[u8], sold: &[Sold]) -> Res<Vec<u8>> {
    let orig: Bf = Bf::parse(raw)?;
    let mut bf: Bf = orig.clone();
    let before: &Flow = orig.flows.first().ok_or(E::Format("shelf tiers: the shop flow file holds no flow"))?;
    let f: &mut Flow = bf.flows.first_mut().ok_or(E::Format("shelf tiers: the shop flow file holds no flow"))?;
    apply(f, sold)?;
    if let Some(m) = audit(before, f).first() {
        return Err(E::Msg(format!("shelf tiers: the edited shop flow fails its audit, {m}")));
    }
    let out: Vec<u8> = bf.write()?;
    if Bf::parse(&out)? != bf {
        return Err(E::Format("shelf tiers: the written shop flow does not parse back to the edited model"));
    }
    Ok(out)
}

/// The tier each owned-tier answer leads [`PICK`] to name, and whether a shelf sells, walked on
/// the edited flow: `owned` tells which tier owners the player has, `sold` the counter values.
pub fn run_pick(f: &Flow, name: &str, owned: &dyn Fn(&str) -> bool, sold: &dyn Fn(&str) -> i32) -> Res<String> {
    let e: &Entry = f.entries.iter().find(|e| e.name == PICK).ok_or_else(|| E::Msg(format!("the shop flow has no {PICK}")))?;
    let mut at: u16 = e.main;
    let mut cur: String = name.to_string();
    for _ in 0..4096 {
        if at == NONE {
            return Ok(cur);
        }
        let ev: &Event = f.events.get(at as usize).ok_or_else(|| E::Msg(format!("{PICK} leads to event {at} outside the flow")))?;
        at = match &ev.kind {
            Kind::Switch { actor, query, cases, params } => {
                let q: &str = f.actors.get(*actor as usize).and_then(|a| a.queries.get(*query as usize)).map(|s| s.as_str()).unwrap_or("");
                let p = |k: &str| params.as_ref().and_then(|c| c.get(k)).cloned();
                let val: u32 = match q {
                    ASK => u32::from(p("Value") != Some(Val::Str(cur.clone()))),
                    OWNS => match p("ActorName") {
                        Some(Val::Str(a)) => u32::from(!owned(&a)),
                        _ => return Err(E::Msg(format!("{} asks about no actor", ev.name))),
                    },
                    COUNT => match (p("GameDataName"), p("Value")) {
                        (Some(Val::Str(g)), Some(Val::Int(v))) => match sold(&g).cmp(&v) {
                            std::cmp::Ordering::Less => 0,
                            std::cmp::Ordering::Equal => 1,
                            std::cmp::Ordering::Greater => 2,
                        },
                        _ => return Err(E::Msg(format!("{} counts nothing", ev.name))),
                    },
                    _ => return Err(E::Msg(format!("{} asks {q}", ev.name))),
                };
                cases.iter().find(|c| c.val == val).map(|c| c.ev).unwrap_or(NONE)
            }
            Kind::Action { next, params, .. } => {
                if let Some(Val::Str(v)) = params.as_ref().and_then(|c| c.get("Value")) {
                    cur = v.clone();
                }
                *next
            }
            _ => return Err(E::Msg(format!("{PICK} reaches {}, which is neither a question nor an action", ev.name))),
        };
    }
    Err(E::Format("the tier entry does not end"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::romfs::Romfs;

    fn real() -> Option<Romfs> {
        let p = ootrr_game::test_romfs::path();
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Romfs::open(&p).ok()
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    fn job(f: &'static str, item: &str, counters: &[&str]) -> Job {
        Job {
            label: "x".into(),
            shop: "KF Shop".into(),
            slot: 0,
            keepers: Vec::new(),
            bancs: Vec::new(),
            counters: counters.iter().map(|s| s.to_string()).collect(),
            what: What::Deal { item: item.to_string(), price: 10, model: None },
            tiered: Some(f),
        }
    }

    #[test]
    fn the_plan_counts_the_shelves_and_their_counters_per_family() {
        let mut plain: Job = job("Hookshot", "Item_Hookshot", &["C.0"]);
        plain.tiered = None;
        let js: Vec<Job> = vec![job("Strength", "Item_Gauntlet_Goron", &["A.1", "B.1"]), job("Strength", "Item_Gauntlet_Goron", &["A.2"]), plain];
        assert_eq!(plan(&js), vec![Sold { family: "Strength", counters: vec!["A.1".into(), "B.1".into(), "A.2".into()], copies: 2 }]);
    }

    #[test]
    fn the_shop_sells_the_next_tier_once_per_shelf_and_adds_it_after_the_grant() {
        let Some(mut rf) = real() else { return };
        let raw: Vec<u8> = rf.read_zs(FLOW).unwrap();
        let sold: Vec<Sold> = vec![
            Sold { family: "Hookshot", counters: vec!["H.0".into()], copies: 1 },
            Sold { family: "Strength", counters: vec!["S.0".into(), "S.1".into()], copies: 2 },
        ];
        let out: Vec<u8> = edit(&raw, &sold).unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        let f: &Flow = &b.flows[0];
        let none = |_: &str| 0;
        let owns = |have: Vec<&'static str>| move |a: &str| have.contains(&a);
        assert_eq!(run_pick(f, "Item_Hookshot", &owns(vec![]), &none).unwrap(), "Item_Hookshot");
        assert_eq!(run_pick(f, "Item_Hookshot", &owns(vec!["Item_Hookshot"]), &none).unwrap(), "Item_Longshot");
        let once = |g: &str| i32::from(g == "H.0");
        assert_eq!(run_pick(f, "Item_Hookshot", &owns(vec!["Item_Hookshot"]), &once).unwrap(), "Item_Hookshot", "a sold shelf names the owned first tier");
        let g = "Armor_Child_Gauntlet_Goron";
        assert_eq!(run_pick(f, g, &owns(vec![g]), &|c: &str| i32::from(c == "S.1")).unwrap(), "Armor_Adult_Gauntlet_Silver");
        assert_eq!(run_pick(f, g, &owns(vec![g, "Armor_Adult_Gauntlet_Silver"]), &|c: &str| if c == "S.1" { 2 } else { 0 }).unwrap(), g, "two sales of one shelf use both copies");
        assert_eq!(run_pick(f, "Item_Boomerang", &owns(vec![]), &none).unwrap(), "Item_Boomerang");
        for (g, _, next) in GRANTS {
            let Kind::Action { next: s, .. } = &f.events[g].kind else { panic!() };
            assert!(matches!(&f.events[*s as usize].kind, Kind::Sub { next: n, entry, .. } if *n == next && entry == GIVE));
        }
        let n: usize = Bf::parse(&raw).unwrap().flows[0].events.len();
        let into: Vec<usize> = (0..n).filter(|&i| targets(&f.events[i].kind).contains(&(CHECK.0 as u16))).collect();
        assert!(into.iter().all(|&i| matches!(&f.events[i].kind, Kind::Sub { entry, .. } if entry == PICK)), "{into:?}");
        assert!(edit(&out, &sold).is_err());
    }
}
