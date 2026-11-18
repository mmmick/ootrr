use crate::logic_items::ItemId;
use crate::slice::data::Data;
use crate::slice::pickups as pu;
use crate::slice::support::{Plan, Songs};
use crate::world::{LocId, World};

pub struct Spot<'a> {
    pub d: &'a Data,
    pub w: &'a World,
    pub p: &'a Plan,
    pub it: ItemId,
    pub l: LocId,
    pub name: &'a str,
    pub item: &'a str,
}

pub type Rule = fn(&Spot) -> Option<bool>;

pub const RULES: [(&str, Rule); 16] = [
    ("ganon's boss key on the light arrows cutscene", lacs_key),
    ("dungeon rewards at the end of dungeons", end_of_dungeons),
    ("excluded locations", excluded),
    ("junk only spots", junk),
    ("silver rupees", silver),
    ("trade items", trade),
    ("skipped locations", skipped),
    ("gated npc rewards keep their vanilla items", gated),
    ("shops", shops),
    ("song spots", songs),
    ("treasure chest game", treasure),
    ("remake overworld chests", remake),
    ("freestanding pickups", freestanding),
    ("pots, crates and other holders", holders),
    ("gold skulltulas", skulls),
    ("world only pickups", world_only),
];

pub fn verdict(s: &Spot) -> Option<bool> {
    RULES.iter().find_map(|(_, r)| r(s))
}

fn lacs_key(s: &Spot) -> Option<bool> {
    if s.p.keys.ganon != crate::slice::support::Keep::Lacs {
        return None;
    }
    if s.name == crate::lacs_key::LOC {
        return Some(s.item == crate::lacs_key::ITEM);
    }
    (s.item == crate::lacs_key::ITEM).then_some(false)
}

/// Dungeon Rewards = End of dungeons: a Stone or Medallion only goes to a boss or Rauru reward
/// event, and those events only take a Stone or Medallion. A fitting pair is left to the other rules.
fn end_of_dungeons(s: &Spot) -> Option<bool> {
    if !s.p.rewards.dungeon_own {
        return None;
    }
    let item: bool = s.d.item(s.item).is_some_and(|i| i.kind == "DungeonReward");
    let spot: bool = s.d.flow.by_name(s.name).is_some_and(|l| l.cat == crate::slice::support::DUNGEON_REWARD_CAT);
    (item != spot).then_some(false)
}

fn excluded(s: &Spot) -> Option<bool> {
    (!s.p.excluded.is_empty() && s.w.items.adv[s.it] && s.p.excluded.iter().any(|e| *e == s.name)).then_some(false)
}

fn junk(s: &Spot) -> Option<bool> {
    crate::slice::junk_spots::refuses(s.d, s.w, s.p, s.it, s.l).then_some(false)
}

fn silver(s: &Spot) -> Option<bool> {
    crate::slice::silver::at(s.d, s.w, s.p, s.it, s.l)
}

fn trade(s: &Spot) -> Option<bool> {
    s.d.item(s.item).is_some_and(|i| !crate::flow_extra::trade::keeps(s.d, s.name, i)).then_some(false)
}

fn skipped(s: &Spot) -> Option<bool> {
    (crate::slice::edit::skipped_locations(s.p).contains(&s.name) && !s.d.item(s.item).is_some_and(|i| i.actor.is_some() || !i.tiers.is_empty())).then_some(false)
}

fn gated(s: &Spot) -> Option<bool> {
    (s.p.rewards.extra && s.d.item(s.item).is_some_and(|i| !crate::flow_extra::fits(s.d, s.name, i))).then_some(false)
}

fn shops(s: &Spot) -> Option<bool> {
    crate::shops::rules::at(s.d, &s.p.shops, s.p.songs == Songs::Anywhere, s.name, s.item)
}

fn songs(s: &Spot) -> Option<bool> {
    crate::slice::song_spots::gate(s.d, s.p, s.name, s.item)
}

fn treasure(s: &Spot) -> Option<bool> {
    crate::slice::treasure::at(s.p.treasure, s.name, s.item)
}

fn remake(s: &Spot) -> Option<bool> {
    (s.p.remake && crate::slice::remake_overworld::keeps_out(s.d, s.name, s.item)).then_some(false)
}

fn freestanding(s: &Spot) -> Option<bool> {
    if !s.p.world.on() {
        return None;
    }
    pu::at(s.d, s.name, s.item).or_else(|| pu::keeps_out(s.d, s.name, s.item).then_some(false))
}

fn holders(s: &Spot) -> Option<bool> {
    crate::pot_slice::rules::at(s.d, s.name, s.item)
}

fn skulls(s: &Spot) -> Option<bool> {
    if s.p.skulls == crate::slice::skulls::Mode::Off {
        return None;
    }
    crate::slice::skulls::accepts(s.d, s.w, s.it, s.l)
}

fn world_only(s: &Spot) -> Option<bool> {
    (pu::WORLD_ONLY.contains(&s.item) || s.d.pots.only.contains(s.item)).then_some(false)
}
