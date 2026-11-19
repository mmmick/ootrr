use crate::ainb::build::{Spec, QUERY};
use crate::ainb::{audit, Ainb, Input};
use crate::byml::{Byml, Node};
use crate::error::{Error as E, Res};
use crate::flow_locations::dict_id;
use crate::murmur::murmur3_32;
use crate::placement::hash_of;
use crate::qol::Have;
use crate::romfs::Romfs;
use crate::staging::{Out, BANC_DICT};
use crate::zs;

pub const HEART: &str = "Obj_Heart_Container_";
pub const NODE: &str = "ActorLogicSimpleActor";
pub const NAME: &str = "InstanceName";
pub const CREATE: &str = "Logic_Create";
pub const DELETE: &str = "Logic_Delete";
pub const RESPAWN: &str = "RespawnTrigger";
pub const ROOM: &str = "DefaultDungeonRoomIndex";
pub const BOOL: usize = 1;
pub const STR: usize = 3;
pub const IN: usize = 16;
pub const OUT: usize = 4;
pub const UNSET: u32 = 0x180_0000;
pub const READ: u32 = 0x8000_0000;
pub const SEED: &str = "ootrr boss miniboss";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Start {
    pub module: &'static str,
    pub out: &'static str,
}

pub const GOHMA_START: Start = Start { module: "Enemy_DungeonBoss_Gohma_Prefab_b75b.logic.module", out: "End_TrapEvent" };

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub boss: &'static str,
    pub file: &'static str,
    pub actor: &'static str,
    pub mini: &'static str,
    pub label: &'static str,
    pub at: [f32; 3],
    pub start: Option<Start>,
}

pub const SPOTS: [Spot; 4] = [
    Spot {
        boss: "Queen Gohma",
        file: "Banc/InsideTheDekuTree/InsideTheDekuTree_Static.bcett.byml.zs",
        actor: "Enemy_DungeonBoss_Gohma",
        mini: "Enemy_Stalfos_ForChild",
        label: "Stalfos",
        at: [16.5, 47.25, 74.0],
        start: Some(GOHMA_START),
    },
    Spot {
        boss: "Barinade",
        file: "Banc/InsideJabuJabuBelly/InsideJabuJabuBelly_Static.bcett.byml.zs",
        actor: "Enemy_DungeonBoss_Barinade",
        mini: "Enemy_Stalfos_ForChild",
        label: "Stalfos",
        at: [51.5, 102.25, -112.1],
        start: None,
    },
    Spot {
        boss: "Phantom Ganon",
        file: "Banc/ForestTemple/ForestTemple_Static.bcett.byml.zs",
        actor: "Enemy_DungeonBoss_PhantomGanon",
        mini: "Enemy_Stalfos_Weak",
        label: "Stalfos",
        at: [5.5, 74.95, -62.5],
        start: None,
    },
    Spot {
        boss: "Twinrova",
        file: "Banc/SpiritTemple/SpiritTemple_Static.bcett.byml.zs",
        actor: "Enemy_DungeonBoss_Twinrova",
        mini: "Enemy_Dinolfos",
        label: "Dinolfos",
        at: [6.0, 142.4, -157.9],
        start: None,
    },
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub logic: String,
    pub heart: String,
    pub id: String,
    pub hash: u64,
}

pub fn covered() -> String {
    let v: Vec<String> = SPOTS.iter().map(|s| format!("{} ({})", s.boss, s.label)).collect();
    v.join(", ")
}

pub fn logic_file(module: &str) -> Res<String> {
    let base: &str = module.rsplit('/').next().unwrap_or(module);
    if !base.ends_with(".ain") {
        return Err(E::Msg(format!("the boss room logic {module} is not a graph file")));
    }
    Ok(format!("Logic/{base}b"))
}

fn refers(r: &Node, hash: u64) -> bool {
    match r.get("Reference") {
        Some(Node::U64(x)) => *x == hash,
        Some(Node::I64(x)) => *x as u64 == hash,
        _ => false,
    }
}

fn refs(g: &Node) -> &[Node] {
    g.get("References").and_then(|r| r.as_arr()).unwrap_or(&[])
}

