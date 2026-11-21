use crate::ainb::build::{Spec, QUERY};
use crate::ainb::{audit, Ainb};
use crate::byml::{Byml, Node};
use crate::difficulty::ambush::{actor, euler, floats, fresh, link, list, text, vec3};
use crate::difficulty::gates::{id_for, AND_NODE, AREA, AREA_NODE, BOOL, CREATE, DELETE, GRAPH as SOURCE, HIT_NODE, INT, NAME, POINTER, RESPAWN, SHARED, STR};
use crate::error::{Error as E, Res};
use crate::flow_locations::dict_id;
use crate::murmur::murmur3_32;
use crate::placement::hash_of;
use crate::qol::Have;
use crate::romfs::Romfs;
use crate::staging::{Out, BANC_DICT};
use crate::zs;

pub const MANAGER: &str = "Obj_FallingRockManager_A_04";
pub const NODE: &str = "ActorLogicFallingRockManager";
pub const ANCHOR: &str = "DestinationAnchor";
pub const PREFIX: &str = "Hazard_FireballStorm_";
pub const SEED: &str = "ootrr fireball storms";
pub const LIFT: f32 = 0.5;

pub const NOTE: &str = "Burning rocks rain down, as on the Death Mountain Trail, while you stand in three quiet stretches of Hyrule Field (child and adult) or the Hyrule Castle garden (child), and stop when you leave. A rock hits as hard as on the mountain.";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub name: &'static str,
    pub dir: &'static str,
    pub maps: &'static [&'static str],
    pub area: [f32; 3],
    pub size: [f32; 3],
    pub manager: [f32; 3],
    pub anchors: &'static [[f32; 3]],
    pub interval: f32,
    pub fall: f32,
}

pub const FIELD: &[&str] = &["HyruleField_Child", "HyruleField_Adult"];
pub const CASTLE: &[&str] = &["HyruleCastle"];

pub const SPOTS: [Spot; 4] = [
    Spot {
        name: "field_middle_road",
        dir: "HyruleField",
        maps: FIELD,
        area: [75.0, 152.9, 95.0],
        size: [32.0, 12.0, 30.0],
        manager: [170.0, 216.0, 60.0],
        anchors: &[[75.0, 149.4, 95.0], [85.0, 149.9, 100.0], [65.0, 149.7, 100.0], [80.0, 149.1, 85.0], [70.0, 150.2, 105.0], [85.0, 149.3, 90.0]],
        interval: 45.0,
        fall: 3.5,
    },
    Spot {
        name: "field_south_central",
        dir: "HyruleField",
        maps: FIELD,
        area: [55.0, 163.0, 190.0],
        size: [32.0, 12.0, 30.0],
        manager: [150.0, 226.0, 160.0],
        anchors: &[[55.0, 159.5, 190.0], [65.0, 158.3, 195.0], [45.0, 161.1, 195.0], [60.0, 158.1, 180.0], [50.0, 160.2, 200.0], [65.0, 157.7, 185.0]],
        interval: 45.0,
        fall: 3.5,
    },
    Spot {
        name: "field_west_road",
        dir: "HyruleField",
        maps: FIELD,
        area: [-170.0, 137.1, 170.0],
        size: [32.0, 12.0, 30.0],
        manager: [-75.0, 200.0, 135.0],
        anchors: &[[-170.0, 133.6, 170.0], [-160.0, 133.0, 175.0], [-180.0, 132.7, 175.0], [-165.0, 134.3, 160.0], [-175.0, 132.7, 180.0], [-160.0, 133.5, 165.0]],
        interval: 45.0,
        fall: 3.5,
    },
    Spot {
        name: "castle_front_garden",
        dir: "HyruleCastle",
        maps: CASTLE,
        area: [14.0, 118.0, -76.0],
        size: [44.0, 10.0, 26.0],
        manager: [14.0, 186.0, -165.0],
        anchors: &[[25.0, 114.5, -74.0], [28.0, 114.5, -80.0], [4.0, 114.5, -71.0], [25.0, 114.5, -89.0], [-14.0, 114.5, -86.0]],
        interval: 60.0,
        fall: 3.5,
    },
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub module: String,
    pub graph: String,
    pub manager: String,
    pub area: String,
}

impl Spot {
    pub fn file(&self, map: &str, kind: &str) -> String {
        format!("Banc/{}/{map}_{kind}.bcett.byml.zs", self.dir)
    }

