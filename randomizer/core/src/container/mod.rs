//! Read-only readers for game dumps packed into one file: NUP (`.nup`, a55uka's NX Package
//! format, SPEC.md v1) and NX (`.nx`, fab918's nxformat, SPEC.md release 1.0.3, wire
//! versions 0.2 and 0.3). Both are MIT licensed; see NOTICE. Only what the randomizer needs
//! is read: the packages or snapshots of one game, their `romfs/` and `exefs/` trees and
//! the bytes of single files. Every structure is bounds-checked and every hash the formats
//! carry is verified before it is trusted; every file's SHA-256 is checked when it is read.
//!
//! Nothing here writes to the container, and nothing is unpacked unless asked for file by
//! file, so a caller can take only the part of the game it reads.

use std::collections::BTreeMap as BM;
use std::fs::File;
use std::path::Path as P;

use crate::error::{Error, Res};

pub mod nup;
pub mod nx;

/// One file of a package or snapshot tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileRef {
    /// The file's record index in its container.
    pub id: u32,
    pub size: u64,
    pub sha256: [u8; 32],
}

/// One package (NUP) or snapshot (NX) of the game: files by `romfs/...` and `exefs/...` path.
#[derive(Clone, Debug)]
pub struct Tree {
    /// The NCM content version, e.g. 131072 for 1.1.1 (opaque to both formats).
    pub version: u32,
    /// An update rather than the base game.
    pub update: bool,
    /// An update packed without its base, holding only the files it changes.
    pub standalone: bool,
    pub files: BM<String, FileRef>,
}

/// The game in a container: its newest package or snapshot and, when present, the base.
pub struct Game {
    pub newest: Tree,
    pub base: Option<Tree>,
}

pub enum Archive {
    Nup(nup::Nup),
    Nx(nx::Nx),
}

pub fn err(m: impl Into<String>) -> Error {
    Error::Msg(m.into())
}

/// True for a path that names a container file by its extension.
pub fn is_container(p: &P) -> bool {
    p.extension().and_then(|x| x.to_str()).is_some_and(|x| x.eq_ignore_ascii_case("nup") || x.eq_ignore_ascii_case("nx"))
}

impl Archive {
    /// Opens a `.nup` or `.nx`, chosen by its magic bytes.
    pub fn open(p: &P) -> Res<Archive> {
        let f: File = File::open(p)?;
        let head: Vec<u8> = read_at(&f, 0, 4).map_err(|_| err("the file is too short to be a .nup or .nx"))?;
        match head.as_slice() {
            [b'N', b'U', b'P', _] => Ok(Archive::Nup(nup::Nup::open(f)?)),
            b"NXAR" => Ok(Archive::Nx(nx::Nx::open(f)?)),
            _ => Err(err("this file is not a .nup or .nx game package")),
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Archive::Nup(_) => "NUP",
            Archive::Nx(_) => "NX",
        }
    }

    /// The base and newest update of the one game in the container.
    pub fn game(&self) -> Res<Game> {
        match self {
            Archive::Nup(n) => n.game(),
            Archive::Nx(n) => n.game(),
        }
    }

    /// The verified bytes of one file.
    pub fn read(&self, f: &FileRef) -> Res<Vec<u8>> {
        let b: Vec<u8> = match self {
            Archive::Nup(n) => n.read(f)?,
            Archive::Nx(n) => n.read(f)?,
        };
        if b.len() as u64 != f.size {
            return Err(err(format!("file {} decoded to {} bytes, not {}", f.id, b.len(), f.size)));
        }
        if crate::settings::digest::sha256(&b) != f.sha256 {
            return Err(err(format!("file {} failed its SHA-256 check; the package is damaged", f.id)));
        }
        Ok(b)
    }
}

impl Game {
    /// The files of the newest update that the base does not have, or has with other
    /// contents: what a 1.0.0 game needs added to become this version.
    pub fn changed(&self) -> BM<String, FileRef> {
        if !self.newest.update {
            return BM::new();
        }
        let Some(b) = &self.base else { return self.newest.files.clone() };
        self.newest.files.iter().filter(|(p, f)| b.files.get(*p).is_none_or(|x| x.sha256 != f.sha256)).map(|(p, f)| (p.clone(), *f)).collect()
    }
}

pub fn read_at(f: &File, off: u64, len: usize) -> Res<Vec<u8>> {
    let mut b: Vec<u8> = vec![0; len];
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        f.read_exact_at(&mut b, off).map_err(|e| err(format!("could not read {len} bytes at {off}: {e}")))?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileExt;
        let mut done: usize = 0;
        while done < len {
            let n: usize = f.seek_read(&mut b[done..], off + done as u64)?;
            if n == 0 {
                return Err(err(format!("could not read {len} bytes at {off}: the file ends first")));
            }
            done += n;
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        use std::io::{Read, Seek, SeekFrom};
        let mut g: &File = f;
        g.seek(SeekFrom::Start(off))?;
        g.read_exact(&mut b).map_err(|e| err(format!("could not read {len} bytes at {off}: {e}")))?;
    }
    Ok(b)
}

pub fn u16_at(b: &[u8], o: usize) -> Res<u16> {
    b.get(o..o + 2).map(|x| u16::from_le_bytes([x[0], x[1]])).ok_or_else(|| err("record truncated"))
}

pub fn u32_at(b: &[u8], o: usize) -> Res<u32> {
    b.get(o..o + 4).map(|x| u32::from_le_bytes(x.try_into().unwrap())).ok_or_else(|| err("record truncated"))
}

pub fn u64_at(b: &[u8], o: usize) -> Res<u64> {
    b.get(o..o + 8).map(|x| u64::from_le_bytes(x.try_into().unwrap())).ok_or_else(|| err("record truncated"))
}

/// `b[off..off+len]`, refusing a range outside `b` (checked arithmetic, as both specs ask).
pub fn slice(b: &[u8], off: u64, len: u64, what: &str) -> Res<std::ops::Range<usize>> {
    let end: u64 = off.checked_add(len).ok_or_else(|| err(format!("{what}: range overflows")))?;
    if end > b.len() as u64 {
        return Err(err(format!("{what}: range {off}+{len} is outside its {}-byte region", b.len())));
    }
    Ok(off as usize..end as usize)
}

/// A tree entry name must be one plain path component.
pub fn safe_name(n: &[u8]) -> Res<String> {
    let s: &str = std::str::from_utf8(n).map_err(|_| err("a file name is not UTF-8"))?;
    let bad: bool = s.is_empty() || s == "." || s == ".." || s.chars().any(|c| matches!(c, '/' | '\\' | '\0' | ':'));
    if bad {
        return Err(err(format!("unsafe file name {s:?} in the package")));
    }
    Ok(s.to_string())
}

pub fn zstd(b: &[u8], size: u64) -> Res<Vec<u8>> {
    let out: Vec<u8> = crate::zs::Zs::new().decompress(b)?;
    if out.len() as u64 != size {
        return Err(err(format!("a zstd frame decoded to {} bytes, not {size}", out.len())));
    }
    Ok(out)
}
