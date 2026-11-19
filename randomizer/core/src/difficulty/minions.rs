use std::collections::BTreeSet as Set;
use std::f32::consts::{PI, TAU};

use crate::ainb::Ainb;
use crate::byml::{Byml, Node};
use crate::difficulty::minibosses::{self, Start, GOHMA_START};
use crate::flow_locations::dict_id;
use crate::error::{Error, Res};
use crate::murmur::murmur3_32;
use crate::placement::hash_of;
use crate::qol::{edit_zs, Have};
use crate::romfs::Romfs;
use crate::staging::{Out, BANC_DICT};
use crate::zs;

pub const KEESE: &str = "Enemy_Keese";
pub const FIRE: &str = "Enemy_Keese_Fire";
pub const ICE: &str = "Enemy_Keese_Ice";
pub const BIRI: &str = "Enemy_Biri";
pub const ROOM: &str = "DefaultDungeonRoomIndex";
pub const MUSIC: &str = "BattlePresentation_OverriteBattlePresentation";
pub const QUIET: &str = "EnemyNoBattleBgm";
pub const ROAM: &str = "FlyingUser_WandarMaxRange";
pub const ROAM_RANGE: f32 = 3.0;
pub const NEAR: f32 = 1.0;
pub const DODONGO_PACK: &str = "Pack/Actor/Enemy_DungeonBoss_Dodongo.pack.zs";
pub const DODONGO_PARAM: &str = "GameParameter/DungeonBossDodongoParam/Enemy_DungeonBoss_Dodongo.game__enemy__DungeonBossDodongoParam.bgyml";
pub const STAGES: &str = "SpawnBabyDodongoParams";
pub const AFTER_ROLL: &str = "RollingAfterSpawnNum";
pub const FIRST_BABIES: i32 = 2;
pub const NOTE: &str = "Bosses have a few weak enemies with them. Keese fly around the rooms of Gohma, Phantom Ganon, Morpha and Bongo Bongo, Fire Keese around Volvagia, Fire and Ice Keese around Twinrova and Biri around Barinade. King Dodongo calls Baby Dodongos from the start of his fight and calls more of them. The Keese in Gohma's room appear when the door has shut behind you, as the boss introduction starts, and the others are there whenever you are in the boss room. They stay after the boss is beaten. Ganondorf, Ganon and the dream world boss stages are unchanged. Experimental, untested in the game.";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Room {
    pub boss: &'static str,
    pub fixed: &'static str,
    pub free: &'static str,
    pub at: [f32; 3],
    pub mid: [f32; 3],
    pub ring: f32,
    pub foes: &'static [&'static str],
    pub start: Option<Start>,
}

