use crate::error::{Error, Res};
use crate::rsizetable::hash;

pub const FILE: &str = "ELink2/elink2.Product.110.belnk.zs";
pub const VERSION: u32 = 0x24;
pub const HEAD: usize = 0x60;
pub const CALL: usize = 0x30;
pub const USER_HEAD: usize = 0x38;
pub const USER_PARAMS: usize = 16;
pub const TEXT: u32 = 1;
pub const NONE: u64 = 0xFFFF_FFFF;
pub const CONTAINER: u16 = 0xFFFF;
pub const ALIGN: usize = 8;

const AT_SIZE: usize = 0x04;
const AT_VALUES: usize = 0x0C;
const AT_PARAMS: usize = 0x10;
const AT_OVERWRITE: usize = 0x18;
const AT_LOCAL: usize = 0x20;
const AT_LOCAL_NAMES: usize = 0x28;
const AT_LOCAL_ENUMS: usize = 0x2C;
const AT_DIRECT: usize = 0x30;
const AT_USERS_FROM: usize = 0x40;
const AT_USERS: usize = 0x48;
const AT_CONDITIONS: usize = 0x50;
const AT_NAMES: usize = 0x58;
const MOVED: [usize; 5] = [AT_OVERWRITE, AT_LOCAL, AT_USERS_FROM, AT_CONDITIONS, AT_NAMES];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    pub mask: u64,
    pub vals: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    pub raw: [u8; CALL],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User {
    pub hash: u32,
    pub head: [u32; 14],
    pub pre: Vec<u8>,
    pub sort: Vec<u16>,
    pub pad: Vec<u8>,
    pub calls: Vec<Call>,
    pub boxes: Vec<u8>,
    pub triggers: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub head: Vec<u8>,
    pub hashes: Vec<u32>,
    pub gap: Vec<u8>,
    pub define: Vec<u8>,
    pub params: Vec<Param>,
    pub mid: Vec<u8>,
    pub users: Vec<User>,
    pub slots: Vec<usize>,
    pub cond: Vec<u8>,
    pub names: Vec<u8>,
}

fn bad(m: &'static str) -> Error {
    Error::Format(m)
}

fn u16_at(b: &[u8], o: usize) -> Res<u16> {
    Ok(u16::from_le_bytes(b.get(o..o + 2).ok_or_else(|| bad("elink: a field leaves the file"))?.try_into().map_err(|_| bad("elink: short field"))?))
}

fn u32_at(b: &[u8], o: usize) -> Res<u32> {
    Ok(u32::from_le_bytes(b.get(o..o + 4).ok_or_else(|| bad("elink: a field leaves the file"))?.try_into().map_err(|_| bad("elink: short field"))?))
}

fn u64_at(b: &[u8], o: usize) -> Res<u64> {
    Ok(u64::from_le_bytes(b.get(o..o + 8).ok_or_else(|| bad("elink: a field leaves the file"))?.try_into().map_err(|_| bad("elink: short field"))?))
}

fn pos_at(b: &[u8], o: usize) -> Res<usize> {
    usize::try_from(u64_at(b, o)?).map_err(|_| bad("elink: a position is too large"))
}

fn slice(b: &[u8], from: usize, to: usize) -> Res<Vec<u8>> {
    if from > to {
        return Err(bad("elink: sections are out of order"));
    }
    Ok(b.get(from..to).ok_or_else(|| bad("elink: a section leaves the file"))?.to_vec())
}

fn sort_len(n: usize) -> usize {
    (2 * n + 3) & !3
}

impl Call {
    pub fn key_pos(&self) -> u64 {
        u64::from_le_bytes(self.raw[0..8].try_into().unwrap_or([0; 8]))
    }

    pub fn asset(&self) -> u16 {
        u16::from_le_bytes([self.raw[8], self.raw[9]])
    }

    pub fn key_hash(&self) -> u32 {
        u32::from_le_bytes(self.raw[0x18..0x1C].try_into().unwrap_or([0; 4]))
    }

    pub fn guid(&self) -> u32 {
        u32::from_le_bytes(self.raw[0x14..0x18].try_into().unwrap_or([0; 4]))
    }

    pub fn param_pos(&self) -> u64 {
        u64::from_le_bytes(self.raw[0x20..0x28].try_into().unwrap_or([0; 8]))
    }

