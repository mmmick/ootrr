use std::path::PathBuf as PB;

use crate::byml::{Byml, Node};
use crate::chest_slot::LIST;
use crate::error::{Error as E, Res};
use crate::flow_locations::dict_id;
use crate::murmur::murmur3_32;
use crate::romfs::Romfs;
use crate::staging::{self, rewrite, Out};
use crate::zs;

pub const NAME: &str = "OOTRR_PierreCanAppear_Exp";
pub const SONG: &str = "IsGet.Item_Melody_ScarecrowSong";
pub const OCARINAS: [&str; 2] = ["IsGet.Item_Ocarina_Fairy", "IsGet.Item_Ocarina_Time"];
pub const PREFIX: &str = "Appear_SCARECROW_";
pub const PUSH: u64 = 0;
pub const AND: u64 = 3;
pub const OR: u64 = 4;

pub const FILES: [&str; 12] = [
    "Banc/DeathMountainCrater/DeathMountainCrater_Adult_Static.bcett.byml.zs",
    "Banc/DodongoCavern/DodongoCavern_Static.bcett.byml.zs",
    "Banc/FireTemple/FireTemple_Static.bcett.byml.zs",
    "Banc/ForestTemple/ForestTemple_Static.bcett.byml.zs",
    "Banc/GerudoFortress/GerudoFortress_Adult_Static.bcett.byml.zs",
    "Banc/GerudoTrainingGround/GerudoTrainingGround_Static.bcett.byml.zs",
    "Banc/HyruleField/HyruleField_Adult_Static.bcett.byml.zs",
    "Banc/SacredForestMeadow/SacredForestMeadow_Adult_Static.bcett.byml.zs",
    "Banc/ShadowTemple/ShadowTemple_Static.bcett.byml.zs",
    "Banc/SpiritTemple/SpiritTemple_Static.bcett.byml.zs",
    "Banc/WaterTemple/WaterTemple_Static.bcett.byml.zs",
    "Banc/ZoraFountain/ZoraFountain_Adult_Static.bcett.byml.zs",
];

fn h(n: &str) -> u64 {
    murmur3_32(n.as_bytes(), 0) as u64
}

fn tok(op: u64, flag: Option<&str>) -> Node {
    let mut v: Vec<Node> = vec![Node::U64(op)];
    if let Some(f) = flag {
        v.push(Node::U64(h(f)));
    }
    Node::Arr(v)
}

pub fn row() -> Node {
    Node::Map(vec![
        ("Hash".to_string(), Node::U32(h(NAME) as u32)),
        ("ResetTypeValue".to_string(), Node::I32(0)),
        ("SaveFileIndex".to_string(), Node::I32(-1)),
        ("Values".to_string(), Node::Arr(vec![tok(PUSH, Some(SONG)), tok(PUSH, Some(OCARINAS[0])), tok(PUSH, Some(OCARINAS[1])), tok(OR, None), tok(AND, None)])),
    ])
}

pub fn add_exp(y: &mut Byml) -> Res<bool> {
    let list: &mut Vec<Node> = y
        .root
        .get_mut("Data")
        .and_then(|d| d.get_mut("BoolExp"))
        .and_then(|b| b.as_arr_mut())
        .ok_or(E::Format("pierre: the flag list has no BoolExp table"))?;
    let me: u32 = h(NAME) as u32;
    if list.iter().any(|r| matches!(r.get("Hash"), Some(Node::U32(x)) if *x == me)) {
        return Ok(false);
    }
    list.push(row());
    Ok(true)
}

pub fn retarget(n: &mut Node) -> usize {
    match n {
        Node::Map(m) => {
            let mut c: usize = 0;
            for (k, v) in m.iter_mut() {
                if k == "Presence"
                    && let Some(Node::Str(f)) = v.get_mut("FlagName")
                    && f.starts_with(PREFIX)
                {
                    *f = NAME.to_string();
                    c += 1;
                    continue;
                }
                c += retarget(v);
            }
            c
        }
        Node::Arr(a) => a.iter_mut().map(retarget).sum(),
        _ => 0,
    }
}

fn banc_bytes(f: &str, raw: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    if retarget(&mut y.root) == 0 {
        return Err(E::Msg(format!("pierre: {f} has no scarecrow spot")));
    }
    y.write()
}

fn list_bytes(raw: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    add_exp(&mut y)?;
    y.write()
}