pub const ROOMS: [Room; 7] = [
    Room {
        boss: "Enemy_DungeonBoss_Gohma",
        fixed: "Banc/InsideTheDekuTree/InsideTheDekuTree_Static.bcett.byml.zs",
        free: "Banc/InsideTheDekuTree/InsideTheDekuTree_Dynamic.bcett.byml.zs",
        at: [18.49, 59.97, 67.79],
        mid: [20.0, 50.5, 67.8],
        ring: 5.0,
        foes: &[KEESE, KEESE, KEESE],
        start: Some(GOHMA_START),
    },
    Room {
        boss: "Enemy_DungeonBoss_Barinade",
        fixed: "Banc/InsideJabuJabuBelly/InsideJabuJabuBelly_Static.bcett.byml.zs",
        free: "Banc/InsideJabuJabuBelly/InsideJabuJabuBelly_Dynamic.bcett.byml.zs",
        at: [42.27, 100.65, -112.1],
        mid: [42.3, 103.5, -112.1],
        ring: 8.0,
        foes: &[BIRI, BIRI, BIRI],
        start: None,
    },
    Room {
        boss: "Enemy_DungeonBoss_PhantomGanon",
        fixed: "Banc/ForestTemple/ForestTemple_Static.bcett.byml.zs",
        free: "Banc/ForestTemple/ForestTemple_Dynamic.bcett.byml.zs",
        at: [-0.09, 80.91, -62.58],
        mid: [0.0, 78.2, -62.5],
        ring: 5.0,
        foes: &[KEESE, KEESE, KEESE],
        start: None,
    },
    Room {
        boss: "Enemy_DungeonBoss_Volvagia",
        fixed: "Banc/FireTemple/FireTemple_Static.bcett.byml.zs",
        free: "Banc/FireTemple/FireTemple_Dynamic.bcett.byml.zs",
        at: [-80.63, 93.33, 16.31],
        mid: [-80.2, 106.0, 0.0],
        ring: 6.0,
        foes: &[FIRE, FIRE, FIRE],
        start: None,
    },
    Room {
        boss: "Enemy_DungeonBoss_Morpha",
        fixed: "Banc/WaterTemple/WaterTemple_Static.bcett.byml.zs",
        free: "Banc/WaterTemple/WaterTemple_Dynamic.bcett.byml.zs",
        at: [-4.5, 129.26, -91.8],
        mid: [-4.5, 133.5, -91.8],
        ring: 6.0,
        foes: &[KEESE, KEESE, KEESE],
        start: None,
    },
    Room {
        boss: "Enemy_DungeonBoss_BongoBongo",
        fixed: "Banc/ShadowTemple/ShadowTemple_Static.bcett.byml.zs",
        free: "Banc/ShadowTemple/ShadowTemple_Dynamic.bcett.byml.zs",
        at: [-138.33, -11.46, 48.1],
        mid: [-138.3, -8.0, 30.1],
        ring: 5.0,
        foes: &[KEESE, KEESE, KEESE],
        start: None,
    },
    Room {
        boss: "Enemy_DungeonBoss_Twinrova",
        fixed: "Banc/SpiritTemple/SpiritTemple_Static.bcett.byml.zs",
        free: "Banc/SpiritTemple/SpiritTemple_Dynamic.bcett.byml.zs",
        at: [12.66, 144.65, -145.25],
        mid: [0.0, 146.0, -157.9],
        ring: 6.0,
        foes: &[FIRE, ICE, FIRE, ICE],
        start: None,
    },
];

fn actors(y: &Byml) -> &[Node] {
    y.root.get("Actors").and_then(|a| a.as_arr()).unwrap_or(&[])
}

fn pos(a: &Node) -> Option<[f32; 3]> {
    match a.get("Translate")?.as_arr()? {
        [Node::F32(x), Node::F32(y), Node::F32(z)] => Some([*x, *y, *z]),
        _ => None,
    }
}

pub fn room_of(y: &Byml, r: &Room) -> Res<i32> {
    let a: &Node = actors(y)
        .iter()
        .find(|a| a.get("Gyaml").and_then(|g| g.as_str()) == Some(r.boss))
        .ok_or_else(|| Error::Msg(format!("{} is not in {}, the game files differ", r.boss, r.fixed)))?;
    let p: [f32; 3] = pos(a).ok_or_else(|| Error::Msg(format!("{} has no place in {}, the game files differ", r.boss, r.fixed)))?;
    if (0..3).any(|i| (p[i] - r.at[i]).abs() > NEAR) {
        return Err(Error::Msg(format!("{} stands at {p:?} and not at {:?}, the game files differ", r.boss, r.at)));
    }
    match a.get("Dynamic").and_then(|d| d.get(ROOM)) {
        Some(Node::I32(n)) => Ok(*n),
        _ => Err(Error::Msg(format!("{} has no dungeon room in {}, the game files differ", r.boss, r.fixed))),
    }
}

pub fn hashes(y: &Byml, used: &mut Set<u64>) {
    used.extend(actors(y).iter().filter_map(hash_of));
}

pub fn fresh(seed: &str, used: &mut Set<u64>) -> u64 {
    let b: &[u8] = seed.as_bytes();
    let mut h: u64 = ((murmur3_32(b, 0x4f4f) as u64) << 32) | murmur3_32(b, 0x5452) as u64;
    while h == 0 || !used.insert(h) {
        h = h.wrapping_add(1);
    }
    h
}