    pub fn new(key_pos: u64, asset: u16, guid: u32, key_hash: u32, param_pos: u64) -> Call {
        let mut raw: [u8; CALL] = [0; CALL];
        raw[0..8].copy_from_slice(&key_pos.to_le_bytes());
        raw[8..10].copy_from_slice(&asset.to_le_bytes());
        raw[0x0C..0x10].copy_from_slice(&1u32.to_le_bytes());
        raw[0x10..0x14].copy_from_slice(&u32::MAX.to_le_bytes());
        raw[0x14..0x18].copy_from_slice(&guid.to_le_bytes());
        raw[0x18..0x1C].copy_from_slice(&key_hash.to_le_bytes());
        raw[0x20..0x28].copy_from_slice(&param_pos.to_le_bytes());
        raw[0x28..0x30].copy_from_slice(&NONE.to_le_bytes());
        Call { raw }
    }
}

impl Param {
    pub fn size(&self) -> usize {
        8 + 4 * self.vals.len()
    }

    pub fn get(&self, bit: u32) -> Option<u32> {
        if bit >= 64 || self.mask >> bit & 1 == 0 {
            return None;
        }
        let k: usize = (self.mask & ((1u64 << bit) - 1)).count_ones() as usize;
        self.vals.get(k).copied()
    }

    pub fn from_pairs(pairs: &[(u32, u32)]) -> Res<Param> {
        let mut sorted: Vec<(u32, u32)> = pairs.to_vec();
        sorted.sort_by_key(|p| p.0);
        let mut mask: u64 = 0;
        let mut vals: Vec<u32> = Vec::new();
        for (bit, v) in sorted {
            if bit >= 64 || mask >> bit & 1 == 1 {
                return Err(bad("elink: a parameter bit is out of range or repeated"));
            }
            mask |= 1u64 << bit;
            vals.push(v);
        }
        Ok(Param { mask, vals })
    }

    pub fn pairs(&self) -> Vec<(u32, u32)> {
        (0..64u32).filter(|i| self.mask >> i & 1 == 1).zip(self.vals.iter().copied()).collect()
    }
}

impl User {
    pub fn calls_end(&self) -> usize {
        USER_HEAD + self.pre.len() + 2 * self.sort.len() + self.pad.len() + CALL * self.calls.len()
    }

    pub fn size(&self) -> usize {
        self.calls_end() + self.boxes.len() + self.triggers.len()
    }

    fn read(b: &[u8], at: usize, end: usize, hash: u32) -> Res<User> {
        let mut head: [u32; 14] = [0; 14];
        for (i, h) in head.iter_mut().enumerate() {
            *h = u32_at(b, at + 4 * i)?;
        }
        let local: usize = (head[1] & 0xFFFF) as usize;
        let n: usize = head[2] as usize;
        let sort_at: usize = at + USER_HEAD + 8 * local + USER_PARAMS;
        let mut sort: Vec<u16> = Vec::with_capacity(n);
        for i in 0..n {
            sort.push(u16_at(b, sort_at + 2 * i)?);
        }
        let calls_at: usize = sort_at + sort_len(n);
        let mut calls: Vec<Call> = Vec::with_capacity(n);
        for i in 0..n {
            let raw: [u8; CALL] = b.get(calls_at + CALL * i..calls_at + CALL * (i + 1)).ok_or_else(|| bad("elink: a call leaves the user"))?.try_into().map_err(|_| bad("elink: short call"))?;
            calls.push(Call { raw });
        }
        let boxes_at: usize = calls_at + CALL * n;
        let trig_at: usize = at + head[12] as usize;
        if head[13] != 0 || trig_at < boxes_at || trig_at > end {
            return Err(bad("elink: a user's trigger table is out of place"));
        }
        Ok(User {
            hash,
            head,
            pre: slice(b, at + USER_HEAD, sort_at)?,
            sort,
            pad: slice(b, sort_at + 2 * n, calls_at)?,
            calls,
            boxes: slice(b, boxes_at, trig_at)?,
            triggers: slice(b, trig_at, end)?,
        })
    }

