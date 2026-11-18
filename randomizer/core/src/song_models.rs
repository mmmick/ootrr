use std::collections::BTreeMap as BM;

use crate::bfres;
use crate::byml::{Byml, Node};
use crate::pickup_pack::model::ROW_KEYS;
use crate::pickup_pack::rows::{self, ACTOR_INFO};
use crate::error::{Error, Res};
use crate::romfs::Romfs;
use crate::sarc;
use crate::staging::{rewrite, Out};
use crate::tex::shift::lin;
use crate::zs;

pub const MODEL: &str = "Item_Harp_Sheik";
pub const DONOR: &str = crate::pickup_pack::model::HARP;
pub const SETTING: &str = "song_harp_colors";
pub const HARP_FILE: &str = "Model/Item_Harp_Sheik.Item_Harp_Sheik.bfres.zs";
pub const NAMES: &str = "Shader/ExternalBinaryString.bfres.zs";
pub const MATERIAL: &str = "Mt_Harp_Sheik";
pub const PARAMS: [&str; 6] = ["ns_param_color0", "ns_param_color1", "ns_param_color2", "ns_param_color3", "ns_param_color4", "ns_param_color5"];
pub const SONGS: [&str; 12] = [
    "ZeldaLullaby",
    "EponaSong",
    "SariaSong",
    "SunSong",
    "TimeSong",
    "StormSong",
    "ForestMinuet",
    "FireBolero",
    "WaterSerenade",
    "SpiritRequiem",
    "ShadowNocturne",
    "LightPrelude",
];
pub const COLORS: [(&str, [u8; 3]); 12] = [
    ("ZeldaLullaby", [255, 120, 200]),
    ("TimeSong", [80, 200, 235]),
    ("SunSong", [255, 215, 60]),
    ("StormSong", [110, 100, 235]),
    ("EponaSong", [240, 95, 40]),
    ("SariaSong", [90, 205, 90]),
    ("ForestMinuet", [104, 172, 82]),
    ("FireBolero", [200, 80, 72]),
    ("WaterSerenade", [87, 120, 217]),
    ("SpiritRequiem", [201, 126, 59]),
    ("ShadowNocturne", [137, 89, 219]),
    ("LightPrelude", [208, 172, 76]),
];

/// The harp's albedo texture. Its colour parameters are white and its shader does not read them
/// (the tinted copies looked the same in the game), so each song's harp copy gets its own albedo,
/// recoloured, under a name of the same length that only swaps the middle "Sheik" for a five
/// letter song code. The name stays as long and keeps its "_Alb" end, so the name lookups of the
/// model and the texture file, which branch on the last letters, still find it.
pub const ALBEDO: &str = "Mt_Harp_Sheik_Item_Harp_Sheik_Alb";
const ALBEDO_HEAD: &str = "Mt_Harp_Sheik_Item_Harp_";
const ALBEDO_TAIL: &str = "_Alb";
pub const CODES: [(&str, &str); 12] = [
    ("ZeldaLullaby", "Zelda"),
    ("EponaSong", "Epona"),
    ("SariaSong", "Saria"),
    ("SunSong", "SunSg"),
    ("TimeSong", "TimeS"),
    ("StormSong", "Storm"),
    ("ForestMinuet", "Minut"),
    ("FireBolero", "Boler"),
    ("WaterSerenade", "Seren"),
    ("SpiritRequiem", "Requm"),
    ("ShadowNocturne", "Noctn"),
    ("LightPrelude", "Prelu"),
];

pub fn albedo_of(song: &str) -> Option<String> {
    let code: &str = CODES.iter().find(|c| c.0 == song)?.1;
    Some(format!("{ALBEDO_HEAD}{code}{ALBEDO_TAIL}"))
}

pub fn tex_file(name: &str) -> String {
    format!("Tex/{name}.bntx.zs")
}

/// Swaps every `old` for `new` (the same length) in `b`. Returns how many were swapped.
pub fn rename(b: &mut [u8], old: &str, new: &str) -> Res<usize> {
    let (o, n): (&[u8], &[u8]) = (old.as_bytes(), new.as_bytes());
    if o.len() != n.len() {
        return Err(Error::Format("song models: a texture name must keep its length"));
    }
    let mut hits: usize = 0;
    let mut i: usize = 0;
    while i + o.len() <= b.len() {
        if &b[i..i + o.len()] == o {
            b[i..i + o.len()].copy_from_slice(n);
            hits += 1;
            i += o.len();
        } else {
            i += 1;
        }
    }
    Ok(hits)
}