    pub fn plan(&self) -> Plan {
        let manager: String = id_for(MANAGER, &format!("{SEED} {} manager", self.name), &[]);
        let area: String = id_for(AREA, &format!("{SEED} {} area", self.name), &[manager.clone()]);
        let tag: u32 = murmur3_32(format!("{SEED} {} module", self.name).as_bytes(), 0) & 0xffff;
        let name: String = format!("{PREFIX}{}_{tag:04x}", self.name);
        Plan { module: format!("Logic/Module/{PREFIX}{}/{name}.logic.module.ain", self.name), graph: format!("Logic/{name}.logic.module.ainb"), manager, area }
    }
}

pub fn files() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in SPOTS.iter() {
        for m in s.maps {
            let f: String = s.file(m, "Static");
            if !out.contains(&f) {
                out.push(f);
            }
        }
    }
    out
}

fn reference(src: u64, dst: u64) -> Node {
    let mut l: Node = Node::Map(Vec::new());
    l.set("Dst", Node::U64(dst));
    l.set("Gyaml", Node::Str("Reference".to_string()));
    l.set("Name", Node::Str("Reference".to_string()));
    l.set("Src", Node::U64(src));
    l
}

pub fn graph(raw: &[u8], s: &Spot) -> Res<Out> {
    let p: Plan = s.plan();
    let mut a: Ainb = Ainb::parse(raw)?;
    for id in [&p.manager, &p.area] {
        if (0..a.els.len()).any(|i| a.prop_str(i, NAME) == Some(id.as_str())) {
            return Err(E::Msg(format!("fireball storms: the graph already has a node named {id}")));
        }
    }
    let zero: Vec<u8> = 0u32.to_le_bytes().to_vec();
    let one: Vec<u8> = 1u32.to_le_bytes().to_vec();
    let area_name: Vec<u8> = a.pool.add(&p.area).to_le_bytes().to_vec();
    let manager_name: Vec<u8> = a.pool.add(&p.manager).to_le_bytes().to_vec();
    let empty: Vec<u8> = a.pool.add("").to_le_bytes().to_vec();
    let player: Vec<u8> = a.pool.add("Player").to_le_bytes().to_vec();
    let seed_a: String = format!("{SEED} {} area node", s.name);
    let seed_h: String = format!("{SEED} {} hit", s.name);
    let seed_i: String = format!("{SEED} {} inside", s.name);
    let seed_m: String = format!("{SEED} {} manager node", s.name);
    let mut area: Spec = Spec::node(AREA_NODE, &seed_a)
        .prop(STR, "AddLayer", empty)
        .prop(STR, NAME, area_name)
        .prop(STR, "Layer", player.clone())
        .idle(BOOL, CREATE, one.clone())
        .idle(BOOL, DELETE, zero.clone())
        .idle(BOOL, RESPAWN, zero.clone())
        .spare(BOOL, "Dead")
        .spare(BOOL, "Logic_Alive")
        .out_ptr("SensorHitInfo", POINTER);
    area.role = QUERY;
    let area_at: usize = a.add_node(&area)?;
    let mut hit: Spec = Spec::node(HIT_NODE, &seed_h)
        .prop(BOOL, "NoEventPause", zero.clone())
        .idle(INT, "ActorNum", one.clone())
        .idle(BOOL, "IsRecordContactActorLink", zero.clone())
        .idle(BOOL, "SendMessage", zero.clone())
        .constant(STR, "ActorName", player)
        .linked_ptr("SensorHitInfoMulti", POINTER, area_at)
        .out(BOOL, "IsHit")
        .spare(BOOL, "IsNotHit")
        .spare_ptr("ContactActorLinkPtr", POINTER);
    hit.role = QUERY;
    let hit_at: usize = a.add_node(&hit)?;
    let mut inside: Spec = Spec::node(AND_NODE, &seed_i).linked(BOOL, "BoolMulti", hit_at, 0).idle(BOOL, "UpdateOnce", zero.clone()).spare(BOOL, "False").out(BOOL, "True");
    inside.role = QUERY;
    let inside_at: usize = a.add_node(&inside)?;
    let mut man: Spec = Spec::node(NODE, &seed_m)
        .prop(STR, NAME, manager_name)
        .linked(BOOL, "IsActive", inside_at, 1)
        .idle(BOOL, "IsDisableRare", zero.clone())
        .idle(BOOL, CREATE, one)
        .idle(BOOL, DELETE, zero.clone())
        .idle(BOOL, RESPAWN, zero)
        .spare(BOOL, "Dead")
        .spare(BOOL, "isFalling")
        .spare(BOOL, "Logic_Alive");
    man.role = QUERY;
    a.add_node(&man)?;
    let bad: Vec<String> = audit(&a);
    if !bad.is_empty() {
        return Err(E::Msg(format!("the fireball storm graph no longer checks out: {}", bad.join(", "))));
    }
    let g: Vec<u8> = a.write()?;
    Ok(Out { path: p.graph, old: 0, new: g.len(), z: g })
}

