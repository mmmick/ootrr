//! NUP reader (a55uka/nup SPEC.md v1). Reader conformance per §6.1: the superblock is
//! validated, the TOC SHA-256 is verified before any TOC value is used, every offset and
//! size is bounds-checked, unknown codecs, kinds and flags are refused, and names are
//! treated as labels, never as paths.

use std::collections::{BTreeMap as BM, HashSet};
use std::fs::File;

use super::{FileRef, Game, Tree, err, read_at, safe_name, slice, u16_at, u32_at, u64_at, zstd};
use crate::error::Res;
use crate::settings::digest::sha256;

const HEADER: usize = 0x80;
const TOC_HEAD: usize = 0x30;
const PKG: usize = 0x40;
const DIR: usize = 0x18;
const FILE: usize = 0x50;
const ALIGN: u64 = 16;
const NONE: u32 = 0xFFFF_FFFF;
const TOC_MAX: u64 = 1 << 30;
pub const KIND_BASE: u8 = 0;
pub const KIND_UPDATE: u8 = 1;
const KINDS: u8 = 5;
const STANDALONE: u8 = 0x01;
/// The upper bits of a title ID name the application; an update's ID is the base's + 0x800.
const APP_MASK: u64 = !0xFFF;

#[derive(Clone, Copy, Debug)]
pub struct Package {
    pub title_id: u64,
    pub exefs: u32,
    pub romfs: u32,
    pub version: u32,
    pub kind: u8,
    pub flags: u8,
}

#[derive(Clone, Copy)]
struct Node {
    name: (u32, u32),
    first: u32,
    count: u32,
}

#[derive(Clone, Copy)]
struct Rec {
    name: (u32, u32),
    size: u64,
    off: u64,
    stored: u64,
    sha: [u8; 32],
    codec: u8,
}

pub struct Nup {
    f: File,
    pub packages: Vec<Package>,
    dirs: Vec<Node>,
    files: Vec<Rec>,
    children: Vec<u32>,
    pool: Vec<u8>,
}

fn zero(b: &[u8]) -> bool {
    b.iter().all(|x| *x == 0)
}

impl Nup {
    pub fn open(f: File) -> Res<Nup> {
        let real: u64 = f.metadata()?.len();
        let h: Vec<u8> = read_at(&f, 0, HEADER).map_err(|_| err("NUP superblock truncated"))?;
        if &h[0..3] != b"NUP" {
            return Err(err("not a NUP file"));
        }
        if h[3] != 1 {
            return Err(err(format!("NUP format version {} is not supported (only 1)", h[3])));
        }
        let header_size: u64 = u16_at(&h, 4)? as u64;
        let toc_off: u64 = u64_at(&h, 0x08)?;
        let toc_size: u64 = u64_at(&h, 0x10)?;
        let toc_sha: &[u8] = &h[0x18..0x38];
        let blob: u64 = u64_at(&h, 0x38)?;
        let file_size: u64 = u64_at(&h, 0x40)?;
        let pkg_count: u32 = u32_at(&h, 0x48)?;
        if header_size < HEADER as u64 || toc_off != header_size {
            return Err(err("NUP header size or TOC offset is wrong"));
        }
        if u32_at(&h, 0x4C)? != 0 || !zero(&h[0x60..0x80]) {
            return Err(err("NUP superblock has unknown flags or nonzero reserved bytes"));
        }
        if file_size != real {
            return Err(err(format!("NUP says it is {file_size} bytes but the file is {real}; it is incomplete or damaged")));
        }
        let toc_end: u64 = toc_off.checked_add(toc_size).ok_or_else(|| err("NUP TOC range overflows"))?;
        if blob % ALIGN != 0 || blob < toc_end || blob > file_size || toc_size > TOC_MAX || pkg_count == 0 {
            return Err(err("NUP region layout is invalid"));
        }
        let toc: Vec<u8> = read_at(&f, toc_off, toc_size as usize)?;
        if sha256(&toc) != toc_sha {
            return Err(err("NUP TOC SHA-256 mismatch; the file is damaged"));
        }
        let mut n: Nup = Nup { f, packages: Vec::new(), dirs: Vec::new(), files: Vec::new(), children: Vec::new(), pool: Vec::new() };
        n.parse(&toc, pkg_count, blob, file_size)?;
        Ok(n)
    }

