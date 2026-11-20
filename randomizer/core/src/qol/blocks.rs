use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::qol::{edit_zs, Have};
use crate::romfs::Romfs;
use crate::sarc;
use crate::staging::Out;

pub const SETTING: &str = "faster_blocks";
pub const PARAM: &str = "GameParameter/GrabMoveActorParam/Obj_MovableBox_A_0";
pub const CURVE: &str = "MoveSpeed";
pub const DATA: &str = "Data";
pub const MAX_X: &str = "MaxX";
pub const KIND: &str = "Type";
pub const SPLINE: &str = "Hermit2DSplit";
pub const KNOT: usize = 4;
pub const FACTOR: f32 = 2.0;

pub const PACKS: [&str; 13] = [
    "Obj_MovableBox_A_01",
    "Obj_MovableBox_A_04",
    "Obj_MovableBox_A_05",
    "Obj_FireTemple_MovableBox_A_01",
    "Obj_FireTemple_MovableBox_A_02",
    "Obj_ForestTemple_MovableBox_A_01",
    "Obj_ForestTemple_MovableBox_A_02",
    "Obj_ForestTemple_MovableBox_A_03",
    "Obj_ForestTemple_MovableBox_A_04",
    "Obj_ForestTemple_MovableBox_A_05",
    "Obj_LonLonRanch_MovableBox_A_01",
    "Obj_SpiritTemple_MovableBox_A_01",
    "ForestTemple_MovableBox_Puzzle_Manager",
];

pub const NOTE: &str = "Pushable and pullable blocks slide twice as fast. Blocks still stop on the same spots.";

pub fn pack_of(name: &str) -> String {
    format!("Pack/Actor/{name}.pack.zs")
}

pub fn files() -> Vec<String> {
    PACKS.iter().map(|p| pack_of(p)).collect()
}

fn floats(a: &[Node]) -> Res<Vec<f32>> {
    a.iter()
        .map(|n| match n {
            Node::F32(x) => Ok(*x),
            _ => Err(Error::Format("faster blocks: a block speed curve holds something other than floats")),
        })
        .collect()
}

pub fn quicken(root: &mut Node) -> Res<bool> {
    let Some(c) = root.get_mut(CURVE) else {
        return Ok(false);
    };
    if c.get(KIND).and_then(|k| k.as_str()) != Some(SPLINE) {
        return Err(Error::Format("faster blocks: a block speed curve is not a Hermite spline"));
    }
    let data: Vec<f32> = match c.get(DATA).and_then(|d| d.as_arr()) {
        Some(a) => floats(a)?,
        None => return Err(Error::Format("faster blocks: a block speed curve has no data")),
    };
    if data.is_empty() || data.len() % KNOT != 0 {
        return Err(Error::Format("faster blocks: a block speed curve does not hold whole knots"));
    }
    let scaled: Vec<Node> = data
        .iter()
        .enumerate()
        .map(|(i, v)| Node::F32(if i % KNOT == 0 { v / FACTOR } else if i % KNOT >= 2 { v * FACTOR } else { *v }))
        .collect();
    c.set(DATA, Node::Arr(scaled));
    if let Some(Node::F32(m)) = c.get(MAX_X).cloned() {
        c.set(MAX_X, Node::F32(m / FACTOR));
    }
    Ok(true)
}

pub fn faster(raw: &[u8]) -> Res<(Vec<u8>, usize)> {
    let mut es: Vec<sarc::Entry> = sarc::parse(raw)?;
    let mut n: usize = 0;
    for e in es.iter_mut().filter(|e| e.name.starts_with(PARAM)) {
        let mut y: Byml = Byml::parse(&e.data)?;
        if quicken(&mut y.root)? {
            e.data = y.write()?;
            n += 1;
        }
    }
    Ok((sarc::write(&es)?, n))
}

