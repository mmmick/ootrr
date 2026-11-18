use std::fs;
use std::path::{Path as P, PathBuf as PB};

use ootrr_core::json::Json;

use std::collections::BTreeSet as BS;

use ootrr_core::container::is_container;

use crate::error::Hr;
use crate::game_source::{PackInfo, Side, Source, pack_info};
use crate::pathsafe;

const MARKERS: [(&str, bool); 2] = [("Pack/ZsDic.pack.zs", false), ("Banc", true)];
const SCAN_LIMIT: usize = 300;
pub const VERSION: &str = "System/Resource/ResourceSizeTable.Product.111.Nin_Ounce_NVN2.rsizetable.zs";
const BASE_VERSIONS: [&str; 2] = [
    "System/Resource/ResourceSizeTable.Product.100.Nin_Ounce_NVN2.rsizetable.zs",
    "GameData/GameDataList.Product.100.byml.zs",
];
pub const V111: &str = "1.1.1";
pub const V100: &str = "1.0.0";
const OLD: &str = "v1.0.0 detected; v1.1.1 is required. Please specify a folder with the v1.1.1 update below, or try a different folder that contains the full v1.1.1 game.";

#[cfg(test)]
pub(crate) fn fake_version(root: &P) {
    let data: ootrr_core::slice::data::Data = ootrr_core::slice::data::Data::load().unwrap();
    for r in ootrr_core::required::list(&data) {
        let t: PB = r.split('/').fold(root.to_path_buf(), |a, s| a.join(s));
        if t.is_file() {
            continue;
        }
        fs::create_dir_all(t.parent().unwrap()).unwrap();
        fs::write(&t, b"").unwrap();
    }
}

pub struct Report {
    pub ok: bool,
    pub romfs: Option<PB>,
    pub checks: Vec<(String, bool)>,
    pub problems: Vec<String>,
    /// "1.1.1" or "1.0.0" when the files tell, else None.
    pub version: Option<&'static str>,
    /// The game folder is 1.0.0, so a 1.1.1 update folder is needed.
    pub needs_update: bool,
    /// The 1.1.1 update's romfs folder laid over `romfs`, when one is used.
    pub update: Option<PB>,
    /// The update is inside the game's own package.
    pub update_auto: bool,
    /// A v1.1.1 update folder found next to a v1.0.0 folder (`updates/<n>/` beside `base/`).
    /// Only offered to the user; it is never used unless picked.
    pub update_hint: Option<PB>,
    pub update_problems: Vec<String>,
    /// "NUP" or "NX" when the game is a package.
    pub pack: Option<&'static str>,
    /// Where a job reads the game from; set when `ok`.
    pub source: Option<Source>,
}

impl Report {
    fn new() -> Report {
        Report { ok: false, romfs: None, checks: Vec::new(), problems: Vec::new(), version: None, needs_update: false, update: None, update_auto: false, update_hint: None, update_problems: Vec::new(), pack: None, source: None }
    }
}

fn version_of(r: &P) -> Option<&'static str> {
    if has(r, VERSION, false) {
        Some(V111)
    } else if BASE_VERSIONS.iter().any(|v| has(r, v, false)) {
        Some(V100)
    } else {
        None
    }
}

/// `[base romfs, newest update romfs]` of a dump folder in the layout the NUP packer reads
/// (`base/romfs` and `updates/<version>/romfs`). The update is only suggested, never used on its own.
/// `p` may be the dump folder itself, its `base` folder or `base/romfs`.
fn dump_layers(p: &P) -> Option<(PB, Option<PB>)> {
    let mut tops: Vec<PB> = vec![p.to_path_buf()];
    if named(p, "base") {
        tops.extend(p.parent().map(|x| x.to_path_buf()));
    }
    if named(p, "romfs") && p.parent().is_some_and(|x| named(x, "base")) {
        tops.extend(p.parent().and_then(|x| x.parent()).map(|x| x.to_path_buf()));
    }
    for top in tops {
        let base: PB = top.join("base").join("romfs");
        if !base.is_dir() {
            continue;
        }
        let ups: PB = top.join("updates");
        let newest: Option<PB> = fs::read_dir(&ups)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().to_str().and_then(|n| n.parse::<u64>().ok()).map(|v| (v, e.path().join("romfs"))))
            .filter(|(_, r)| r.is_dir())
            .max_by_key(|(v, _)| *v)
            .map(|(_, r)| r);
        return Some((base, newest));
    }
    None
}

