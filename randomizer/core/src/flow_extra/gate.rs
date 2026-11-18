use crate::flow_locations::Loc;
use crate::slice::data::{Data, Item};
use crate::slice::support::Plan;

pub const POUCH_ONLY: [&str; 1] = ["GC Medigoron"];

pub const MEDIGORON: &str = "GC Medigoron";

pub fn open(l: &Loc, p: &Plan) -> bool {
    if crate::scrubs_a::is_a(l) {
        return p.rewards.scrubs;
    }
    if crate::scrubs_b::is_b(l) {
        return p.rewards.scrubs_b;
    }
    if crate::flow_extra::trade::is(&l.name) {
        return p.rewards.trade;
    }
    if crate::flow_extra::cows::is_other(&l.name) {
        return p.rewards.cows_other;
    }
    if crate::flow_extra::cows::is(&l.name) {
        return p.rewards.cows;
    }
    if crate::flow_extra::frogs::is_song(&l.name) {
        return p.rewards.frog_songs;
    }
    if crate::flow_extra::frogs::is_bug(&l.name) {
        return p.rewards.frog_bug;
    }
    if crate::flow_extra::potion::is(&l.name) {
        return p.rewards.potion;
    }
    if crate::open_world::skips::is_bean_sale(&l.name) {
        return p.rewards.bean_sale;
    }
    if l.name == MEDIGORON {
        return p.rewards.medigoron;
    }
    !l.gated || p.rewards.extra
}

pub fn pouch(i: &Item) -> bool {
    i.actor.as_deref().is_some_and(|a| (a.starts_with("Item_") || a.starts_with("Weapon_")) && (!a.starts_with("Item_Melody_") || crate::slice::generate::is_song(a)))
}

pub fn bound(d: &Data, item: &str) -> bool {
    d.flow.locs.iter().any(|l| l.gated && !crate::flow_extra::trade::is(&l.name) && !crate::flow_extra::cows::is(&l.name) && !crate::flow_extra::frogs::is(&l.name) && !crate::flow_extra::potion::is(&l.name) && !crate::scrubs_a::is_a(l) && !crate::scrubs_b::is_b(l) && !crate::open_world::skips::is_bean_sale(&l.name) && d.vanilla_of(&l.name) == Some(item) && d.flow_ok(&l.name))
}

/// Whether a gated reward spot can hold `i`. The items bound to a gated reward (bound: the
/// Biggoron Sword, the Giant's Knife, Kak 20's item) may go anywhere: the Biggoron and Medigoron
/// edits (data/flow_extra.json) drop the checks that made owning them early close those events
/// (Biggoron's knife checks, Medigoron's offer on the Biggoron Sword and his stock check on the
/// knife). An unshuffled Medigoron still closes his Giant's Knife sale on an early Biggoron Sword,
/// which only glitch logic uses.
pub fn fits(_d: &Data, name: &str, i: &Item) -> bool {
    !(POUCH_ONLY.contains(&name) && !pouch(i))
}

#[cfg(test)]
mod tests {
    use crate::settings::{Settings, Value};
    use crate::slice::data::Data;
    use crate::slice::generate::{generate_data, GenOpts, Generated};
    use crate::slice::support::plan;

    const SHUFFLED: [&str; 3] = ["Kak 20 Gold Skulltula Reward", "DMT Biggoron", "GC Medigoron"];

    fn with(extra: Option<bool>) -> Settings {
        let mut s: Settings = Settings::default();
        if let Some(x) = extra {
            s.set("shuffle_rewards", Value::s("supported")).unwrap();
            s.set(crate::flow_extra::SETTING, Value::Bool(x)).unwrap();
        }
        s
    }

    fn gated(d: &Data) -> Vec<String> {
        d.flow.locs.iter().filter(|l| l.gated && !crate::flow_extra::trade::is(&l.name) && !crate::flow_extra::cows::is(&l.name) && !crate::flow_extra::frogs::is(&l.name) && !crate::flow_extra::potion::is(&l.name) && !crate::scrubs_a::is_a(l) && !crate::scrubs_b::is_b(l) && !crate::open_world::skips::is_bean_sale(&l.name)).map(|l| l.name.clone()).collect()
    }