    fn parse(&mut self, toc: &[u8], pkg_count: u32, blob: u64, file_size: u64) -> Res<()> {
        if toc.get(0..4) != Some(b"TOC\x01") {
            return Err(err("bad NUP TOC magic"));
        }
        if u32_at(toc, 4)? != pkg_count {
            return Err(err("NUP package count differs between superblock and TOC"));
        }
        let dir_count: u64 = u64_at(toc, 0x08)?;
        let file_count: u64 = u64_at(toc, 0x10)?;
        let pool_off: u64 = u64_at(toc, 0x18)?;
        let pool_size: u64 = u64_at(toc, 0x20)?;
        if u64_at(toc, 0x28)? != 0 {
            return Err(err("NUP TOC reserved field is not zero"));
        }
        let need: u64 = (pkg_count as u64)
            .checked_mul(PKG as u64)
            .and_then(|a| dir_count.checked_mul(DIR as u64).and_then(|b| a.checked_add(b)))
            .and_then(|a| file_count.checked_mul(FILE as u64).and_then(|b| a.checked_add(b)))
            .and_then(|a| a.checked_add(TOC_HEAD as u64))
            .ok_or_else(|| err("NUP TOC counts overflow"))?;
        if need > toc.len() as u64 || pool_off < need {
            return Err(err("NUP TOC truncated"));
        }
        self.pool = toc[slice(toc, pool_off, pool_size, "NUP string pool")?].to_vec();
        let mut pos: usize = TOC_HEAD;
        for _ in 0..pkg_count {
            let r: &[u8] = &toc[pos..pos + PKG];
            pos += PKG;
            let kind: u8 = r[0x18];
            if kind >= KINDS {
                return Err(err(format!("unknown NUP package kind {kind}")));
            }
            if u16_at(r, 0x1A)? != 0 || u32_at(r, 0x1C)? != 0 || !zero(&r[0x2C..0x40]) {
                return Err(err("NUP package reserved bytes are not zero"));
            }
            if r[0x19] & !STANDALONE != 0 {
                return Err(err("unknown NUP package flags"));
            }
            self.packages.push(Package { title_id: u64_at(r, 0)?, exefs: u32_at(r, 0x0C)?, romfs: u32_at(r, 0x10)?, version: u32_at(r, 0x14)?, kind, flags: r[0x19] });
            let (root, meta_off, meta_size): (u32, u64, u64) = (u32_at(r, 0x08)?, u64_at(r, 0x20)?, u32_at(r, 0x28)? as u64);
            if root as u64 >= dir_count || (meta_size > 0 && meta_off.checked_add(meta_size).is_none_or(|e| e > file_size)) {
                return Err(err("NUP package root or meta block is out of bounds"));
            }
        }
        for _ in 0..dir_count {
            let r: &[u8] = &toc[pos..pos + DIR];
            pos += DIR;
            self.dirs.push(Node { name: (u32_at(r, 0)?, u32_at(r, 4)?), first: u32_at(r, 8)?, count: u32_at(r, 12)? });
        }
        for _ in 0..file_count {
            let r: &[u8] = &toc[pos..pos + FILE];
            pos += FILE;
            let codec: u8 = r[0x40];
            if codec > 1 || r[0x41] != 0 {
                return Err(err(format!("unknown NUP codec {codec}")));
            }
            let mut sha: [u8; 32] = [0; 32];
            sha.copy_from_slice(&r[0x20..0x40]);
            let rec: Rec = Rec { name: (u32_at(r, 0)?, u32_at(r, 4)?), size: u64_at(r, 8)?, off: u64_at(r, 0x10)?, stored: u64_at(r, 0x18)?, sha, codec };
            if rec.size == 0 {
                if rec.off != 0 || rec.stored != 0 || rec.codec != 0 {
                    return Err(err("NUP empty file has a nonempty extent"));
                }
            } else {
                let end: Option<u64> = rec.off.checked_add(rec.stored);
                if rec.stored == 0 || rec.off < blob || rec.off % ALIGN != 0 || end.is_none_or(|e| e > file_size) || (codec == 0 && rec.stored != rec.size) {
                    return Err(err("NUP file extent is out of bounds or misaligned"));
                }
            }
            self.files.push(rec);
        }
        if (pool_off as usize - pos) % 4 != 0 {
            return Err(err("NUP children array is not 4-byte aligned"));
        }
        self.children = toc[pos..pool_off as usize].chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
        let mut used: u64 = 0;
        for d in &self.dirs {
            if (d.first as u64 + d.count as u64) > self.children.len() as u64 {
                return Err(err("NUP directory child range is out of bounds"));
            }
            used += d.count as u64;
        }
        if used != self.children.len() as u64 {
            return Err(err("NUP child slots do not add up"));
        }
        let has_base: bool = self.packages.iter().any(|p| p.kind == KIND_BASE);
        for p in &self.packages {
            if [p.exefs, p.romfs].iter().any(|d| *d != NONE && *d as u64 >= dir_count) {
                return Err(err("NUP package exefs or romfs folder is out of bounds"));
            }
            if p.kind == KIND_UPDATE && !has_base && p.flags & STANDALONE == 0 {
                return Err(err("NUP update has no base in the file and is not marked standalone"));
            }
        }
        Ok(())
    }

