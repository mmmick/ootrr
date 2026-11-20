use std::path::Path as P;

use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::romfs::Romfs;
use crate::staging::Out;

pub const FILE: &str = "Banc/HyruleField/HyruleField_Adult_Dynamic.bcett.byml.zs";
pub const WALL: u64 = 2935529029587700987;
pub const WALL_GYAML: &str = "FldObj_AirWall_All";
pub const GATE: &str = "SageOfWater_IsAfter_Step03_Exp";
pub const NOTE: &str = "The Water Temple gate switch in Lake Hylia can be hookshotted without the Ice Cavern. An invisible wall around the switch and the gate (FldObj_AirWall_All) stayed until the Water Sage quest passed Step03, which happens at the end of the Ice Cavern. Its presence now reads Get_StoneOfFire_IsAfter_Ready_Exp (always true) negated, so it never appears.";

pub fn patch(y: &mut Byml) -> Res<()> {
    let actors: &mut Vec<Node> = y.root.get_mut("Actors").and_then(|n| n.as_arr_mut()).ok_or(Error::Format("water gate: the adult Hyrule Field map has no actors"))?;
    let a: &mut Node = actors
        .iter_mut()
        .find(|a| matches!(a.get("Hash"), Some(Node::U64(h)) if *h == WALL))
        .ok_or(Error::Format("water gate: the adult Hyrule Field map has no Water Temple gate wall"))?;
    if a.get("Gyaml").and_then(|g| g.as_str()) != Some(WALL_GYAML) {
        return Err(Error::Format("water gate: the gate wall is not an all air wall"));
    }
    let p: &mut Node = a.get_mut("Presence").ok_or(Error::Format("water gate: the gate wall has no presence"))?;
    if p.get("IsNegation") != Some(&Node::Bool(true)) {
        return Err(Error::Format("water gate: the gate wall's presence is not negated"));
    }
    match p.get_mut("FlagName") {
        Some(n) if n.as_str() == Some(GATE) => *n = Node::Str(crate::always::NAME.to_string()),
        _ => return Err(Error::Format("water gate: the gate wall does not wait for the Water Sage quest")),
    }
    Ok(())
}

pub fn bytes(raw: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    patch(&mut y)?;
    let out: Vec<u8> = y.write()?;
    if Byml::parse(&out)?.root != y.root {
        return Err(Error::Format("water gate: the patched map does not read back"));
    }
    Ok(out)
}

pub fn stage(rf: &mut Romfs, more: &mut Vec<Out>, work: &P) -> Res<()> {
    crate::staging::edit_staged(rf, more, work, FILE, true, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gate_wall_never_appears() {
        let p: std::path::PathBuf = ootrr_game::test_romfs::path();
        if !p.join("Pack").join("ZsDic.pack.zs").is_file() {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            return;
        }
        let mut rf: Romfs = Romfs::open(&p).unwrap();
        let raw: Vec<u8> = rf.read_zs(FILE).unwrap();
        let out: Vec<u8> = bytes(&raw).unwrap();
        let y: Byml = Byml::parse(&out).unwrap();
        let acts: &[Node] = y.root.get("Actors").and_then(|n| n.as_arr()).unwrap();
        let w: &Node = acts.iter().find(|a| a.get("Hash") == Some(&Node::U64(WALL))).unwrap();
        assert_eq!(w.get("Presence").and_then(|p| p.get("FlagName")).and_then(|n| n.as_str()), Some(crate::always::NAME));
        assert!(bytes(&out).is_err());
    }
}
