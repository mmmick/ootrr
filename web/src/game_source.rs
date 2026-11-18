//! Where the game files come from, and how a job gets one folder to read them from.
//!
//! A game is an extracted folder or a packed `.nup` / `.nx`, optionally with a separate
//! v1.1.1 update that is also a folder or a package. The build and the tracker read one
//! folder, so a job prepares one:
//!
//! - Folders only: the game folder itself, or, with an update, the link-only view of
//!   `game_view` (no game data is copied).
//! - A package anywhere: only the part of the game the build and tracker read
//!   (`unpack_set`, about 370 MB, or about 2.3 GB with music or sound effect shuffle) is
//!   unpacked into a temporary folder in the app's data folder, and that folder is deleted
//!   when the job ends. Files that come from a folder are hard-linked there, not copied.

use std::collections::{BTreeMap as BM, BTreeSet as BS};
use std::fs;
use std::io;
use std::path::{Path as P, PathBuf as PB};
use std::sync::atomic::{AtomicU64, Ordering};

use ootrr_core::container::{Archive, FileRef, Game};
use ootrr_core::required::{self, Unpack};

use crate::tracker::{icon_map, index};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Side {
    /// A romfs folder.
    Folder(PB),
    /// A `.nup` or `.nx` file.
    Pack(PB),
}

#[derive(Clone, Debug)]
pub struct Source {
    pub base: Side,
    /// A separate v1.1.1 update laid over a v1.0.0 `base`.
    pub update: Option<Side>,
    /// `base` is a package holding both the base game and its v1.1.1 update.
    pub update_in_pack: bool,
}

impl Source {
    pub fn folder(r: PB) -> Source {
        Source { base: Side::Folder(r), update: None, update_in_pack: false }
    }

    pub fn packed(&self) -> bool {
        matches!(self.base, Side::Pack(_)) || matches!(self.update, Some(Side::Pack(_)))
    }

    /// There is a v1.1.1 update whose files can be added to a mod.
    pub fn has_update(&self) -> bool {
        self.update.is_some() || self.update_in_pack
    }

    /// The job step that prepares the files, when there is one.
    pub fn step(&self) -> Option<&'static str> {
        if self.packed() {
            Some(UNPACK_STEP)
        } else if self.update.is_some() {
            Some(VIEW_STEP)
        } else {
            None
        }
    }
}

pub const VIEW_STEP: &str = "Lay the v1.1.1 update over the v1.0.0 game (links only, nothing is copied)";
pub const UNPACK_STEP: &str = "Unpack the game files the build reads into a temporary folder";

/// What a container holds, for checking it: its newest tree's romfs file names.
pub struct PackInfo {
    pub kind: &'static str,
    pub files: BS<String>,
    pub standalone: bool,
    /// The newest tree is an update and the base is in the same file.
    pub update_in_pack: bool,
}

pub fn pack_info(p: &P) -> Result<PackInfo, String> {
    let a: Archive = Archive::open(p).map_err(|e| e.to_string())?;
    let g: Game = a.game().map_err(|e| e.to_string())?;
    let files: BS<String> = g.newest.files.keys().filter_map(|k| k.strip_prefix("romfs/")).map(|s| s.to_string()).collect();
    Ok(PackInfo { kind: a.kind(), files, standalone: g.newest.standalone, update_in_pack: g.newest.update && g.base.is_some() })
}

/// The files a build and the tracker read. `sound` adds what music and sound effect
/// shuffle read.
pub fn unpack_set(sound: bool) -> Result<Unpack, String> {
    let d = ootrr_core::slice::data::Data::load().map_err(|e| e.to_string())?;
    let mut u: Unpack = required::unpack(&d, sound);
    u.trees.insert("UI/Tex".to_string());
    u.files.extend([index::CAPTURE, index::MENU, index::MAP_PARAM].map(|s| s.to_string()));
    u.files.extend(icon_map::all().into_iter().map(|s| s.file));
    Ok(u)
}

/// One folder holding the game files a job reads. A temporary unpack is removed on drop.
pub struct Prepared {
    pub romfs: PB,
    temp: Option<PB>,
}

impl Drop for Prepared {
    fn drop(&mut self) {
        if let Some(t) = &self.temp {
            let _ = remove_ours(t);
        }
    }
}

const MARK: &str = ".ootrr-unpack";
static SEQ: AtomicU64 = AtomicU64::new(0);

fn unpack_root(data: &P) -> PB {
    data.join("ootrr").join("unpack")
}

/// Removes a folder only when it carries our marker. `remove_dir_all` removes links
/// themselves and never follows them, so a hard link back to a game folder only drops a link.
fn remove_ours(t: &P) -> io::Result<()> {
    if t.join(MARK).is_file() { fs::remove_dir_all(t) } else { Ok(()) }
}

/// Removes temporary unpacks that an earlier run of the app left behind.
fn sweep(root: &P) {
    let mine: String = format!("{}-", std::process::id());
    for e in fs::read_dir(root).into_iter().flatten().flatten() {
        let n: String = e.file_name().to_string_lossy().into_owned();
        if !n.starts_with(&mine) {
            let _ = remove_ours(&e.path());
        }
    }
}

enum From {
    Dir(PB),
    Pack(usize, FileRef),
}

