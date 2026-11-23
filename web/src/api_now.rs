use ootrr_core::json::Json;

use crate::api_tracker;
use crate::body;
use crate::error::{Fail, Hr};
use crate::pathsafe;
use crate::state::Shared;
use crate::tracker::Inner;

fn live_json(g: &Inner) -> Json {
    Json::obj()
        .with("on", g.src.live.is_some().into())
        .with("pid", g.src.live.map(|p| Json::Int(p as i64)).unwrap_or(Json::Null))
        .with("msg", g.src.live_msg.as_deref().map(Json::from).unwrap_or(Json::Null))
        .with("child", g.place.as_ref().map(|p| Json::from(p.child)).unwrap_or(Json::Null))
}

pub fn state(sh: &Shared) -> Json {
    api_tracker::ensure(sh);
    let seq: u64 = sh.tracker.events.seq();
    let g = sh.tracker.lock();
    g.now.json().with("seq", Json::Int(seq as i64)).with("live", live_json(&g))
}

pub fn mark(sh: &Shared, b: &Json) -> Hr<Json> {
    let name: &str = body::short(b, "name")?;
    let on: bool = b.get("on").and_then(|x| x.as_bool()).ok_or_else(|| Fail::bad("on must be true or false"))?;
    let changed: bool = sh.tracker.now_mark(name, on).map_err(|e| Fail::new(404, &e.to_string()))?;
    Ok(Json::obj().with("ok", true.into()).with("changed", changed.into()))
}

pub fn spoiler(sh: &Shared, b: &Json) -> Hr<Json> {
    api_tracker::ensure(sh);
    let pin = match body::opt_text(b, "path")?.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        Some(s) => {
            let p = pathsafe::abs(s)?;
            if !p.is_file() && !p.is_dir() {
                return Err(Fail::new(422, "That spoiler path does not exist."));
            }
            Some(p)
        }
        None => None,
    };
    sh.tracker.set_pin(pin);
    Ok(Json::obj().with("ok", true.into()))
}