pub fn spots(r: &Room) -> Vec<([f32; 3], f32)> {
    let n: usize = r.foes.len();
    (0..n)
        .map(|i| {
            let t: f32 = TAU * (i as f32 + 0.5) / n as f32;
            ([r.mid[0] + r.ring * t.sin(), r.mid[1], r.mid[2] + r.ring * t.cos()], if t > PI { t - TAU } else { t })
        })
        .collect()
}

pub fn foe(gyaml: &str, hash: u64, room: i32, at: [f32; 3], yaw: f32) -> Node {
    let mut d: Vec<(String, Node)> = vec![(MUSIC.to_string(), Node::Str(QUIET.to_string())), (ROOM.to_string(), Node::I32(room))];
    if gyaml != BIRI {
        d.push((ROAM.to_string(), Node::F32(ROAM_RANGE)));
    }
    let id: Node = Node::Map(vec![("ID".to_string(), Node::U64(hash))]);
    Node::Map(vec![
        ("Dynamic".to_string(), Node::Map(d)),
        ("Gyaml".to_string(), Node::Str(gyaml.to_string())),
        ("Hash".to_string(), Node::U64(hash)),
        ("Phive".to_string(), Node::Map(vec![("Placement".to_string(), id)])),
        ("Rotate".to_string(), Node::Arr(vec![Node::F32(0.0), Node::F32(yaw), Node::F32(0.0)])),
        ("SRTHash".to_string(), Node::U32(murmur3_32(&hash.to_le_bytes(), 0))),
        ("Translate".to_string(), Node::Arr(at.iter().map(|v| Node::F32(*v)).collect())),
    ])
}

pub fn place(y: &mut Byml, r: &Room, room: i32, used: &mut Set<u64>) -> Res<usize> {
    let list: &mut Vec<Node> = y.root.get_mut("Actors").and_then(|a| a.as_arr_mut()).ok_or_else(|| Error::Msg(format!("{} has no actor list", r.free)))?;
    let at: Vec<([f32; 3], f32)> = spots(r);
    for (i, g) in r.foes.iter().enumerate() {
        let h: u64 = fresh(&format!("ootrr/minion/{}/{i}", r.boss), used);
        list.push(foe(g, h, room, at[i].0, at[i].1));
    }
    Ok(r.foes.len())
}

pub fn babies(raw: &[u8]) -> Res<Option<Vec<u8>>> {
    let mut y: Byml = Byml::parse(raw)?;
    let list: &mut Vec<Node> = y
        .root
        .get_mut(STAGES)
        .and_then(|a| a.as_arr_mut())
        .filter(|a| !a.is_empty())
        .ok_or(Error::Format("King Dodongo has no Baby Dodongo stages, the game files differ"))?;
    for s in list.iter_mut() {
        let was: i32 = match s.get(AFTER_ROLL) {
            Some(Node::I32(n)) => *n,
            None => 0,
            Some(_) => return Err(Error::Format("King Dodongo counts his Baby Dodongos in an unknown way, the game files differ")),
        };
        let now: i32 = if was <= 0 { FIRST_BABIES } else { was + 1 };
        if !s.set(AFTER_ROLL, Node::I32(now)) {
            return Err(Error::Format("a Baby Dodongo stage of King Dodongo is not a map, the game files differ"));
        }
    }
    Ok(Some(y.write()?))
}

fn pack_of(g: &str) -> String {
    format!("Pack/Actor/{g}.pack.zs")
}

fn source(rf: &mut Romfs, have: Have, p: &str) -> Res<Vec<u8>> {
    match have(p) {
        Some(b) => Ok(b),
        None => rf.read(p),
    }
}