    fn name(&self, (off, len): (u32, u32)) -> Res<String> {
        safe_name(&self.pool[slice(&self.pool, off as u64, len as u64, "NUP name")?])
    }

    fn walk(&self, top: u32, prefix: &str, out: &mut BM<String, FileRef>) -> Res<()> {
        let dc: u32 = self.dirs.len() as u32;
        let mut seen: HashSet<u32> = HashSet::new();
        let mut stack: Vec<(u32, String)> = vec![(top, prefix.to_string())];
        while let Some((d, pre)) = stack.pop() {
            if !seen.insert(d) {
                return Err(err("NUP folder tree has a cycle"));
            }
            let node: Node = self.dirs[d as usize];
            let mut names: HashSet<String> = HashSet::new();
            for &c in &self.children[node.first as usize..(node.first + node.count) as usize] {
                let is_dir: bool = c < dc;
                let name: String = if is_dir {
                    self.name(self.dirs[c as usize].name)?
                } else {
                    let i: usize = (c - dc) as usize;
                    self.name(self.files.get(i).ok_or_else(|| err("NUP child index out of bounds"))?.name)?
                };
                if !names.insert(name.clone()) {
                    return Err(err(format!("duplicate name {name:?} in the NUP")));
                }
                let path: String = format!("{pre}/{name}");
                if is_dir {
                    stack.push((c, path));
                } else {
                    let r: Rec = self.files[(c - dc) as usize];
                    out.insert(path, FileRef { id: c - dc, size: r.size, sha256: r.sha });
                }
            }
        }
        Ok(())
    }

    pub fn tree(&self, p: &Package) -> Res<Tree> {
        let mut files: BM<String, FileRef> = BM::new();
        for (d, name) in [(p.romfs, "romfs"), (p.exefs, "exefs")] {
            if d != NONE {
                self.walk(d, name, &mut files)?;
            }
        }
        Ok(Tree { version: p.version, update: p.kind == KIND_UPDATE, standalone: p.flags & STANDALONE != 0, files })
    }

    /// The newest base or update package of one game. An update package may hold only the
    /// files it changes (the reference packer stores an update folder as given), so it is
    /// laid over the base by path, as SPEC.md Appendix A describes.
    pub fn game(&self) -> Res<Game> {
        let ps: Vec<&Package> = self.packages.iter().filter(|p| p.kind == KIND_BASE || p.kind == KIND_UPDATE).collect();
        let apps: HashSet<u64> = ps.iter().map(|p| p.title_id & APP_MASK).collect();
        if apps.len() != 1 {
            return Err(err("the NUP must hold the base or update of exactly one game"));
        }
        let top: u32 = ps.iter().map(|p| p.version).max().unwrap_or(0);
        let newest: Vec<&&Package> = ps.iter().filter(|p| p.version == top).collect();
        if newest.len() != 1 || newest[0].romfs == NONE {
            return Err(err("the NUP's newest package must be one package with a romfs folder"));
        }
        let base: Option<Tree> = match ps.iter().find(|p| p.kind == KIND_BASE) {
            Some(b) if b.version != top => Some(self.tree(b)?),
            _ => None,
        };
        let mut t: Tree = self.tree(newest[0])?;
        if let Some(b) = &base {
            let mut files: BM<String, FileRef> = b.files.clone();
            files.extend(std::mem::take(&mut t.files));
            t.files = files;
        }
        Ok(Game { newest: t, base })
    }

