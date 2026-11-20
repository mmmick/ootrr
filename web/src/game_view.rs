//! A 1.1.1 view of the game made from a 1.0.0 base folder and a separate 1.1.1 update folder.
//!
//! The view holds no game data. It is built only from links, in the app's data folder:
//! a folder the update does not touch is one symlink to the base folder, and inside the
//! folders the update does touch each file is a hard link (or, across drives, a symlink)
//! to the update's file or the base's file. Nothing is ever copied, and neither the base
//! nor the update folder is ever written to. If links cannot be made, the view is refused
//! rather than falling back to a copy.
//!
//! Lists of removed files that some unpackers write are not read: for this build they
//! name only superseded `.Product.100.` files, which the build never reads.

use std::collections::BTreeSet as BS;
use std::fs;
use std::io;
use std::path::{Path as P, PathBuf as PB};
use std::time::UNIX_EPOCH;

use ootrr_core::json::{self, Json};
use ootrr_core::settings::digest::sha256_hex;

use crate::datadir;
use crate::pathsafe;

const STAMP: &str = "view.json";
const FORMAT: i64 = 1;

pub fn root(data: &P) -> PB {
    data.join("ootrr").join("game-view")
}

fn rel_of(r: &str) -> PB {
    r.split('/').fold(PB::new(), |a, s| a.join(s))
}

fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() { name.to_string() } else { format!("{dir}/{name}") }
}

fn names(d: &P) -> BS<String> {
    match fs::read_dir(d) {
        Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect(),
        Err(_) => BS::new(),
    }
}

/// Every file in the update, as romfs-relative paths with `/` separators.
pub fn files(update: &P) -> io::Result<BS<String>> {
    let mut out: BS<String> = BS::new();
    let mut stack: Vec<(PB, String)> = vec![(update.to_path_buf(), String::new())];
    while let Some((d, r)) = stack.pop() {
        for e in fs::read_dir(&d)? {
            let e: fs::DirEntry = e?;
            let n: String = e.file_name().to_string_lossy().into_owned();
            let p: PB = e.path();
            if p.is_dir() {
                stack.push((p, join(&r, &n)));
            } else {
                out.insert(join(&r, &n));
            }
        }
    }
    Ok(out)
}

fn ancestors(paths: &BS<String>) -> BS<String> {
    let mut out: BS<String> = BS::new();
    out.insert(String::new());
    for p in paths {
        let mut cur: &str = p.as_str();
        while let Some(i) = cur.rfind('/') {
            cur = &cur[..i];
            out.insert(cur.to_string());
        }
    }
    out
}

fn link_file(src: &P, dst: &P) -> io::Result<()> {
    if fs::hard_link(src, dst).is_ok() {
        return Ok(());
    }
    #[cfg(unix)]
    return std::os::unix::fs::symlink(src, dst);
    #[cfg(windows)]
    return std::os::windows::fs::symlink_file(src, dst);
    #[cfg(not(any(unix, windows)))]
    return fs::copy(src, dst).map(|_| ());
}

fn link_dir(src: &P, dst: &P) -> io::Result<()> {
    #[cfg(unix)]
    return std::os::unix::fs::symlink(src, dst);
    #[cfg(windows)]
    return std::os::windows::fs::symlink_dir(src, dst);
    #[cfg(not(any(unix, windows)))]
    return Err(io::Error::new(io::ErrorKind::Unsupported, format!("cannot link folder {} to {} here", src.display(), dst.display())));
}

struct Job<'a> {
    base: &'a P,
    update: &'a P,
    upd: &'a BS<String>,
    touched: BS<String>,
    out: &'a P,
}

