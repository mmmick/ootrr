use std::fs;
use std::path::{Path as P, PathBuf as PB};
use std::time::SystemTime;

pub const NAME: &str = "Spoiler.txt";

pub fn stamp(p: &P) -> Option<SystemTime> {
    fs::metadata(p).ok().filter(|m| m.is_file()).and_then(|m| m.modified().ok())
}

fn in_mods(root: &P) -> Option<PB> {
    let rd = fs::read_dir(root.join("mods")).ok()?;
    let mut best: Option<(SystemTime, PB)> = None;
    for e in rd.flatten() {
        let n: String = e.file_name().to_string_lossy().into_owned();
        if n.starts_with('_') || n.starts_with('.') {
            continue;
        }
        let f: PB = e.path().join(NAME);
        if let Some(t) = stamp(&f)
            && best.as_ref().is_none_or(|b| t > b.0)
        {
            best = Some((t, f));
        }
    }
    best.map(|b| b.1)
}

pub fn find(pin: Option<&P>, exe: Option<&P>) -> Option<PB> {
    if let Some(p) = pin {
        if p.is_dir() {
            let f: PB = p.join(NAME);
            return if f.is_file() { Some(f) } else { in_mods(p) };
        }
        return Some(p.to_path_buf());
    }
    let bin: &P = exe?.parent()?;
    in_mods(bin).or_else(|| bin.parent().and_then(in_mods))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn the_spoiler_of_the_enabled_mod_next_to_the_game_is_found() {
        let d: PB = env::temp_dir().join(format!("ootrr-spoiler-find-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("bin")).unwrap();
        fs::create_dir_all(d.join("mods").join("_Old")).unwrap();
        fs::create_dir_all(d.join("mods").join("Rando")).unwrap();
        fs::write(d.join("mods").join("_Old").join(NAME), "old").unwrap();
        fs::write(d.join("mods").join("Rando").join(NAME), "new").unwrap();
        let exe: PB = d.join("bin").join("longshot.exe");
        assert_eq!(find(None, Some(&exe)), Some(d.join("mods").join("Rando").join(NAME)));
        assert_eq!(find(Some(&d), None), Some(d.join("mods").join("Rando").join(NAME)));
        assert_eq!(find(None, None), None);
        let _ = fs::remove_dir_all(&d);
    }
}