fn next(raws: &[&[u8]], taken: &mut Vec<u64>, seed: &str) -> u64 {
    let mut n: u32 = 0;
    loop {
        let h: u64 = fresh(raws, &format!("{seed} {n}"));
        if !taken.contains(&h) {
            taken.push(h);
            return h;
        }
        n += 1;
    }
}

pub fn place(y: &mut Byml, raws: &[&[u8]], s: &Spot, map: &str) -> Res<usize> {
    let p: Plan = s.plan();
    let seed: String = format!("{SEED} {map} {}", s.name);
    let mut taken: Vec<u64> = Vec::new();
    for a in y.root.get("Actors").and_then(|a| a.as_arr()).unwrap_or(&[]) {
        taken.extend(hash_of(a));
    }
    let manager: u64 = next(raws, &mut taken, &format!("{seed} manager"));
    let area: u64 = next(raws, &mut taken, &format!("{seed} area"));
    let group: u64 = next(raws, &mut taken, &format!("{seed} group"));
    let mut anchors: Vec<u64> = Vec::new();
    for i in 0..s.anchors.len() {
        anchors.push(next(raws, &mut taken, &format!("{seed} anchor {i}")));
    }
    let mut m: Node = actor(MANAGER, manager, s.manager, euler(0.0, 0.0));
    m.set("Dynamic", Node::Map(vec![("FallTime".to_string(), Node::F32(s.fall)), ("IntervalFrame".to_string(), Node::F32(s.interval))]));
    m.set("Links", Node::Arr(anchors.iter().map(|a| reference(manager, *a)).collect()));
    let mut b: Node = actor(AREA, area, s.area, euler(0.0, 0.0));
    b.set("Scale", floats(s.size));
    let acts: &mut Vec<Node> = list(y, "Actors")?;
    acts.push(m);
    for (i, at) in s.anchors.iter().enumerate() {
        let mut n: Node = actor(ANCHOR, anchors[i], *at, [0.0; 3]);
        n.set("Name", Node::Str(i.to_string()));
        acts.push(n);
    }
    acts.push(b);
    let mut all: Vec<u64> = anchors.clone();
    all.push(manager);
    all.push(area);
    all.sort();
    for name in SHARED {
        if y.root.get(name).is_some() {
            list(y, name)?.push(Node::Arr(all.iter().map(|h| Node::U64(*h)).collect()));
        }
    }
    let mut g: Node = Node::Map(Vec::new());
    g.set("Hash", Node::U64(group));
    g.set("Logic", Node::Str(p.module.clone()));
    g.set("References", Node::Arr(vec![link(&p.manager, manager), link(&p.area, area)]));
    let gs: &mut Vec<Node> = list(y, "AiGroups")?;
    let i: usize = gs.iter().position(|x| hash_of(x).is_some_and(|v| v > group)).unwrap_or(gs.len());
    gs.insert(i, g);
    Ok(s.anchors.len() + 2)
}

fn source(rf: &Romfs, have: Have, p: &str) -> Res<Vec<u8>> {
    match have(p) {
        Some(b) => Ok(b),
        None => rf.read(p),
    }
}

fn one(rf: &mut Romfs, have: Have, path: &str, spots: &[(&Spot, &str)]) -> Res<Out> {
    let z: Vec<u8> = source(rf, have, path)?;
    let dict: u32 = dict_id(&z).unwrap_or(BANC_DICT);
    let raw: Vec<u8> = rf.zs.decompress(&z)?;
    let dynamic: String = path.replace("_Static", "_Dynamic");
    let land: Vec<u8> = rf.zs.decompress(&source(rf, have, &dynamic)?)?;
    let mut y: Byml = Byml::parse(&raw)?;
    for (s, map) in spots {
        place(&mut y, &[&raw, &land], s, map)?;
    }
    let bytes: Vec<u8> = y.write()?;
    Ok(Out { path: path.to_string(), old: raw.len(), new: bytes.len(), z: zs::compress_raw(&bytes, Some(dict))? })
}