/// A pixel in the song's colour at the pixel's own brightness.
pub fn recolor(px: [u8; 3], to: [u8; 3]) -> [u8; 3] {
    let l: f32 = crate::tex::shift::luma(lin(px));
    let t: [f32; 3] = lin(to);
    let lt: f32 = crate::tex::shift::luma(t).max(1e-4);
    crate::tex::shift::srgb(t.map(|v| v * l / lt))
}

pub fn pack_of(song: &str) -> String {
    format!("Pack/Actor/Item_Melody_{song}.pack.zs")
}

pub fn info_of(song: &str) -> String {
    format!("Component/ModelInfo/Item_Melody_{song}.engine__component__ModelInfo.bgyml")
}

pub fn project_of(song: &str) -> String {
    format!("Item_Harp_{song}")
}

pub fn model_file(song: &str) -> String {
    format!("Model/{}.{MODEL}.bfres.zs", project_of(song))
}

pub fn color_of(song: &str) -> Option<[u8; 3]> {
    COLORS.iter().find(|c| c.0 == song).map(|c| c.1)
}

pub fn size_peer(path: &str) -> Option<String> {
    if SONGS.iter().any(|s| model_file(s) == path) {
        return Some(HARP_FILE.to_string());
    }
    SONGS.iter().any(|s| albedo_of(s).is_some_and(|a| tex_file(&a) == path)).then(|| tex_file(ALBEDO))
}

pub fn repoint(raw: &[u8], file: &str, project: &str) -> Res<Vec<u8>> {
    let mut es: Vec<sarc::Entry> = sarc::parse(raw)?;
    let e: &sarc::Entry = sarc::get(&es, file).ok_or(Error::Format("song models: the pack has no model info"))?;
    let mut y: Byml = Byml::parse(&e.data)?;
    for (k, v) in [("FmdbName", MODEL), ("ModelProjectName", project)] {
        if !y.root.set(k, Node::Str(v.to_string())) {
            return Err(Error::Format("song models: the model info lacks a model name"));
        }
    }
    sarc::set(&mut es, file, y.write()?);
    sarc::write(&es)
}

pub fn rows_of(raw: &[u8], colored: bool) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    let donor: Node = rows::need(&y, DONOR, ACTOR_INFO)?.clone();
    if donor.get("FmdbName").and_then(|v| v.as_str()) != Some(MODEL) {
        return Err(Error::Format("song models: the harp row names another model, the game files differ"));
    }
    let list: &mut Vec<Node> = y.root.as_arr_mut().ok_or(Error::Format("song models: the actor table is not a row list"))?;
    for s in SONGS {
        let id: String = format!("Item_Melody_{s}");
        let r: &mut Node = list.iter_mut().find(|r| rows::id(r) == Some(id.as_str())).ok_or(Error::Format("song models: a song has no actor row"))?;
        for k in ROW_KEYS {
            if let Some(v) = donor.get(k) {
                r.set(k, v.clone());
            }
        }
        if colored {
            r.set("ModelProjectName", Node::Str(project_of(s)));
        }
    }
    y.write()
}

pub fn shade(c: [u8; 3]) -> [f32; 3] {
    let l: [f32; 3] = lin(c);
    let top: f32 = l.iter().copied().fold(0.0, f32::max);
    if top <= 0.0 { [1.0, 1.0, 1.0] } else { l.map(|v| v / top) }
}

pub fn tint(raw: &[u8], keys: &BM<String, u64>, c: [u8; 3]) -> Res<Vec<u8>> {
    let mut b: Vec<u8> = raw.to_vec();
    let ms: Vec<bfres::Mat> = bfres::mats(&b)?;
    let v: [f32; 3] = shade(c);
    for p in PARAMS {
        let k: u64 = *keys.get(p).ok_or(Error::Format("song models: the shader name table lacks a colour parameter"))?;
        let at: usize = bfres::find(&ms, MATERIAL, k).ok_or(Error::Format("song models: the harp material lacks a colour parameter, the game files differ"))?.at;
        let old: [f32; 4] = bfres::read4(&b, at)?;
        if old[..3] != [1.0, 1.0, 1.0] {
            return Err(Error::Format("song models: a harp colour parameter is not white, the game files differ"));
        }
        bfres::write4(&mut b, at, [v[0], v[1], v[2], old[3]])?;
    }
    Ok(b)
}

