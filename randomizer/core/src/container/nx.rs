//! NX archive reader (fab918/nxformat docs/SPEC.md release 1.0.3, wire versions 0.2 and
//! 0.3). Mount follows §1 and §12: both header slots are
//! validated with their CRC-32C, the newest valid generation whose Catalog, Manifest and
//! index SHA-256 digests check out is used, unknown required features and extensions are
//! refused, and each file's SHA-256 is checked when it is read. Media payloads are never read.
//!
//! Every NX snapshot is a complete tree (§8), so the newest one is read on its own, never
//! laid over the base. An archive packed from a decrypted NSP stores native NCAs instead of a
//! `romfs/` tree; it cannot be read here and is refused with a reason.

use std::collections::{BTreeMap as BM, HashSet};
use std::fs::File;

use super::{FileRef, Game, Tree, err, read_at, safe_name, slice, u16_at, u32_at, u64_at, zstd};
use crate::error::Res;
use crate::settings::digest::sha256;

const HEADER: usize = 256;
const SLOTS: [u64; 2] = [0, 4096];
const PAGE: usize = 4096;
const CHECK_SIZE: u32 = 16384;
const CATALOG_MAX: u64 = 16 << 20;
const MANIFEST_MAX: u64 = 1 << 30;
const BLOCK_MAX: u32 = (1 << 20) + (64 << 10);
const KNOWN_REQUIRED: [u32; 3] = [0x0001_0004, 0x0001_0005, 0x0001_0008];
pub const SNAP_BASE: u8 = 0;
pub const SNAP_UPDATE: u8 = 1;
pub const SNAP_STANDALONE: u8 = 3;
const KIND_DIR: u8 = 1;
const KIND_FILE: u8 = 2;

pub fn crc32c(b: &[u8]) -> u32 {
    let mut c: u32 = 0xFFFF_FFFF;
    for x in b {
        c ^= *x as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0x82F6_3B78 } else { c >> 1 };
        }
    }
    !c
}

#[derive(Clone)]
struct Header {
    generation: u64,
    catalog: (u64, u64),
    manifest: (u64, u64),
    index: (u64, u64),
    integrity: (u64, u64),
    shas: [[u8; 32]; 3],
    raw: Vec<u8>,
}

#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    pub application_id: u64,
    pub kind: u8,
    pub version: u32,
    pub root: u32,
}

pub struct Nx {
    f: File,
    pub snapshots: Vec<Snapshot>,
    blocks: Vec<u8>,
    block_count: u64,
    manifest: Vec<u8>,
    dirs: (usize, u32),
    files: (usize, u32),
    entries: (usize, u32),
    refs: (usize, u32),
    pool: (u64, u64),
}

fn parse_header(raw: &[u8], file_size: u64) -> Res<Header> {
    if raw.get(0..4) != Some(b"NXAR") {
        return Err(err("not an NX header"));
    }
    let ver: (u16, u16) = (u16_at(raw, 4)?, u16_at(raw, 6)?);
    if ver != (0, 2) && ver != (0, 3) {
        return Err(err(format!("NX wire version {}.{} is not supported", ver.0, ver.1)));
    }
    if u32_at(raw, 8)? != HEADER as u32 || u16_at(raw, 12)? != 1 || u16_at(raw, 14)? != 0 {
        return Err(err("unsupported NX header size or feature bits"));
    }
    if raw[228..252].iter().any(|x| *x != 0) {
        return Err(err("NX header reserved bytes are not zero"));
    }
    if crc32c(&raw[..252]) != u32_at(raw, 252)? {
        return Err(err("NX header CRC-32C mismatch"));
    }
    if u32_at(raw, 224)? != CHECK_SIZE {
        return Err(err("unsupported NX integrity check size"));
    }
    if u64_at(raw, 16)? > file_size {
        return Err(err("NX archive_size is past the end of the file; it is incomplete"));
    }
    let catalog: (u64, u64) = (u64_at(raw, 24)?, u64_at(raw, 32)?);
    if !(128..=CATALOG_MAX).contains(&catalog.1) {
        return Err(err("bad NX catalog size"));
    }
    let manifest: (u64, u64) = (u64_at(raw, 72)?, u64_at(raw, 80)?);
    if !(96..=MANIFEST_MAX).contains(&manifest.1) {
        return Err(err("bad NX manifest size"));
    }
    let block_count: u64 = u64_at(raw, 96)?;
    if block_count > 1 << 32 {
        return Err(err("bad NX block count"));
    }
    let mut shas: [[u8; 32]; 3] = [[0; 32]; 3];
    for (i, s) in shas.iter_mut().enumerate() {
        s.copy_from_slice(&raw[120 + i * 32..152 + i * 32]);
    }
    let index_size: u64 = block_count.checked_mul(24).ok_or_else(|| err("NX block index overflows"))?;
    Ok(Header {
        generation: u64_at(raw, 216)?,
        catalog,
        manifest,
        index: (u64_at(raw, 88)?, index_size),
        integrity: (u64_at(raw, 104)?, u64_at(raw, 112)?),
        shas,
        raw: raw.to_vec(),
    })
}