    fn write(&self, out: &mut Vec<u8>) {
        let mut head: [u32; 14] = self.head;
        head[2] = self.calls.len() as u32;
        head[12] = (self.calls_end() + self.boxes.len()) as u32;
        for h in head {
            out.extend_from_slice(&h.to_le_bytes());
        }
        out.extend_from_slice(&self.pre);
        for s in &self.sort {
            out.extend_from_slice(&s.to_le_bytes());
        }
        out.extend_from_slice(&self.pad);
        for c in &self.calls {
            out.extend_from_slice(&c.raw);
        }
        out.extend_from_slice(&self.boxes);
        out.extend_from_slice(&self.triggers);
    }
}

impl Link {
    pub fn parse(b: &[u8]) -> Res<Link> {
        if b.get(..4) != Some(b"XLNK".as_slice()) || u32_at(b, 8)? != VERSION {
            return Err(bad("elink: not a version 0x24 XLNK file"));
        }
        if u32_at(b, AT_SIZE)? as usize != b.len() {
            return Err(bad("elink: the stored size is not the file size"));
        }
        for at in MOVED {
            if u32_at(b, at + 4)? != 0 {
                return Err(bad("elink: a section position is out of range"));
            }
        }
        let n: usize = u32_at(b, AT_USERS)? as usize;
        let hashes_end: usize = HEAD + 4 * n;
        let offs_at: usize = (hashes_end + 7) & !7;
        let define_at: usize = offs_at + 8 * n;
        let mut hashes: Vec<u32> = Vec::with_capacity(n);
        let mut offs: Vec<usize> = Vec::with_capacity(n);
        for i in 0..n {
            hashes.push(u32_at(b, HEAD + 4 * i)?);
            offs.push(pos_at(b, offs_at + 8 * i)?);
        }
        let params_at: usize = define_at + u32_at(b, define_at)? as usize;
        let mid_at: usize = pos_at(b, AT_OVERWRITE)?;
        let users_at: usize = pos_at(b, AT_USERS_FROM)?;
        let cond_at: usize = pos_at(b, AT_CONDITIONS)?;
        let names_at: usize = pos_at(b, AT_NAMES)?;
        let count: usize = u32_at(b, AT_PARAMS)? as usize;
        let mut params: Vec<Param> = Vec::with_capacity(count);
        let mut at: usize = params_at;
        for _ in 0..count {
            let mask: u64 = u64_at(b, at)?;
            let k: usize = mask.count_ones() as usize;
            let mut vals: Vec<u32> = Vec::with_capacity(k);
            for j in 0..k {
                vals.push(u32_at(b, at + 8 + 4 * j)?);
            }
            at += 8 + 4 * k;
            params.push(Param { mask, vals });
        }
        if at != mid_at {
            return Err(bad("elink: the asset parameters do not end where the next table starts"));
        }
        let mut by_pos: Vec<(usize, usize)> = offs.iter().copied().enumerate().map(|(i, o)| (o, i)).collect();
        by_pos.sort_unstable();
        let mut users: Vec<User> = Vec::with_capacity(n);
        let mut slots: Vec<usize> = vec![0; n];
        let mut expect: usize = users_at;
        for (k, (o, i)) in by_pos.iter().enumerate() {
            if *o != expect {
                return Err(bad("elink: users are not stored back to back"));
            }
            let end: usize = by_pos.get(k + 1).map(|x| x.0).unwrap_or(cond_at);
            let u: User = User::read(b, *o, end, hashes[*i])?;
            expect = o + u.size();
            if expect != end {
                return Err(bad("elink: a user's parts do not add up to its size"));
            }
            slots[*i] = k;
            users.push(u);
        }
        if expect != cond_at || cond_at > names_at {
            return Err(bad("elink: the users do not end where the conditions start"));
        }
        let link: Link = Link {
            head: slice(b, 0, HEAD)?,
            hashes,
            gap: slice(b, hashes_end, offs_at)?,
            define: slice(b, define_at, params_at)?,
            params,
            mid: slice(b, mid_at, users_at)?,
            users,
            slots,
            cond: slice(b, cond_at, names_at)?,
            names: slice(b, names_at, b.len())?,
        };
        if link.values() != u32_at(b, AT_VALUES)? as usize || pos_at(b, AT_LOCAL)? < mid_at || pos_at(b, AT_LOCAL)? > users_at {
            return Err(bad("elink: the parameter value count or the property table is out of place"));
        }
        Ok(link)
    }