    pub fn read(&self, f: &FileRef) -> Res<Vec<u8>> {
        let r: Rec = *self.files.get(f.id as usize).ok_or_else(|| err("NUP file id out of bounds"))?;
        if r.size == 0 {
            return Ok(Vec::new());
        }
        let raw: Vec<u8> = read_at(&self.f, r.off, r.stored as usize)?;
        if r.codec == 0 { Ok(raw) } else { zstd(&raw, r.size) }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::container::Archive;
    use std::path::PathBuf as PB;

    pub enum Item<'a> {
        Dir(&'a str, Vec<Item<'a>>),
        File(&'a str, &'a [u8], bool),
    }

    /// A minimal NUP writer for tests: packages of (title, version, kind, flags, romfs, exefs).
    pub fn write(path: &PB, pkgs: &[(u64, u32, u8, u8, Vec<Item<'_>>, Vec<Item<'_>>)]) {
        let mut pool: Vec<u8> = Vec::new();
        let mut dirs: Vec<(u32, u32, Vec<u32>)> = Vec::new();
        let mut files: Vec<(u32, u32, Vec<u8>, bool)> = Vec::new();
        let mut dir_kids: Vec<Vec<(bool, usize)>> = Vec::new();
        fn add(pool: &mut Vec<u8>, s: &str) -> (u32, u32) {
            let o: u32 = pool.len() as u32;
            pool.extend_from_slice(s.as_bytes());
            (o, s.len() as u32)
        }
        fn put(it: &[Item<'_>], name: &str, pool: &mut Vec<u8>, dirs: &mut Vec<(u32, u32, Vec<u32>)>, files: &mut Vec<(u32, u32, Vec<u8>, bool)>, kids: &mut Vec<Vec<(bool, usize)>>) -> usize {
            let (o, l) = add(pool, name);
            let me: usize = dirs.len();
            dirs.push((o, l, Vec::new()));
            kids.push(Vec::new());
            for x in it {
                match x {
                    Item::Dir(n, sub) => {
                        let d: usize = put(sub, n, pool, dirs, files, kids);
                        kids[me].push((true, d));
                    }
                    Item::File(n, b, z) => {
                        let (o, l) = add(pool, n);
                        files.push((o, l, b.to_vec(), *z));
                        kids[me].push((false, files.len() - 1));
                    }
                }
            }
            me
        }
        let mut roots: Vec<(usize, Option<usize>, Option<usize>)> = Vec::new();
        for (_, _, _, _, rom, exe) in pkgs {
            let root: usize = put(&[], "", &mut pool, &mut dirs, &mut files, &mut dir_kids);
            let r: Option<usize> = (!rom.is_empty()).then(|| put(rom, "romfs", &mut pool, &mut dirs, &mut files, &mut dir_kids));
            let e: Option<usize> = (!exe.is_empty()).then(|| put(exe, "exefs", &mut pool, &mut dirs, &mut files, &mut dir_kids));
            for x in [r, e].into_iter().flatten() {
                dir_kids[root].push((true, x));
            }
            roots.push((root, r, e));
        }
        let dc: u32 = dirs.len() as u32;
        let mut children: Vec<u32> = Vec::new();
        for (i, k) in dir_kids.iter().enumerate() {
            dirs[i].2 = vec![children.len() as u32, k.len() as u32];
            children.extend(k.iter().map(|(d, j)| if *d { *j as u32 } else { dc + *j as u32 }));
        }
        let head: usize = TOC_HEAD + pkgs.len() * PKG + dirs.len() * DIR + files.len() * FILE + children.len() * 4;
        let toc_size: usize = head + pool.len();
        let blob: u64 = ((HEADER + toc_size) as u64).div_ceil(16) * 16;
        let mut data: Vec<u8> = Vec::new();
        let mut ext: Vec<(u64, u64, u8)> = Vec::new();
        for (_, _, b, z) in &files {
            if b.is_empty() {
                ext.push((0, 0, 0));
                continue;
            }
            while (blob + data.len() as u64) % 16 != 0 {
                data.push(0);
            }
            let stored: Vec<u8> = if *z { crate::zs::compress_raw(b, None).unwrap() } else { b.clone() };
            ext.push((blob + data.len() as u64, stored.len() as u64, *z as u8));
            data.extend_from_slice(&stored);
        }
        let mut toc: Vec<u8> = Vec::new();
        toc.extend_from_slice(b"TOC\x01");
        toc.extend_from_slice(&(pkgs.len() as u32).to_le_bytes());
        for v in [dirs.len() as u64, files.len() as u64, head as u64, pool.len() as u64, 0] {
            toc.extend_from_slice(&v.to_le_bytes());
        }
        for ((title, ver, kind, flags, _, _), (root, r, e)) in pkgs.iter().zip(&roots) {
            let mut p: Vec<u8> = Vec::new();
            p.extend_from_slice(&title.to_le_bytes());
            for v in [*root as u32, e.map(|x| x as u32).unwrap_or(NONE), r.map(|x| x as u32).unwrap_or(NONE), *ver] {
                p.extend_from_slice(&v.to_le_bytes());
            }
            p.extend_from_slice(&[*kind, *flags, 0, 0, 0, 0, 0, 0]);
            p.resize(PKG, 0);
            toc.extend_from_slice(&p);
        }
        for (o, l, k) in &dirs {
            for v in [*o, *l, k[0], k[1]] {
                toc.extend_from_slice(&v.to_le_bytes());
            }
            toc.extend_from_slice(&[0; 8]);
        }
        for ((o, l, b, _), (eo, es, c)) in files.iter().zip(&ext) {
            let mut r: Vec<u8> = Vec::new();
            r.extend_from_slice(&o.to_le_bytes());
            r.extend_from_slice(&l.to_le_bytes());
            for v in [b.len() as u64, *eo, *es] {
                r.extend_from_slice(&v.to_le_bytes());
            }
            r.extend_from_slice(&sha256(b));
            r.push(*c);
            r.resize(FILE, 0);
            toc.extend_from_slice(&r);
        }
        for c in &children {
            toc.extend_from_slice(&c.to_le_bytes());
        }
        toc.extend_from_slice(&pool);
        let total: u64 = blob + data.len() as u64;
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"NUP\x01");
        out.extend_from_slice(&(HEADER as u16).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&(HEADER as u64).to_le_bytes());
        out.extend_from_slice(&(toc_size as u64).to_le_bytes());
        out.extend_from_slice(&sha256(&toc));
        out.extend_from_slice(&blob.to_le_bytes());
        out.extend_from_slice(&total.to_le_bytes());
        out.extend_from_slice(&(pkgs.len() as u32).to_le_bytes());
        out.resize(HEADER, 0);
        out.extend_from_slice(&toc);
        out.resize(blob as usize, 0);
        out.extend_from_slice(&data);
        std::fs::write(path, out).unwrap();
    }