pub fn stage(rf: &mut Romfs, have: Have) -> Res<Vec<Out>> {
    let was: Vec<u8> = source(rf, have, SOURCE)?;
    let mut outs: Vec<Out> = Vec::new();
    for s in SPOTS.iter() {
        outs.push(graph(&was, s)?);
    }
    for f in files() {
        let mut here: Vec<(&Spot, &str)> = Vec::new();
        for s in SPOTS.iter() {
            for m in s.maps {
                if s.file(m, "Static") == f {
                    here.push((s, m));
                }
            }
        }
        outs.push(one(rf, have, &f, &here)?);
    }
    Ok(outs)
}

pub fn describe(y: &Byml) -> Vec<(String, [f32; 3])> {
    let list: &[Node] = y.root.get("Actors").and_then(|a| a.as_arr()).unwrap_or(&[]);
    list.iter().filter(|a| text(a, "Gyaml") == MANAGER).filter_map(|a| vec3(a, "Translate").map(|t| (text(a, "Gyaml").to_string(), t))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dump() -> Option<std::path::PathBuf> {
        std::env::var("OOTRR_ROMFS").ok().map(std::path::PathBuf::from).filter(|p| p.join("Banc").is_dir())
    }

    fn none(_: &str) -> Option<Vec<u8>> {
        None
    }

    fn near(a: [f32; 3], b: [f32; 3]) -> f32 {
        ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    }

    #[test]
    fn spots_are_laid_out_to_be_dodged() {
        assert_eq!(SPOTS.len(), 4);
        for s in SPOTS.iter() {
            assert!(s.anchors.len() >= 3 && s.anchors.len() <= 8, "{}", s.name);
            assert!(s.interval >= 30.0 && s.fall >= 3.0, "{}", s.name);
            let far: f32 = near(s.area, s.manager);
            assert!((60.0..=110.0).contains(&far), "{} manager {far} away", s.name);
            assert!((55.0..=80.0).contains(&(s.manager[1] - s.area[1])), "{} manager height", s.name);
            for a in s.anchors {
                assert!(near(*a, s.area) <= 30.0, "{} anchor too far from the box", s.name);
                assert!((s.area[1] - a[1]).abs() < 6.0, "{} anchor height", s.name);
            }
            let mut names: Vec<String> = Vec::new();
            for i in 0..s.anchors.len() {
                names.push(i.to_string());
            }
            assert_eq!(names.len(), s.anchors.len());
        }
        let mut seen: Vec<String> = SPOTS.iter().map(|s| s.plan().graph).collect();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), SPOTS.len());
    }

    #[test]
    fn the_storm_graph_wires_the_area_to_the_manager() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read(SOURCE).unwrap();
        let was: Ainb = Ainb::parse(&raw).unwrap();
        for s in SPOTS.iter() {
            let o: Out = graph(&raw, s).unwrap();
            let a: Ainb = Ainb::parse(&o.z).unwrap();
            assert_eq!(a.els.len(), was.els.len() + 4);
            assert!(audit(&a).is_empty());
            let p: Plan = s.plan();
            let m: usize = (0..a.els.len()).find(|&i| a.prop_str(i, NAME) == Some(p.manager.as_str())).unwrap();
            assert_eq!(a.el_name(m), NODE);
            let areas: Vec<usize> = (0..a.els.len()).filter(|&i| a.prop_str(i, NAME) == Some(p.area.as_str())).collect();
            assert_eq!(areas.len(), 1);
            assert_eq!(a.el_name(areas[0]), AREA_NODE);
            assert_eq!(a.write().unwrap(), o.z);
            assert_eq!(graph(&raw, s).unwrap().z, o.z);
        }
    }

    #[test]
    fn the_game_maps_take_the_storms_and_the_eruption_is_untouched() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let all: Vec<Out> = stage(&mut rf, &none).unwrap();
        let (graphs, maps): (Vec<Out>, Vec<Out>) = all.into_iter().partition(|o| o.path.ends_with(".ainb"));
        assert_eq!(graphs.len(), SPOTS.len());
        assert_eq!(maps.iter().map(|o| o.path.clone()).collect::<Vec<String>>(), files());
        assert!(maps.iter().all(|o| !o.path.contains("DeathMountain")));
        for o in &maps {
            let was: Byml = rf.read_byml(&o.path).unwrap();
            let now: Byml = Byml::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap();
            let here: Vec<&Spot> = SPOTS.iter().filter(|s| s.maps.iter().any(|m| s.file(m, "Static") == o.path)).collect();
            let old: &[Node] = was.root.get("Actors").unwrap().as_arr().unwrap();
            let new: &[Node] = now.root.get("Actors").unwrap().as_arr().unwrap();
            let added: usize = here.iter().map(|s| s.anchors.len() + 2).sum();
            assert_eq!(new.len(), old.len() + added, "{}", o.path);
            assert_eq!(&new[..old.len()], old, "the actors that were there are untouched in {}", o.path);
            let count = |y: &[Node], g: &str| y.iter().filter(|a| text(a, "Gyaml") == g).count();
            assert_eq!(count(new, MANAGER), count(old, MANAGER) + here.len());
            assert_eq!(count(new, ANCHOR), count(old, ANCHOR) + here.iter().map(|s| s.anchors.len()).sum::<usize>());
            let mut hs: Vec<u64> = new.iter().filter_map(hash_of).collect();
            let n: usize = hs.len();
            hs.sort();
            hs.dedup();
            assert_eq!(hs.len(), n, "no hash is used twice in {}", o.path);
            let gs: &[Node] = now.root.get("AiGroups").unwrap().as_arr().unwrap();
            assert!(gs.windows(2).all(|w| hash_of(&w[0]) < hash_of(&w[1])));
            assert_eq!(gs.len(), was.root.get("AiGroups").unwrap().as_arr().unwrap().len() + here.len());
            for s in here {
                let p: Plan = s.plan();
                let g: &Node = gs.iter().find(|g| text(g, "Logic") == p.module).unwrap();
                let refs: &[Node] = g.get("References").unwrap().as_arr().unwrap();
                assert_eq!(refs.len(), 2);
                let m: &Node = new.iter().find(|a| text(a, "Gyaml") == MANAGER && vec3(a, "Translate") == Some(s.manager)).unwrap();
                assert_eq!(refs[0].get("Reference"), m.get("Hash"));
                assert_eq!(m.get("Links").unwrap().as_arr().unwrap().len(), s.anchors.len());
                assert_eq!(m.get("Dynamic").and_then(|d| d.get("FallTime")), Some(&Node::F32(s.fall)));
                for at in s.anchors {
                    for n in old.iter().filter(|n| text(n, "Gyaml").starts_with("Npc_") || text(n, "Gyaml").starts_with("Enemy_")) {
                        let t: [f32; 3] = vec3(n, "Translate").unwrap();
                        assert!(near(*at, t) >= 9.0 || (at[1] - t[1]).abs() > 4.0, "{} anchor {at:?} lands within 9 of {} at {t:?}", s.name, text(n, "Gyaml"));
                    }
                }
                for (i, at) in s.anchors.iter().enumerate() {
                    let a: &Node = new.iter().find(|a| text(a, "Gyaml") == ANCHOR && vec3(a, "Translate") == Some(*at)).unwrap();
                    assert_eq!(text(a, "Name"), i.to_string());
                }
            }
        }
        let trail: Byml = rf.read_byml("Banc/DeathMountainTrail/DeathMountainTrail_Child_Static.bcett.byml.zs").unwrap();
        assert_eq!(describe(&trail).len(), 5);
    }

    #[test]
    fn the_setting_off_writes_nothing_and_on_writes_only_these_files() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let off: Vec<Out> = crate::difficulty::stage(&mut rf, &crate::difficulty::Opts::default(), &none, "seed").unwrap();
        assert!(off.is_empty());
        let on: Vec<Out> = crate::difficulty::stage(&mut rf, &crate::difficulty::Opts { fireballs: true, ..crate::difficulty::Opts::default() }, &none, "seed").unwrap();
        let mut paths: Vec<String> = on.iter().map(|o| o.path.clone()).collect();
        let mut want: Vec<String> = SPOTS.iter().map(|s| s.plan().graph).chain(files()).collect();
        paths.sort();
        want.sort();
        assert_eq!(paths, want);
    }

    #[test]
    fn the_anchors_stand_on_the_ground_in_both_ages() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        for s in SPOTS.iter() {
            for m in s.maps {
                let g: crate::difficulty::ground::Ground = crate::difficulty::ground::load(&mut rf, m).unwrap().unwrap();
                for a in s.anchors {
                    let (y, _): (f32, u32) = g.floor_near(a[0], a[1], a[2], 2.0, 2.0).unwrap_or_else(|| panic!("{} anchor {a:?} is off the ground in {m}", s.name));
                    assert!((a[1] - LIFT - y).abs() < 1.0, "{} anchor {a:?} is {} above the ground in {m}", s.name, a[1] - y);
                }
            }
        }
    }
}