    pub fn values(&self) -> usize {
        self.params.iter().map(|p| p.vals.len()).sum()
    }

    pub fn param_pos(&self, i: usize) -> u64 {
        self.params[..i].iter().map(|p| p.size() as u64).sum()
    }

    pub fn direct(&self, i: usize) -> Option<u32> {
        let count: usize = u32_at(&self.head, AT_DIRECT).ok()? as usize;
        if i >= count {
            return None;
        }
        let local: usize = pos_at(&self.head, AT_LOCAL).ok()? - pos_at(&self.head, AT_OVERWRITE).ok()?;
        let refs: usize = u32_at(&self.head, AT_LOCAL_NAMES).ok()? as usize + u32_at(&self.head, AT_LOCAL_ENUMS).ok()? as usize;
        u32_at(&self.mid, local + 8 * refs + 4 * i).ok()
    }

    pub fn direct_f32(&self, want: f32) -> Option<u32> {
        let count: usize = u32_at(&self.head, AT_DIRECT).ok()? as usize;
        (0..count).find(|i| self.direct(*i).is_some_and(|v| f32::from_bits(v) == want)).map(|i| i as u32)
    }

    pub fn param_at(&self, pos: u64) -> Option<&Param> {
        let mut at: u64 = 0;
        for p in &self.params {
            if at == pos {
                return Some(p);
            }
            at += p.size() as u64;
        }
        None
    }

    pub fn name(&self, pos: usize) -> Option<&str> {
        let tail: &[u8] = self.names.get(pos..)?;
        let z: usize = tail.iter().position(|c| *c == 0)?;
        std::str::from_utf8(&tail[..z]).ok()
    }

    pub fn text(&self, v: u32) -> Option<&str> {
        if v >> 24 != TEXT {
            return None;
        }
        self.name((v & 0xFF_FFFF) as usize)
    }

    pub fn find_name(&self, s: &str) -> Option<usize> {
        let mut at: usize = 0;
        while at < self.names.len() {
            let z: usize = self.names[at..].iter().position(|c| *c == 0)?;
            if &self.names[at..at + z] == s.as_bytes() {
                return Some(at);
            }
            at += z + 1;
        }
        None
    }

    pub fn add_name(&mut self, s: &str) -> Res<usize> {
        if s.is_empty() || s.as_bytes().contains(&0) {
            return Err(bad("elink: a new name must be a plain string"));
        }
        if let Some(at) = self.find_name(s) {
            return Ok(at);
        }
        let at: usize = self.names.len();
        if at + s.len() >= 0x100_0000 {
            return Err(bad("elink: the name table is full"));
        }
        self.names.extend_from_slice(s.as_bytes());
        self.names.push(0);
        Ok(at)
    }

    pub fn add_param(&mut self, p: Param) -> u64 {
        if let Some(i) = self.params.iter().position(|q| *q == p) {
            return self.param_pos(i);
        }
        let pos: u64 = self.param_pos(self.params.len());
        self.params.push(p);
        pos
    }

    pub fn user(&self, name: &str) -> Option<usize> {
        let h: u32 = hash(name);
        self.hashes.iter().position(|x| *x == h).map(|i| self.slots[i])
    }

    pub fn key(&self, u: usize, c: usize) -> Option<&str> {
        let call: &Call = self.users.get(u)?.calls.get(c)?;
        self.name(usize::try_from(call.key_pos()).ok()?)
    }

    pub fn find_key(&self, u: usize, key: &str) -> Option<usize> {
        (0..self.users.get(u)?.calls.len()).find(|c| self.key(u, *c) == Some(key))
    }

    pub fn assets(&self, u: usize) -> usize {
        self.users[u].head[3] as usize
    }