fn text<'a>(n: &'a Node, k: &str) -> &'a str {
    n.get(k).and_then(|v| v.as_str()).unwrap_or("")
}

fn used(raw: &[u8], hash: u64) -> bool {
    let b: [u8; 8] = hash.to_le_bytes();
    raw.windows(8).any(|w| w == b)
}

pub fn fresh(raw: &[u8], seed: &str) -> u64 {
    let b: &[u8] = seed.as_bytes();
    let mut h: u64 = (u64::from(murmur3_32(b, 1)) << 32) | u64::from(murmur3_32(b, 2));
    while h == 0 || used(raw, h) {
        h = h.wrapping_add(1);
    }
    h
}

pub fn actor(s: &Spot, hash: u64, room: i32) -> Node {
    let mut a: Node = Node::Map(Vec::new());
    a.set("Dynamic", Node::Map(vec![(ROOM.to_string(), Node::I32(room))]));
    a.set("Gyaml", Node::Str(s.mini.to_string()));
    a.set("Hash", Node::U64(hash));
    a.set("Phive", Node::Map(vec![("Placement".to_string(), Node::Map(vec![("ID".to_string(), Node::U64(hash))]))]));
    a.set("SRTHash", Node::U32(murmur3_32(&hash.to_le_bytes(), 0)));
    a.set("Translate", Node::Arr(s.at.iter().map(|v| Node::F32(*v)).collect()));
    a
}

pub fn place(y: &mut Byml, raw: &[u8], s: &Spot) -> Res<Plan> {
    let seed: String = format!("{SEED} {} {}", s.actor, s.mini);
    attach(y, raw, s.actor, s.file, s.mini, &seed, &|hash: u64, room: i32| actor(s, hash, room))
}

pub fn attach(y: &mut Byml, raw: &[u8], boss_kind: &str, file: &str, kind: &str, seed: &str, make: &dyn Fn(u64, i32) -> Node) -> Res<Plan> {
    let list: &[Node] = y.root.get("Actors").and_then(|a| a.as_arr()).ok_or(E::Format("boss minibosses: the map has no actor list"))?;
    let boss: &Node = list.iter().find(|a| text(a, "Gyaml") == boss_kind).ok_or_else(|| E::Msg(format!("{boss_kind} is not in {file}, the game files differ")))?;
    let bh: u64 = hash_of(boss).ok_or(E::Format("boss minibosses: the boss has no hash"))?;
    let room: i32 = match boss.get("Dynamic").and_then(|d| d.get(ROOM)) {
        Some(Node::I32(v)) => *v,
        _ => return Err(E::Msg(format!("{boss_kind} has no dungeon room, the game files differ"))),
    };
    let groups: &[Node] = y.root.get("AiGroups").and_then(|g| g.as_arr()).unwrap_or(&[]);
    let own: Vec<usize> = (0..groups.len()).filter(|&i| !text(&groups[i], "Logic").is_empty() && refs(&groups[i]).iter().any(|r| refers(r, bh))).collect();
    let [gi] = own.as_slice() else {
        return Err(E::Msg(format!("{boss_kind} is steered by {} room graphs and not one, the game files differ", own.len())));
    };
    let module: String = text(&groups[*gi], "Logic").to_string();
    if groups.iter().filter(|g| text(g, "Logic") == module).count() != 1 {
        return Err(E::Msg(format!("the room graph {module} is used more than once, a new node would break the other room")));
    }
    let hearts: Vec<&str> = refs(&groups[*gi]).iter().map(|r| text(r, "Id")).filter(|i| i.starts_with(HEART)).collect();
    let [heart] = hearts.as_slice() else {
        return Err(E::Msg(format!("the room of {boss_kind} has {} heart containers and not one, the game files differ", hearts.len())));
    };
    let heart: String = heart.to_string();
    let mut n: u32 = murmur3_32(seed.as_bytes(), 0) & 0xffff;
    let mut id: String = format!("{kind}_{n:04x}");
    while refs(&groups[*gi]).iter().any(|r| text(r, "Id") == id) {
        n = (n + 1) & 0xffff;
        id = format!("{kind}_{n:04x}");
    }
    let hash: u64 = fresh(raw, seed);
    let logic: String = logic_file(&module)?;
    let mut r: Node = Node::Map(Vec::new());
    r.set("Id", Node::Str(id.clone()));
    r.set("Path", Node::Str(id.clone()));
    r.set("Reference", Node::U64(hash));
    let gs: &mut Vec<Node> = y.root.get_mut("AiGroups").and_then(|g| g.as_arr_mut()).ok_or(E::Format("boss minibosses: the map has no logic groups"))?;
    gs[*gi].get_mut("References").and_then(|r| r.as_arr_mut()).ok_or(E::Format("boss minibosses: the room graph has no references"))?.push(r);
    let acts: &mut Vec<Node> = y.root.get_mut("Actors").and_then(|a| a.as_arr_mut()).ok_or(E::Format("boss minibosses: the map has no actor list"))?;
    acts.push(make(hash, room));
    Ok(Plan { logic, heart, id, hash })
}