/// Answers "is this romfs file there" for a folder or a package's file list.
enum Look {
    Dir(PB),
    Set(BS<String>),
}

impl Look {
    fn has(&self, r: &str) -> bool {
        match self {
            Look::Dir(d) => has(d, r, false),
            Look::Set(s) => s.contains(r),
        }
    }

    fn complete(&self) -> bool {
        match self {
            Look::Dir(d) => complete(d),
            Look::Set(s) => s.contains(MARKERS[0].0) && s.iter().any(|f| f.starts_with("Banc/")),
        }
    }

    fn version(&self) -> Option<&'static str> {
        if self.has(VERSION) {
            Some(V111)
        } else if BASE_VERSIONS.iter().any(|v| self.has(v)) {
            Some(V100)
        } else {
            None
        }
    }
}

fn open_pack(p: &P) -> Result<(PackInfo, Look), String> {
    let i: PackInfo = pack_info(p).map_err(|e| format!("This package could not be read: {e}."))?;
    let l: Look = Look::Set(i.files.clone());
    Ok((i, l))
}

/// The 1.1.1 update picked by the user: a `.nup` or `.nx`, or a folder: the folder itself,
/// its `romfs`, or one level down (`updates/` holding `131072/romfs`).
fn find_update(u: &P, base: &P) -> Result<(Side, Look), String> {
    match fs::metadata(u) {
        Ok(m) if m.is_dir() => {}
        Ok(_) if is_container(u) => {
            let (i, l) = open_pack(u)?;
            if l.version() != Some(V111) {
                return Err(match l.version() {
                    Some(_) => format!("This {} package is v1.0.0 too. Pick the v1.1.1 update.", i.kind),
                    None => format!("This {} package has no v1.1.1 update files ({VERSION}).", i.kind),
                });
            }
            if l.complete() && !i.standalone {
                return Err(format!("This {} package holds the complete v1.1.1 game. Pick it as the game above instead of the v1.0.0 game.", i.kind));
            }
            return Ok((Side::Pack(u.to_path_buf()), l));
        }
        Ok(_) => return Err("That path is a file, not a folder, a .nup or a .nx.".to_string()),
        Err(_) => return Err("That folder does not exist or cannot be read.".to_string()),
    }
    let mut cands: Vec<PB> = vec![u.to_path_buf(), u.join("romfs")];
    for s in subdirs(u) {
        cands.push(s.join("romfs"));
        cands.push(s);
    }
    let Some(found) = cands.iter().find(|c| has(c, VERSION, false)).cloned() else {
        if cands.iter().any(|c| version_of(c) == Some(V100)) {
            return Err("This folder is v1.0.0 too. Pick the folder with the v1.1.1 update files.".to_string());
        }
        return Err(format!("No v1.1.1 update files were found here. Pick the folder that holds the update's romfs folder (with {VERSION}), or the update's .nup or .nx."));
    };
    let same: bool = match (fs::canonicalize(&found), fs::canonicalize(base)) {
        (Ok(a), Ok(b)) => a.starts_with(&b) || b.starts_with(&a),
        _ => false,
    };
    if same {
        return Err("The update folder must be a different folder from the v1.0.0 game folder.".to_string());
    }
    Ok((Side::Folder(found.clone()), Look::Dir(found)))
}

/// The build-list files that neither the update nor the base holds.
fn pair_missing(base: &Look, update: &Look) -> Vec<String> {
    match ootrr_core::slice::data::Data::load() {
        Ok(d) => ootrr_core::required::list(&d).into_iter().filter(|r| !update.has(r) && !base.has(r)).collect(),
        Err(_) => Vec::new(),
    }
}