pub fn stage(rf: &mut Romfs, have: Have) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    for p in PACKS {
        let mut n: usize = 0;
        let o: Out = edit_zs(rf, have, &pack_of(p), |raw| {
            let (b, c): (Vec<u8>, usize) = faster(raw)?;
            n = c;
            Ok(b)
        })?;
        if n == 0 {
            return Err(Error::Msg(format!("faster blocks: {p} has no block speed curve, the game files may have changed")));
        }
        outs.push(o);
    }
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn curve(data: &[f32], max: f32, kind: &str) -> Node {
        Node::Map(vec![(
            CURVE.to_string(),
            Node::Map(vec![
                (DATA.to_string(), Node::Arr(data.iter().map(|v| Node::F32(*v)).collect())),
                (MAX_X.to_string(), Node::F32(max)),
                (KIND.to_string(), Node::Str(kind.to_string())),
            ]),
        )])
    }

    fn data_of(n: &Node) -> Vec<f32> {
        floats(n.get(CURVE).unwrap().get(DATA).unwrap().as_arr().unwrap()).unwrap()
    }

    const VANILLA: [f32; 12] = [0.0, 0.0, 0.0, 0.0, 8.0, 0.0, 0.0, 0.0, 18.0, 0.5, 0.25, 0.125];

    #[test]
    fn frames_halve_slopes_double_and_progress_stays() {
        let mut n: Node = curve(&VANILLA, 44.0, SPLINE);
        assert!(quicken(&mut n).unwrap());
        assert_eq!(data_of(&n), vec![0.0, 0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 0.0, 9.0, 0.5, 0.5, 0.25]);
        assert_eq!(n.get(CURVE).unwrap().get(MAX_X), Some(&Node::F32(22.0)));
        assert_eq!(n.get(CURVE).unwrap().get(KIND).and_then(|k| k.as_str()), Some(SPLINE));
    }

    #[test]
    fn other_shapes_are_left_alone_or_refused() {
        let mut none: Node = Node::Map(vec![("Other".to_string(), Node::F32(1.0))]);
        assert!(!quicken(&mut none).unwrap());
        assert!(quicken(&mut curve(&VANILLA, 44.0, "Linear")).is_err());
        assert!(quicken(&mut curve(&VANILLA[..10], 44.0, SPLINE)).is_err());
        assert!(quicken(&mut curve(&[], 44.0, SPLINE)).is_err());
    }

    #[test]
    fn only_block_curves_in_a_pack_change() {
        let y: Byml = Byml { root: curve(&VANILLA, 44.0, SPLINE), version: 2, big: false };
        let other: Vec<u8> = vec![1, 2, 3, 4];
        let es: Vec<sarc::Entry> = vec![
            sarc::Entry { name: format!("{PARAM}1.game__object__GrabMoveActorParam.bgyml"), data: y.write().unwrap() },
            sarc::Entry { name: "GameParameter/GrabMoveActorParam/Obj_Tombstone_A_01.bgyml".to_string(), data: y.write().unwrap() },
            sarc::Entry { name: "Other/File.bin".to_string(), data: other.clone() },
        ];
        let (out, n): (Vec<u8>, usize) = faster(&sarc::write(&es).unwrap()).unwrap();
        assert_eq!(n, 1);
        let back: Vec<sarc::Entry> = sarc::parse(&out).unwrap();
        let read = |name: &str| -> Vec<f32> { data_of(&Byml::parse(&sarc::get(&back, name).unwrap().data).unwrap().root) };
        assert_eq!(read(&format!("{PARAM}1.game__object__GrabMoveActorParam.bgyml"))[4], 4.0);
        assert_eq!(read("GameParameter/GrabMoveActorParam/Obj_Tombstone_A_01.bgyml"), VANILLA.to_vec());
        assert_eq!(sarc::get(&back, "Other/File.bin").unwrap().data, other);
    }

    #[test]
    fn the_pack_list_is_unique_and_leaves_statues_and_tombstones_alone() {
        let mut v: Vec<&str> = PACKS.to_vec();
        v.sort();
        v.dedup();
        assert_eq!(v.len(), PACKS.len());
        assert!(PACKS.iter().all(|p| p.contains("MovableBox")));
        assert_eq!(files().len(), PACKS.len());
    }

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = ootrr_game::test_romfs::path();
        if p.is_dir() {
            Some(p)
        } else {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            None
        }
    }

    #[test]
    fn the_game_packs_change_only_their_block_curves_and_double_them() {
        let Some(root) = dump() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let outs: Vec<Out> = stage(&mut rf, &|_: &str| None).unwrap();
        assert_eq!(outs.len(), PACKS.len());
        let mut curves: usize = 0;
        for o in &outs {
            let old: Vec<sarc::Entry> = sarc::parse(&rf.read_zs(&o.path).unwrap()).unwrap();
            let new: Vec<sarc::Entry> = sarc::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap();
            assert_eq!(old.len(), new.len(), "{}", o.path);
            for (a, b) in old.iter().zip(&new) {
                assert_eq!(a.name, b.name);
                if !a.name.starts_with(PARAM) {
                    assert_eq!(a.data, b.data, "{} in {}", a.name, o.path);
                    continue;
                }
                let (x, y): (Byml, Byml) = (Byml::parse(&a.data).unwrap(), Byml::parse(&b.data).unwrap());
                let (da, db): (Vec<f32>, Vec<f32>) = (data_of(&x.root), data_of(&y.root));
                assert_eq!(da.len(), db.len());
                for (i, (p, q)) in da.iter().zip(&db).enumerate() {
                    let want: f32 = if i % KNOT == 0 { p / 2.0 } else if i % KNOT >= 2 { p * 2.0 } else { *p };
                    assert_eq!(*q, want, "{} knot value {i}", a.name);
                }
                let (ma, mb): (Option<&Node>, Option<&Node>) = (x.root.get(CURVE).unwrap().get(MAX_X), y.root.get(CURVE).unwrap().get(MAX_X));
                assert_eq!(mb, Some(&Node::F32(match ma { Some(Node::F32(m)) => m / 2.0, _ => panic!("no MaxX in {}", a.name) })));
                for k in da.chunks(KNOT).zip(db.chunks(KNOT)).collect::<Vec<_>>().windows(2) {
                    assert!(k[0].1[0] < k[1].1[0], "frames must still increase in {}", a.name);
                }
                curves += 1;
            }
        }
        assert!(curves >= PACKS.len(), "{curves}");
    }

    #[test]
    fn the_setting_off_writes_nothing_and_on_writes_exactly_the_block_packs() {
        let Some(root) = dump() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let none: Vec<Out> = crate::qol::stage(&mut rf, &crate::qol::Opts::default(), &|_: &str| None).unwrap();
        assert!(none.iter().all(|o| !files().contains(&o.path)));
        let on: crate::qol::Opts = crate::qol::Opts { blocks: true, ..crate::qol::Opts::default() };
        let outs: Vec<Out> = crate::qol::stage(&mut rf, &on, &|_: &str| None).unwrap();
        let mut got: Vec<String> = outs.iter().map(|o| o.path.clone()).collect();
        got.sort();
        let mut want: Vec<String> = files();
        want.sort();
        assert_eq!(got, want);
    }

    #[test]
    fn the_setting_turns_the_option_on_and_defaults_off() {
        use crate::settings::{Settings, Value};
        let mut bad: Vec<String> = Vec::new();
        assert!(!crate::qol::read(&Settings::default(), &mut bad).blocks);
        let mut s: Settings = Settings::default();
        s.set(SETTING, Value::Bool(true)).unwrap();
        let o: crate::qol::Opts = crate::qol::read(&s, &mut bad);
        assert!(bad.is_empty() && o.blocks && o.any());
        assert!(o.notes().contains(&NOTE.to_string()));
    }
}