pub fn harps(rf: &mut Romfs) -> Res<Vec<Out>> {
    let z: Vec<u8> = rf.read(HARP_FILE)?;
    let raw: Vec<u8> = rf.zs.decompress(&z)?;
    let keys: BM<String, u64> = bfres::keys(&rf.read_zs(NAMES)?)?;
    let tz: Vec<u8> = rf.read(&tex_file(ALBEDO))?;
    let tex: Vec<u8> = rf.zs.decompress(&tz)?;
    let mut outs: Vec<Out> = Vec::new();
    for s in SONGS {
        let c: [u8; 3] = color_of(s).ok_or(Error::Format("song models: a song has no harp colour"))?;
        let name: String = albedo_of(s).ok_or(Error::Format("song models: a song has no texture code"))?;
        let mut b: Vec<u8> = tint(&raw, &keys, c)?;
        if rename(&mut b, ALBEDO, &name)? == 0 {
            return Err(Error::Format("song models: the harp model does not name its albedo texture, the game files differ"));
        }
        outs.push(Out { path: model_file(s), old: 0, new: b.len(), z: zs::compress_raw(&b, zs::dict_of(&z))? });
        let mut t: Vec<u8> = tex.clone();
        if crate::tex::repaint::repaint(&mut t, &|px| recolor(px, c))?.changed == 0 {
            return Err(Error::Format("song models: the harp albedo could not be recoloured"));
        }
        if rename(&mut t, ALBEDO, &name)? == 0 {
            return Err(Error::Format("song models: the harp albedo file does not carry its name, the game files differ"));
        }
        outs.push(Out { path: tex_file(&name), old: 0, new: t.len(), z: zs::compress_raw(&t, zs::dict_of(&tz))? });
    }
    Ok(outs)
}

pub const OBJ_PREFIX: &str = "Obj_Melody_";
pub const SHARED_INFO: &str = "Component/ModelInfo/Item_Harp_Sheik.engine__component__ModelInfo.bgyml";

pub fn song_of_pack(name: &str) -> Option<&'static str> {
    let rest: &str = name.strip_prefix(OBJ_PREFIX)?;
    let s: &str = rest.split('_').next()?;
    SONGS.iter().copied().find(|x| *x == s)
}

pub fn actor_of(name: &str) -> String {
    format!("Actor/{name}.engine__actor__ActorParam.bgyml")
}

pub fn color_pack(raw: &[u8], name: &str, song: &str, info: &[u8]) -> Res<Vec<u8>> {
    let mut es: Vec<sarc::Entry> = sarc::parse(raw)?;
    let a: String = actor_of(name);
    let e: &sarc::Entry = sarc::get(&es, &a).ok_or_else(|| Error::Msg(format!("song models: {name} has no actor parameter file")))?;
    let mut y: Byml = Byml::parse(&e.data)?;
    let c: &mut Node = y.root.get_mut("Components").ok_or_else(|| Error::Msg(format!("song models: {name} has no components")))?;
    let was: String = c.get("ModelInfoRef").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if was != format!("?{SHARED_INFO}") {
        return Err(Error::Msg(format!("song models: {name} does not show Sheik's harp, the pack differs")));
    }
    c.set("ModelInfoRef", Node::Str(format!("?{}", info_of(song))));
    sarc::set(&mut es, &a, y.write()?);
    sarc::set(&mut es, &info_of(song), info.to_vec());
    sarc::write(&es)
}

pub fn color_row(raw: &[u8], name: &str, song: &str) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    let list: &mut Vec<Node> = y.root.as_arr_mut().ok_or(Error::Format("song models: the actor table is not a row list"))?;
    let r: &mut Node = list.iter_mut().find(|r| rows::id(r) == Some(name)).ok_or_else(|| Error::Msg(format!("song models: {name} has no actor row")))?;
    if r.get("FmdbName").and_then(|v| v.as_str()) != Some(MODEL) {
        return Err(Error::Msg(format!("song models: the actor row of {name} names another model")));
    }
    r.set("ModelProjectName", Node::Str(project_of(song)));
    y.write()
}

