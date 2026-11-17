use std::collections::BTreeSet as BS;
use std::path::PathBuf as PB;
use std::sync::Arc;
use std::time::SystemTime;

use ootrr_core::json::Json;
use ootrr_core::rules::Age;
use ootrr_core::spoiler::shown;
use crate::tracker::now::{Now, PIECE};
use crate::tracker::peek::Peek;
use crate::tracker::rules::Hit;
use crate::tracker::state::Track;

#[derive(Default)]
pub struct NowState {
    pub now: Option<Arc<Now>>,
    pub path: Option<PB>,
    pub stamp: Option<SystemTime>,
    pub tried: Option<(PB, Option<SystemTime>)>,
    pub error: Option<String>,
    pub pin: Option<PB>,
    pub manual: BS<String>,
    pub checked: Vec<bool>,
    pub open: Vec<bool>,
    pub maybe: Vec<bool>,
    pub active: Option<String>,
    pub last: Option<usize>,
}

const BURST: usize = 3;

impl NowState {
    pub fn key(&self) -> Option<String> {
        self.now.as_ref().map(|n| format!("{} {}", n.log.seed, n.log.checksum))
    }

    pub fn refresh(&mut self, track: &Track, p: Option<&Peek>) -> bool {
        let Some(n) = self.now.clone() else {
            let had: bool = !self.checked.is_empty();
            self.checked.clear();
            self.open.clear();
            self.maybe.clear();
            self.active = None;
            self.last = None;
            return had;
        };
        let found = |i: usize| track.last.get(i) == Some(&Hit::On);
        let man = |s: &str| self.manual.contains(s);
        let c: Vec<bool> = n.checked(&found, p, &man);
        let o: Vec<bool> = if c == self.checked && self.open.len() == c.len() { self.open.clone() } else { n.open(&c) };
        let spare: Option<i64> = p.and_then(|p| n.spare_pieces(p, &c));
        let m: Vec<bool> = n
            .spots
            .iter()
            .enumerate()
            .map(|(k, s)| {
                !c[k] && o[k] && s.targets.iter().any(|i| track.last.get(*i) == Some(&Hit::Weak)) && !(n.item(k) == PIECE && spare.is_some_and(|x| x <= 0))
            })
            .collect();
        if c == self.checked && m == self.maybe && self.open.len() == c.len() {
            return false;
        }
        if self.checked.len() != c.len() {
            self.active = None;
            self.last = None;
        } else {
            let new: Vec<usize> = (0..c.len()).filter(|k| c[*k] && !self.checked[*k]).collect();
            if !new.is_empty() && new.len() <= BURST {
                self.active = new.last().map(|k| n.spots[*k].area.clone());
                self.last = new.last().copied();
            }
        }
        self.open = o;
        self.checked = c;
        self.maybe = m;
        true
    }

    fn last_json(&self, n: &Now) -> Json {
        let Some((k, s)) = self.last.and_then(|k| n.spots.get(k).map(|s| (k, s))) else {
            return Json::Null;
        };
        Json::obj()
            .with("item", shown(n.item(k)).into())
            .with("label", s.label.as_str().into())
            .with("area", s.area.as_str().into())
    }

    pub fn toggle(&mut self, name: &str, on: bool) -> bool {
        if on { self.manual.insert(name.to_string()) } else { self.manual.remove(name) }
    }

    pub fn has(&self, name: &str) -> bool {
        self.now.as_ref().is_some_and(|n| n.spots.iter().any(|s| s.name == name))
    }