impl Job<'_> {
    fn layer(&self, dir: &str) -> io::Result<()> {
        let rd: PB = rel_of(dir);
        let (b, u, o): (PB, PB, PB) = (self.base.join(&rd), self.update.join(&rd), self.out.join(&rd));
        fs::create_dir_all(&o)?;
        let mut all: BS<String> = names(&b);
        all.extend(names(&u));
        for n in all {
            let r: String = join(dir, &n);
            let (bp, up, op): (PB, PB, PB) = (b.join(&n), u.join(&n), o.join(&n));
            if self.upd.contains(&r) {
                link_file(&up, &op)?;
            } else if bp.is_dir() || up.is_dir() {
                if self.touched.contains(&r) || (bp.is_dir() && up.is_dir()) {
                    self.layer(&r)?;
                } else {
                    let src: &PB = if bp.is_dir() { &bp } else { &up };
                    if link_dir(src, &op).is_err() {
                        self.layer(&r)?;
                    }
                }
            } else if bp.is_file() {
                link_file(&bp, &op)?;
            }
        }
        Ok(())
    }
}

fn mtime(p: &P) -> i64 {
    fs::metadata(p).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn signature(base: &P, update: &P, upd: &BS<String>) -> String {
    let mut buf: String = String::new();
    for (k, v) in [("base", base), ("update", update)] {
        buf.push_str(&format!("{k}={}|{}\n", pathsafe::text(v), mtime(v)));
    }
    for n in names(base) {
        buf.push_str(&format!("b:{n}|{}\n", mtime(&base.join(&n))));
    }
    for f in upd {
        let p: PB = update.join(rel_of(f));
        let len: u64 = fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
        buf.push_str(&format!("u:{f}|{len}|{}\n", mtime(&p)));
    }
    sha256_hex(buf.as_bytes())
}

fn stamp_of(dir: &P) -> Option<String> {
    let t: String = fs::read_to_string(dir.join(STAMP)).ok()?;
    let j: Json = json::parse(&t).ok()?;
    (j.get("format").and_then(|x| x.as_i64()) == Some(FORMAT)).then_some(())?;
    j.get("signature").and_then(|x| x.as_str()).map(|s| s.to_string())
}

/// Removes a view folder, but only one this module made: it must sit in `views` and carry
/// our stamp, or be one of our unfinished `.tmp-` folders. `remove_dir_all` removes
/// symlinks themselves and never follows them, and removing a hard link leaves the
/// game's own file untouched.
fn clear(views: &P, dir: &P) -> io::Result<()> {
    if !dir.exists() && fs::symlink_metadata(dir).is_err() {
        return Ok(());
    }
    let ours: bool = dir.parent() == Some(views)
        && (dir.join(STAMP).is_file() || dir.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.contains(".tmp-")));
    if !ours {
        return Err(io::Error::other(format!("{} was not made by the randomizer, so it was left alone", dir.display())));
    }
    fs::remove_dir_all(dir)
}