fn get(f: &File, (off, len): (u64, u64), file_size: u64, what: &str) -> Res<Vec<u8>> {
    if off.checked_add(len).is_none_or(|e| e > file_size) {
        return Err(err(format!("NX {what} is outside the file")));
    }
    read_at(f, off, len as usize)
}

impl Nx {
    pub fn open(f: File) -> Res<Nx> {
        let size: u64 = f.metadata()?.len();
        let mut valid: Vec<Header> = Vec::new();
        let mut why: Vec<String> = Vec::new();
        for off in SLOTS {
            let len: usize = if off + PAGE as u64 <= size { PAGE } else { HEADER };
            let r: Res<Header> = read_at(&f, off, len).and_then(|page| {
                if page[HEADER..].iter().any(|x| *x != 0) {
                    return Err(err("nonzero header page padding"));
                }
                parse_header(&page[..HEADER], size)
            });
            match r {
                Ok(h) => valid.push(h),
                Err(e) => why.push(format!("slot at {off}: {e}")),
            }
        }
        if valid.is_empty() {
            return Err(err(format!("no valid NX header ({})", why.join("; "))));
        }
        valid.sort_by_key(|h| std::cmp::Reverse(h.generation));
        if valid.len() == 2 && valid[0].generation == valid[1].generation && valid[0].raw != valid[1].raw {
            return Err(err("both NX header slots have the same generation but differ"));
        }
        let mut last: Option<crate::error::Error> = None;
        for h in valid {
            match Nx::mount(&f, &h, size) {
                Ok(parts) => return Ok(Nx { f, ..parts }),
                Err(e) => last = Some(e),
            }
        }
        Err(err(format!("no NX header generation mounts: {}", last.map(|e| e.to_string()).unwrap_or_default())))
    }

