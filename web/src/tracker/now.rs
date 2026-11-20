use std::collections::BTreeMap as BM;

use ootrr_core::error::{Error as E, Res};
use ootrr_core::murmur::murmur3_32;
use ootrr_core::rules::Age;
use ootrr_core::settings::Settings;
use ootrr_core::settings::seed::SeedFile;
use ootrr_core::slice::data::Data;
use ootrr_core::slice::rebuild::{rebuild_data, Rebuilt};
use crate::tracker::area;
use ootrr_core::slice::edit::skipped_locations;
use crate::tracker::cond;
use crate::tracker::npc_flags;
use crate::tracker::peek::Peek;
use crate::tracker::reach::{only_age, open_now};
use crate::tracker::seedlog::{self, SeedLog};
use crate::tracker::targets::{self, Target};
use ootrr_core::world::LocId;

pub const GAME: &str = "oot-remake:spoiler";
pub const PIECE: &str = "Piece of Heart";
pub const CONTAINER: &str = "Heart Container";
pub const QUARTERS: i64 = 4;

const EXTRA: [(&str, &str); 2] = [
    ("Market Potion Shop Free Bottle", "Market_Item_Bottle_TutorialEvent_Started"),
    ("Market Potion Shop Red Potion", "Market_Item_Bottle_TutorialEvent_Cleared"),
];

#[derive(Debug, Clone, PartialEq)]
pub struct Spot {
    pub name: String,
    pub label: String,
    pub area: String,
    pub kind: String,
    pub targets: Vec<usize>,
    pub flags: Vec<u32>,
    pub eqs: Vec<(u32, u32)>,
    pub keys: Vec<u64>,
    pub dungeon: bool,
    pub age: Option<Age>,
    pub hidden: bool,
}

impl Spot {
    pub fn sensed(&self) -> bool {
        !self.targets.is_empty() || !self.flags.is_empty() || !self.eqs.is_empty() || !self.keys.is_empty()
    }
}

pub struct Now {
    pub log: SeedLog,
    pub rb: Rebuilt,
    pub spots: Vec<Spot>,
    pub start_hearts: i64,
    pub given: Vec<String>,
}

fn h(s: &str) -> u32 {
    murmur3_32(s.as_bytes(), 0)
}

fn spot(d: &Data, ts: &[Target], rb: &Rebuilt, l: LocId) -> Spot {
    let name: String = rb.w.locs[l].name.clone();
    let tix: Vec<usize> = ts.iter().enumerate().filter(|(_, t)| t.ootr.as_deref() == Some(name.as_str())).map(|(i, _)| i).collect();
    let cat = d.cat.by_name(&name);
    let mut flags: Vec<u32> = Vec::new();
    let mut eqs: Vec<(u32, u32)> = Vec::new();
    let mut keys: Vec<u64> = Vec::new();
    if tix.is_empty() {
        if let Some(c) = cat {
            for r in &c.refs {
                if c.kind == "chest" {
                    flags.push(h(&format!("IsTreasureBoxOpen.{}", r.hash)));
                } else {
                    keys.push(r.hash);
                }
            }
        }
        for f in npc_flags::of(&name).unwrap_or(&[]) {
            match cond::eq_of(f) {
                Some(e) => eqs.push(e),
                None => flags.push(h(f)),
            }
        }
        for (n, f) in EXTRA {
            if n == name {
                flags.push(h(f));
            }
        }
    }
    let reg = &rb.w.regions[rb.w.locs[l].region];
    let area: String = reg
        .dungeon
        .clone()
        .or_else(|| area::of_name(&name).map(|a| a.to_string()))
        .or_else(|| tix.iter().find_map(|i| ts[*i].area.clone()))
        .or_else(|| cat.map(|c| c.area.clone()).filter(|a| !a.is_empty()))
        .or_else(|| reg.scene.clone())
        .unwrap_or_else(|| reg.name.clone());
    let kind: String = tix.first().map(|i| ts[*i].kind.clone()).or_else(|| cat.map(|c| c.kind.clone())).unwrap_or_else(|| "npc".to_string());
    let area: String = area::canon(&area, &rb.w);
    let label: String = area::short(&name, &area);
    let dungeon: bool = reg.dungeon.is_some();
    let hidden: bool = ootrr_core::slice::tracker_fixes::GONE.contains(&name.as_str());
    Spot { name, label, area, kind, targets: tix, flags, eqs, keys, dungeon, age: None, hidden }
}