    pub fn json(&self) -> Json {
        let p = |x: &Option<PB>| x.as_ref().map(|p| Json::from(p.to_string_lossy().into_owned())).unwrap_or(Json::Null);
        let spoiler: Json = Json::obj()
            .with("path", p(&self.path))
            .with("pin", p(&self.pin))
            .with("seed", self.now.as_ref().map(|n| Json::from(n.log.seed.as_str())).unwrap_or(Json::Null))
            .with("error", self.error.as_deref().map(Json::from).unwrap_or(Json::Null));
        let Some(n) = &self.now else {
            return Json::obj().with("spoiler", spoiler).with("areas", Json::Arr(Vec::new()));
        };
        let mut order: Vec<String> = Vec::new();
        for s in n.spots.iter().filter(|s| !s.hidden) {
            if !order.contains(&s.area) {
                order.push(s.area.clone());
            }
        }
        let mut areas: Vec<Json> = Vec::new();
        let (mut n_open, mut n_checked): (i64, i64) = (0, 0);
        for a in &order {
            let mut rows: Vec<(&str, Json)> = Vec::new();
            let (mut open, mut locked, mut done): (i64, i64, i64) = (0, 0, 0);
            for (k, s) in n.spots.iter().enumerate() {
                if &s.area != a || s.hidden {
                    continue;
                }
                let c: bool = self.checked.get(k).copied().unwrap_or(false);
                let o: bool = self.open.get(k).copied().unwrap_or(false);
                match (c, o) {
                    (true, _) => done += 1,
                    (false, true) => open += 1,
                    (false, false) => locked += 1,
                }
                rows.push((
                    s.label.as_str(),
                    Json::obj()
                        .with("name", s.name.as_str().into())
                        .with("label", s.label.as_str().into())
                        .with("kind", s.kind.as_str().into())
                        .with(
                            "age",
                            match s.age {
                                Some(Age::Child) => Json::from("child"),
                                Some(Age::Adult) => Json::from("adult"),
                                None => Json::Null,
                            },
                        )
                        .with("sensed", s.sensed().into())
                        .with("manual", self.manual.contains(&s.name).into())
                        .with("checked", c.into())
                        .with("maybe", self.maybe.get(k).copied().unwrap_or(false).into())
                        .with("open", o.into()),
                ));
            }
            rows.sort_by(|x, y| x.0.cmp(y.0));
            n_open += open;
            n_checked += done;
            areas.push(
                Json::obj()
                    .with("area", a.as_str().into())
                    .with("open", Json::Int(open))
                    .with("locked", Json::Int(locked))
                    .with("done", Json::Int(done))
                    .with("spots", Json::Arr(rows.into_iter().map(|r| r.1).collect())),
            );
        }
        Json::obj()
            .with("spoiler", spoiler)
            .with("total", Json::Int(n.spots.iter().filter(|s| !s.hidden).count() as i64))
            .with("open", Json::Int(n_open))
            .with("checked", Json::Int(n_checked))
            .with("active", self.active.as_deref().map(Json::from).unwrap_or(Json::Null))
            .with("last", self.last_json(n))
            .with("areas", Json::Arr(areas))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ootrr_core::settings::Settings;
    use ootrr_core::slice::generate::generate;
    use crate::tracker::seedlog::SeedLog;

    fn now() -> Arc<Now> {
        let s: Settings = Settings::default();
        let g = generate(&s, "active").unwrap();
        let log: SeedLog = SeedLog { seed: "active".to_string(), settings_string: s.encode().unwrap(), checksum: String::new(), placements: g.seed_file.placements };
        Arc::new(Now::from_log(log).unwrap())
    }

    #[test]
    fn the_area_of_the_latest_check_is_active_and_a_reattach_burst_leaves_it() {
        let n: Arc<Now> = now();
        let t: Track = Track { ids: Vec::new(), rules: Vec::new(), life: Vec::new(), marks: Vec::new(), last: Vec::new() };
        let mut st: NowState = NowState { now: Some(n.clone()), ..NowState::default() };
        st.refresh(&t, None);
        assert_eq!(st.active, None);
        let a: &str = &n.spots[0].area;
        let other: usize = n.spots.iter().position(|s| s.area != a).unwrap();
        st.toggle(&n.spots[0].name, true);
        st.refresh(&t, None);
        assert_eq!(st.active.as_deref(), Some(a));
        st.toggle(&n.spots[other].name, true);
        st.refresh(&t, None);
        assert_eq!(st.active.as_deref(), Some(n.spots[other].area.as_str()));
        for s in n.spots.iter().filter(|s| s.area == a).skip(1).take(BURST + 1) {
            st.toggle(&s.name, true);
        }
        st.refresh(&t, None);
        assert_eq!(st.active.as_deref(), Some(n.spots[other].area.as_str()));
        assert_eq!(st.last, Some(other));
        let j: Json = st.json();
        assert!(j.get("active").is_some());
        assert_eq!(j.get("last").and_then(|l| l.get("label")).and_then(|x| x.as_str()), Some(n.spots[other].label.as_str()));
    }
}