    fn mount(f: &File, h: &Header, size: u64) -> Res<Nx> {
        let catalog: Vec<u8> = get(f, h.catalog, size, "catalog")?;
        if sha256(&catalog) != h.shas[0] {
            return Err(err("NX catalog SHA-256 mismatch"));
        }
        let manifest: Vec<u8> = get(f, h.manifest, size, "manifest")?;
        if sha256(&manifest) != h.shas[1] {
            return Err(err("NX manifest SHA-256 mismatch"));
        }
        let mut index: Vec<u8> = Vec::new();
        if h.index.1 > 0 {
            index = get(f, h.index, size, "block index")?;
            index.extend_from_slice(&get(f, h.integrity, size, "integrity table")?);
        }
        if sha256(&index) != h.shas[2] {
            return Err(err("NX block index SHA-256 mismatch"));
        }
        index.truncate(h.index.1 as usize);
        let snapshots: Vec<Snapshot> = catalog_snapshots(&catalog)?;
        let m: &[u8] = &manifest;
        if m.get(0..4) != Some(b"NXMF") || u16_at(m, 4)? != 1 || u16_at(m, 6)? != 96 || u64_at(m, 8)? != m.len() as u64 {
            return Err(err("bad NX manifest header"));
        }
        let (dc, fc, ec, rc): (u32, u32, u32, u32) = (u32_at(m, 16)?, u32_at(m, 20)?, u32_at(m, 24)?, u32_at(m, 28)?);
        let table = |at: usize, n: u32, w: u64, what: &str| -> Res<(usize, u32)> {
            let off: u64 = u64_at(m, at)?;
            slice(m, off, n as u64 * w, what).map(|r| (r.start, n))
        };
        let dirs: (usize, u32) = table(32, dc, 16, "NX directory table")?;
        let files: (usize, u32) = table(40, fc, 64, "NX file table")?;
        let entries: (usize, u32) = table(48, ec, 24, "NX entry table")?;
        let refs: (usize, u32) = table(56, rc, 4, "NX block reference table")?;
        let pool: (u64, u64) = (u64_at(m, 64)?, u64_at(m, 72)?);
        slice(m, pool.0, pool.1, "NX path pool")?;
        Ok(Nx { f: f.try_clone()?, snapshots, blocks: index, block_count: h.index.1 / 24, manifest, dirs, files, entries, refs, pool })
    }

    fn file(&self, id: u32) -> Res<(u64, u32, u32, u8, [u8; 32])> {
        if id >= self.files.1 {
            return Err(err(format!("NX file id {id} out of range")));
        }
        let r: &[u8] = &self.manifest[self.files.0 + id as usize * 64..][..64];
        let (size, first, count, log2): (u64, u32, u32, u8) = (u64_at(r, 0)?, u32_at(r, 8)?, u32_at(r, 12)?, r[16]);
        if !(16..=20).contains(&log2) {
            return Err(err(format!("NX file {id}: bad chunk size")));
        }
        if count as u64 != size.div_ceil(1 << log2) || first as u64 + count as u64 > self.refs.1 as u64 {
            return Err(err(format!("NX file {id}: block references do not match its size")));
        }
        let mut sha: [u8; 32] = [0; 32];
        sha.copy_from_slice(&r[20..52]);
        Ok((size, first, count, log2, sha))
    }

    pub fn tree(&self, s: &Snapshot) -> Res<Tree> {
        let mut files: BM<String, FileRef> = BM::new();
        let (mut seen_d, mut seen_f): (HashSet<u32>, HashSet<u32>) = (HashSet::new(), HashSet::new());
        let mut stack: Vec<(u32, String)> = vec![(s.root, String::new())];
        while let Some((d, pre)) = stack.pop() {
            if d >= self.dirs.1 || !seen_d.insert(d) {
                return Err(err("NX folder is out of range or reached twice"));
            }
            let r: &[u8] = &self.manifest[self.dirs.0 + d as usize * 16..][..16];
            let (first, count): (u32, u32) = (u32_at(r, 0)?, u32_at(r, 4)?);
            if first as u64 + count as u64 > self.entries.1 as u64 {
                return Err(err("NX folder entry range is out of bounds"));
            }
            for e in first..first + count {
                let row: &[u8] = &self.manifest[self.entries.0 + e as usize * 24..][..24];
                let (no, nl): (u64, u64) = (u64_at(row, 0)?, u32_at(row, 8)? as u64);
                if no < self.pool.0 || no.checked_add(nl).is_none_or(|x| x > self.pool.0 + self.pool.1) {
                    return Err(err("NX entry name is outside the path pool"));
                }
                let name: String = safe_name(&self.manifest[slice(&self.manifest, no, nl, "NX name")?])?;
                let path: String = if pre.is_empty() { name } else { format!("{pre}/{name}") };
                let target: u32 = u32_at(row, 12)?;
                match row[16] {
                    KIND_DIR => stack.push((target, path)),
                    KIND_FILE => {
                        if !seen_f.insert(target) {
                            return Err(err("an NX file appears twice in one snapshot"));
                        }
                        let (size, _, _, _, sha) = self.file(target)?;
                        files.insert(path, FileRef { id: target, size, sha256: sha });
                    }
                    k => return Err(err(format!("bad NX entry kind {k}"))),
                }
            }
        }
        Ok(Tree { version: s.version, update: s.kind != SNAP_BASE, standalone: s.kind == SNAP_STANDALONE, files })
    }