fn missing_text(gone: &[String]) -> String {
    let shown: Vec<&str> = gone.iter().take(6).map(|s| s.as_str()).collect();
    format!(
        "This folder is missing {} files the mod build reads, for example {}. The folder is probably an incomplete or older extraction of the game.",
        gone.len(),
        shown.join(", ")
    )
}

/// The v1.0.0 game `base` with the update the user picked. `hint` is an update found next to it,
/// reported only when nothing was picked: folder names alone don't say which update belongs to a game.
fn with_update(rep: &mut Report, base: Side, look: Look, picked: Option<&P>, hint: Option<PB>) {
    rep.needs_update = true;
    let base_path: PB = match &base {
        Side::Folder(p) | Side::Pack(p) => p.clone(),
    };
    let got: Option<Result<(Side, Look), String>> = picked.map(|u| find_update(u, &base_path));
    match got {
        None => {
            rep.problems.push(OLD.to_string());
            rep.update_hint = hint;
        }
        Some(Err(e)) => {
            rep.problems.push(OLD.to_string());
            rep.update_problems.push(e);
        }
        Some(Ok((side, ul))) => {
            rep.checks.push((format!("v1.1.1 update: {VERSION}"), true));
            let gone: Vec<String> = pair_missing(&look, &ul);
            if gone.is_empty() {
                rep.ok = true;
                rep.update = Some(match &side {
                    Side::Folder(p) | Side::Pack(p) => p.clone(),
                });
                rep.source = Some(Source { base, update: Some(side), update_in_pack: false });
            } else {
                rep.checks.push((format!("{} files the build reads are missing", gone.len()), false));
                rep.update_problems.push(missing_text(&gone));
            }
        }
    }
}

const NOT_111: &str = "This folder is missing the files of game version 1.1.1 (System/Resource/ResourceSizeTable.Product.111...). The mod build needs the game extracted with the 1.1.1 update installed. Extract it again from the emulator with the update applied and pick that folder.";

/// A `.nup` or `.nx` picked as the game.
fn inspect_pack(p: &P, update: Option<&P>) -> Report {
    let mut rep: Report = Report::new();
    let (i, l) = match open_pack(p) {
        Ok(x) => x,
        Err(e) => {
            rep.problems.push(e);
            return rep;
        }
    };
    rep.pack = Some(i.kind);
    rep.romfs = Some(p.to_path_buf());
    for (r, d) in MARKERS {
        let ok: bool = if d { i.files.iter().any(|f| f.starts_with(&format!("{r}/"))) } else { l.has(r) };
        rep.checks.push((format!("{r}{}", if d { "/" } else { "" }), ok));
    }
    if !l.complete() {
        rep.problems.push(if i.standalone {
            format!("This {} package holds only an update. Pick the v1.0.0 game here, and this package as the v1.1.1 update below it.", i.kind)
        } else {
            format!("This {} package does not hold the game's romfs files (Pack and Banc).", i.kind)
        });
        return rep;
    }
    rep.version = l.version();
    match rep.version {
        Some(V100) => with_update(&mut rep, Side::Pack(p.to_path_buf()), l, update, None),
        Some(_) => {
            let gone: Vec<String> = match ootrr_core::slice::data::Data::load() {
                Ok(d) => ootrr_core::required::list(&d).into_iter().filter(|r| !l.has(r)).collect(),
                Err(_) => Vec::new(),
            };
            rep.checks.push((VERSION.to_string(), true));
            if gone.is_empty() {
                rep.ok = true;
                if i.update_in_pack {
                    rep.update = Some(p.to_path_buf());
                    rep.update_auto = true;
                }
                rep.source = Some(Source { base: Side::Pack(p.to_path_buf()), update: None, update_in_pack: i.update_in_pack });
            } else {
                rep.checks.push((format!("{} files the build reads are missing", gone.len()), false));
                rep.problems.push(missing_text(&gone).replace("This folder", "This package"));
            }
        }
        None => {
            rep.checks.push((VERSION.to_string(), false));
            rep.problems.push(NOT_111.replace("This folder", "This package").replace("Extract it again from the emulator with the update applied and pick that folder.", "Pack it again with the update included."));
        }
    }
    rep
}

