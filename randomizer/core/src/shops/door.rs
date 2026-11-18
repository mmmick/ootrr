//! The child Market Bombchu Shop door.
//!
//! Logic/Market_Child_Prefab_0c76 opens the shop's door (child module Obj_Door_ShopBombChu_26fc)
//! only after Goron's Ruby: its EnableOpen input is AND(Get_StoneOfFire_IsCompleted_Exp, night)
//! and its EnableDo input is AND(Get_StoneOfFire_IsCompleted_Exp, day or night). Without the Ruby
//! the door cannot even be talked to. With shops shuffled the shop sells checks from the start,
//! so EnableDo becomes true and EnableOpen reads the night output of the same time node: the
//! shop keeps its night only hours and loses the Ruby. Nothing outside this module changes; the
//! Ruby still drives the Market's other readers of the quest step.

use std::fs;

use crate::ainb::Ainb;
use crate::error::{Error, Res};
use crate::open_world::skips::{force_bool, relink_bool};
use crate::qol::edit_plain;
use crate::romfs::Romfs;
use crate::staging::Out;

pub const MODULE: &str = "Logic/Market_Child_Prefab_0c76.logic.module.ainb";
const DOOR_MODULE: &str = "Obj_Door_ShopBombChu_26fc.logic.module";
const DOOR: &str = "Obj_Door_ShopBombChu_26fc_b542";
const TIME: &str = "HoldWmGetTimeDiv";
/// HoldWmGetTimeDiv's outputs are Day (0) and Night (1).
const NIGHT: u16 = 1;
const BOOL: usize = 1;

/// The logic setting that opens the shop's door without Goron's Ruby (data/logic/settings.json).
pub const LOGIC_KEY: &str = "bombchu_shop_open";

pub const NOTE: &str = "The child Market Bombchu Shop opens at night from the start instead of after Goron's Ruby, so its shuffled shelves can be reached. Experimental, untested in the game.";

fn door(a: &Ainb) -> Res<usize> {
    let hits: Vec<usize> = a.find_els(DOOR_MODULE).into_iter().filter(|&i| a.prop_str(i, "InstanceName") == Some(DOOR)).collect();
    match hits[..] {
        [i] => Ok(i),
        _ => Err(Error::Msg(format!("shops: expected one Bombchu Shop door in {MODULE}, found {}", hits.len()))),
    }
}

pub fn open_bytes(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    let d: usize = door(&a)?;
    let time: usize = match a.find_els(TIME)[..] {
        [t] => t,
        _ => return Err(Error::Msg(format!("shops: expected one time of day node in {MODULE}"))),
    };
    force_bool(&mut a, d, "EnableDo", true)?;
    relink_bool(&mut a, d, "EnableOpen", time, NIGHT)?;
    let out: Vec<u8> = a.write()?;
    let b: Ainb = Ainb::parse(&out)?;
    let audit: Vec<String> = crate::ainb::audit(&b);
    if !audit.is_empty() {
        return Err(Error::Msg(format!("shops: the Bombchu Shop module fails its audit: {}", audit.join("; "))));
    }
    let open = b.find_in(d, BOOL, "EnableOpen").and_then(|g| b.input(BOOL, g));
    let act = b.find_in(d, BOOL, "EnableDo").and_then(|g| b.input(BOOL, g));
    if b.write()? != out || !open.is_some_and(|x| x.src == time as i16 && x.plug == NIGHT as i16) || !act.is_some_and(|x| x.src < 0 && x.val == 1u32.to_le_bytes()) {
        return Err(Error::Format("shops: the Bombchu Shop module does not read back"));
    }
    Ok(out)
}

pub fn worked(rf: &mut Romfs, pk: &crate::modpack::Pack) -> Res<Out> {
    let root: std::path::PathBuf = pk.work.join("romfs");
    let have = |p: &str| -> Option<Vec<u8>> { fs::read(p.split('/').fold(root.clone(), |a, s| a.join(s))).ok() };
    edit_plain(rf, &have, MODULE, open_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_real_bombchu_shop_door_opens_at_night_without_the_ruby() {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("D:/Oot/romfs"));
        let Ok(raw) = fs::read(p.join(MODULE)) else {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            return;
        };
        let before: Ainb = Ainb::parse(&raw).unwrap();
        let d: usize = door(&before).unwrap();
        let old = before.input(BOOL, before.find_in(d, BOOL, "EnableOpen").unwrap()).unwrap();
        assert_eq!(before.el_name(old.src as usize), "OperateAND");
        let out: Vec<u8> = open_bytes(&raw).unwrap();
        let a: Ainb = Ainb::parse(&out).unwrap();
        let open = a.input(BOOL, a.find_in(d, BOOL, "EnableOpen").unwrap()).unwrap();
        assert_eq!((a.el_name(open.src as usize), open.plug), (TIME, NIGHT as i16));
        assert!(open_bytes(&out).is_err(), "the door is already open");
    }
}
