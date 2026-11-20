use std::path::Path as P;

use crate::ainb::Ainb;
use crate::bfevfl::{Bf, Flow, Kind, Val};
use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::romfs::Romfs;
use crate::staging::Out;

pub const GATE: &str = "Get_StoneOfFire_IsCompleted_Exp";
pub const ALWAYS: &str = crate::always::NAME;
pub const MODULES: [&str; 3] = [
    "Logic/Market_Child_Prefab_0c76.logic.module.ainb",
    "Logic/Market_Child_Prefab_d0eb.logic.module.ainb",
    "Logic/MiniGame_BombchuBowling_e64e.logic.module.ainb",
];
pub const FLOWS: [(&str, &str); 2] = [("MiniGame_BombchuBowling", "Event137"), ("Npc_Hylia_018_Shop_Child", "Event14")];
pub const MAPS: [(&str, &[u64]); 2] = [
    ("Banc/Market/Market_Child_Static.bcett.byml.zs", &[13279196239484838949, 14705828610563193869]),
    ("Banc/Market/Market_Child_Dynamic.bcett.byml.zs", &[7765672504758317189]),
];
pub const NOTE: &str = "Bombchu Bowling and the Bombchu Shop in child Castle Town are open from the start instead of after Dodongo's Cavern. Their readers of Get_StoneOfFire_IsCompleted_Exp (three logic modules, the bowling and shop clerk flows, the bowling attendant, the shop music and the closed sign in the Market maps) read Get_StoneOfFire_IsAfter_Ready_Exp, rewritten to always hold. The Bombchu Shop keeps its night only hours.";
const READ_EXP: &str = "HoldGameDataGetBoolExpSimple";
const EXP_IN: &str = "GameDataNameBoolExp";
const STR: usize = 3;

pub fn patch_module(a: &mut Ainb) -> Res<()> {
    let hits: Vec<usize> = a.find_els(READ_EXP).into_iter().filter(|&i| a.in_str(i, EXP_IN) == Some(GATE)).collect();
    let [i] = hits.as_slice() else {
        return Err(Error::Msg(format!("bombchu open: expected one read of {GATE}, found {}", hits.len())));
    };
    let g: usize = a.find_in(*i, STR, EXP_IN).ok_or(Error::Format("bombchu open: the read has no flag input"))?;
    let o: u32 = a.pool.add(ALWAYS);
    a.set_in_val(STR, g, &o.to_le_bytes())
}

pub fn patch_flow(b: &mut Bf, flow: &str, event: &str) -> Res<()> {
    let f: &mut Flow = b.flows.iter_mut().find(|x| x.name == flow).ok_or_else(|| Error::Msg(format!("bombchu open: no {flow} flow")))?;
    let e = f.events.iter_mut().find(|e| e.name == event).ok_or_else(|| Error::Msg(format!("bombchu open: {flow} has no {event}")))?;
    let Kind::Switch { params: Some(p), .. } = &mut e.kind else {
        return Err(Error::Msg(format!("bombchu open: {flow} {event} is not a switch")));
    };
    let v: &mut Val = p.items.iter_mut().find(|(k, _)| k == "GameDataName").map(|(_, v)| v).ok_or_else(|| Error::Msg(format!("bombchu open: {flow} {event} has no flag")))?;
    if *v != Val::Str(GATE.to_string()) {
        return Err(Error::Msg(format!("bombchu open: {flow} {event} does not read {GATE}")));
    }
    *v = Val::Str(ALWAYS.to_string());
    Ok(())
}

pub fn patch_map(y: &mut Byml, hashes: &[u64]) -> Res<()> {
    let actors: &mut Vec<Node> = y.root.get_mut("Actors").and_then(|n| n.as_arr_mut()).ok_or(Error::Format("bombchu open: the map has no actors"))?;
    for h in hashes {
        let a: &mut Node = actors
            .iter_mut()
            .find(|a| matches!(a.get("Hash"), Some(Node::U64(x)) if x == h))
            .ok_or_else(|| Error::Msg(format!("bombchu open: the map has no actor {h}")))?;
        let p: &mut Node = a.get_mut("Presence").ok_or_else(|| Error::Msg(format!("bombchu open: actor {h} has no presence")))?;
        match p.get_mut("FlagName") {
            Some(n) if n.as_str() == Some(GATE) => *n = Node::Str(ALWAYS.to_string()),
            _ => return Err(Error::Msg(format!("bombchu open: actor {h} does not wait for {GATE}"))),
        }
    }
    Ok(())
}

fn map_bytes(raw: &[u8], hashes: &[u64]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    patch_map(&mut y, hashes)?;
    let out: Vec<u8> = y.write()?;
    if Byml::parse(&out)?.root != y.root {
        return Err(Error::Format("bombchu open: the patched map does not read back"));
    }
    Ok(out)
}

fn flow_bytes(raw: &[u8], flow: &str, event: &str) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    patch_flow(&mut b, flow, event)?;
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(Error::Format("bombchu open: the patched flow does not parse back"));
    }
    Ok(out)
}

fn module_bytes(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    patch_module(&mut a)?;
    let out: Vec<u8> = a.write()?;
    let back: Ainb = Ainb::parse(&out)?;
    if back.write()? != out || !crate::ainb::audit(&back).is_empty() {
        return Err(Error::Format("bombchu open: the patched module does not read back"));
    }
    Ok(out)
}

pub fn stage(rf: &mut Romfs, more: &mut Vec<Out>, work: &P) -> Res<()> {
    for m in MODULES {
        crate::staging::edit_staged(rf, more, work, m, false, module_bytes)?;
    }
    for (flow, event) in FLOWS {
        crate::staging::edit_staged(rf, more, work, &format!("Event/EventFlow/{flow}.bfevfl.zs"), true, |raw| flow_bytes(raw, flow, event))?;
    }
    for (path, hashes) in MAPS {
        crate::staging::edit_staged(rf, more, work, path, true, |raw| map_bytes(raw, hashes))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real() -> Option<Romfs> {
        let p: std::path::PathBuf = ootrr_game::test_romfs::path();
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Romfs::open(&p).ok()
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    #[test]
    fn every_reader_of_the_fire_stone_quest_moves_to_the_always_flag() {
        let Some(mut rf) = real() else { return };
        let mut more: Vec<Out> = Vec::new();
        let none: std::path::PathBuf = std::env::temp_dir().join("ootrr_bombchu_open_no_work");
        stage(&mut rf, &mut more, &none).unwrap();
        assert_eq!(more.len(), MODULES.len() + FLOWS.len() + MAPS.len());
        for m in MODULES {
            let o: &Out = more.iter().find(|o| o.path == m).unwrap();
            assert!(module_bytes(&o.z).is_err());
        }
        let mut again: Vec<Out> = more.clone();
        assert!(stage(&mut rf, &mut again, &none).is_err());
    }

}