fn has(base: &P, rel: &str, dir: bool) -> bool {
    let t: PB = rel.split('/').fold(base.to_path_buf(), |a, s| a.join(s));
    match fs::metadata(&t) {
        Ok(m) => {
            if dir {
                m.is_dir()
            } else {
                m.is_file()
            }
        }
        Err(_) => false,
    }
}

fn complete(c: &P) -> bool {
    MARKERS.iter().all(|(r, d)| has(c, r, *d))
}

pub fn inspect(p: &P) -> Report {
    inspect_with(p, None)
}

/// Checks a game folder and, when it is v1.0.0, the separate v1.1.1 update folder `update`.
pub fn inspect_with(p: &P, update: Option<&P>) -> Report {
    let mut rep: Report = Report::new();
    match fs::metadata(p) {
        Ok(m) if m.is_dir() => {}
        Ok(_) if is_container(p) => return inspect_pack(p, update),
        Ok(_) => {
            rep.problems.push("That path is a file, not a folder, a .nup or a .nx.".to_string());
            return rep;
        }
        Err(_) => {
            rep.problems.push("That folder does not exist or cannot be read.".to_string());
            return rep;
        }
    }
    let dump: Option<(PB, Option<PB>)> = dump_layers(p);
    let mut cands: Vec<PB> = vec![p.to_path_buf(), p.join("romfs")];
    cands.extend(dump.as_ref().map(|d| d.0.clone()));
    let found: Option<PB> = cands.into_iter().find(|c| complete(c));
    let base: PB = found.clone().unwrap_or_else(|| p.to_path_buf());
    for (r, d) in MARKERS {
        let ok: bool = has(&base, r, d);
        rep.checks.push((format!("{r}{}", if d { "/" } else { "" }), ok));
    }
    rep.version = found.as_deref().and_then(version_of);
    if found.is_some() && rep.version != Some(V100) {
        rep.checks.push((VERSION.to_string(), has(&base, VERSION, false)));
    }
    match found {
        Some(f) if rep.version == Some(V100) => {
            // `updates/<n>/romfs` -> `updates/<n>`, the folder a user would pick.
            let hint: Option<PB> = dump.and_then(|d| d.1).filter(|u| has(u, VERSION, false)).map(|u| u.parent().map(|x| x.to_path_buf()).unwrap_or(u));
            rep.romfs = Some(f.clone());
            with_update(&mut rep, Side::Folder(f.clone()), Look::Dir(f), update, hint);
        }
        Some(f) if !has(&f, VERSION, false) => rep.problems.push(NOT_111.to_string()),
        Some(f) => {
            let gone: Vec<String> = match ootrr_core::slice::data::Data::load() {
                Ok(d) => ootrr_core::required::missing(&f, &d),
                Err(_) => Vec::new(),
            };
            if gone.is_empty() {
                rep.ok = true;
                rep.romfs = Some(f.clone());
                rep.source = Some(Source::folder(f));
            } else {
                rep.checks.push((format!("{} files the build reads are missing", gone.len()), false));
                rep.problems.push(missing_text(&gone));
            }
        }
        None => rep.problems.push(not_a_game(p)),
    }
    rep
}

fn subdirs(p: &P) -> Vec<PB> {
    let mut out: Vec<PB> = match fs::read_dir(p) {
        Ok(r) => r.filter_map(|e| e.ok()).map(|e| e.path()).filter(|x| x.is_dir()).take(SCAN_LIMIT).collect(),
        Err(_) => Vec::new(),
    };
    out.sort();
    out
}

fn named(p: &P, n: &str) -> bool {
    p.file_name().and_then(|x| x.to_str()).is_some_and(|x| x.eq_ignore_ascii_case(n))
}

fn shown(p: &P, from: &P) -> String {
    p.strip_prefix(from).map(|x| x.to_string_lossy().replace('\\', "/")).unwrap_or_else(|_| p.to_string_lossy().to_string())
}