impl Now {
    pub fn from_log(log: SeedLog) -> Res<Now> {
        let s: Settings = Settings::decode(&log.settings_string)
            .map_err(|e| E::Msg(format!("spoiler: the settings string is not usable: {e}")))?
            .settings;
        let d: Data = Data::load()?;
        let rb: Rebuilt = rebuild_data(&d, &s, &log.seed, &log.placements)?;
        if !rb.missing.is_empty() {
            return Err(E::Msg(format!(
                "spoiler: {} shuffled locations are not in the spoiler, for example {}. The spoiler was made by another version or other settings",
                rb.missing.len(),
                rb.missing[0]
            )));
        }
        let ts: &[Target] = targets::all()?;
        let mut spots: Vec<Spot> = rb.free.iter().map(|l| spot(&d, ts, &rb, *l)).collect();
        for (s, a) in spots.iter_mut().zip(only_age(&rb)) {
            s.age = a.filter(|_| !s.dungeon);
        }
        let start_hearts: i64 = s.int("starting_hearts").unwrap_or(3);
        let p: Option<ootrr_core::slice::support::Plan> = ootrr_core::slice::support::plan(&s).ok();
        let given: Vec<String> = p.as_ref().map(|p| skipped_locations(p).iter().map(|x| x.to_string()).collect()).unwrap_or_default();
        let out: &[String] = p.as_ref().map(|p| p.excluded.as_slice()).unwrap_or(&[]);
        for sp in spots.iter_mut().filter(|sp| out.contains(&sp.name)) {
            sp.hidden = true;
        }
        Ok(Now { log, rb, spots, start_hearts, given })
    }

    pub fn load(text: &str) -> Res<Now> {
        Now::from_log(seedlog::parse(text)?)
    }

    pub fn seed_file(&self) -> SeedFile {
        SeedFile {
            seed: self.log.seed.clone(),
            settings_string: self.log.settings_string.clone(),
            game: GAME.to_string(),
            generator: "ootrr spoiler".to_string(),
            placements: self.log.placements.clone(),
            shops: BM::new(),
            entrances: BM::new(),
        }
    }

    pub fn checked(&self, found: &dyn Fn(usize) -> bool, p: Option<&Peek>, manual: &dyn Fn(&str) -> bool) -> Vec<bool> {
        self.spots
            .iter()
            .map(|s| {
                manual(&s.name)
                    || self.given.contains(&s.name)
                    || s.targets.iter().any(|i| found(*i))
                    || p.is_some_and(|p| s.flags.iter().any(|f| p.on(*f)) || s.eqs.iter().any(|e| cond::holds(p, e)) || s.keys.iter().any(|k| p.has_key(*k)))
            })
            .collect()
    }

    pub fn item(&self, k: usize) -> &str {
        self.spots.get(k).and_then(|s| self.log.placements.get(&s.name)).map(|x| x.as_str()).unwrap_or("")
    }

    pub fn spare_pieces(&self, p: &Peek, checked: &[bool]) -> Option<i64> {
        let max: i64 = p.val(h("PlayerStatus.MaxLife"))? as i32 as i64;
        let stock: i64 = p.val(h("PouchData.Obj_Heart_Piece.StockNum")).unwrap_or(0) as i32 as i64;
        let held = |it: &str| (0..self.spots.len()).filter(|k| checked.get(*k).copied().unwrap_or(false) && self.item(*k) == it).count() as i64;
        let owned: i64 = max - QUARTERS * (self.start_hearts + held(CONTAINER)) + stock;
        Some(owned - held(PIECE))
    }