pub fn color_packs(rf: &mut Romfs, base: &std::path::Path, names: &[String]) -> Res<Vec<String>> {
    let mut done: Vec<String> = Vec::new();
    let tp: std::path::PathBuf = base.join(crate::rsizetable::TABLE_PATH);
    let mut rst: crate::rsizetable::Rst = crate::rsizetable::Rst::parse(&rf.zs.decompress(&std::fs::read(&tp)?)?)?;
    let ap: std::path::PathBuf = base.join(ACTOR_INFO);
    let az: Vec<u8> = std::fs::read(&ap)?;
    let mut table: Vec<u8> = rf.zs.decompress(&az)?;
    for n in names {
        let Some(song) = song_of_pack(n) else {
            continue;
        };
        let item: Vec<u8> = rf.zs.decompress(&std::fs::read(base.join(pack_of(song)))?)?;
        let info: Vec<u8> = sarc::get(&sarc::parse(&item)?, &info_of(song)).ok_or(Error::Format("song models: the song pack lost its model info"))?.data.clone();
        let pp: std::path::PathBuf = base.join(crate::pickup_pack::pack_of(n));
        let z: Vec<u8> = std::fs::read(&pp)?;
        let out: Vec<u8> = color_pack(&rf.zs.decompress(&z)?, n, song, &info)?;
        rst.set_size(&crate::pickup_pack::pack_of(n), crate::pickup_pack::sizes::pack_size(out.len())?)?;
        std::fs::write(&pp, zs::compress_raw(&out, zs::dict_of(&z))?)?;
        table = color_row(&table, n, song)?;
        done.push(n.clone());
    }
    if !done.is_empty() {
        std::fs::write(&ap, zs::compress_raw(&table, zs::dict_of(&az))?)?;
        std::fs::write(&tp, rst.to_zs()?)?;
    }
    Ok(done)
}