fn named(a: &Ainb, id: &str) -> Vec<usize> {
    (0..a.els.len()).filter(|&i| a.prop_str(i, NAME) == Some(id)).collect()
}

fn unset(a: &mut Ainb, i: usize, name: &str) -> Res<()> {
    let g: usize = a.find_in(i, BOOL, name).ok_or(E::Format("boss minibosses: the new node lost an input"))?;
    let o: usize = g * IN + 8;
    a.ins[BOOL].get_mut(o..o + 4).ok_or(E::Format("boss minibosses: input out of range"))?.copy_from_slice(&UNSET.to_le_bytes());
    Ok(())
}

pub fn wire(a: &mut Ainb, heart: &str, id: &str) -> Res<usize> {
    wire_with(a, Some(heart), id, None)
}

pub fn signal(a: &Ainb, s: &Start) -> Res<(usize, i16)> {
    let ms: Vec<usize> = a.find_els(s.module);
    let [m] = ms.as_slice() else {
        return Err(E::Msg(format!("the room graph has {} nodes of {} and not one, the game files differ", ms.len(), s.module)));
    };
    let (b, n): (usize, usize) = (a.params[*m].ioe[BOOL][2] as usize, a.params[*m].ioe[BOOL][3] as usize);
    for k in b..b + n {
        let e: &[u8] = a.outs[BOOL].get(k * OUT..k * OUT + 4).ok_or(E::Format("boss minibosses: output out of range"))?;
        let w: u32 = u32::from_le_bytes([e[0], e[1], e[2], e[3]]);
        if a.s(w & 0x3fff_ffff) == s.out {
            return Ok((*m, (k - b) as i16));
        }
    }
    Err(E::Msg(format!("{} has no output {}, the game files differ", s.module, s.out)))
}