fn not_a_game(p: &P) -> String {
    let mut near: Vec<PB> = Vec::new();
    for a in subdirs(p) {
        near.push(a.clone());
        near.extend(subdirs(&a));
    }
    if let Some(g) = near.iter().find(|c| complete(c)) {
        return format!("This folder is not the extracted game, but one that looks like it is inside: {}. Pick that folder instead.", shown(g, p));
    }
    if let Some(f) = near.iter().find(|c| named(c, "romfs") || named(c, "exefs")) {
        return format!("This looks like a mods folder ({} holds a mod's files), not the extracted game. A mod only holds the files it changes, so it cannot be used here. Pick the folder of the extracted game that contains the Pack and Banc folders.", shown(f, p));
    }
    "This does not look like an extracted game folder. Pick the folder that contains the Pack and Banc folders.".to_string()
}

pub fn opt_path(s: Option<&str>) -> Hr<Option<PB>> {
    match s.map(str::trim) {
        None | Some("") => Ok(None),
        Some(t) => Ok(Some(pathsafe::abs(t)?)),
    }
}

pub fn run(path: Option<&str>, update: Option<&str>) -> Hr<Json> {
    let raw: &str = path.unwrap_or("");
    let p: PB = pathsafe::abs(raw)?;
    let u: Option<PB> = opt_path(update)?;
    let rep: Report = inspect_with(&p, u.as_deref());
    let checks: Vec<Json> = rep
        .checks
        .iter()
        .map(|(l, ok)| Json::obj().with("label", l.as_str().into()).with("ok", (*ok).into()))
        .collect();
    Ok(Json::obj()
        .with("path", pathsafe::text(&p).into())
        .with("ok", rep.ok.into())
        .with("romfs", rep.romfs.as_deref().map(|x| Json::Str(pathsafe::text(x))).unwrap_or(Json::Null))
        .with("checks", Json::Arr(checks))
        .with("problems", Json::strs(rep.problems.iter().map(|s| s.as_str())))
        .with("version", rep.version.map(Json::from).unwrap_or(Json::Null))
        .with("needs_update", rep.needs_update.into())
        .with("update", rep.update.as_deref().map(|x| Json::Str(pathsafe::text(x))).unwrap_or(Json::Null))
        .with("update_auto", rep.update_auto.into())
        .with("update_hint", rep.update_hint.as_deref().map(|x| Json::Str(pathsafe::text(x))).unwrap_or(Json::Null))
        .with("pack", rep.pack.map(Json::from).unwrap_or(Json::Null))
        .with("update_problems", Json::strs(rep.update_problems.iter().map(|s| s.as_str()))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn tmp(tag: &str) -> PB {
        let p: PB = env::temp_dir().join(format!("ootrr_web_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn fake_romfs(root: &P) {
        fs::create_dir_all(root.join("Pack")).unwrap();
        fs::create_dir_all(root.join("Banc")).unwrap();
        fs::write(root.join("Pack").join("ZsDic.pack.zs"), b"").unwrap();
        fake_version(root);
    }

    #[test]
    fn a_folder_without_the_1_1_1_files_is_refused_with_a_reason() {
        let d: PB = tmp("old");
        fake_romfs(&d);
        fs::remove_dir_all(d.join("System")).unwrap();
        let r: Report = inspect(&d);
        assert!(!r.ok);
        assert!(r.romfs.is_none());
        assert!(r.problems.iter().any(|p| p.contains("1.1.1")));
        assert!(r.checks.iter().any(|(l, ok)| l.contains("Product.111") && !*ok));
    }

    #[test]
    fn a_folder_with_a_few_files_missing_is_refused_and_names_them() {
        let d: PB = tmp("partial");
        fake_romfs(&d);
        fs::remove_file(d.join("Event").join("EventFlow").join("Dm_SY_Treasurebox.bfevfl.zs")).unwrap();
        let r: Report = inspect(&d);
        assert!(!r.ok);
        assert!(r.problems.iter().any(|p| p.contains("Dm_SY_Treasurebox") || p.contains("missing 1 files")));
    }

    #[test]
    fn finds_a_folder_by_names_alone() {
        let d: PB = tmp("direct");
        fake_romfs(&d);
        let r: Report = inspect(&d);
        assert!(r.ok);
        assert_eq!(r.romfs.as_deref(), Some(d.as_path()));
        assert!(r.checks.iter().all(|c| c.1));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn finds_romfs_one_level_down() {
        let d: PB = tmp("nested");
        fake_romfs(&d.join("romfs"));
        let r: Report = inspect(&d);
        assert!(r.ok);
        assert_eq!(r.romfs.as_deref(), Some(d.join("romfs").as_path()));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn empty_and_partial_folders_fail_with_a_plain_message() {
        let d: PB = tmp("empty");
        let r: Report = inspect(&d);
        assert!(!r.ok && r.romfs.is_none() && r.problems.len() == 1);
        assert!(r.checks.iter().all(|c| !c.1));
        fs::create_dir_all(d.join("Banc")).unwrap();
        let r: Report = inspect(&d);
        assert!(!r.ok);
        assert_eq!(r.checks.iter().filter(|c| c.1).count(), 1);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_mods_tree_is_named_as_one_within_two_levels_and_a_game_next_to_it_is_pointed_at() {
        let d: PB = tmp("mods");
        let one: PB = d.join("mods").join("My Mod");
        fs::create_dir_all(one.join("romfs").join("Event")).unwrap();
        fs::create_dir_all(one.join("exefs")).unwrap();
        fs::write(one.join("romfs").join("Event").join("a.zs"), b"").unwrap();
        let r: Report = inspect(&d.join("mods"));
        assert!(!r.ok && r.romfs.is_none() && r.problems.len() == 1);
        assert!(r.problems[0].contains("mods folder") && r.problems[0].contains("My Mod/exefs"), "{}", r.problems[0]);
        let r: Report = inspect(&one);
        assert!(r.problems.len() == 1 && r.problems[0].contains("mods folder"), "{:?}", r.problems);
        let r: Report = inspect(&d);
        assert!(!r.ok && r.problems.len() == 1 && !r.problems[0].contains("mods folder"), "{:?}", r.problems);
        fake_romfs(&d.join("dump").join("romfs"));
        let r: Report = inspect(&d);
        assert!(!r.ok && r.romfs.is_none());
        assert!(r.problems[0].contains("dump/romfs") && r.problems[0].contains("Pick that folder"), "{}", r.problems[0]);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn files_and_missing_paths_fail() {
        let d: PB = tmp("file");
        fs::write(d.join("x.txt"), b"hi").unwrap();
        assert!(!inspect(&d.join("x.txt")).ok);
        assert!(!inspect(&d.join("nope")).ok);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn the_report_never_carries_file_contents_and_names_no_source() {
        let d: PB = tmp("json");
        fake_romfs(&d);
        fs::write(d.join("Pack").join("ZsDic.pack.zs"), b"SECRET-CONTENT").unwrap();
        let j: Json = run(d.to_str(), None).unwrap();
        let t: String = j.to_string();
        assert!(!t.contains("SECRET-CONTENT"));
        assert!(!t.to_ascii_lowercase().contains("download"));
        assert_eq!(j.get("ok").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(j.get("version").and_then(|x| x.as_str()), Some(V111));
        let _ = fs::remove_dir_all(&d);
    }

    fn rel(root: &P, r: &str) -> PB {
        r.split('/').fold(root.to_path_buf(), |a, s| a.join(s))
    }

    fn touch(root: &P, r: &str) {
        let t: PB = rel(root, r);
        fs::create_dir_all(t.parent().unwrap()).unwrap();
        fs::write(&t, b"").unwrap();
    }

    fn fake_old(root: &P) {
        fake_romfs(root);
        fs::remove_file(rel(root, VERSION)).unwrap();
        touch(root, BASE_VERSIONS[0]);
    }

    #[test]
    fn a_1_0_0_folder_asks_for_the_update() {
        let d: PB = tmp("v100");
        fake_old(&d);
        let r: Report = inspect(&d);
        assert!(!r.ok && r.needs_update && r.version == Some(V100));
        assert!(r.problems.iter().any(|p| p.starts_with("v1.0.0 detected; v1.1.1 is required")), "{:?}", r.problems);
        assert!(r.update_problems.is_empty());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_separate_update_folder_completes_a_1_0_0_game() {
        let d: PB = tmp("pair");
        let (g, u): (PB, PB) = (d.join("base"), d.join("upd"));
        fake_old(&g);
        let moved: &str = "Event/EventFlow/Dm_SY_Treasurebox.bfevfl.zs";
        fs::remove_file(rel(&g, moved)).unwrap();
        touch(&u.join("1").join("romfs"), VERSION);
        let r: Report = inspect_with(&g, Some(&u));
        assert!(!r.ok, "a file in neither folder must be missing");
        assert!(r.update_problems.iter().any(|p| p.contains("missing 1 files")), "{:?}", r.update_problems);
        touch(&u.join("1").join("romfs"), moved);
        let r: Report = inspect_with(&g, Some(&u));
        assert!(r.ok && r.needs_update && !r.update_auto, "{:?} {:?}", r.problems, r.update_problems);
        assert_eq!(r.update.as_deref(), Some(u.join("1").join("romfs").as_path()));
        let src: Source = r.source.clone().unwrap();
        assert_eq!(src.base, Side::Folder(g.clone()));
        assert_eq!(src.update, Some(Side::Folder(u.join("1").join("romfs"))));
        let r: Report = inspect_with(&g, Some(&d.join("nope")));
        assert!(!r.ok && !r.update_problems.is_empty());
        let r: Report = inspect_with(&g, Some(&g));
        assert!(!r.ok && !r.update_problems.is_empty());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn an_old_update_folder_is_named_as_old() {
        let d: PB = tmp("oldupd");
        fake_old(&d.join("g"));
        touch(&d.join("u"), BASE_VERSIONS[0]);
        let r: Report = inspect_with(&d.join("g"), Some(&d.join("u")));
        assert!(!r.ok && r.update_problems[0].contains("v1.0.0 too"), "{:?}", r.update_problems);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_dump_folder_suggests_its_newest_update_but_does_not_use_it() {
        let d: PB = tmp("dump");
        fake_old(&d.join("base").join("romfs"));
        touch(&d.join("updates").join("65536").join("romfs"), BASE_VERSIONS[0]);
        touch(&d.join("updates").join("131072").join("romfs"), VERSION);
        let newest: PB = d.join("updates").join("131072");
        for pick in [d.clone(), d.join("base"), d.join("base").join("romfs")] {
            let r: Report = inspect(&pick);
            assert!(!r.ok && r.needs_update && !r.update_auto && r.update.is_none(), "{pick:?} {:?}", r.problems);
            assert!(r.problems.iter().any(|p| p.starts_with("v1.0.0 detected")), "{:?}", r.problems);
            assert_eq!(r.update_hint.as_deref(), Some(newest.as_path()));
            let r: Report = inspect_with(&pick, Some(&newest));
            assert!(r.ok && r.update_hint.is_none(), "{pick:?} {:?} {:?}", r.problems, r.update_problems);
            assert_eq!(r.update.as_deref(), Some(newest.join("romfs").as_path()));
        }
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn unsafe_update_paths_are_refused_and_an_empty_one_is_none() {
        for u in ["relative", "/a/../b", "C:\\a\\..\\b"] {
            assert!(run(Some("/a"), Some(u)).is_err(), "{u}");
        }
        assert!(opt_path(Some("  ")).unwrap().is_none());
        assert!(opt_path(None).unwrap().is_none());
    }

    #[test]
    fn unsafe_paths_are_refused_before_touching_the_disk() {
        for p in [None, Some(""), Some("relative"), Some("/a/../b"), Some("C:\\a\\..\\b")] {
            assert!(run(p, None).is_err(), "{p:?}");
        }
    }
}
