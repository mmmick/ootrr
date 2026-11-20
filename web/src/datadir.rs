use std::env;
use std::path::PathBuf as PB;

use crate::pathsafe;

pub const OVERRIDE: &str = "OOTRR_DATA_DIR";

fn var(k: &str) -> Option<PB> {
    env::var(k).ok().and_then(|s| pathsafe::abs(&s).ok())
}

/// The file that, next to the program, turns on portable mode.
pub const PORTABLE_MARK: &str = "portable.txt";

/// In portable mode the data lives in a `data` folder beside the program, so
/// the whole app is one folder that can be moved or deleted.
fn portable_in(exe_dir: &std::path::Path) -> Option<PB> {
    exe_dir.join(PORTABLE_MARK).is_file().then(|| exe_dir.join("data"))
}

fn portable() -> Option<PB> {
    let exe: PB = env::current_exe().ok()?;
    portable_in(exe.parent()?)
}

pub fn base() -> Option<PB> {
    if let Some(p) = var(OVERRIDE) {
        return Some(p);
    }
    if let Some(p) = portable() {
        return Some(p);
    }
    if cfg!(windows) {
        return var("LOCALAPPDATA").or_else(|| var("APPDATA"));
    }
    if cfg!(target_os = "macos") {
        return var("HOME").map(|h| h.join("Library").join("Application Support"));
    }
    var("XDG_DATA_HOME").or_else(|| var("HOME").map(|h| h.join(".local").join("share")))
}

pub fn cache_root(base: &PB) -> PB {
    base.join("ootrr").join("tracker-cache")
}

pub fn game_dir_name(fp: &str) -> String {
    fp.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fingerprint_becomes_a_safe_folder_name() {
        assert_eq!(game_dir_name("oot-remake:c944ba62294a93fa"), "oot-remake-c944ba62294a93fa");
        assert_eq!(game_dir_name("a/../b"), "a----b");
        let r: PB = cache_root(&PB::from("base"));
        assert!(r.ends_with(PB::from("ootrr").join("tracker-cache")));
    }

    #[test]
    fn the_portable_mark_keeps_the_data_beside_the_program() {
        let d: PB = env::temp_dir().join(format!("ootrr-portable-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        assert_eq!(portable_in(&d), None);
        std::fs::write(d.join(PORTABLE_MARK), "").unwrap();
        assert_eq!(portable_in(&d), Some(d.join("data")));
        std::fs::remove_dir_all(&d).unwrap();
    }
}