pub fn signalled(rf: &mut Romfs, have: Have, r: &Room, start: &Start) -> Res<Vec<Out>> {
    let z: Vec<u8> = source(rf, have, r.fixed)?;
    let dict: u32 = dict_id(&z).unwrap_or(BANC_DICT);
    let mut raw: Vec<u8> = rf.zs.decompress(&z)?;
    let mut y: Byml = Byml::parse(&raw)?;
    room_of(&y, r)?;
    let old: usize = raw.len();
    let dz: Vec<u8> = source(rf, have, r.free)?;
    raw.extend(rf.zs.decompress(&dz)?);
    let at: Vec<([f32; 3], f32)> = spots(r);
    let mut plans: Vec<minibosses::Plan> = Vec::new();
    for (i, g) in r.foes.iter().enumerate() {
        let seed: String = format!("ootrr/minion/{}/{i}", r.boss);
        let (p, a): ([f32; 3], f32) = at[i];
        let make = |hash: u64, room: i32| -> Node { foe(g, hash, room, p, a) };
        plans.push(minibosses::attach(&mut y, &raw, r.boss, r.fixed, g, &seed, &make)?);
    }
    let bytes: Vec<u8> = y.write()?;
    let banc: Out = Out { path: r.fixed.to_string(), old, new: bytes.len(), z: zs::compress_raw(&bytes, Some(dict))? };
    let logic: String = plans[0].logic.clone();
    let was: Vec<u8> = source(rf, have, &logic)?;
    let mut a: Ainb = Ainb::parse(&was)?;
    for p in &plans {
        minibosses::wire_with(&mut a, None, &p.id, Some(start))?;
    }
    let g: Vec<u8> = a.write()?;
    Ok(vec![banc, Out { path: logic, old: was.len(), new: g.len(), z: g }])
}

