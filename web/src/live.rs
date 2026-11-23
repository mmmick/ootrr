use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use ootrr_core::error::Res;
use ootrr_core::murmur::murmur3_32;
use ootrr_mem::reader::Reader;
use ootrr_mem::snap::Snap;

use crate::tracker::Tracker;
use crate::tracker::peek::{Peek, Place};
use crate::watch::Handle;

pub const TICK: Duration = Duration::from_millis(100);
pub const SPOILER_EVERY: Duration = Duration::from_secs(3);

pub fn peek_of(s: &Snap) -> Peek {
    let mut p: Peek = Peek { rows: s.rows.clone(), keys: s.keys.clone(), len: s.rows.len(), place: Place { map: None, child: false, room: None, pos: None } };
    p.place.child = p.on(murmur3_32(b"Sequence_IsChildhood", 0));
    p
}

pub fn spawn(me: Arc<Tracker>) -> Res<Handle> {
    let stop: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let flag: Arc<AtomicBool> = Arc::clone(&stop);
    let join: JoinHandle<()> = thread::Builder::new().name("ootrr-tracker-live".to_string()).spawn(move || {
        let mut r: Reader = Reader::new();
        let mut last: Option<Snap> = None;
        let mut down: Option<String> = None;
        let mut looked: Option<Instant> = None;
        while !flag.load(Ordering::SeqCst) {
            let now: Instant = Instant::now();
            match r.poll(now) {
                Ok(s) => {
                    if last.as_ref() != Some(&s) {
                        me.live_snap(&peek_of(&s), r.attached().unwrap_or(0));
                        last = Some(s);
                    }
                    down = None;
                }
                Err(e) => {
                    let m: String = e.to_string();
                    if down.as_deref() != Some(m.as_str()) {
                        me.live_down(&m);
                        down = Some(m);
                    }
                    last = None;
                }
            }
            if looked.is_none_or(|t| now.saturating_duration_since(t) >= SPOILER_EVERY) {
                looked = Some(now);
                me.spoiler_check(r.exe().as_deref());
            }
            let spent: Duration = now.elapsed();
            if spent < TICK {
                thread::sleep(TICK - spent);
            }
        }
    })?;
    Ok(Handle::new(stop, join))
}
