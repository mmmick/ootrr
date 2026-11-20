mod api_build;
mod api_fs;
mod api_generate;
mod api_info;
mod api_items;
mod api_now;
mod api_open;
mod api_seed;
mod api_settings;
mod api_tracker;
mod api_validate;
mod assets;
mod body;
mod datadir;
mod e2e;
mod error;
mod events;
mod game_source;
mod game_view;
mod gen_live;
mod guard;
mod icon_store;
mod jobs;
mod live;
mod native_pick;
mod now_state;
mod pathsafe;
pub mod opener;
mod req;
mod resp;
mod router;
pub mod server;
mod spoiler_find;
mod state;
pub mod stdio;
mod token;
pub mod tracker;
mod tracker_view;
mod update;
mod watch;

pub use server::{DEFAULT_PORT, Handle, Opts, start};

/// Game input for `ootrr build`: the same checks and preparation the Configuration page uses.
pub mod game_input {
    use std::path::Path as P;

    pub use crate::game_source::{Prepared, Source, add_update, prepare};

    /// What checking a game folder or package found.
    pub enum Found {
        /// Usable: read it through `prepare`.
        Ok(Source),
        /// A v1.0.0 game, or a usable update was given or expected but is wrong; the text says why.
        Refused(String),
        /// Not recognised as a whole game (for example a partial romfs used in development).
        Unknown(String),
    }

    /// Checks `game` (a romfs folder, a dump folder's base, a `.nup` or a `.nx`) and the separate
    /// v1.1.1 `update` (a folder, `.nup` or `.nx`) when given.
    pub fn check(game: &P, update: Option<&P>) -> Found {
        let r = crate::api_validate::inspect_with(game, update);
        let mut why: Vec<String> = r.problems.clone();
        if !r.update_problems.is_empty() {
            why.push(format!("Update: {}", r.update_problems.join(" ")));
        }
        match r.source {
            Some(s) if r.ok => Found::Ok(s),
            _ if r.needs_update || update.is_some() || r.pack.is_some() => Found::Refused(why.join(" ")),
            _ => Found::Unknown(why.join(" ")),
        }
    }
}
