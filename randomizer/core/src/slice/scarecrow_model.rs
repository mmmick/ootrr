use std::path::PathBuf as PB;

use crate::byml::{Byml, Node};
use crate::error::{Error as E, Res};
use crate::flow_locations::dict_id;
use crate::pickup_pack::model::ROW_KEYS;
use crate::pickup_pack::rows::{self, ACTOR_INFO};
use crate::romfs::Romfs;
use crate::slice::scarecrow::ACTOR;
use crate::song_models as sm;
use crate::staging::{banc_raw, rewrite, Out};
use crate::zs;

pub const SONG: &str = "ScarecrowSong";

pub fn rows_of(raw: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    let donor: Node = rows::need(&y, sm::DONOR, ACTOR_INFO)?.clone();
    if donor.get("FmdbName").and_then(|v| v.as_str()) != Some(sm::MODEL) {
        return Err(E::Format("scarecrow model: the harp row names another model, the game files differ"));
    }
    let list: &mut Vec<Node> = y.root.as_arr_mut().ok_or(E::Format("scarecrow model: the actor table is not a row list"))?;
    let r: &mut Node = list.iter_mut().find(|r| rows::id(r) == Some(ACTOR)).ok_or(E::Format("scarecrow model: the song has no actor row"))?;
    for k in ROW_KEYS {
        if let Some(v) = donor.get(k) {
            r.set(k, v.clone());
        }
    }
    y.write()
}

pub fn stage(rf: &mut Romfs, more: &mut Vec<Out>, work: Option<&PB>) -> Res<()> {
    let file: String = sm::info_of(SONG);
    more.push(rewrite(rf, &sm::pack_of(SONG), |raw| sm::repoint(raw, &file, sm::MODEL))?);
    let z0: Vec<u8> = rf.read(ACTOR_INFO)?;
    let id: Option<u32> = dict_id(&z0);
    if let Some(o) = more.iter_mut().find(|o| o.path == ACTOR_INFO) {
        let out: Vec<u8> = rows_of(&rf.zs.decompress(&o.z)?)?;
        o.z = zs::compress_raw(&out, id)?;
        o.new = out.len();
        return Ok(());
    }
    let old: usize = rf.zs.decompress(&z0)?.len();
    let out: Vec<u8> = rows_of(&banc_raw(rf, ACTOR_INFO, work)?)?;
    more.push(Out { path: ACTOR_INFO.to_string(), z: zs::compress_raw(&out, id)?, old, new: out.len() });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sarc;

    fn real() -> Option<Romfs> {
        let p: PB = std::env::var("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|_| PB::from("C:/o/nup dump/updates/131072/romfs"));
        if !p.is_dir() {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            return None;
        }
        Some(Romfs::open(&p).unwrap())
    }

    fn model_of(y: &Byml, id: &str) -> Option<String> {
        rows::need(y, id, ACTOR_INFO).ok()?.get("FmdbName")?.as_str().map(|s| s.to_string())
    }

    #[test]
    fn the_scarecrow_song_takes_the_harp_and_keeps_the_other_song_rows() {
        let Some(mut rf) = real() else {
            return;
        };
        let mut more: Vec<Out> = sm::stage(&mut rf).unwrap();
        let n: usize = more.len();
        stage(&mut rf, &mut more, None).unwrap();
        assert_eq!(more.len(), n + 1);
        assert_eq!(more.iter().filter(|o| o.path == ACTOR_INFO).count(), 1);
        let t: &Out = more.iter().find(|o| o.path == ACTOR_INFO).unwrap();
        let y: Byml = Byml::parse(&rf.zs.decompress(&t.z).unwrap()).unwrap();
        assert_eq!(model_of(&y, ACTOR).as_deref(), Some(sm::MODEL));
        assert_eq!(model_of(&y, "Item_Melody_SunSong").as_deref(), Some(sm::MODEL));
        let pk: &Out = more.iter().find(|o| o.path == sm::pack_of(SONG)).unwrap();
        let es: Vec<sarc::Entry> = sarc::parse(&rf.zs.decompress(&pk.z).unwrap()).unwrap();
        let info: Byml = Byml::parse(&sarc::get(&es, &sm::info_of(SONG)).unwrap().data).unwrap();
        assert_eq!(info.root.get("FmdbName").and_then(|v| v.as_str()), Some(sm::MODEL));
        let mut alone: Vec<Out> = Vec::new();
        stage(&mut rf, &mut alone, None).unwrap();
        let y2: Byml = Byml::parse(&rf.zs.decompress(&alone[1].z).unwrap()).unwrap();
        assert_eq!(model_of(&y2, ACTOR).as_deref(), Some(sm::MODEL));
        assert_ne!(model_of(&y2, "Item_Melody_SunSong").as_deref(), Some(sm::MODEL));
    }
}