    fn tmp(tag: &str) -> PB {
        let d: PB = std::env::temp_dir().join(format!("ootrr_nup_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn sample(path: &PB) {
        let base = vec![Item::Dir("Pack", vec![Item::File("ZsDic.pack.zs", b"dic", false)]), Item::Dir("Event", vec![Item::File("x.zs", b"old x", true), Item::File("same.zs", b"same", false)])];
        let upd = vec![
            Item::Dir("Pack", vec![Item::File("ZsDic.pack.zs", b"dic", false)]),
            Item::Dir("Event", vec![Item::File("x.zs", b"new x", true), Item::File("same.zs", b"same", false), Item::File("empty", b"", false)]),
        ];
        write(path, &[(0x0400486024DEC000, 0, KIND_BASE, 0, base, vec![Item::File("main", b"m0", false)]), (0x0400486024DEC800, 131072, KIND_UPDATE, 0, upd, vec![Item::File("main", b"m1", true)])]);
    }

    #[test]
    fn reads_the_newest_package_and_names_what_the_update_changes() {
        let d: PB = tmp("read");
        let p: PB = d.join("g.nup");
        sample(&p);
        let a: Archive = Archive::open(&p).unwrap();
        assert_eq!(a.kind(), "NUP");
        let g: Game = a.game().unwrap();
        assert!(g.newest.update && g.newest.version == 131072 && g.base.is_some());
        assert_eq!(a.read(&g.newest.files["romfs/Event/x.zs"]).unwrap(), b"new x");
        assert_eq!(a.read(&g.newest.files["romfs/Event/empty"]).unwrap(), b"");
        assert_eq!(a.read(&g.newest.files["exefs/main"]).unwrap(), b"m1");
        let ch: Vec<String> = g.changed().into_keys().collect();
        assert_eq!(ch, vec!["exefs/main", "romfs/Event/empty", "romfs/Event/x.zs"]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_update_holding_only_its_changes_is_laid_over_the_base() {
        let d: PB = tmp("partial");
        let p: PB = d.join("g.nup");
        let base = vec![Item::Dir("Pack", vec![Item::File("ZsDic.pack.zs", b"dic", false)]), Item::File("x", b"old", false)];
        let upd = vec![Item::File("x", b"new", false), Item::File("y", b"added", true)];
        write(&p, &[(0x0400486024DEC000, 0, KIND_BASE, 0, base, vec![]), (0x0400486024DEC800, 131072, KIND_UPDATE, 0, upd, vec![])]);
        let a: Archive = Archive::open(&p).unwrap();
        let g: Game = a.game().unwrap();
        assert_eq!(a.read(&g.newest.files["romfs/Pack/ZsDic.pack.zs"]).unwrap(), b"dic");
        assert_eq!(a.read(&g.newest.files["romfs/x"]).unwrap(), b"new");
        assert_eq!(g.changed().into_keys().collect::<Vec<_>>(), vec!["romfs/x", "romfs/y"]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn damage_is_refused() {
        let d: PB = tmp("bad");
        let p: PB = d.join("g.nup");
        sample(&p);
        let good: Vec<u8> = std::fs::read(&p).unwrap();
        let mut b: Vec<u8> = good.clone();
        b[HEADER + 10] ^= 1;
        std::fs::write(&p, &b).unwrap();
        assert!(Archive::open(&p).err().unwrap().to_string().contains("SHA-256"));
        let mut b: Vec<u8> = good.clone();
        let n: usize = b.len();
        b[n - 1] ^= 0xFF;
        std::fs::write(&p, &b).unwrap();
        let a: Archive = Archive::open(&p).unwrap();
        let g: Game = a.game().unwrap();
        let bad: usize = g.newest.files.values().filter(|f| a.read(f).is_err()).count();
        assert_eq!(bad, 1);
        std::fs::write(&p, &good[..n - 1]).unwrap();
        assert!(Archive::open(&p).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_update_alone_must_say_so_and_unsafe_names_are_refused() {
        let d: PB = tmp("alone");
        let p: PB = d.join("u.nup");
        write(&p, &[(0x0400486024DEC800, 131072, KIND_UPDATE, 0, vec![Item::File("a", b"a", false)], vec![])]);
        assert!(Archive::open(&p).is_err());
        write(&p, &[(0x0400486024DEC800, 131072, KIND_UPDATE, STANDALONE, vec![Item::File("a", b"a", false)], vec![])]);
        let g: Game = Archive::open(&p).unwrap().game().unwrap();
        assert!(g.newest.standalone && g.base.is_none());
        assert_eq!(g.changed().len(), 1);
        write(&p, &[(1, 0, KIND_BASE, 0, vec![Item::File("..", b"a", false)], vec![])]);
        assert!(Archive::open(&p).unwrap().game().is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