pub fn prepare(src: &Source, sound: bool) -> Result<Prepared, String> {
    let data = || crate::datadir::base().ok_or_else(|| "Game folder: there is no app data folder to work in. Set OOTRR_DATA_DIR.".to_string());
    if !src.packed() {
        return match (&src.base, &src.update) {
            (Side::Folder(b), None) => Ok(Prepared { romfs: b.clone(), temp: None }),
            (Side::Folder(b), Some(Side::Folder(u))) => crate::game_view::layered(&data()?, b, u).map(|r| Prepared { romfs: r, temp: None }).map_err(|e| format!("Game folder: {e}")),
            _ => unreachable!(),
        };
    }
    let want: Unpack = unpack_set(sound)?;
    let mut arcs: Vec<Archive> = Vec::new();
    let mut pick: BM<String, From> = BM::new();
    for side in std::iter::once(&src.base).chain(src.update.iter()) {
        match side {
            Side::Folder(r) => {
                for f in crate::game_view::files(r).map_err(|e| format!("Game folder: {e}"))? {
                    if want.wants(&f) {
                        let p: PB = f.split('/').fold(r.clone(), |a, s| a.join(s));
                        pick.insert(f, From::Dir(p));
                    }
                }
            }
            Side::Pack(p) => {
                let a: Archive = Archive::open(p).map_err(|e| format!("Game package: {e}"))?;
                let g: Game = a.game().map_err(|e| format!("Game package: {e}"))?;
                let i: usize = arcs.len();
                for (k, f) in g.newest.files {
                    if let Some(r) = k.strip_prefix("romfs/").filter(|r| want.wants(r)) {
                        pick.insert(r.to_string(), From::Pack(i, f));
                    }
                }
                arcs.push(a);
            }
        }
    }
    let root: PB = unpack_root(&data()?);
    fs::create_dir_all(&root).map_err(|e| format!("Game folder: {e}"))?;
    sweep(&root);
    let dir: PB = root.join(format!("{}-{}", std::process::id(), SEQ.fetch_add(1, Ordering::Relaxed)));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).and_then(|_| fs::write(dir.join(MARK), b"A temporary unpack of game files for one build. Safe to delete.")).map_err(|e| format!("Game folder: {e}"))?;
    let out: Prepared = Prepared { romfs: dir.join("romfs"), temp: Some(dir) };
    for (r, from) in pick {
        let to: PB = r.split('/').fold(out.romfs.clone(), |a, s| a.join(s));
        let made: Result<(), String> = (|| {
            fs::create_dir_all(to.parent().unwrap()).map_err(|e| e.to_string())?;
            match from {
                From::Dir(p) => fs::hard_link(&p, &to).or_else(|_| fs::copy(&p, &to).map(|_| ())).map_err(|e| e.to_string()),
                From::Pack(i, f) => fs::write(&to, arcs[i].read(&f).map_err(|e| e.to_string())?).map_err(|e| e.to_string()),
            }
        })();
        made.map_err(|e| format!("Game folder: could not unpack {r}: {e}"))?;
    }
    Ok(out)
}

/// The v1.1.1 update's own `romfs` and `exefs` files, for adding to a mod.
enum UpdateFiles {
    Dir(PB),
    Pack(Archive, BM<String, FileRef>),
}

fn update_files(src: &Source) -> Result<UpdateFiles, String> {
    let from_pack = |p: &P| -> Result<UpdateFiles, String> {
        let a: Archive = Archive::open(p).map_err(|e| e.to_string())?;
        let ch: BM<String, FileRef> = a.game().map_err(|e| e.to_string())?.changed();
        Ok(UpdateFiles::Pack(a, ch))
    };
    match (&src.update, src.update_in_pack, &src.base) {
        (Some(Side::Folder(r)), _, _) => match r.file_name().and_then(|n| n.to_str()) {
            Some(n) if n.eq_ignore_ascii_case("romfs") => Ok(UpdateFiles::Dir(r.parent().unwrap().to_path_buf())),
            _ => Err("the update files can only be added when the update's romfs folder sits next to its exefs folder".to_string()),
        },
        (Some(Side::Pack(p)), _, _) => from_pack(p),
        (None, true, Side::Pack(p)) => from_pack(p),
        _ => Err("there is no separate v1.1.1 update to add".to_string()),
    }
}

/// Adds the update's own `romfs` and `exefs` files to the mod, under the mod's files: a
/// file the mod already writes is kept. Files are copied or written, never linked, so
/// nothing written to the mod later can reach the update; on APFS, Btrfs and ReFS a copy
/// is a clone that takes no extra space.
pub fn add_update(src: &Source, root: &P) -> Result<usize, String> {
    let mut n: usize = 0;
    let fresh = |rel: &str| -> Option<PB> {
        if rel.split('/').any(|x| x.starts_with('.')) {
            return None;
        }
        let to: PB = rel.split('/').fold(root.to_path_buf(), |a, s| a.join(s));
        (!to.exists() && fs::symlink_metadata(&to).is_err()).then_some(to)
    };
    let put = |to: &P, f: &dyn Fn(&P) -> io::Result<()>| -> Result<(), String> {
        fs::create_dir_all(to.parent().unwrap()).and_then(|_| f(to)).map_err(|e| e.to_string())
    };
    match update_files(src)? {
        UpdateFiles::Dir(top) => {
            for part in ["romfs", "exefs"] {
                let s: PB = top.join(part);
                if !s.is_dir() {
                    continue;
                }
                for f in crate::game_view::files(&s).map_err(|e| e.to_string())? {
                    let Some(to) = fresh(&format!("{part}/{f}")) else { continue };
                    let from: PB = f.split('/').fold(s.clone(), |a, x| a.join(x));
                    put(&to, &|t| fs::copy(&from, t).map(|_| ()))?;
                    n += 1;
                }
            }
        }
        UpdateFiles::Pack(a, files) => {
            for (k, f) in files {
                if !(k.starts_with("romfs/") || k.starts_with("exefs/")) {
                    continue;
                }
                let Some(to) = fresh(&k) else { continue };
                let b: Vec<u8> = a.read(&f).map_err(|e| e.to_string())?;
                put(&to, &|t| fs::write(t, &b))?;
                n += 1;
            }
        }
    }
    Ok(n)
}
