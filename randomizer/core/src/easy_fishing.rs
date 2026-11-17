use crate::bfevfl::{Bf, Cont, Event, Flow, Val};
use crate::error::{Error as E, Res};
use crate::flow_locations::{dict_id, path_of, StagedFile};
use crate::romfs::Romfs;
use crate::staging::{rewrite, Out};
use crate::zs;

pub const SETTING: &str = "easy_fishing";
pub const NUM: u16 = 202;
pub const NOTE: &str = "Easy fishing: the fishing pond owner gives the child and adult prizes for any fish, whatever its size.";
const FILE: &str = "MiniGame_Fishing.bfevfl";
const SIZE: &str = "MiniGame_Fishing_KeepFishSize";
const CHECKS: [(&str, i32); 3] = [("Event581", 40), ("Event489", 50), ("Event585", 60)];

fn lower(f: &mut Flow, name: &str, was: i32) -> Res<()> {
    let e: &mut Event = f.events.iter_mut().find(|e| e.name == name).ok_or_else(|| E::Msg(format!("easy fishing: no event {name}")))?;
    let c: &mut Cont = e.kind.params_mut().ok_or_else(|| E::Msg(format!("easy fishing: {name} has no parameters")))?;
    let on_size: bool = c.items.iter().any(|(k, v)| k == "GameDataName" && *v == Val::Str(SIZE.to_string()));
    if !on_size {
        return Err(E::Msg(format!("easy fishing: {name} does not check {SIZE}, the game files differ")));
    }
    match c.items.iter_mut().find(|(k, _)| k == "Value") {
        Some((_, Val::Int(x))) if *x == was => {
            *x = 0;
            Ok(())
        }
        Some((_, v)) => Err(E::Msg(format!("easy fishing: {name} checks {v:?}, expected {was}"))),
        None => Err(E::Msg(format!("easy fishing: {name} has no Value"))),
    }
}

pub fn flow_bytes(raw: &[u8]) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    let f: &mut Flow = b.flows.first_mut().ok_or(E::Format("easy fishing: the fishing file holds no flow"))?;
    for (name, was) in CHECKS {
        lower(f, name, was)?;
    }
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(E::Format("easy fishing: the edited fishing flow does not parse back"));
    }
    Ok(out)
}

pub fn stage(rf: &mut Romfs, files: &mut Vec<StagedFile>) -> Res<()> {
    let path: String = path_of(FILE);
    match files.iter().position(|x| x.path == path) {
        Some(i) => {
            let id: Option<u32> = dict_id(&files[i].z);
            let raw: Vec<u8> = rf.zs.decompress(&files[i].z)?;
            let out: Vec<u8> = flow_bytes(&raw)?;
            files[i].new = out.len();
            files[i].z = zs::compress_raw(&out, id)?;
        }
        None => {
            let o: Out = rewrite(rf, &path, flow_bytes)?;
            files.push(StagedFile { path: o.path, z: o.z, old: o.old, new: o.new });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf as PB;

    fn sizes(raw: &[u8]) -> Vec<(String, i32)> {
        let b: Bf = Bf::parse(raw).unwrap();
        let f: &Flow = b.flows.first().unwrap();
        CHECKS
            .iter()
            .map(|(n, _)| {
                let e: &Event = f.events.iter().find(|e| e.name == *n).unwrap();
                let v: i32 = e.kind.params().unwrap().items.iter().find_map(|(k, v)| match (k.as_str(), v) { ("Value", Val::Int(x)) => Some(*x), _ => None }).unwrap();
                (n.to_string(), v)
            })
            .collect()
    }

    #[test]
    fn the_real_fishing_flow_takes_any_fish_and_a_second_pass_refuses() {
        let p: PB = std::env::var("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|_| PB::from("C:/o/nup dump/updates/131072/romfs"));
        if !p.is_dir() {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            return;
        }
        let mut rf: Romfs = Romfs::open(&p).unwrap();
        let raw: Vec<u8> = rf.read_zs(&path_of(FILE)).unwrap();
        let old: Vec<(String, i32)> = sizes(&raw);
        assert_eq!(old, CHECKS.iter().map(|(n, v)| (n.to_string(), *v)).collect::<Vec<_>>());
        let out: Vec<u8> = flow_bytes(&raw).unwrap();
        assert!(sizes(&out).iter().all(|(_, v)| *v == 0));
        assert!(flow_bytes(&out).is_err());
    }
}