pub fn stage(rf: &mut Romfs, have: Have) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    for r in ROOMS.iter() {
        for g in r.foes {
            rf.read(&pack_of(g)).map_err(|_| Error::Msg(format!("the game has no {g}, the game files differ")))?;
        }
        if let Some(start) = &r.start {
            outs.extend(signalled(rf, have, r, start)?);
            continue;
        }
        let z: Vec<u8> = match have(r.fixed) {
            Some(b) => b,
            None => rf.read(r.fixed)?,
        };
        let fixed: Byml = Byml::parse(&rf.zs.decompress(&z)?)?;
        let room: i32 = room_of(&fixed, r)?;
        let mut used: Set<u64> = Set::new();
        hashes(&fixed, &mut used);
        outs.push(edit_zs(rf, have, r.free, |raw| {
            let mut y: Byml = Byml::parse(raw)?;
            hashes(&y, &mut used);
            place(&mut y, r, room, &mut used)?;
            y.write()
        })?);
    }
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boss(r: &Room, at: [f32; 3], room: Option<i32>) -> Node {
        let mut m: Vec<(String, Node)> = vec![("Gyaml".to_string(), Node::Str(r.boss.to_string())), ("Hash".to_string(), Node::U64(7))];
        if let Some(n) = room {
            m.insert(0, ("Dynamic".to_string(), Node::Map(vec![(ROOM.to_string(), Node::I32(n))])));
        }
        m.push(("Translate".to_string(), Node::Arr(at.iter().map(|v| Node::F32(*v)).collect())));
        Node::Map(m)
    }

    fn scene(list: Vec<Node>) -> Byml {
        Byml { root: Node::Map(vec![("Actors".to_string(), Node::Arr(list))]), version: 7, big: false }
    }

    #[test]
    fn the_room_comes_from_the_boss_and_a_moved_boss_is_refused() {
        let r: &Room = &ROOMS[0];
        assert_eq!(room_of(&scene(vec![boss(r, r.at, Some(10))]), r).unwrap(), 10);
        assert!(room_of(&scene(vec![boss(r, [r.at[0] + 3.0, r.at[1], r.at[2]], Some(10))]), r).is_err());
        assert!(room_of(&scene(vec![boss(r, r.at, None)]), r).is_err());
        assert!(room_of(&scene(Vec::new()), r).is_err());
    }

    #[test]
    fn the_minions_join_the_room_with_fresh_hashes() {
        let r: &Room = &ROOMS[6];
        let mut y: Byml = scene(vec![boss(r, r.at, Some(33))]);
        let mut used: Set<u64> = Set::new();
        hashes(&y, &mut used);
        let first: u64 = fresh(&format!("ootrr/minion/{}/0", r.boss), &mut Set::new());
        used.insert(first);
        assert_eq!(place(&mut y, r, 33, &mut used).unwrap(), 4);
        let list: &[Node] = actors(&y);
        assert_eq!(list.len(), 5);
        let ids: Set<u64> = list.iter().filter_map(hash_of).collect();
        assert_eq!(ids.len(), 5);
        assert!(!ids.contains(&first));
        for (i, a) in list[1..].iter().enumerate() {
            assert_eq!(a.get("Gyaml").and_then(|g| g.as_str()), Some(r.foes[i]));
            let d: &Node = a.get("Dynamic").unwrap();
            assert_eq!(d.get(ROOM), Some(&Node::I32(33)));
            assert_eq!(d.get(MUSIC).and_then(|v| v.as_str()), Some(QUIET));
            assert_eq!(d.get(ROAM), Some(&Node::F32(ROAM_RANGE)));
            assert_eq!(a.get("Phive").and_then(|p| p.get("Placement")).and_then(|p| p.get("ID")), a.get("Hash"));
            let p: [f32; 3] = pos(a).unwrap();
            let far: f32 = ((p[0] - r.mid[0]).powi(2) + (p[2] - r.mid[2]).powi(2)).sqrt();
            assert!((far - r.ring).abs() < 0.01 && p[1] == r.mid[1]);
            let Node::Map(m) = a else { panic!() };
            assert!(m.windows(2).all(|w| w[0].0 < w[1].0));
            assert!(matches!(a.get("Rotate").and_then(|v| v.as_arr()), Some([Node::F32(_), Node::F32(yaw), Node::F32(_)]) if yaw.abs() <= PI));
        }
        let back: Byml = Byml::parse(&y.write().unwrap()).unwrap();
        assert_eq!(back.root, y.root);
    }

    #[test]
    fn a_biri_gets_no_flying_range() {
        let a: Node = foe(BIRI, 5, 15, [1.0, 2.0, 3.0], 0.0);
        assert!(a.get("Dynamic").unwrap().get(ROAM).is_none());
        assert_eq!(a.get("Dynamic").unwrap().get(ROOM), Some(&Node::I32(15)));
    }

    #[test]
    fn every_room_has_its_own_files_and_a_few_minions() {
        let files: Set<&str> = ROOMS.iter().flat_map(|r| [r.fixed, r.free]).collect();
        assert_eq!(files.len(), ROOMS.len() * 2);
        for r in ROOMS.iter() {
            assert!((3..=4).contains(&r.foes.len()));
            assert!(r.fixed.ends_with("_Static.bcett.byml.zs") && r.free == r.fixed.replace("_Static", "_Dynamic"));
        }
    }

    #[test]
    fn only_gohmas_minions_wait_for_a_start_signal() {
        for r in ROOMS.iter() {
            assert_eq!(r.start.is_some(), r.boss.ends_with("Gohma"), "{}", r.boss);
        }
        assert_eq!(ROOMS[0].start, Some(GOHMA_START));
    }

    #[test]
    fn king_dodongo_calls_babies_from_the_start() {
        let st = |n: Option<i32>| {
            let mut m: Vec<(String, Node)> = vec![("HPRate".to_string(), Node::F32(1.0))];
            if let Some(v) = n {
                m.push((AFTER_ROLL.to_string(), Node::I32(v)));
            }
            Node::Map(m)
        };
        let y: Byml = Byml { root: Node::Map(vec![(STAGES.to_string(), Node::Arr(vec![st(None), st(Some(2)), st(Some(3))]))]), version: 7, big: false };
        let out: Byml = Byml::parse(&babies(&y.write().unwrap()).unwrap().unwrap()).unwrap();
        let got: Vec<Option<&Node>> = out.root.get(STAGES).unwrap().as_arr().unwrap().iter().map(|s| s.get(AFTER_ROLL)).collect();
        assert_eq!(got, vec![Some(&Node::I32(2)), Some(&Node::I32(3)), Some(&Node::I32(4))]);
        let none: Byml = Byml { root: Node::Map(vec![("Other".to_string(), Node::I32(1))]), version: 7, big: false };
        assert!(babies(&none.write().unwrap()).is_err());
    }
}