    pub fn add_call(&mut self, u: usize, key: &str, param: Param) -> Res<usize> {
        if self.find_key(u, key).is_some() {
            return Err(bad("elink: the user already has this key"));
        }
        let n: usize = self.assets(u);
        let ids: Vec<u16> = self.users[u].calls.iter().map(|c| c.asset()).filter(|a| *a != CONTAINER).collect();
        let mut want: Vec<u16> = ids.clone();
        want.sort_unstable();
        if want.len() != n || want.iter().enumerate().any(|(i, a)| *a as usize != i) || n >= CONTAINER as usize {
            return Err(bad("elink: the user's asset calls are not numbered 0 to n-1"));
        }
        let key_pos: usize = self.add_name(key)?;
        let param_pos: u64 = self.add_param(param);
        let user_name: u32 = self.users[u].hash;
        let mut guid: u32 = hash(&format!("ootrr {user_name:08x} {key}"));
        while self.users[u].calls.iter().any(|c| c.guid() == guid) {
            guid = guid.wrapping_add(1);
        }
        let idx: usize = self.users[u].calls.len();
        let call: Call = Call::new(key_pos as u64, n as u16, guid, hash(key), param_pos);
        let keys: Vec<Vec<u8>> = (0..idx).map(|c| self.key(u, c).unwrap_or("").as_bytes().to_vec()).collect();
        let user: &mut User = &mut self.users[u];
        let place: usize = user.sort.iter().position(|s| keys.get(*s as usize).is_some_and(|k| k.as_slice() > key.as_bytes())).unwrap_or(user.sort.len());
        user.calls.push(call);
        user.sort.insert(place, idx as u16);
        user.pad = vec![0; sort_len(user.sort.len()) - 2 * user.sort.len()];
        user.head[3] += 1;
        Ok(idx)
    }

