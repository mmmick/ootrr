pub mod area;
pub mod cond;
pub mod grid;
pub mod icon_map;
pub mod index;
pub mod now;
pub mod npc_flags;
pub mod peek;
pub mod project;
pub mod reach;
pub mod rules;
pub mod seed;
pub mod seedlog;
pub mod state;
pub mod targets;

use std::collections::BTreeSet as BS;
use std::fs;
use std::path::{Path as P, PathBuf as PB};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ootrr_core::error::{Error, Res};
use ootrr_core::json::{self, Json};
use index::Index;
use crate::tracker::peek::{Peek, Place};
use crate::tracker::rules::Rule;
use crate::tracker::state::{By, S, Track};
use crate::tracker::now::Now;
use ootrr_core::items::load;
use ootrr_core::settings::seed::SeedFile;
use targets::{KINDS, OFF_BY_DEFAULT, Target};

use crate::datadir;
use crate::events::{Ev, Events, MAX_WAIT};
use crate::icon_store::{self, Icons};
use crate::live;
use crate::now_state::NowState;
use crate::pathsafe;
use crate::spoiler_find;
use crate::watch::{self, Out, Watcher};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Hidden,
    All,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Hidden => "hidden",
            Mode::All => "all",
        }
    }

    pub fn parse(s: &str) -> Option<Mode> {
        match s {
            "hidden" => Some(Mode::Hidden),
            "all" => Some(Mode::All),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Opts {
    pub mode: Mode,
    pub kinds: Vec<String>,
    pub dream: bool,
    pub live: bool,
}

impl Default for Opts {
    fn default() -> Opts {
        Opts {
            mode: Mode::Hidden,
            kinds: KINDS.iter().filter(|k| !OFF_BY_DEFAULT.contains(k)).map(|k| k.to_string()).collect(),
            dream: false,
            live: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Source {
    pub asked: Option<PB>,
    pub folder: Option<PB>,
    pub pinned: Option<String>,
    pub slot: Option<String>,
    pub slots: Vec<String>,
    pub read_at: Option<u64>,
    pub keys: usize,
    pub error: Option<String>,
    pub reads: u64,
    pub live: Option<u32>,
    pub live_msg: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Seeded {
    pub seed: String,
    pub checksum: String,
    pub rewired: usize,
}

pub struct Inner {
    pub seeded: Option<Seeded>,
    pub peek: Option<Peek>,
    pub index: Arc<Index>,
    pub game_dir: Option<PB>,
    pub job: Option<u64>,
    pub index_error: Option<String>,
    pub track: Track,
    pub opts: Opts,
    pub src: Source,
    pub place: Option<Place>,
    pub loaded: bool,
    pub items: Option<Vec<grid::St>>,
    pub now: NowState,
}

pub struct Tracker {
    inner: Mutex<Inner>,
    pub events: Events,
    pub data: Option<PB>,
    watch: Mutex<Option<watch::Handle>>,
    live: Mutex<Option<watch::Handle>>,
    pub auto_live: bool,
    boot: Mutex<()>,
    pub poll: Duration,
    pub icons: Icons,
}

pub fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn secs(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn write_atomic(p: &P, text: &str) -> Res<()> {
    if let Some(d) = p.parent() {
        fs::create_dir_all(d)?;
    }
    let tmp: PB = p.with_extension("tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, p)?;
    Ok(())
}

fn path_of(j: &Json, k: &str) -> Option<PB> {
    j.get(k).and_then(|x| x.as_str()).and_then(|s| pathsafe::abs(s).ok())
}

impl Tracker {
    pub fn new(data: Option<PB>) -> Tracker {
        let ts: &[Target] = targets::all().unwrap_or(&[]);
        let ix: Index = index::empty().unwrap_or(Index { game: String::new(), table: 0, maps: Vec::new(), locs: Vec::new(), shapes: Vec::new() });
        Tracker {
            inner: Mutex::new(Inner {
                seeded: None,
                peek: None,
                index: Arc::new(ix),
                game_dir: None,
                job: None,
                index_error: None,
                track: Track::new(ts),
                opts: Opts::default(),
                src: Source::default(),
                place: None,
                loaded: false,
                items: None,
                now: NowState::default(),
            }),
            events: Events::new(),
            data: data.map(|d| datadir::cache_root(&d)),
            watch: Mutex::new(None),
            live: Mutex::new(None),
            auto_live: false,
            boot: Mutex::new(()),
            poll: MAX_WAIT,
            icons: Icons::new(icon_store::disk_on()),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn boot(&self) -> MutexGuard<'_, ()> {
        self.boot.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn settings_file(&self) -> Option<PB> {
        self.data.as_ref().map(|d| d.join("settings.json"))
    }

    fn manual_file(&self) -> Option<PB> {
        self.data.as_ref().map(|d| d.join("now-manual.json"))
    }

    fn marks_file(&self) -> Option<PB> {
        self.data.as_ref().map(|d| d.join("profile-vanilla.json"))
    }

    pub fn index_file(&self, game: &str) -> Option<PB> {
        self.data.as_ref().map(|d| d.join(datadir::game_dir_name(game)).join("index.json"))
    }

    fn save_settings(&self, g: &Inner) {
        let Some(p) = self.settings_file() else {
            return;
        };
        let s = |x: &Option<PB>| x.as_ref().map(|p| Json::from(pathsafe::text(p))).unwrap_or(Json::Null);
        let j: Json = Json::obj()
            .with("format", Json::Int(1))
            .with("game_dir", s(&g.game_dir))
            .with("save_dir", s(&g.src.asked))
            .with("slot", g.src.pinned.as_deref().map(Json::from).unwrap_or(Json::Null))
            .with("mode", g.opts.mode.name().into())
            .with("kinds", Json::strs(g.opts.kinds.iter().map(|k| k.as_str())))
            .with("dream", g.opts.dream.into())
            .with("live", g.opts.live.into())
            .with("spoiler", s(&g.now.pin));
        let _ = write_atomic(&p, &j.pretty());
    }

    fn save_marks(&self, g: &Inner) {
        if let Some(p) = self.marks_file() {
            let _ = write_atomic(&p, &g.track.to_json().to_string());
        }
    }

    pub fn load_saved(&self) -> (Option<PB>, Option<PB>, Option<String>) {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        if g.loaded {
            return (None, None, None);
        }
        g.loaded = true;
        if let Some(t) = self.marks_file().and_then(|p| fs::read_to_string(p).ok())
            && let Ok(j) = json::parse(&t)
        {
            g.track.load(&j);
        }
        let Some(t) = self.settings_file().and_then(|p| fs::read_to_string(p).ok()) else {
            return (None, None, None);
        };
        let Ok(j) = json::parse(&t) else {
            return (None, None, None);
        };
        if let Some(m) = j.get("mode").and_then(|x| x.as_str()).and_then(Mode::parse) {
            g.opts.mode = m;
        }
        if let Some(a) = j.get("kinds").and_then(|x| x.as_arr()) {
            g.opts.kinds = a.iter().filter_map(|x| x.as_str()).filter(|k| KINDS.contains(k)).map(|k| k.to_string()).collect();
        }
        if let Some(d) = j.get("dream").and_then(|x| x.as_bool()) {
            g.opts.dream = d;
        }
        if let Some(l) = j.get("live").and_then(|x| x.as_bool()) {
            g.opts.live = l;
        }
        g.now.pin = path_of(&j, "spoiler");
        let slot: Option<String> = j.get("slot").and_then(|x| x.as_str()).map(|s| s.to_string());
        (path_of(&j, "game_dir"), path_of(&j, "save_dir"), slot)
    }

    pub fn set_index(&self, game_dir: PB, ix: Index) {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        g.index = Arc::new(ix);
        g.game_dir = Some(game_dir);
        g.job = None;
        g.index_error = None;
        self.save_settings(&g);
        drop(g);
        self.events.push(&[Ev::Reset]);
    }

    pub fn index_failed(&self, msg: &str) {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        g.job = None;
        g.index_error = Some(msg.to_string());
        drop(g);
        self.events.push(&[Ev::Source]);
    }

    pub fn load_or_build(&self, romfs: &P) -> Res<Index> {
        let fp: String = index::fingerprint(romfs)?;
        let file: Option<PB> = self.index_file(&fp);
        if let Some(f) = &file
            && let Ok(t) = fs::read_to_string(f)
            && let Ok(ix) = index::from_json(&t)
            && ix.game == fp
        {
            return Ok(ix);
        }
        let ix: Index = index::build(romfs)?;
        if let Some(f) = &file {
            write_atomic(f, &ix.to_json().to_string())?;
        }
        Ok(ix)
    }

    pub fn publish(&self, slot: &str, p: &Peek, at: Option<SystemTime>, names: Vec<String>) {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        if g.src.live.is_some() {
            return;
        }
        let quiet: bool = g.src.reads == 0;
        let base: u64 = self.events.seq() + 1;
        let changed: Vec<usize> = g.track.apply(p, base);
        g.src.reads += 1;
        g.src.slot = Some(slot.to_string());
        g.src.slots = names;
        g.src.read_at = at.map(secs);
        g.src.keys = p.keys.len();
        g.src.error = None;
        g.place = Some(p.place.clone());
        g.items = Some(grid::read(p));
        g.peek = Some(p.clone());
        refresh_now(&mut g);
        let mut evs: Vec<Ev> = changed
            .iter()
            .map(|i| {
                let m = g.track.marks[*i];
                Ev::Loc { i: *i, fresh: !quiet && m.s == S::Found && m.by == Some(By::Save) && m.at == base }
            })
            .collect();
        evs.push(Ev::Source);
        if !changed.is_empty() {
            self.save_marks(&g);
        }
        drop(g);
        self.events.push(&evs);
    }

    pub fn set_seed(&self, sf: Option<&SeedFile>) -> Res<Option<Seeded>> {
        let ts: &[Target] = targets::all()?;
        let over: Vec<Option<Rule>> = match sf {
            Some(f) => seed::rules(ts, &f.placements, &load::load()?),
            None => Vec::new(),
        };
        let mut g: MutexGuard<'_, Inner> = self.lock();
        g.track.rewire(ts, &over);
        g.seeded = sf.map(|f| Seeded { seed: f.seed.clone(), checksum: f.checksum(), rewired: seed::rewired(&over) });
        let base: u64 = self.events.seq() + 1;
        if let Some(p) = g.peek.clone() {
            g.track.apply(&p, base);
        }
        refresh_now(&mut g);
        self.save_marks(&g);
        let out: Option<Seeded> = g.seeded.clone();
        drop(g);
        self.events.push(&[Ev::Reset]);
        Ok(out)
    }

    pub fn fail(&self, slot: &str, msg: &str) {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        if g.src.live.is_some() {
            return;
        }
        let m: String = format!("{slot}: {msg}");
        if g.src.error.as_deref() == Some(m.as_str()) {
            return;
        }
        g.src.error = Some(m);
        drop(g);
        self.events.push(&[Ev::Source]);
    }

    pub fn user(&self, id: &str, on: bool) -> Res<bool> {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        let i: usize = g.track.find(id).ok_or_else(|| Error::Msg("no such location".to_string()))?;
        let seq: u64 = self.events.seq() + 1;
        if !g.track.user(i, on, seq) {
            return Ok(false);
        }
        refresh_now(&mut g);
        self.save_marks(&g);
        drop(g);
        self.events.push(&[Ev::Loc { i, fresh: false }, Ev::Source]);
        Ok(true)
    }

    pub fn set_opts(&self, o: Opts) {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        g.opts = o;
        self.save_settings(&g);
        drop(g);
        self.events.push(&[Ev::Reset]);
    }

    pub fn stop_watch(&self) {
        let h: Option<watch::Handle> = self.watch.lock().unwrap_or_else(|e| e.into_inner()).take();
        drop(h);
    }

    pub fn watch(self: &Arc<Self>, asked: PB, pinned: Option<String>) -> Res<PB> {
        let root: PB = watch::resolve(&asked)?;
        self.stop_watch();
        {
            let mut g: MutexGuard<'_, Inner> = self.lock();
            let (live, live_msg): (Option<u32>, Option<String>) = (g.src.live, g.src.live_msg.take());
            g.src = Source { asked: Some(asked), folder: Some(root.clone()), pinned: pinned.clone(), live, live_msg, ..Source::default() };
            self.save_settings(&g);
        }
        let me: Arc<Tracker> = Arc::clone(self);
        let w: Watcher = Watcher::new(root.clone(), pinned);
        let h: watch::Handle = watch::spawn(w, move |o: Out, w: &Watcher| match o {
            Out::Read { slot, peek, at } => me.publish(&slot, &peek, at, w.names()),
            Out::Fail { slot, msg } => me.fail(&slot, &msg),
        })?;
        *self.watch.lock().unwrap_or_else(|e| e.into_inner()) = Some(h);
        self.events.push(&[Ev::Source]);
        Ok(root)
    }
}

impl Tracker {
    pub fn start_live(self: &Arc<Self>) {
        if !self.auto_live && !self.lock().opts.live {
            return;
        }
        let mut l: MutexGuard<'_, Option<watch::Handle>> = self.live.lock().unwrap_or_else(|e| e.into_inner());
        if l.is_some() {
            return;
        }
        match live::spawn(Arc::clone(self)) {
            Ok(h) => *l = Some(h),
            Err(e) => self.lock().src.live_msg = Some(e.to_string()),
        }
    }

    pub fn stop_live(&self) {
        let h: Option<watch::Handle> = self.live.lock().unwrap_or_else(|e| e.into_inner()).take();
        drop(h);
        let mut g: MutexGuard<'_, Inner> = self.lock();
        g.src.live = None;
        g.src.live_msg = None;
    }

    pub fn set_live(self: &Arc<Self>, on: bool) {
        {
            let mut g: MutexGuard<'_, Inner> = self.lock();
            g.opts.live = on;
            self.save_settings(&g);
        }
        if on {
            self.start_live();
        } else {
            self.stop_live();
        }
        self.events.push(&[Ev::Source]);
    }

    pub fn live_snap(&self, p: &Peek, pid: u32) {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        let first: bool = g.src.live != Some(pid);
        let base: u64 = self.events.seq() + 1;
        let changed: Vec<usize> = g.track.apply(p, base);
        let items: Vec<grid::St> = grid::read(p);
        let moved: bool = g.items.as_ref() != Some(&items);
        g.src.live = Some(pid);
        g.src.live_msg = None;
        g.src.reads += 1;
        g.src.slot = Some("live".to_string());
        g.src.read_at = Some(now_secs());
        g.src.keys = p.keys.len();
        g.src.error = None;
        g.place = Some(p.place.clone());
        g.items = Some(items);
        g.peek = Some(p.clone());
        let now_moved: bool = refresh_now(&mut g);
        if changed.is_empty() && !moved && !now_moved && !first {
            return;
        }
        let mut evs: Vec<Ev> = changed
            .iter()
            .map(|i| {
                let m = g.track.marks[*i];
                Ev::Loc { i: *i, fresh: !first && m.s == S::Found && m.by == Some(By::Save) && m.at == base }
            })
            .collect();
        evs.push(Ev::Source);
        if !changed.is_empty() {
            self.save_marks(&g);
        }
        drop(g);
        self.events.push(&evs);
    }

    pub fn live_down(&self, msg: &str) {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        g.src.live = None;
        g.src.live_msg = Some(msg.to_string());
        drop(g);
        self.events.push(&[Ev::Source]);
    }

    fn manual_of(&self, key: &str) -> BS<String> {
        let Some(t) = self.manual_file().and_then(|p| fs::read_to_string(p).ok()) else {
            return BS::new();
        };
        let Ok(j) = json::parse(&t) else {
            return BS::new();
        };
        j.get(key).and_then(|x| x.as_arr()).map(|a| a.iter().filter_map(|x| x.as_str()).map(|x| x.to_string()).collect()).unwrap_or_default()
    }

    fn save_manual(&self, g: &Inner) {
        let (Some(p), Some(k)) = (self.manual_file(), g.now.key()) else {
            return;
        };
        let mut j: Json = fs::read_to_string(&p).ok().and_then(|t| json::parse(&t).ok()).filter(|j| matches!(j, Json::Obj(_))).unwrap_or_else(Json::obj);
        j.put(&k, Json::strs(g.now.manual.iter().map(|x| x.as_str())));
        let _ = write_atomic(&p, &j.to_string());
    }

    pub fn spoiler_check(&self, exe: Option<&P>) {
        let (pin, cur, stamp, tried): (Option<PB>, Option<PB>, Option<SystemTime>, Option<(PB, Option<SystemTime>)>) = {
            let g: MutexGuard<'_, Inner> = self.lock();
            (g.now.pin.clone(), g.now.path.clone(), g.now.stamp, g.now.tried.clone())
        };
        let Some(path) = spoiler_find::find(pin.as_deref(), exe) else {
            return;
        };
        let st: Option<SystemTime> = spoiler_find::stamp(&path);
        if cur.as_ref() == Some(&path) && st == stamp {
            return;
        }
        if tried.as_ref() == Some(&(path.clone(), st)) {
            return;
        }
        let res: Res<Now> = fs::read_to_string(&path).map_err(Error::from).and_then(|t| Now::load(&t));
        match res {
            Ok(n) => {
                let n: Arc<Now> = Arc::new(n);
                let key: String = format!("{} {}", n.log.seed, n.log.checksum);
                let manual: BS<String> = self.manual_of(&key);
                {
                    let mut g: MutexGuard<'_, Inner> = self.lock();
                    g.now.now = Some(Arc::clone(&n));
                    g.now.path = Some(path.clone());
                    g.now.stamp = st;
                    g.now.tried = Some((path, st));
                    g.now.error = None;
                    g.now.manual = manual;
                    g.now.checked.clear();
                }
                if let Err(e) = self.set_seed(Some(&n.seed_file())) {
                    self.lock().now.error = Some(e.to_string());
                }
                let mut g: MutexGuard<'_, Inner> = self.lock();
                refresh_now(&mut g);
                drop(g);
                self.events.push(&[Ev::Source]);
            }
            Err(e) => {
                let mut g: MutexGuard<'_, Inner> = self.lock();
                g.now.error = Some(format!("{}: {e}", pathsafe::text(&path)));
                g.now.tried = Some((path, st));
                drop(g);
                self.events.push(&[Ev::Source]);
            }
        }
    }

    pub fn now_mark(&self, name: &str, on: bool) -> Res<bool> {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        if !g.now.has(name) {
            return Err(Error::Msg("no such location in the spoiler".to_string()));
        }
        if !g.now.toggle(name, on) {
            return Ok(false);
        }
        self.save_manual(&g);
        refresh_now(&mut g);
        drop(g);
        self.events.push(&[Ev::Source]);
        Ok(true)
    }

    pub fn set_pin(&self, pin: Option<PB>) {
        let mut g: MutexGuard<'_, Inner> = self.lock();
        g.now.pin = pin;
        g.now.path = None;
        g.now.tried = None;
        g.now.error = None;
        self.save_settings(&g);
        drop(g);
        self.events.push(&[Ev::Source]);
    }
}

pub fn refresh_now(g: &mut Inner) -> bool {
    let Inner { now, track, peek, .. } = g;
    now.refresh(track, peek.as_ref())
}

impl Drop for Tracker {
    fn drop(&mut self) {
        self.stop_watch();
        let h: Option<watch::Handle> = self.live.lock().unwrap_or_else(|e| e.into_inner()).take();
        drop(h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watch::tests::{put, save_bytes, tmp};
    use ootrr_core::murmur::murmur3_32;
    use crate::tracker::peek::{Lim, peek_with};
    use std::thread;
    use std::time::Instant;

    fn pick(kind: &str) -> (usize, &'static Target) {
        targets::all().unwrap().iter().enumerate().find(|(_, t)| t.kind == kind && !t.keys.is_empty() || t.kind == kind && !t.flags.is_empty()).unwrap()
    }

    #[test]
    fn live_reads_win_over_the_save_and_only_later_live_reads_are_fresh() {
        let t: Tracker = Tracker::new(None);
        let (ci, chest): (usize, &Target) = pick("chest");
        let fl: u32 = murmur3_32(chest.flags[0].as_bytes(), 0);
        let none: Peek = peek_with(&save_bytes(&[], &[]), &Lim::ANY).unwrap();
        let got: Peek = peek_with(&save_bytes(&[(fl, 1)], &[]), &Lim::ANY).unwrap();
        t.live_snap(&none, 7);
        assert_eq!(t.lock().src.live, Some(7));
        let s0: u64 = t.events.seq();
        t.live_snap(&got, 7);
        let w = t.events.wait(s0, Duration::from_millis(1));
        assert!(w.items.iter().any(|x| matches!(x.1, Ev::Loc { i, fresh: true } if i == ci)));
        t.publish("slot_00", &none, None, vec!["slot_00".into()]);
        assert_eq!(t.lock().track.marks[ci].s, S::Found);
        t.fail("slot_00", "bad");
        assert_eq!(t.lock().src.error, None);
        t.live_down("game closed");
        {
            let g = t.lock();
            assert_eq!((g.src.live, g.src.live_msg.as_deref()), (None, Some("game closed")));
        }
        t.stop_live();
        assert_eq!(t.lock().src.live_msg, None);
    }

    #[test]
    fn publish_ticks_sticky_kinds_and_only_later_reads_are_fresh() {
        let t: Tracker = Tracker::new(None);
        let (pi, pot): (usize, &Target) = pick("pot");
        let (ci, chest): (usize, &Target) = pick("chest");
        let fl: u32 = murmur3_32(chest.flags[0].as_bytes(), 0);
        let a: Peek = peek_with(&save_bytes(&[(fl, 1)], &[pot.keys[0]]), &Lim::ANY).unwrap();
        t.publish("slot_00", &a, None, vec!["slot_00".into()]);
        let first = t.events.wait(0, Duration::from_millis(1));
        assert!(first.items.iter().all(|x| !matches!(x.1, Ev::Loc { fresh: true, .. })));
        let b: Peek = peek_with(&save_bytes(&[], &[]), &Lim::ANY).unwrap();
        let s0: u64 = t.events.seq();
        t.publish("slot_00", &b, None, vec!["slot_00".into()]);
        {
            let g = t.lock();
            assert_eq!((g.track.marks[pi].s, g.track.marks[pi].by), (S::Found, Some(By::Sticky)));
            assert_eq!(g.track.marks[ci].s, S::Lost);
        }
        t.publish("slot_00", &a, None, vec!["slot_00".into()]);
        let later = t.events.wait(s0, Duration::from_millis(1));
        assert!(later.items.iter().any(|x| x.1 == Ev::Loc { i: ci, fresh: true }));
    }

    #[test]
    fn a_seed_rewires_the_gift_rows_and_leaving_it_restores_them() {
        use ootrr_core::settings::Settings;
        use std::collections::BTreeMap;
        let t: Tracker = Tracker::new(None);
        let ts: &[Target] = targets::all().unwrap();
        let impa: usize = ts.iter().position(|x| x.ootr.as_deref() == Some("ZD King Zora Thawed")).unwrap();
        let sheik: usize = ts.iter().position(|x| x.ootr.as_deref() == Some("Sheik in Forest")).unwrap();
        let fl: u32 = murmur3_32(b"HavePlayedGetEvent.Item_Melody_ForestMinuet", 0);
        let a: Peek = peek_with(&save_bytes(&[(fl, 1)], &[]), &Lim::ANY).unwrap();
        t.publish("slot_00", &a, None, vec!["slot_00".into()]);
        assert_eq!(t.lock().track.marks[sheik].s, S::Found);
        let mut p: BTreeMap<String, String> = BTreeMap::new();
        p.insert("ZD King Zora Thawed".into(), "Minuet of Forest".into());
        p.insert("Sheik in Forest".into(), "Zeldas Lullaby".into());
        let sf: SeedFile = SeedFile::new("abc", &Settings::default(), "oot-remake:test", p).unwrap();
        let s: Seeded = t.set_seed(Some(&sf)).unwrap().unwrap();
        assert_eq!(s.rewired, 2);
        {
            let g = t.lock();
            assert_eq!(g.track.marks[impa].s, S::Found);
            assert_eq!(g.track.marks[sheik].s, S::Open);
        }
        assert_eq!(t.set_seed(None).unwrap(), None);
        let g = t.lock();
        assert_eq!(g.track.marks[impa].s, S::Open);
        assert_eq!(g.track.marks[sheik].s, S::Found);
    }

    #[test]
    fn settings_and_marks_persist_in_the_data_folder() {
        let d: PB = tmp("persist");
        let t: Tracker = Tracker::new(Some(d.clone()));
        let (ci, chest): (usize, &Target) = pick("chest");
        t.load_saved();
        t.user(&chest.id, true).unwrap();
        t.set_opts(Opts { mode: Mode::All, kinds: vec!["chest".into()], dream: true, live: true });
        let u: Tracker = Tracker::new(Some(d.clone()));
        u.load_saved();
        let g = u.lock();
        assert_eq!(g.track.marks[ci].by, Some(By::User));
        assert_eq!(g.opts, Opts { mode: Mode::All, kinds: vec!["chest".into()], dream: true, live: true });
        drop(g);
        assert!(d.join("ootrr").join("tracker-cache").join("settings.json").is_file());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn the_watch_thread_publishes_a_rewritten_save() {
        let d: PB = tmp("live");
        let (ci, chest): (usize, &Target) = pick("chest");
        let fl: u32 = murmur3_32(chest.flags[0].as_bytes(), 0);
        put(&d.join("slot_00"), &save_bytes(&[], &[]), 1_700_000_000);
        let t: Arc<Tracker> = Arc::new(Tracker::new(None));
        t.watch(d.clone(), None).unwrap();
        let t0: Instant = Instant::now();
        while t.lock().src.reads == 0 && t0.elapsed() < Duration::from_secs(5) {
            thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(t.lock().src.reads, 1);
        put(&d.join("slot_00"), &save_bytes(&[(fl, 1)], &[]), 1_700_000_100);
        while t.lock().track.marks[ci].s != S::Found && t0.elapsed() < Duration::from_secs(8) {
            thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(t.lock().track.marks[ci].s, S::Found);
        t.stop_watch();
        let _ = fs::remove_dir_all(&d);
    }
}