pub fn stage(rf: &mut Romfs, more: &mut Vec<Out>, from: Option<&PB>) -> Res<()> {
    match more.iter().position(|o| o.path == LIST) {
        Some(i) => {
            let id: Option<u32> = dict_id(&more[i].z);
            let raw: Vec<u8> = rf.zs.decompress(&more[i].z)?;
            let out: Vec<u8> = list_bytes(&raw)?;
            more[i].new = out.len();
            more[i].z = zs::compress_raw(&out, id)?;
        }
        None => more.push(rewrite(rf, LIST, list_bytes)?),
    }
    for f in FILES {
        match more.iter().position(|o| o.path == f) {
            Some(i) => {
                let raw: Vec<u8> = rf.zs.decompress(&more[i].z)?;
                let out: Vec<u8> = banc_bytes(f, &raw)?;
                let old: usize = more[i].old;
                more[i] = staging::banc_out(rf, f, old, &out)?;
            }
            None => {
                let raw: Vec<u8> = staging::banc_raw(rf, f, from)?;
                let out: Vec<u8> = banc_bytes(f, &raw)?;
                more.push(staging::banc_out(rf, f, raw.len(), &out)?);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(flag: &str, neg: bool) -> Node {
        Node::Map(vec![
            ("Gyaml".to_string(), Node::Str("Npc_Pierre_CallOcarina".to_string())),
            ("Presence".to_string(), Node::Map(vec![("FlagName".to_string(), Node::Str(flag.to_string())), ("IsNegation".to_string(), Node::Bool(neg))])),
        ])
    }

    #[test]
    fn every_scarecrow_presence_follows_the_song_and_ocarina_formula_and_others_stay() {
        let mut n: Node = Node::Arr(vec![actor("Appear_SCARECROW_LAKE_HYLIA_03", false), actor("Appear_SCARECROW_LAKE_HYLIA_03", true), actor("Some_Other_Flag", false)]);
        assert_eq!(retarget(&mut n), 2);
        let flags: Vec<&str> = n.as_arr().unwrap().iter().map(|a| a.get("Presence").and_then(|p| p.get("FlagName")).and_then(|f| f.as_str()).unwrap()).collect();
        assert_eq!(flags, [NAME, NAME, "Some_Other_Flag"]);
        assert_eq!(n.as_arr().unwrap()[1].get("Presence").and_then(|p| p.get("IsNegation")), Some(&Node::Bool(true)));
    }

    #[test]
    fn the_formula_is_song_and_any_ocarina_in_postfix() {
        let r: Node = row();
        let v: &[Node] = r.get("Values").and_then(|x| x.as_arr()).unwrap();
        let ops: Vec<u64> = v.iter().map(|t| match t.as_arr().unwrap()[0] { Node::U64(x) => x, _ => 99 }).collect();
        assert_eq!(ops, [PUSH, PUSH, PUSH, OR, AND]);
        assert_eq!(r.get("SaveFileIndex"), Some(&Node::I32(-1)));
    }

    #[test]
    fn the_real_files_hold_all_sixteen_spots_and_the_list_takes_the_formula_once() {
        let p: PB = std::env::var("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|_| PB::from("C:/o/nup dump/updates/131072/romfs"));
        if !p.is_dir() {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            return;
        }
        let mut rf: Romfs = Romfs::open(&p).unwrap();
        let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for f in FILES {
            let raw: Vec<u8> = rf.read_zs(f).unwrap();
            let t: String = String::from_utf8_lossy(&raw).to_string();
            for (i, _) in t.match_indices(PREFIX) {
                let end: usize = t[i..].find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).map(|e| i + e).unwrap_or(t.len());
                seen.insert(t[i..end].to_string());
            }
            let out: Vec<u8> = banc_bytes(f, &raw).unwrap();
            assert!(!String::from_utf8_lossy(&out).contains(PREFIX), "{f}");
        }
        assert_eq!(seen.len(), 16, "{seen:?}");
        let raw: Vec<u8> = rf.read_zs(LIST).unwrap();
        let mut y: Byml = Byml::parse(&raw).unwrap();
        let known: Vec<u32> = [SONG, OCARINAS[0], OCARINAS[1]].iter().map(|f| h(f) as u32).collect();
        let bools: &[Node] = y.root.get("Data").and_then(|d| d.get("Bool")).and_then(|b| b.as_arr()).unwrap();
        for k in &known {
            assert!(bools.iter().any(|r| matches!(r.get("Hash"), Some(Node::U32(x)) if x == k)), "{k:x}");
        }
        assert!(add_exp(&mut y).unwrap());
        assert!(!add_exp(&mut y).unwrap());
        let back: Byml = Byml::parse(&y.write().unwrap()).unwrap();
        assert_eq!(back.root, y.root);
    }
}