    pub fn write(&self) -> Res<Vec<u8>> {
        let n: usize = self.hashes.len();
        if self.users.len() != n || self.slots.len() != n {
            return Err(bad("elink: users and hashes differ in number"));
        }
        let mut out: Vec<u8> = self.head.clone();
        for h in &self.hashes {
            out.extend_from_slice(&h.to_le_bytes());
        }
        out.extend_from_slice(&self.gap);
        let offs_at: usize = out.len();
        out.resize(offs_at + 8 * n, 0);
        out.extend_from_slice(&self.define);
        for p in &self.params {
            out.extend_from_slice(&p.mask.to_le_bytes());
            for v in &p.vals {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        let mid_at: usize = out.len();
        let local: usize = pos_at(&self.head, AT_LOCAL)? - pos_at(&self.head, AT_OVERWRITE)? + mid_at;
        out.extend_from_slice(&self.mid);
        let users_at: usize = out.len();
        let mut starts: Vec<usize> = Vec::with_capacity(n);
        for u in &self.users {
            starts.push(out.len());
            u.write(&mut out);
        }
        for (i, k) in self.slots.iter().enumerate() {
            let at: usize = *starts.get(*k).ok_or_else(|| bad("elink: a hash points at no user"))?;
            out[offs_at + 8 * i..offs_at + 8 * i + 8].copy_from_slice(&(at as u64).to_le_bytes());
        }
        let cond_at: usize = out.len();
        out.extend_from_slice(&self.cond);
        let names_at: usize = out.len();
        out.extend_from_slice(&self.names);
        while out.len() % ALIGN != 0 {
            out.push(0);
        }
        let size: u32 = u32::try_from(out.len()).map_err(|_| bad("elink: the file is too large"))?;
        let put = |out: &mut Vec<u8>, at: usize, v: u64| out[at..at + 8].copy_from_slice(&v.to_le_bytes());
        out[AT_SIZE..AT_SIZE + 4].copy_from_slice(&size.to_le_bytes());
        out[AT_VALUES..AT_VALUES + 4].copy_from_slice(&(self.values() as u32).to_le_bytes());
        out[AT_PARAMS..AT_PARAMS + 4].copy_from_slice(&(self.params.len() as u32).to_le_bytes());
        out[AT_USERS..AT_USERS + 4].copy_from_slice(&(n as u32).to_le_bytes());
        put(&mut out, AT_OVERWRITE, mid_at as u64);
        put(&mut out, AT_LOCAL, local as u64);
        put(&mut out, AT_USERS_FROM, users_at as u64);
        put(&mut out, AT_CONDITIONS, cond_at as u64);
        put(&mut out, AT_NAMES, names_at as u64);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::romfs::Romfs;

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        if p.is_dir() { Some(p) } else { None }
    }

    fn game() -> Option<Vec<u8>> {
        let dir: std::path::PathBuf = dump()?;
        let mut rf: Romfs = Romfs::open(&dir).ok()?;
        rf.read_zs(FILE).ok()
    }

    #[test]
    fn a_file_that_is_not_an_effect_link_is_refused() {
        assert!(Link::parse(b"nope").is_err());
        assert!(Link::parse(b"XLNK\x10\0\0\0\x22\0\0\0\0\0\0\0").is_err());
    }

    #[test]
    fn params_keep_their_bits_in_order() {
        let p: Param = Param::from_pairs(&[(12, 7), (0, 3), (1, 5)]).unwrap();
        assert_eq!(p.mask, 0b1_0000_0000_0011);
        assert_eq!(p.vals, vec![3, 5, 7]);
        assert_eq!((p.get(0), p.get(1), p.get(12), p.get(2)), (Some(3), Some(5), Some(7), None));
        assert_eq!(p.pairs(), vec![(0, 3), (1, 5), (12, 7)]);
        assert!(Param::from_pairs(&[(1, 1), (1, 2)]).is_err());
        assert!(Param::from_pairs(&[(64, 1)]).is_err());
    }

    #[test]
    fn the_game_effect_link_rewrites_byte_for_byte() {
        let Some(raw) = game() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let link: Link = Link::parse(&raw).unwrap();
        assert_eq!(link.users.len(), 1267);
        assert_eq!(link.write().unwrap(), raw);
        let stalfos: usize = link.user("Enemy_Stalfos").unwrap();
        assert!(link.find_key(stalfos, "デクの実有効").is_some());
        assert_eq!(link.direct_f32(30.0), Some(0x43));
        assert_eq!(link.direct_f32(1.5), Some(0x4C));
        for (u, user) in link.users.iter().enumerate() {
            let keys: Vec<&[u8]> = user.sort.iter().map(|s| link.key(u, *s as usize).unwrap().as_bytes()).collect();
            assert!(keys.windows(2).all(|w| w[0] <= w[1]), "user {u} keeps its keys sorted");
            assert!(user.calls.iter().all(|c| link.key(u, user.calls.iter().position(|d| d == c).unwrap()).is_some_and(|k| hash(k) == c.key_hash())));
        }
    }

    #[test]
    fn a_new_call_lands_in_one_user_and_leaves_the_rest_alone() {
        let Some(raw) = game() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let was: Link = Link::parse(&raw).unwrap();
        let mut link: Link = was.clone();
        let u: usize = link.user("Enemy_Keese").unwrap();
        let effect: u32 = (TEXT << 24) | link.find_name("Ply_Magic_FaroreWind_Trigger_00").unwrap() as u32;
        let key: &str = "OotrrTest";
        let key_at: u32 = (TEXT << 24) | link.add_name(key).unwrap() as u32;
        let p: Param = Param::from_pairs(&[(0, key_at), (1, effect)]).unwrap();
        let c: usize = link.add_call(u, key, p.clone()).unwrap();
        assert!(link.add_call(u, key, p.clone()).is_err(), "a key is added once");
        let out: Vec<u8> = link.write().unwrap();
        assert_eq!(out.len() % ALIGN, 0);
        let back: Link = Link::parse(&out).unwrap();
        assert_eq!(back.write().unwrap(), out);
        assert_eq!(back.find_key(u, key), Some(c));
        assert_eq!(back.users[u].calls.len(), was.users[u].calls.len() + 1);
        assert_eq!(back.assets(u), was.assets(u) + 1);
        let call: &Call = &back.users[u].calls[c];
        assert_eq!(call.asset() as usize, was.assets(u));
        assert_eq!(back.param_at(call.param_pos()), Some(&p));
        assert_eq!(back.text(back.param_at(call.param_pos()).unwrap().get(1).unwrap()), Some("Ply_Magic_FaroreWind_Trigger_00"));
        for (i, user) in back.users.iter().enumerate() {
            if i != u {
                assert_eq!(user, &was.users[i]);
            }
        }
        assert_eq!(back.users[u].triggers, was.users[u].triggers);
        assert_eq!(back.users[u].boxes, was.users[u].boxes);
        assert_eq!(&back.params[..was.params.len()], &was.params[..]);
        assert_eq!((back.mid.clone(), back.cond.clone(), back.define.clone()), (was.mid.clone(), was.cond.clone(), was.define.clone()));
        assert_eq!(&back.names[..was.names.len()], &was.names[..]);
    }
}