    #[test]
    fn the_setting_is_on_by_default_and_needs_supported_rewards() {
        assert!(plan(&Settings::default()).unwrap().rewards.extra);
        assert!(!plan(&with(Some(false))).unwrap().rewards.extra);
        assert!(plan(&with(Some(true))).unwrap().rewards.extra);
        let mut s: Settings = Settings::default();
        s.set("shuffle_rewards", Value::s("off")).unwrap();
        s.set(crate::flow_extra::SETTING, Value::Bool(true)).unwrap();
        assert!(!plan(&s).unwrap().rewards.extra);
        assert_eq!(Settings::default().encode().unwrap(), with(None).encode().unwrap());
    }

    #[test]
    fn gated_locations_stay_vanilla_unless_the_setting_is_on() {
        let d: Data = Data::load().unwrap();
        let names: Vec<String> = gated(&d);
        assert_eq!(names.len(), 3);
        let g: Generated = generate_data(&d, &with(Some(false)), "gate 1", &GenOpts::default()).unwrap();
        assert!(names.iter().all(|x| !g.seed_file.placements.contains_key(x)));
        // On by default, and Medigoron follows Shuffle Merchants, which is off by default.
        let g: Generated = generate_data(&d, &with(None), "gate 0", &GenOpts::default()).unwrap();
        for x in &names {
            assert_eq!(g.seed_file.placements.contains_key(x), x != super::MEDIGORON, "{x}");
        }
    }

    #[test]
    fn the_setting_and_shuffle_merchants_add_three_beatable_locations_and_rolling_goron_needs_no_setting() {
        let d: Data = Data::load().unwrap();
        let mut s: Settings = with(Some(true));
        s.set(crate::flow_extra::MERCHANTS_SETTING, Value::s("no_beans")).unwrap();
        let off: Generated = generate_data(&d, &with(Some(false)), "extra", &GenOpts::default()).unwrap();
        for n in 0..3 {
            let g: Generated = generate_data(&d, &s, &format!("extra {n}"), &GenOpts::default()).unwrap();
            for x in SHUFFLED {
                assert!(g.seed_file.placements.contains_key(x), "{x}");
            }
            assert!(g.seed_file.placements.contains_key("GC Rolling Goron as Child"));
            assert_eq!(g.seed_file.placements.len(), off.seed_file.placements.len() + SHUFFLED.len());
            assert!(g.report.contains("structural event edits"));
            for x in SHUFFLED {
                let it: &str = &g.seed_file.placements[x];
                assert!(d.item(it).is_some_and(|i| crate::slice::flow_rules::allows(&d, x, i)), "{x} holds {it}");
            }
            let medi: &str = &g.seed_file.placements["GC Medigoron"];
            assert!(d.item(medi).is_some_and(super::pouch), "GC Medigoron holds {medi}");
        }
    }

    #[test]
    fn only_the_vanilla_items_of_the_pooled_gated_locations_are_bound() {
        let d: Data = Data::load().unwrap();
        let mut got: Vec<&str> = ["Stone of Agony", "Biggoron Sword", "Giants Knife"].into_iter().filter(|x| super::bound(&d, x)).collect();
        got.sort_unstable();
        assert_eq!(got, vec!["Biggoron Sword", "Giants Knife", "Stone of Agony"]);
        for x in ["Rupees (5)", "Bomb Bag", "Progressive Hookshot", "Piece of Heart"] {
            assert!(!super::bound(&d, x), "{x}");
        }
        assert!(!super::pouch(d.item("Rupees (50)").unwrap()));
        assert!(!super::pouch(d.item("Piece of Heart").unwrap()));
        assert!(super::pouch(d.item("Giants Knife").unwrap()));
    }
}