    /// The newest base or update snapshot; its tree is complete on its own.
    pub fn game(&self) -> Res<Game> {
        let ss: Vec<&Snapshot> = self.snapshots.iter().filter(|s| matches!(s.kind, SNAP_BASE | SNAP_UPDATE | SNAP_STANDALONE)).collect();
        let apps: HashSet<u64> = ss.iter().map(|s| s.application_id).collect();
        if apps.len() != 1 {
            return Err(err("the NX archive must hold the base or update of exactly one game"));
        }
        let top: u32 = ss.iter().map(|s| s.version).max().unwrap_or(0);
        let newest: Vec<&&Snapshot> = ss.iter().filter(|s| s.version == top).collect();
        if newest.len() != 1 {
            return Err(err("the NX archive has more than one snapshot of its newest version"));
        }
        let t: Tree = self.tree(newest[0])?;
        if !t.files.keys().any(|p| p.starts_with("romfs/")) {
            return Err(err(
                "this NX archive has no romfs folder (it was probably packed from a decrypted NSP, which stores native NCAs). Convert the game from its NUP or an extracted dump folder instead, or pick those directly",
            ));
        }
        let base: Option<Tree> = match ss.iter().find(|s| s.kind == SNAP_BASE) {
            Some(b) if b.version != top => Some(self.tree(b)?),
            _ => None,
        };
        Ok(Game { newest: t, base })
    }

    fn block(&self, id: u32) -> Res<Vec<u8>> {
        if id as u64 >= self.block_count {
            return Err(err(format!("NX block {id} out of range")));
        }
        let r: &[u8] = &self.blocks[id as usize * 24..][..24];
        let (off, stored, logical, codec): (u64, u32, u32, u8) = (u64_at(r, 0)?, u32_at(r, 8)?, u32_at(r, 12)?, r[16]);
        if stored == 0 || stored > BLOCK_MAX || logical == 0 || logical > 1 << 20 {
            return Err(err(format!("NX block {id} has a bad size")));
        }
        let data: Vec<u8> = read_at(&self.f, off, stored as usize)?;
        match codec {
            0 if stored == logical => Ok(data),
            0 => Err(err(format!("NX block {id}: RAW sizes differ"))),
            1 => zstd(&data, logical as u64),
            c => Err(err(format!("NX block {id}: unknown codec {c}"))),
        }
    }

    pub fn read(&self, f: &FileRef) -> Res<Vec<u8>> {
        let (size, first, count, log2, _) = self.file(f.id)?;
        let chunk: u64 = 1 << log2;
        let mut out: Vec<u8> = Vec::with_capacity(size as usize);
        for k in 0..count {
            let at: usize = self.refs.0 + (first + k) as usize * 4;
            let id: u32 = u32_at(&self.manifest, at)?;
            let b: Vec<u8> = self.block(id)?;
            let want: u64 = chunk.min(size - k as u64 * chunk);
            if b.len() as u64 != want {
                return Err(err(format!("NX block {id} holds {} bytes where the file needs {want}", b.len())));
            }
            out.extend_from_slice(&b);
        }
        Ok(out)
    }
}