/// Returns the romfs folder of a 1.1.1 view of `base` with `update` laid over it, building
/// it in `data` if it is missing or out of date.
pub fn layered(data: &P, base: &P, update: &P) -> io::Result<PB> {
    let base: PB = fs::canonicalize(base)?;
    let update: PB = fs::canonicalize(update)?;
    let upd: BS<String> = files(&update)?;
    let views: PB = root(data);
    fs::create_dir_all(&views)?;
    let views: PB = fs::canonicalize(&views)?;
    let key: String = sha256_hex(format!("{}\n{}", pathsafe::text(&base), pathsafe::text(&update)).as_bytes());
    let dir: PB = views.join(datadir::game_dir_name(&key[..16]));
    let sig: String = signature(&base, &update, &upd);
    if stamp_of(&dir).as_deref() == Some(sig.as_str()) && dir.join("romfs").is_dir() {
        return Ok(dir.join("romfs"));
    }
    let tmp: PB = views.join(format!("{}.tmp-{}", &key[..16], std::process::id()));
    clear(&views, &tmp)?;
    fs::create_dir_all(&tmp)?;
    let out: PB = tmp.join("romfs");
    let job: Job<'_> = Job { base: &base, update: &update, upd: &upd, touched: ancestors(&upd), out: &out };
    let made: io::Result<()> = job.layer("");
    if let Err(e) = made {
        let _ = clear(&views, &tmp);
        return Err(io::Error::other(format!(
            "could not link the game files into {} ({e}). The 1.0.0 game and the 1.1.1 update are only linked there, never copied, so that folder must be on a drive that allows links",
            views.display()
        )));
    }
    let stamp: Json = Json::obj()
        .with("format", Json::Int(FORMAT))
        .with("base", pathsafe::text(&base).into())
        .with("update", pathsafe::text(&update).into())
        .with("signature", sig.as_str().into())
        .with("note", "Links only. Every file here is a link to your own game folders; nothing was copied. Safe to delete.".into());
    fs::write(tmp.join(STAMP), stamp.pretty())?;
    clear(&views, &dir)?;
    fs::rename(&tmp, &dir)?;
    Ok(dir.join("romfs"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn tmp(tag: &str) -> PB {
        let p: PB = env::temp_dir().join(format!("ootrr_view_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        fs::canonicalize(&p).unwrap()
    }

    fn put(root: &P, r: &str, body: &[u8]) {
        let p: PB = root.join(rel_of(r));
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }

    #[test]
    fn the_update_wins_and_untouched_folders_are_one_link() {
        let d: PB = tmp("layer");
        let (b, u, data): (PB, PB, PB) = (d.join("base"), d.join("upd"), d.join("data"));
        put(&b, "Pack/ZsDic.pack.zs", b"dic");
        put(&b, "Banc/Deep/a.bin", b"old-a");
        put(&b, "Event/x.zs", b"old-x");
        put(&b, "Untouched/Inner/y.zs", b"y");
        put(&u, "Event/x.zs", b"new-x");
        put(&u, "System/Resource/t.zs", b"new-t");
        let v: PB = layered(&data, &b, &u).unwrap();
        assert_eq!(fs::read(v.join("Event/x.zs")).unwrap(), b"new-x");
        assert_eq!(fs::read(v.join("System/Resource/t.zs")).unwrap(), b"new-t");
        assert_eq!(fs::read(v.join("Banc/Deep/a.bin")).unwrap(), b"old-a");
        assert_eq!(fs::read(v.join("Pack/ZsDic.pack.zs")).unwrap(), b"dic");
        // Windows without the symlink right falls back to layering the folder with hard links.
        let m: fs::Metadata = fs::symlink_metadata(v.join("Untouched")).unwrap();
        assert!(m.file_type().is_symlink() || (cfg!(windows) && m.is_dir()));
        assert!(v.join("Untouched/Inner/y.zs").is_file());
        let again: PB = layered(&data, &b, &u).unwrap();
        assert_eq!(again, v);
        fs::remove_dir_all(v.parent().unwrap()).unwrap();
        assert_eq!(fs::read(b.join("Event/x.zs")).unwrap(), b"old-x");
        assert_eq!(fs::read(b.join("Untouched/Inner/y.zs")).unwrap(), b"y");
        assert_eq!(fs::read(u.join("Event/x.zs")).unwrap(), b"new-x");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_change_in_the_update_rebuilds_the_view() {
        let d: PB = tmp("rebuild");
        let (b, u, data): (PB, PB, PB) = (d.join("base"), d.join("upd"), d.join("data"));
        put(&b, "Event/x.zs", b"old");
        put(&u, "Event/x.zs", b"new");
        let v: PB = layered(&data, &b, &u).unwrap();
        put(&u, "Event/more.zs", b"more");
        let v2: PB = layered(&data, &b, &u).unwrap();
        assert_eq!(v, v2);
        assert_eq!(fs::read(v2.join("Event/more.zs")).unwrap(), b"more");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_folder_it_did_not_make_is_never_removed() {
        let d: PB = tmp("clear");
        let views: PB = d.join("views");
        let other: PB = views.join("someone-else");
        fs::create_dir_all(&other).unwrap();
        fs::write(other.join("keep.txt"), b"keep").unwrap();
        assert!(clear(&views, &other).is_err());
        assert!(other.join("keep.txt").is_file());
        let _ = fs::remove_dir_all(&d);
    }
}