pub fn stage(rf: &mut Romfs, colored: bool) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    for s in SONGS {
        let file: String = info_of(s);
        let project: String = if colored { project_of(s) } else { MODEL.to_string() };
        outs.push(rewrite(rf, &pack_of(s), |raw| repoint(raw, &file, &project))?);
    }
    outs.push(rewrite(rf, ACTOR_INFO, |raw| rows_of(raw, colored))?);
    if colored {
        outs.extend(harps(rf)?);
    }
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(m: &str) -> Vec<u8> {
        let y: Byml = Byml {
            root: Node::Map(vec![
                ("FmdbName".to_string(), Node::Str(m.to_string())),
                ("ModelProjectName".to_string(), Node::Str(m.to_string())),
            ]),
            version: 2,
            big: false,
        };
        y.write().unwrap()
    }

    #[test]
    fn the_song_rows_take_the_harp_model_fields() {
        let mut list: Vec<Node> = Vec::new();
        let mut harp: Node = Node::Map(Vec::new());
        for k in ROW_KEYS {
            harp.set(k, Node::Str(format!("harp {k}")));
        }
        harp.set("FmdbName", Node::Str(MODEL.to_string()));
        harp.set(rows::ROW_ID, Node::Str(DONOR.to_string()));
        list.push(harp);
        for s in SONGS {
            let mut r: Node = Node::Map(Vec::new());
            r.set("FmdbName", Node::Str("Obj_Rupee".to_string()));
            r.set("ActorName", Node::Str(format!("Item_Melody_{s}")));
            r.set(rows::ROW_ID, Node::Str(format!("Item_Melody_{s}")));
            list.push(r);
        }
        let raw: Vec<u8> = Byml { root: Node::Arr(list), version: 7, big: false }.write().unwrap();
        let y: Byml = Byml::parse(&rows_of(&raw, false).unwrap()).unwrap();
        let r: &Node = rows::need(&y, "Item_Melody_SunSong", ACTOR_INFO).unwrap();
        assert_eq!(r.get("FmdbName").and_then(|v| v.as_str()), Some(MODEL));
        assert_eq!(r.get("ModelAabbMax").and_then(|v| v.as_str()), Some("harp ModelAabbMax"));
        assert_eq!(r.get("ActorName").and_then(|v| v.as_str()), Some("Item_Melody_SunSong"));
        assert!(rows_of(&Byml { root: Node::Arr(Vec::new()), version: 7, big: false }.write().unwrap(), false).is_err());
        let c: Byml = Byml::parse(&rows_of(&raw, true).unwrap()).unwrap();
        let r: &Node = rows::need(&c, "Item_Melody_SunSong", ACTOR_INFO).unwrap();
        assert_eq!(r.get("FmdbName").and_then(|v| v.as_str()), Some(MODEL));
        assert_eq!(r.get("ModelProjectName").and_then(|v| v.as_str()), Some("Item_Harp_SunSong"));
    }

    #[test]
    fn the_model_names_change_and_the_rest_of_the_pack_stays() {
        let f: String = info_of("SariaSong");
        let es: Vec<sarc::Entry> = vec![
            sarc::Entry { name: f.clone(), data: info("Obj_Rupee") },
            sarc::Entry { name: "Other/File.bin".to_string(), data: vec![1, 2, 3] },
        ];
        let raw: Vec<u8> = sarc::write(&es).unwrap();
        let out: Vec<u8> = repoint(&raw, &f, "Item_Harp_SariaSong").unwrap();
        let back: Vec<sarc::Entry> = sarc::parse(&out).unwrap();
        let y: Byml = Byml::parse(&sarc::get(&back, &f).unwrap().data).unwrap();
        assert_eq!(y.root.get("FmdbName").and_then(|v| v.as_str()), Some(MODEL));
        assert_eq!(y.root.get("ModelProjectName").and_then(|v| v.as_str()), Some("Item_Harp_SariaSong"));
        assert_eq!(sarc::get(&back, "Other/File.bin").unwrap().data, vec![1, 2, 3]);
        assert!(repoint(&raw, "missing", MODEL).is_err());
    }

    #[test]
    fn a_song_pickup_pack_takes_its_own_model_info_and_row() {
        assert_eq!(song_of_pack("Obj_Melody_SunSong"), Some("SunSong"));
        assert_eq!(song_of_pack("Obj_Melody_SariaSong_GH"), Some("SariaSong"));
        assert_eq!(song_of_pack("Obj_Boomerang"), None);
        assert_eq!(song_of_pack("Obj_Melody_Nothing"), None);
        let actor: Byml = Byml {
            root: Node::Map(vec![("Components".to_string(), Node::Map(vec![("ModelInfoRef".to_string(), Node::Str(format!("?{SHARED_INFO}")))]))]),
            version: 2,
            big: false,
        };
        let es: Vec<sarc::Entry> = vec![
            sarc::Entry { name: actor_of("Obj_Melody_SunSong"), data: actor.write().unwrap() },
            sarc::Entry { name: SHARED_INFO.to_string(), data: info(MODEL) },
        ];
        let raw: Vec<u8> = sarc::write(&es).unwrap();
        let out: Vec<u8> = color_pack(&raw, "Obj_Melody_SunSong", "SunSong", &info("Item_Harp_SunSong")).unwrap();
        let back: Vec<sarc::Entry> = sarc::parse(&out).unwrap();
        let a: Byml = Byml::parse(&sarc::get(&back, &actor_of("Obj_Melody_SunSong")).unwrap().data).unwrap();
        assert_eq!(a.root.get("Components").and_then(|c| c.get("ModelInfoRef")).and_then(|v| v.as_str()), Some(format!("?{}", info_of("SunSong")).as_str()));
        assert_eq!(sarc::get(&back, &info_of("SunSong")).unwrap().data, info("Item_Harp_SunSong"));
        assert!(color_pack(&out, "Obj_Melody_SunSong", "SunSong", &info("x")).is_err());
        let rows_raw: Vec<u8> = Byml {
            root: Node::Arr(vec![Node::Map(vec![
                (rows::ROW_ID.to_string(), Node::Str("Obj_Melody_SunSong".to_string())),
                ("FmdbName".to_string(), Node::Str(MODEL.to_string())),
                ("ModelProjectName".to_string(), Node::Str(MODEL.to_string())),
            ])]),
            version: 7,
            big: false,
        }
        .write()
        .unwrap();
        let t: Byml = Byml::parse(&color_row(&rows_raw, "Obj_Melody_SunSong", "SunSong").unwrap()).unwrap();
        assert_eq!(rows::need(&t, "Obj_Melody_SunSong", ACTOR_INFO).unwrap().get("ModelProjectName").and_then(|v| v.as_str()), Some("Item_Harp_SunSong"));
        assert!(color_row(&rows_raw, "Obj_Melody_Other", "SunSong").is_err());
    }

    #[test]
    fn every_song_has_one_colour_and_its_own_model_file() {
        for s in SONGS {
            assert_eq!(COLORS.iter().filter(|c| c.0 == s).count(), 1, "{s}");
            assert_eq!(size_peer(&model_file(s)).as_deref(), Some(HARP_FILE));
            let t: String = albedo_of(s).unwrap();
            assert_eq!(t.len(), ALBEDO.len(), "{s}");
            assert!(t.ends_with("_Alb") && t != ALBEDO, "{s}");
            assert_eq!(size_peer(&tex_file(&t)), Some(tex_file(ALBEDO)));
        }
        let names: std::collections::BTreeSet<String> = SONGS.iter().filter_map(|s| albedo_of(s)).collect();
        assert_eq!(names.len(), SONGS.len());
        let mut b: Vec<u8> = format!("x{ALBEDO}y{ALBEDO}").into_bytes();
        assert_eq!(rename(&mut b, ALBEDO, &albedo_of("SunSong").unwrap()).unwrap(), 2);
        assert!(rename(&mut b, ALBEDO, "short").is_err());
        let r: [u8; 3] = recolor([128, 128, 128], [255, 0, 0]);
        assert!(r[0] > 0 && r[1] == 0 && r[2] == 0);
        assert_eq!(COLORS.len(), SONGS.len());
        assert_eq!(model_file("SunSong"), "Model/Item_Harp_SunSong.Item_Harp_Sheik.bfres.zs");
        assert_eq!(size_peer(HARP_FILE), None);
        let v: [f32; 3] = shade([255, 120, 200]);
        assert_eq!(v[0], 1.0);
        assert!(v[1] < v[2] && v[2] < 1.0);
    }

    #[test]
    fn the_real_harp_takes_each_song_colour_and_nothing_else() {
        let root = ootrr_game::test_romfs::path();
        if !root.join("Pack").join("ZsDic.pack.zs").is_file() {
            eprintln!("skipped, no RomFS dump at {}", root.display());
            return;
        }
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read_zs(HARP_FILE).unwrap();
        let keys: BM<String, u64> = bfres::keys(&rf.read_zs(NAMES).unwrap()).unwrap();
        let outs: Vec<Out> = harps(&mut rf).unwrap();
        // Each song writes its harp model and its own albedo texture.
        assert_eq!(outs.len(), 2 * SONGS.len());
        let tex: Vec<u8> = rf.read_zs(&tex_file(ALBEDO)).unwrap();
        for (s, pair) in SONGS.iter().zip(outs.chunks(2)) {
            let (o, t): (&Out, &Out) = (&pair[0], &pair[1]);
            assert_eq!(o.path, model_file(s));
            let name: String = albedo_of(s).unwrap();
            assert_eq!(t.path, tex_file(&name));
            let tb: Vec<u8> = rf.zs.decompress(&t.z).unwrap();
            assert_eq!(tb.len(), tex.len(), "{s}");
            assert_eq!(crate::tex::bntx::parse(&tb).unwrap()[0].name, name, "{s}");
            let b: Vec<u8> = rf.zs.decompress(&o.z).unwrap();
            assert_eq!(b.len(), raw.len());
            let ms: Vec<bfres::Mat> = bfres::mats(&b).unwrap();
            let want: [f32; 3] = shade(color_of(s).unwrap());
            let mut at: Vec<usize> = Vec::new();
            for p in PARAMS {
                let x: usize = bfres::find(&ms, MATERIAL, keys[p]).unwrap().at;
                let v: [f32; 4] = bfres::read4(&b, x).unwrap();
                assert_eq!([v[0], v[1], v[2]], want, "{s} {p}");
                at.push(x);
            }
            let names: Vec<usize> = (0..b.len().saturating_sub(name.len())).filter(|&i| b[i..i + name.len()] == *name.as_bytes()).collect();
            assert!(!names.is_empty(), "{s} names its albedo");
            let changed: Vec<usize> = (0..b.len()).filter(|&i| b[i] != raw[i]).collect();
            assert!(changed.iter().all(|i| at.iter().any(|x| *i >= *x && *i < *x + 12) || names.iter().any(|x| *i >= *x && *i < *x + name.len())), "{s}");
        }
    }
}