pub fn wire_with(a: &mut Ainb, heart: Option<&str>, id: &str, start: Option<&Start>) -> Res<usize> {
    if !named(a, id).is_empty() {
        return Err(E::Msg(format!("the room graph already has a node for {id}")));
    }
    let gone: Option<Input> = match heart {
        Some(heart) => {
            let hs: Vec<usize> = named(a, heart);
            let [h] = hs.as_slice() else {
                return Err(E::Msg(format!("the room graph has {} nodes for {heart} and not one, the game files differ", hs.len())));
            };
            if a.el_name(*h) != NODE {
                return Err(E::Msg(format!("{heart} is not a plain actor node, the game files differ")));
            }
            let g: usize = a.find_in(*h, BOOL, CREATE).ok_or_else(|| E::Msg(format!("{heart} has no create input, the game files differ")))?;
            let x: Input = a.input(BOOL, g).ok_or(E::Format("boss minibosses: input out of range"))?;
            if x.src < 0 {
                return Err(E::Msg(format!("{heart} does not wait for the boss, so nothing says when the fight is over")));
            }
            Some(x)
        }
        None => None,
    };
    let from: Option<(usize, i16)> = match start {
        Some(s) => Some(signal(a, s)?),
        None => None,
    };
    let name: Vec<u8> = a.pool.add(id).to_le_bytes().to_vec();
    let seed: String = format!("{SEED} {id}");
    let zero: Vec<u8> = 0u32.to_le_bytes().to_vec();
    let mut sp: Spec = Spec::node(NODE, &seed).prop(STR, NAME, name);
    sp = match from {
        Some((m, plug)) => sp.linked(BOOL, CREATE, m, plug),
        None => sp.constant(BOOL, CREATE, 1u32.to_le_bytes().to_vec()),
    };
    sp = match &gone {
        Some(x) => sp.linked(BOOL, DELETE, x.src as usize, x.plug),
        None => sp.idle(BOOL, DELETE, zero.clone()),
    };
    sp = match from {
        Some(_) => sp.idle(BOOL, RESPAWN, zero),
        None => sp.constant(BOOL, RESPAWN, zero),
    };
    sp = sp.out(BOOL, "Dead").out(BOOL, "Logic_Alive");
    sp.role = QUERY;
    let i: usize = a.add_node(&sp)?;
    if from.is_none() {
        unset(a, i, CREATE)?;
        unset(a, i, RESPAWN)?;
    }
    let (b, n): (usize, usize) = (a.params[i].ioe[BOOL][2] as usize, a.params[i].ioe[BOOL][3] as usize);
    for k in b..b + n {
        let e: &mut [u8] = a.outs[BOOL].get_mut(k * OUT..k * OUT + 4).ok_or(E::Format("boss minibosses: output out of range"))?;
        let w: u32 = u32::from_le_bytes([e[0], e[1], e[2], e[3]]) & !READ;
        e.copy_from_slice(&w.to_le_bytes());
    }
    let bad: Vec<String> = audit(a);
    if !bad.is_empty() {
        return Err(E::Msg(format!("the room graph no longer checks out: {}", bad.join(", "))));
    }
    Ok(i)
}