fn catalog_snapshots(c: &[u8]) -> Res<Vec<Snapshot>> {
    if c.get(0..4) != Some(b"NXCT") || u16_at(c, 4)? != 1 || u16_at(c, 6)? != 128 || u64_at(c, 8)? != c.len() as u64 {
        return Err(err("bad NX catalog header"));
    }
    let (apps_n, snaps_n, exts_n): (u32, u32, u32) = (u32_at(c, 20)?, u32_at(c, 28)?, u32_at(c, 40)?);
    let apps = slice(c, u64_at(c, 48)?, apps_n as u64 * 80, "NX application table")?;
    let snaps = slice(c, u64_at(c, 64)?, snaps_n as u64 * 120, "NX snapshot table")?;
    let exts = slice(c, u64_at(c, 88)?, exts_n as u64 * 32, "NX extension table")?;
    for i in 0..exts_n as usize {
        let e: &[u8] = &c[exts.start + i * 32..][..32];
        let (id, flags): (u32, u32) = (u32_at(e, 0)?, u32_at(e, 4)?);
        if flags & 1 != 0 && !KNOWN_REQUIRED.contains(&id) {
            return Err(err(format!("unknown required NX extension {id:#010x}")));
        }
    }
    let mut owner: BM<u32, u64> = BM::new();
    for a in 0..apps_n as usize {
        let r: &[u8] = &c[apps.start + a * 80..][..80];
        let (first, count): (u32, u32) = (u32_at(r, 20)?, u32_at(r, 24)?);
        if first as u64 + count as u64 > snaps_n as u64 {
            return Err(err("NX application snapshot range is out of bounds"));
        }
        for s in first..first + count {
            owner.insert(s, u64_at(r, 0)?);
        }
    }
    let mut out: Vec<Snapshot> = Vec::new();
    for s in 0..snaps_n {
        let r: &[u8] = &c[snaps.start + s as usize * 120..][..120];
        let app: u64 = *owner.get(&s).ok_or_else(|| err(format!("NX snapshot {s} belongs to no application")))?;
        out.push(Snapshot { application_id: app, kind: r[4], version: u32_at(r, 8)?, root: u32_at(r, 12)? });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::Archive;
    use std::path::PathBuf as PB;

    #[test]
    fn crc32c_matches_the_spec_check_value() {
        assert_eq!(crc32c(b"123456789"), 0xE306_9283);
    }

    struct W {
        pool: Vec<u8>,
        dirs: Vec<(u32, u32)>,
        entries: Vec<(u64, u32, u32, u8)>,
        files: Vec<(u64, u32, u32, [u8; 32])>,
        refs: Vec<u32>,
        data: Vec<u8>,
        blocks: Vec<(u64, u32, u32, u8)>,
    }

    /// Writes a directory (name, children) where a child is a dir or a file (name, bytes).
    enum N<'a> {
        D(&'a str, Vec<N<'a>>),
        F(&'a str, &'a [u8]),
    }

    impl W {
        fn file(&mut self, b: &[u8], zst: bool) -> u32 {
            let first: u32 = self.refs.len() as u32;
            let chunks: Vec<&[u8]> = if b.is_empty() { Vec::new() } else { b.chunks(1 << 16).collect() };
            for c in &chunks {
                while (8192 + self.data.len()) % 16 != 0 {
                    self.data.push(0);
                }
                let stored: Vec<u8> = if zst { crate::zs::compress_raw(c, None).unwrap() } else { c.to_vec() };
                self.blocks.push((8192 + self.data.len() as u64, stored.len() as u32, c.len() as u32, zst as u8));
                self.data.extend_from_slice(&stored);
                self.refs.push(self.blocks.len() as u32 - 1);
            }
            self.files.push((b.len() as u64, first, chunks.len() as u32, sha256(b)));
            self.files.len() as u32 - 1
        }

        /// Directory entries are written in a later pass so ranges are contiguous.
        fn dir(&mut self, kids: &[N<'_>], zst: bool) -> u32 {
            let me: u32 = self.dirs.len() as u32;
            self.dirs.push((0, 0));
            let mut rows: Vec<(String, u32, u8)> = Vec::new();
            for k in kids {
                match k {
                    N::D(n, sub) => rows.push((n.to_string(), self.dir(sub, zst), KIND_DIR)),
                    N::F(n, b) => rows.push((n.to_string(), self.file(b, zst), KIND_FILE)),
                }
            }
            rows.sort();
            self.dirs[me as usize] = (self.entries.len() as u32, rows.len() as u32);
            for (n, t, k) in rows {
                let o: u64 = self.pool.len() as u64;
                self.pool.extend_from_slice(n.as_bytes());
                self.entries.push((o, n.len() as u32, t, k));
            }
            me
        }
    }

    fn pad8(v: &mut Vec<u8>) {
        while v.len() % 8 != 0 {
            v.push(0);
        }
    }

    /// A minimal NX writer for tests: one application with snapshots (type, version, tree).
    fn write(path: &PB, snaps: &[(u8, u32, Vec<N<'_>>)], zst: bool, gen_b: Option<u64>) {
        let mut w: W = W { pool: Vec::new(), dirs: Vec::new(), entries: Vec::new(), files: Vec::new(), refs: Vec::new(), data: Vec::new(), blocks: Vec::new() };
        let roots: Vec<u32> = snaps.iter().map(|s| w.dir(&s.2, zst)).collect();
        let mut m: Vec<u8> = vec![0; 96];
        let dt: u64 = m.len() as u64;
        for (f, c) in &w.dirs {
            m.extend_from_slice(&f.to_le_bytes());
            m.extend_from_slice(&c.to_le_bytes());
            m.extend_from_slice(&[0; 8]);
        }
        pad8(&mut m);
        let ft: u64 = m.len() as u64;
        for (s, f, c, h) in &w.files {
            m.extend_from_slice(&s.to_le_bytes());
            m.extend_from_slice(&f.to_le_bytes());
            m.extend_from_slice(&c.to_le_bytes());
            m.extend_from_slice(&[16, 0, 0, 0]);
            m.extend_from_slice(h);
            m.extend_from_slice(&[0; 12]);
        }
        pad8(&mut m);
        let et: u64 = m.len() as u64;
        let entries_at: usize = m.len();
        m.resize(entries_at + w.entries.len() * 24, 0);
        let rt: u64 = m.len() as u64;
        for r in &w.refs {
            m.extend_from_slice(&r.to_le_bytes());
        }
        pad8(&mut m);
        let pt: u64 = m.len() as u64;
        m.extend_from_slice(&w.pool);
        pad8(&mut m);
        for (i, (o, l, t, k)) in w.entries.iter().enumerate() {
            let r: &mut [u8] = &mut m[entries_at + i * 24..entries_at + i * 24 + 24];
            r[0..8].copy_from_slice(&(pt + o).to_le_bytes());
            r[8..12].copy_from_slice(&l.to_le_bytes());
            r[12..16].copy_from_slice(&t.to_le_bytes());
            r[16] = *k;
        }
        let total: u64 = m.len() as u64;
        m[0..4].copy_from_slice(b"NXMF");
        m[4..6].copy_from_slice(&1u16.to_le_bytes());
        m[6..8].copy_from_slice(&96u16.to_le_bytes());
        m[8..16].copy_from_slice(&total.to_le_bytes());
        for (i, v) in [w.dirs.len(), w.files.len(), w.entries.len(), w.refs.len()].iter().enumerate() {
            m[16 + i * 4..20 + i * 4].copy_from_slice(&(*v as u32).to_le_bytes());
        }
        for (i, v) in [dt, ft, et, rt, pt, w.pool.len() as u64].iter().enumerate() {
            m[32 + i * 8..40 + i * 8].copy_from_slice(&v.to_le_bytes());
        }
        let mut c: Vec<u8> = vec![0; 128];
        let at: u64 = c.len() as u64;
        let mut app: Vec<u8> = vec![0; 80];
        app[0..8].copy_from_slice(&0x0400486024DEC000u64.to_le_bytes());
        app[24..28].copy_from_slice(&(snaps.len() as u32).to_le_bytes());
        c.extend_from_slice(&app);
        let st: u64 = c.len() as u64;
        for ((kind, ver, _), root) in snaps.iter().zip(&roots) {
            let mut s: Vec<u8> = vec![0; 120];
            s[4] = *kind;
            s[8..12].copy_from_slice(&ver.to_le_bytes());
            s[12..16].copy_from_slice(&root.to_le_bytes());
            c.extend_from_slice(&s);
        }
        let sp: u64 = c.len() as u64;
        let total: u64 = c.len() as u64;
        c[0..4].copy_from_slice(b"NXCT");
        c[4..6].copy_from_slice(&1u16.to_le_bytes());
        c[6..8].copy_from_slice(&128u16.to_le_bytes());
        c[8..16].copy_from_slice(&total.to_le_bytes());
        c[20..24].copy_from_slice(&1u32.to_le_bytes());
        c[28..32].copy_from_slice(&(snaps.len() as u32).to_le_bytes());
        c[48..56].copy_from_slice(&at.to_le_bytes());
        c[64..72].copy_from_slice(&st.to_le_bytes());
        c[96..104].copy_from_slice(&sp.to_le_bytes());
        let mut idx: Vec<u8> = Vec::new();
        let mut checks: Vec<u8> = Vec::new();
        let mut first: u32 = 0;
        for (o, s, l, k) in &w.blocks {
            idx.extend_from_slice(&o.to_le_bytes());
            idx.extend_from_slice(&s.to_le_bytes());
            idx.extend_from_slice(&l.to_le_bytes());
            idx.extend_from_slice(&[*k, 0, 0, 0]);
            idx.extend_from_slice(&first.to_le_bytes());
            let n: u32 = l.div_ceil(CHECK_SIZE);
            for _ in 0..n {
                checks.extend_from_slice(&0u32.to_le_bytes());
            }
            first += n;
        }
        let mut out: Vec<u8> = vec![0; 8192];
        out.extend_from_slice(&w.data);
        let a16 = |o: &mut Vec<u8>| -> u64 {
            while o.len() % 16 != 0 {
                o.push(0);
            }
            o.len() as u64
        };
        let co: u64 = a16(&mut out);
        out.extend_from_slice(&c);
        let mo: u64 = a16(&mut out);
        out.extend_from_slice(&m);
        let (io, ito): (u64, u64) = if w.blocks.is_empty() {
            (0, 0)
        } else {
            let io: u64 = a16(&mut out);
            out.extend_from_slice(&idx);
            let ito: u64 = a16(&mut out);
            out.extend_from_slice(&checks);
            (io, ito)
        };
        let mut index_hashed: Vec<u8> = idx.clone();
        index_hashed.extend_from_slice(&checks);
        let head = |generation: u64| -> Vec<u8> {
            let mut h: Vec<u8> = vec![0; HEADER];
            h[0..4].copy_from_slice(b"NXAR");
            h[6..8].copy_from_slice(&2u16.to_le_bytes());
            h[8..12].copy_from_slice(&(HEADER as u32).to_le_bytes());
            h[12..14].copy_from_slice(&1u16.to_le_bytes());
            let fields: [(usize, u64); 13] = [
                (16, out.len() as u64),
                (24, co),
                (32, c.len() as u64),
                (56, if w.blocks.is_empty() { 0 } else { 8192 }),
                (64, w.data.len() as u64),
                (72, mo),
                (80, m.len() as u64),
                (88, io),
                (96, w.blocks.len() as u64),
                (104, ito),
                (112, checks.len() as u64),
                (216, generation),
                (0, 0),
            ];
            for (o, v) in fields.iter().filter(|x| x.0 != 0) {
                h[*o..*o + 8].copy_from_slice(&v.to_le_bytes());
            }
            h[120..152].copy_from_slice(&sha256(&c));
            h[152..184].copy_from_slice(&sha256(&m));
            h[184..216].copy_from_slice(&sha256(&index_hashed));
            h[224..228].copy_from_slice(&CHECK_SIZE.to_le_bytes());
            let crc: u32 = crc32c(&h[..252]);
            h[252..256].copy_from_slice(&crc.to_le_bytes());
            h
        };
        let (a, b): (Vec<u8>, Vec<u8>) = (head(0), head(gen_b.unwrap_or(0)));
        out[0..HEADER].copy_from_slice(&a);
        out[4096..4096 + HEADER].copy_from_slice(&b);
        std::fs::write(path, out).unwrap();
    }

    fn tmp(tag: &str) -> PB {
        let d: PB = std::env::temp_dir().join(format!("ootrr_nx_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn sample(p: &PB, zst: bool) -> Vec<u8> {
        let big: Vec<u8> = (0..200_000u32).map(|i| (i * 7 % 251) as u8).collect();
        let base = vec![N::D("romfs", vec![N::D("Pack", vec![N::F("ZsDic.pack.zs", b"dic")]), N::F("x", b"old")]), N::D("exefs", vec![N::F("main", b"m0")])];
        let upd = vec![N::D("romfs", vec![N::D("Pack", vec![N::F("ZsDic.pack.zs", b"dic")]), N::F("x", b"new"), N::F("big", &big), N::F("e", b"")]), N::D("exefs", vec![N::F("main", b"m1")])];
        write(p, &[(SNAP_BASE, 0, base), (SNAP_UPDATE, 131072, upd)], zst, None);
        big
    }

    #[test]
    fn reads_the_newest_snapshot_raw_and_zstd() {
        for zst in [false, true] {
            let d: PB = tmp(if zst { "z" } else { "r" });
            let p: PB = d.join("g.nx");
            let big: Vec<u8> = sample(&p, zst);
            let a: Archive = Archive::open(&p).unwrap();
            assert_eq!(a.kind(), "NX");
            let g: Game = a.game().unwrap();
            assert!(g.newest.update && g.newest.version == 131072 && g.base.is_some());
            assert_eq!(a.read(&g.newest.files["romfs/x"]).unwrap(), b"new");
            assert_eq!(a.read(&g.newest.files["romfs/big"]).unwrap(), big);
            assert_eq!(a.read(&g.newest.files["romfs/e"]).unwrap(), b"");
            let ch: Vec<String> = g.changed().into_keys().collect();
            assert_eq!(ch, vec!["exefs/main", "romfs/big", "romfs/e", "romfs/x"]);
            let _ = std::fs::remove_dir_all(&d);
        }
    }

    #[test]
    fn a_damaged_header_slot_falls_back_and_damaged_data_is_refused() {
        let d: PB = tmp("dmg");
        let p: PB = d.join("g.nx");
        sample(&p, false);
        let good: Vec<u8> = std::fs::read(&p).unwrap();
        let mut b: Vec<u8> = good.clone();
        b[40] ^= 1;
        std::fs::write(&p, &b).unwrap();
        assert!(Archive::open(&p).is_ok(), "slot B still mounts");
        b[4096 + 40] ^= 1;
        std::fs::write(&p, &b).unwrap();
        assert!(Archive::open(&p).is_err());
        let mut b: Vec<u8> = good.clone();
        let at: usize = 8192 + b[8192..].windows(2).position(|w| w == b"m1").unwrap();
        b[at] ^= 1;
        std::fs::write(&p, &b).unwrap();
        let a: Archive = Archive::open(&p).unwrap();
        let g: Game = a.game().unwrap();
        assert_eq!(g.newest.files.values().filter(|f| a.read(f).is_err()).count(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_archive_without_a_romfs_tree_is_refused_with_a_reason() {
        let d: PB = tmp("nca");
        let p: PB = d.join("g.nx");
        write(&p, &[(SNAP_BASE, 0, vec![N::F("game.nca", b"nca")])], false, None);
        let e: String = Archive::open(&p).unwrap().game().err().unwrap().to_string();
        assert!(e.contains("no romfs folder"), "{e}");
        let _ = std::fs::remove_dir_all(&d);
    }
}