    pub fn open(&self, checked: &[bool]) -> Vec<bool> {
        open_now(&self.rb, checked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ootrr_core::slice::generate::generate;

    fn log_of(seed: &str) -> SeedLog {
        let s: Settings = Settings::default();
        let g = generate(&s, seed).unwrap();
        SeedLog { seed: seed.to_string(), settings_string: s.encode().unwrap(), checksum: String::new(), placements: g.seed_file.placements }
    }

    #[test]
    fn a_spoiler_maps_most_locations_to_a_sensor_and_opens_some_at_the_start() {
        let n: Now = Now::from_log(log_of("now")).unwrap();
        let sensed: usize = n.spots.iter().filter(|s| s.sensed()).count();
        assert!(sensed * 10 >= n.spots.len() * 9, "{sensed} of {}", n.spots.len());
        let none: Vec<bool> = n.checked(&|_| false, None, &|_| false);
        assert!(none.iter().all(|x| !*x));
        let open: Vec<bool> = n.open(&none);
        assert!(open.iter().any(|x| *x) && !open.iter().all(|x| *x));
        let first: &str = &n.spots[0].name;
        let one: Vec<bool> = n.checked(&|_| false, None, &|s| s == first);
        assert!(one[0] && one.iter().filter(|x| **x).count() == 1);
    }

    #[test]
    fn only_chests_the_game_never_spawns_are_hidden() {
        let n: Now = Now::from_log(log_of("hide")).unwrap();
        for s in &n.spots {
            assert_eq!(s.hidden, s.name == "KF West Ledge Chest", "{}", s.name);
        }
        assert!(n.spots.iter().any(|s| !s.hidden));
    }

    #[test]
    fn excluded_checks_are_hidden() {
        let mut s: Settings = Settings::default();
        s.set("disabled_locations", ootrr_core::settings::Value::set(&["ZD Chest", "LH Child Fishing"])).unwrap();
        let g = generate(&s, "out").unwrap();
        let log: SeedLog = SeedLog { seed: "out".to_string(), settings_string: s.encode().unwrap(), checksum: String::new(), placements: g.seed_file.placements };
        let n: Now = Now::from_log(log).unwrap();
        for s in &n.spots {
            assert_eq!(s.hidden, ["ZD Chest", "LH Child Fishing", "KF West Ledge Chest"].contains(&s.name.as_str()), "{}", s.name);
        }
    }

    #[test]
    fn heart_pieces_from_known_spots_leave_none_spare() {
        use crate::tracker::peek::Place;
        let n: Now = Now::from_log(log_of("hearts")).unwrap();
        let mut rows: Vec<(u32, u32)> = vec![(h("PlayerStatus.MaxLife"), (QUARTERS * n.start_hearts) as u32), (h("PouchData.Obj_Heart_Piece.StockNum"), 2)];
        rows.sort_unstable();
        let p: Peek = Peek { rows, keys: Vec::new(), len: 2, place: Place { map: None, child: true, room: None, pos: None } };
        let mut c: Vec<bool> = vec![false; n.spots.len()];
        assert_eq!(n.spare_pieces(&p, &c), Some(2));
        for k in (0..n.spots.len()).filter(|k| n.item(*k) == PIECE).take(2) {
            c[k] = true;
        }
        assert_eq!(n.spare_pieces(&p, &c), Some(0));
    }

    #[test]
    fn an_open_kakariko_counts_the_checks_handed_out_at_new_game() {
        let mut s: Settings = Settings::default();
        for (k, v) in [("shuffle_rewards", "supported"), ("open_forest", "open"), ("open_kakariko", "open")] {
            s.set(k, ootrr_core::settings::Value::s(v)).unwrap();
        }
        let g = generate(&s, "kak").unwrap();
        let log: SeedLog = SeedLog { seed: "kak".to_string(), settings_string: s.encode().unwrap(), checksum: String::new(), placements: g.seed_file.placements };
        let n: Now = Now::from_log(log).unwrap();
        let c: Vec<bool> = n.checked(&|_| false, None, &|_| false);
        let on: Vec<&str> = n.spots.iter().zip(c.iter()).filter(|(_, x)| **x).map(|(s, _)| s.name.as_str()).collect();
        assert!(on.contains(&"HC Zeldas Letter"));
        assert!(on.iter().all(|x| n.given.iter().any(|g| g == x)));
        assert!(!n.spots.iter().any(|s| s.name == "HC Malon Egg"));
        assert!(Now::from_log(log_of("plain")).unwrap().given.is_empty());
    }

    #[test]
    fn the_seed_file_carries_the_spoiler_placements() {
        let n: Now = Now::from_log(log_of("now2")).unwrap();
        let f: SeedFile = n.seed_file();
        assert_eq!(f.placements, n.log.placements);
        assert!(f.settings().is_ok());
    }

    #[test]
    fn the_real_spoiler_loads_when_one_is_given() {
        let Ok(p) = std::env::var("OOTRR_SPOILER") else {
            return;
        };
        let n: Now = Now::load(&std::fs::read_to_string(p).unwrap()).unwrap();
        let open: Vec<bool> = n.open(&vec![false; n.spots.len()]);
        for (s, o) in n.spots.iter().zip(open.iter()) {
            if *o {
                println!("open {} | {} | {}", s.area, s.name, s.sensed());
            }
        }
        for s in n.spots.iter().filter(|s| !s.sensed()) {
            println!("blind {}", s.name);
        }
    }
}