pub fn one(rf: &mut Romfs, have: Have, s: &Spot) -> Res<[Out; 2]> {
    let z: Vec<u8> = match have(s.file) {
        Some(b) => b,
        None => rf.read(s.file)?,
    };
    let dict: u32 = dict_id(&z).unwrap_or(BANC_DICT);
    let raw: Vec<u8> = rf.zs.decompress(&z)?;
    let mut y: Byml = Byml::parse(&raw)?;
    let p: Plan = place(&mut y, &raw, s)?;
    let bytes: Vec<u8> = y.write()?;
    let banc: Out = Out { path: s.file.to_string(), old: raw.len(), new: bytes.len(), z: zs::compress_raw(&bytes, Some(dict))? };
    let was: Vec<u8> = match have(&p.logic) {
        Some(b) => b,
        None => rf.read(&p.logic)?,
    };
    let mut a: Ainb = Ainb::parse(&was)?;
    wire_with(&mut a, Some(&p.heart), &p.id, s.start.as_ref())?;
    let g: Vec<u8> = a.write()?;
    Ok([banc, Out { path: p.logic, old: was.len(), new: g.len(), z: g }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ainb::tests::{node, synth};

    fn spot() -> Spot {
        Spot { boss: "Boss", file: "Banc/A/A_Static.bcett.byml.zs", actor: "Enemy_DungeonBoss_X", mini: "Enemy_Stalfos", label: "Stalfos", at: [1.0, 2.0, 3.0], start: None }
    }

    fn rf(id: &str, h: u64) -> Node {
        Node::Map(vec![("Id".to_string(), Node::Str(id.to_string())), ("Path".to_string(), Node::Str(id.to_string())), ("Reference".to_string(), Node::U64(h))])
    }

    fn group(logic: &str, rs: Vec<Node>) -> Node {
        Node::Map(vec![("Hash".to_string(), Node::U64(900)), ("Logic".to_string(), Node::Str(logic.to_string())), ("References".to_string(), Node::Arr(rs))])
    }

    fn map(groups: Vec<Node>) -> Byml {
        let boss: Node = Node::Map(vec![
            ("Dynamic".to_string(), Node::Map(vec![(ROOM.to_string(), Node::I32(22))])),
            ("Gyaml".to_string(), Node::Str("Enemy_DungeonBoss_X".to_string())),
            ("Hash".to_string(), Node::U64(7)),
        ]);
        Byml { root: Node::Map(vec![("Actors".to_string(), Node::Arr(vec![boss])), ("AiGroups".to_string(), Node::Arr(groups))]), version: 7, big: false }
    }

    const ROOM_GRAPH: &str = "Logic/Module/Room_22_Prefab/Room_22_Prefab_c724.logic.module.ain";

    #[test]
    fn the_miniboss_joins_the_room_of_the_boss_and_its_room_graph() {
        let mut y: Byml = map(vec![group(ROOM_GRAPH, vec![rf("Obj_Heart_Container_c072", 8), rf("Enemy_DungeonBoss_X_7c41", 7)]), group("Logic/Module/Other/Other_0001.logic.module.ain", vec![rf("Area_1", 9)])]);
        let raw: Vec<u8> = y.write().unwrap();
        let p: Plan = place(&mut y, &raw, &spot()).unwrap();
        assert_eq!(p.logic, "Logic/Room_22_Prefab_c724.logic.module.ainb");
        assert_eq!(p.heart, "Obj_Heart_Container_c072");
        assert!(p.id.starts_with("Enemy_Stalfos_") && p.id.len() == "Enemy_Stalfos_".len() + 4);
        assert!(![0, 7, 8, 9, 900].contains(&p.hash));
        let acts: &[Node] = y.root.get("Actors").unwrap().as_arr().unwrap();
        assert_eq!(acts.len(), 2);
        let a: &Node = &acts[1];
        assert_eq!(a.get("Gyaml").and_then(|v| v.as_str()), Some("Enemy_Stalfos"));
        assert_eq!(hash_of(a), Some(p.hash));
        assert_eq!(a.get("Dynamic").and_then(|d| d.get(ROOM)), Some(&Node::I32(22)));
        assert_eq!(a.get("Phive").and_then(|d| d.get("Placement")).and_then(|d| d.get("ID")), Some(&Node::U64(p.hash)));
        assert_eq!(a.get("Translate"), Some(&Node::Arr(vec![Node::F32(1.0), Node::F32(2.0), Node::F32(3.0)])));
        let gs: &[Node] = y.root.get("AiGroups").unwrap().as_arr().unwrap();
        assert_eq!(refs(&gs[0]).len(), 3);
        assert_eq!(refs(&gs[0])[2], rf(&p.id, p.hash));
        assert_eq!(refs(&gs[1]).len(), 1);
        let back: Byml = Byml::parse(&y.write().unwrap()).unwrap();
        assert_eq!(back, y);
    }

    #[test]
    fn a_room_that_does_not_look_like_a_boss_room_is_refused() {
        let s: Spot = spot();
        let mut none: Byml = map(vec![group(ROOM_GRAPH, vec![rf("Obj_Heart_Container_c072", 8)])]);
        assert!(place(&mut none, &[], &s).is_err());
        let mut no_heart: Byml = map(vec![group(ROOM_GRAPH, vec![rf("Enemy_DungeonBoss_X_7c41", 7)])]);
        assert!(place(&mut no_heart, &[], &s).is_err());
        let mut twice: Byml = map(vec![group(ROOM_GRAPH, vec![rf("Obj_Heart_Container_c072", 8), rf("Enemy_DungeonBoss_X_7c41", 7)]), group(ROOM_GRAPH, vec![rf("Area_1", 9)])]);
        assert!(place(&mut twice, &[], &s).is_err());
        let mut gone: Byml = map(Vec::new());
        gone.root.set("Actors", Node::Arr(Vec::new()));
        assert!(place(&mut gone, &[], &s).is_err());
        assert_eq!(none.root.get("Actors").unwrap().as_arr().unwrap().len(), 1);
    }

    #[test]
    fn a_new_hash_never_repeats_one_of_the_map() {
        let h: u64 = fresh(&[], "a");
        assert_eq!(h, fresh(&[], "a"));
        assert_ne!(h, fresh(&[], "b"));
        assert_eq!(fresh(&h.to_le_bytes(), "a"), h + 1);
        assert!(logic_file("Logic/Module/A/A_1.logic.module.ain").is_ok());
        assert!(logic_file("Logic/Module/A/A_1.txt").is_err());
    }

    fn name_it(a: &mut Ainb, i: usize, id: &str) {
        let nm: u32 = a.pool.add(NAME);
        let v: u32 = a.pool.add(id);
        let at: u32 = (a.props[STR].len() / 12) as u32;
        a.props[STR].extend_from_slice(&nm.to_le_bytes());
        a.props[STR].extend_from_slice(&0u32.to_le_bytes());
        a.props[STR].extend_from_slice(&v.to_le_bytes());
        a.params[i].pe[STR] = [at, 1];
        for j in i + 1..a.params.len() {
            a.params[j].pe[STR][0] += 1;
        }
    }

    fn graph() -> (Ainb, usize, usize) {
        let mut a: Ainb = synth();
        let flag: usize = a.add_node(&Spec::query("HoldGameDataGetBoolSimple", "t#1").out(BOOL, "Bool")).unwrap();
        let heart: usize = a.add_node(&Spec::query(NODE, "t#2").linked(BOOL, CREATE, flag, 0).out(BOOL, "Dead")).unwrap();
        (a, flag, heart)
    }

    #[test]
    fn the_new_node_is_there_from_the_start_and_leaves_when_the_heart_appears() {
        let (mut a, flag, heart): (Ainb, usize, usize) = graph();
        name_it(&mut a, heart, "Obj_Heart_Container_c072");
        let i: usize = wire(&mut a, "Obj_Heart_Container_c072", "Enemy_Stalfos_00aa").unwrap();
        let c: Ainb = Ainb::parse(&a.write().unwrap()).unwrap();
        assert_eq!(c.el_name(i), NODE);
        assert_eq!(c.prop_str(i, NAME), Some("Enemy_Stalfos_00aa"));
        let made: Input = c.input(BOOL, c.find_in(i, BOOL, CREATE).unwrap()).unwrap();
        assert_eq!((made.src, made.flags, made.val), (-1, UNSET, 1u32.to_le_bytes().to_vec()));
        let gone: Input = c.input(BOOL, c.find_in(i, BOOL, DELETE).unwrap()).unwrap();
        assert_eq!((gone.src, gone.plug, gone.flags), (flag as i16, 0, 0));
        let again: Input = c.input(BOOL, c.find_in(i, BOOL, RESPAWN).unwrap()).unwrap();
        assert_eq!((again.src, again.flags, again.val), (-1, UNSET, vec![0; 4]));
        assert_eq!(c.params[i].group(0).len(), 1);
        assert_eq!(c.params[i].group(0)[0].a as usize, flag);
        assert_eq!(c.s(c.params[i].group(0)[0].b), DELETE);
        assert!(crate::ainb::show::show(&c).contains("out bool Logic_Alive flags 0x0"));
        assert!(audit(&c).is_empty());
        assert!(wire(&mut a, "Obj_Heart_Container_c072", "Enemy_Stalfos_00aa").is_err());
        assert!(wire(&mut a, "Obj_Heart_Container_ffff", "Enemy_Stalfos_00ab").is_err());
    }

    const MODULE: Start = Start { module: "Boss_Prefab_1234.logic.module", out: "End_TrapEvent" };

    fn graph_with_module() -> (Ainb, usize, usize) {
        let (mut a, _flag, heart): (Ainb, usize, usize) = graph();
        name_it(&mut a, heart, "Obj_Heart_Container_c072");
        let m: usize = a.add_node(&Spec::query(MODULE.module, "t#3").out(BOOL, MODULE.out).out(BOOL, "Other")).unwrap();
        (a, heart, m)
    }

    #[test]
    fn a_new_node_with_a_start_signal_is_not_created_at_load() {
        let (mut a, heart, m): (Ainb, usize, usize) = graph_with_module();
        assert_eq!(signal(&a, &MODULE).unwrap(), (m, 0));
        let h: Input = a.input(BOOL, a.find_in(heart, BOOL, CREATE).unwrap()).unwrap();
        let i: usize = wire_with(&mut a, Some("Obj_Heart_Container_c072"), "Enemy_Stalfos_00aa", Some(&MODULE)).unwrap();
        let c: Ainb = Ainb::parse(&a.write().unwrap()).unwrap();
        let made: Input = c.input(BOOL, c.find_in(i, BOOL, CREATE).unwrap()).unwrap();
        assert_eq!((made.src, made.plug), (m as i16, 0));
        assert_eq!(made.flags & UNSET, 0);
        let gone: Input = c.input(BOOL, c.find_in(i, BOOL, DELETE).unwrap()).unwrap();
        assert_eq!((gone.src, gone.plug), (h.src, h.plug));
        assert!(audit(&c).is_empty());
    }

    #[test]
    fn minions_have_a_start_signal_and_never_a_delete() {
        let (mut a, _heart, m): (Ainb, usize, usize) = graph_with_module();
        let i: usize = wire_with(&mut a, None, "Enemy_Keese_00aa", Some(&MODULE)).unwrap();
        let j: usize = wire_with(&mut a, None, "Enemy_Keese_00ab", Some(&MODULE)).unwrap();
        let c: Ainb = Ainb::parse(&a.write().unwrap()).unwrap();
        for k in [i, j] {
            let made: Input = c.input(BOOL, c.find_in(k, BOOL, CREATE).unwrap()).unwrap();
            assert_eq!((made.src, made.plug), (m as i16, 0));
            let gone: Input = c.input(BOOL, c.find_in(k, BOOL, DELETE).unwrap()).unwrap();
            assert_eq!((gone.src, gone.flags, gone.val), (-1, UNSET, vec![0; 4]));
            let again: Input = c.input(BOOL, c.find_in(k, BOOL, RESPAWN).unwrap()).unwrap();
            assert_eq!((again.src, again.flags, again.val), (-1, UNSET, vec![0; 4]));
        }
        assert!(audit(&c).is_empty());
        assert!(wire_with(&mut a, None, "Enemy_Keese_00ab", Some(&MODULE)).is_err());
    }

    #[test]
    fn a_missing_start_signal_is_refused() {
        let (mut a, _heart, _m): (Ainb, usize, usize) = graph_with_module();
        let none: Start = Start { module: "Boss_Prefab_1234.logic.module", out: "Nope" };
        let other: Start = Start { module: "Other.logic.module", out: "End_TrapEvent" };
        let n: usize = a.els.len();
        assert!(signal(&a, &none).is_err() && signal(&a, &other).is_err());
        assert!(wire_with(&mut a, None, "Enemy_Keese_00aa", Some(&none)).is_err());
        assert_eq!(a.els.len(), n);
    }

    #[test]
    fn only_gohma_has_a_start_signal() {
        for s in &SPOTS {
            assert_eq!(s.start.is_some(), s.actor.ends_with("Gohma"), "{}", s.boss);
        }
        assert_eq!(GOHMA_START.out, "End_TrapEvent");
    }

    #[test]
    fn a_heart_that_is_always_there_gives_no_end_of_the_fight() {
        let mut a: Ainb = synth();
        let t: usize = node(&mut a, NODE, 0, &[(BOOL, CREATE, 1u32.to_le_bytes().to_vec())]);
        name_it(&mut a, t, "Obj_Heart_Container_c072");
        let n: usize = a.els.len();
        assert!(wire(&mut a, "Obj_Heart_Container_c072", "Enemy_Stalfos_00aa").is_err());
        assert_eq!(a.els.len(), n);
    }

    #[test]
    fn the_covered_bosses_are_named_with_their_miniboss() {
        let c: String = covered();
        assert!(c.starts_with("Queen Gohma (Stalfos), Barinade (Stalfos)"));
        assert!(!c.contains("Dodongo"), "King Dodongo's room went black in the game with a miniboss");
        assert!(c.ends_with("Twinrova (Dinolfos)"));
        for s in &SPOTS {
            assert!(s.file.starts_with("Banc/") && s.file.ends_with("_Static.bcett.byml.zs"));
            assert!(s.actor.starts_with("Enemy_DungeonBoss_") && !s.mini.contains("Boss"));
        }
    }
}
