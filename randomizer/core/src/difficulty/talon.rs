use std::collections::BTreeSet as Set;

use crate::bfevfl::{Bf, Flow, Kind, Val};
use crate::byml::{Byml, Node};
use crate::difficulty::flows;
use crate::difficulty::minions::{fresh, hashes};
use crate::error::{Error, Res};
use crate::msbt::{self, Msbt, Tok};
use crate::murmur::murmur3_32;
use crate::qol::{edit_zs, Have};
use crate::romfs::Romfs;
use crate::sarc::{self, Entry};
use crate::staging::Out;
use crate::zs;

pub const FLOW_FILE: &str = "Event/EventFlow/Minigame_FindOutCucco.bfevfl.zs";
pub const FLOW: &str = "Minigame_FindOutCucco";
pub const TIMER_ACTOR: &str = "ChallengeSystemActor";
pub const TIMER_ACTION: &str = "EventTriggerMiniGameTimerSetting";
pub const LIMIT: &str = "TimerLimit";
pub const PACK: &str = "Pack/Actor/Animal_Cucco_FindOutCucco.pack.zs";
pub const WANDER: &str = "Game/BSA/ActionSeqContainer/Animal_Cucco_FindOutCucco_Wander.game__bsa__ActionSeqContainer.bgyml";
pub const SEQS: &str = "ActionSeqs";
pub const MOVE_MIN: &str = "MoveIntervalMin";
pub const MOVE_MAX: &str = "MoveIntervalMax";
pub const NO_MOVE: &str = "NoMoveTime";
pub const VANILLA_MIN: f32 = 100.0;
pub const VANILLA_MAX: f32 = 600.0;
pub const VANILLA_NO_MOVE: f32 = 60.0;
pub const FAST_MIN: f32 = 5.0;
pub const FAST_MAX: f32 = 40.0;
pub const FAST_NO_MOVE: f32 = 5.0;
pub const MAP: &str = "Banc/HyruleField/HyruleField_Child_Static.bcett.byml.zs";
pub const COW: &str = "Animal_Cucco_FindOutCucco";
pub const NAME: &str = "Animal_Cucco_";
pub const COPIES: [(f32, f32, f32); 5] = [(-52.6, 192.6, 0.7), (-57.0, 195.0, -1.2), (-54.9, 199.8, 2.4), (-52.0, 198.2, -0.4), (-56.4, 198.6, 1.9)];
pub const FLOOR: f32 = 150.7;
pub const ORIGINAL: usize = 10;
pub const TALON_MSG: &str = "EventFlowMsg/Npc_Hylia_Talon_Child.msbt";
pub const TALON_LINES: [&str; 2] = ["Talk_02", "Talk_07"];
pub const OLD_SECONDS: &str = "30";
pub const NEW_SECONDS: &str = "20";

pub const NOTE: &str = "In Talon's cucco game at Lon Lon Ranch, 5 more ordinary cuccos join the 10 (so 15 decoys and the same 3 super cuccos), the cuccos spend much less time standing still after they land, and you get 20 seconds instead of 30 (Talon says so too).";

fn flow_of<'a>(b: &'a mut Bf) -> Res<&'a mut Flow> {
    flows::flow(b, FLOW)
}

pub fn shorten(raw: &[u8]) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    let f: &mut Flow = flow_of(&mut b)?;
    let mut hit: usize = 0;
    for i in 0..f.events.len() {
        let named: bool = match &f.events[i].kind {
            Kind::Action { actor, act, .. } => f.actors.get(*actor as usize).is_some_and(|a| a.name == TIMER_ACTOR && a.actions.get(*act as usize).is_some_and(|s| s == TIMER_ACTION)),
            _ => false,
        };
        if !named {
            continue;
        }
        let Some(c) = f.events[i].kind.params_mut() else {
            continue;
        };
        for (k, v) in c.items.iter_mut() {
            if k != LIMIT {
                continue;
            }
            match v {
                Val::Int(n) => *n = *n * 2 / 3,
                Val::Float(n) => *n *= 2.0 / 3.0,
                _ => return Err(Error::Format("talon cuccos: the time limit is not a number")),
            }
            hit += 1;
        }
    }
    if hit != 1 {
        return Err(Error::Msg(format!("talon cuccos: expected one time limit, found {hit}")));
    }
    flows::finish(&b)
}

pub fn limit(raw: &[u8]) -> Res<Option<i32>> {
    let mut b: Bf = Bf::parse(raw)?;
    let f: &mut Flow = flow_of(&mut b)?;
    for e in f.events.iter() {
        if let Some(Val::Int(n)) = e.kind.params().and_then(|c| c.get(LIMIT)) {
            return Ok(Some(*n));
        }
    }
    Ok(None)
}

pub fn quicken(raw: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    let seqs: &mut Vec<Node> = y.root.get_mut(SEQS).and_then(|a| a.as_arr_mut()).ok_or(Error::Format("talon cuccos: the wander file has no action list"))?;
    let mut n: usize = 0;
    for s in seqs.iter_mut() {
        if s.get(MOVE_MIN) != Some(&Node::F32(VANILLA_MIN)) || s.get(MOVE_MAX) != Some(&Node::F32(VANILLA_MAX)) || s.get(NO_MOVE) != Some(&Node::F32(VANILLA_NO_MOVE)) {
            return Err(Error::Format("talon cuccos: the wander file is not the one expected"));
        }
        s.set(MOVE_MIN, Node::F32(FAST_MIN));
        s.set(MOVE_MAX, Node::F32(FAST_MAX));
        s.set(NO_MOVE, Node::F32(FAST_NO_MOVE));
        n += 1;
    }
    if n == 0 {
        return Err(Error::Format("talon cuccos: the wander file has no action"));
    }
    let out: Vec<u8> = y.write()?;
    if out.len() != raw.len() {
        return Err(Error::Format("talon cuccos: the wander file changed size"));
    }
    Ok(out)
}

pub fn patch_entries(es: &mut [Entry]) -> Res<usize> {
    let e: &mut Entry = es.iter_mut().find(|e| e.name == WANDER).ok_or(Error::Format("talon cuccos: the cucco pack has no wander file"))?;
    e.data = quicken(&e.data)?;
    Ok(1)
}

fn cuccos(y: &Byml) -> Vec<usize> {
    y.root.get("Actors").and_then(|a| a.as_arr()).map(|a| (0..a.len()).filter(|&i| a[i].get("Gyaml").and_then(|g| g.as_str()) == Some(COW)).collect()).unwrap_or_default()
}

fn copy(like: &Node, name: &str, hash: u64, at: (f32, f32, f32)) -> Res<Node> {
    let mut n: Node = like.clone();
    n.remove("Links");
    let id: Node = Node::Map(vec![("ID".to_string(), Node::U64(hash))]);
    let ok: bool = n.set("Name", Node::Str(name.to_string()))
        && n.set("Hash", Node::U64(hash))
        && n.set("Phive", Node::Map(vec![("Placement".to_string(), id)]))
        && n.set("SRTHash", Node::U32(murmur3_32(&hash.to_le_bytes(), 0)))
        && n.set("Translate", Node::Arr(vec![Node::F32(at.0), Node::F32(FLOOR), Node::F32(at.1)]))
        && n.set("Rotate", Node::Arr(vec![Node::F32(0.0), Node::F32(at.2), Node::F32(0.0)]));
    if !ok {
        return Err(Error::Format("talon cuccos: a cucco placement lacks a field"));
    }
    Ok(n)
}

pub fn crowd(raw: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    let found: Vec<usize> = cuccos(&y);
    if found.len() != ORIGINAL {
        return Err(Error::Msg(format!("talon cuccos: expected {ORIGINAL} cuccos in the ranch, found {}", found.len())));
    }
    let mut used: Set<u64> = Set::new();
    hashes(&y, &mut used);
    let like: Node = y.root.get("Actors").and_then(|a| a.as_arr()).map(|a| a[found[0]].clone()).ok_or(Error::Format("talon cuccos: the map has no actor list"))?;
    let mut more: Vec<Node> = Vec::new();
    for (i, at) in COPIES.iter().enumerate() {
        let h: u64 = fresh(&format!("ootrr/talon/{i}"), &mut used);
        more.push(copy(&like, &format!("{NAME}{}", ORIGINAL + 1 + i), h, *at)?);
    }
    y.root.get_mut("Actors").and_then(|a| a.as_arr_mut()).ok_or(Error::Format("talon cuccos: the map has no actor list"))?.extend(more);
    y.write()
}

fn renumber(t: &str) -> String {
    let b: &[u8] = t.as_bytes();
    let mut out: String = String::new();
    let mut i: usize = 0;
    while i < t.len() {
        let here: bool = t[i..].starts_with(OLD_SECONDS) && (i == 0 || !b[i - 1].is_ascii_digit()) && !b.get(i + OLD_SECONDS.len()).is_some_and(|c| c.is_ascii_digit());
        if here {
            out.push_str(NEW_SECONDS);
            i += OLD_SECONDS.len();
        } else {
            let c: char = t[i..].chars().next().unwrap_or(' ');
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

pub fn apply_text(raw: &[u8]) -> Res<Vec<u8>> {
    let mut es: Vec<Entry> = sarc::parse(raw)?;
    let mut m: Msbt = msbt::parse(&sarc::get(&es, TALON_MSG).ok_or(Error::Format("talon cuccos: the language archive has no Talon texts"))?.data)?;
    for l in TALON_LINES {
        let mut toks: Vec<Tok> = m.text(l)?;
        for t in toks.iter_mut() {
            if let Tok::Text(x) = t {
                *x = renumber(x);
            }
        }
        m.set_tokens(l, &toks)?;
    }
    sarc::set(&mut es, TALON_MSG, m.write()?);
    sarc::write(&es)
}

pub fn stage(rf: &mut Romfs, have: Have) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    outs.push(edit_zs(rf, have, FLOW_FILE, shorten)?);
    outs.push(edit_zs(rf, have, MAP, crowd)?);
    for l in crate::hints::languages(rf)? {
        outs.push(edit_zs(rf, have, &l, apply_text)?);
    }
    let z: Vec<u8> = match have(PACK) {
        Some(b) => b,
        None => rf.read(PACK)?,
    };
    let raw: Vec<u8> = rf.zs.decompress(&z)?;
    let mut es: Vec<Entry> = sarc::parse(&raw)?;
    patch_entries(&mut es)?;
    let out: Vec<u8> = sarc::write(&es)?;
    outs.push(Out { path: PACK.to_string(), old: raw.len(), new: out.len(), z: zs::compress_raw(&out, zs::dict_of(&z))? });
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Option<Romfs> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        Romfs::open(&p).ok()
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn bad_files_are_refused() {
        assert!(shorten(b"not a flow").is_err());
        assert!(quicken(b"not a file").is_err());
        assert!(crowd(b"not a map").is_err());
    }

    #[test]
    fn a_wander_file_with_other_numbers_is_refused() {
        let s: Node = Node::Map(vec![(MOVE_MIN.to_string(), Node::F32(1.0)), (MOVE_MAX.to_string(), Node::F32(VANILLA_MAX)), (NO_MOVE.to_string(), Node::F32(VANILLA_NO_MOVE))]);
        let y: Byml = Byml { root: Node::Map(vec![(SEQS.to_string(), Node::Arr(vec![s]))]), version: 7, big: false };
        assert!(quicken(&y.write().unwrap()).is_err());
    }

    #[test]
    fn the_wander_numbers_are_shortened_and_nothing_else() {
        let s: Node = Node::Map(vec![
            (MOVE_MIN.to_string(), Node::F32(VANILLA_MIN)),
            (MOVE_MAX.to_string(), Node::F32(VANILLA_MAX)),
            (NO_MOVE.to_string(), Node::F32(VANILLA_NO_MOVE)),
            ("WanderRange".to_string(), Node::F32(3.0)),
        ]);
        let y: Byml = Byml { root: Node::Map(vec![(SEQS.to_string(), Node::Arr(vec![s]))]), version: 7, big: false };
        let out: Vec<u8> = quicken(&y.write().unwrap()).unwrap();
        let back: Byml = Byml::parse(&out).unwrap();
        let o: &Node = &back.root.get(SEQS).unwrap().as_arr().unwrap()[0];
        assert_eq!(o.get(MOVE_MIN), Some(&Node::F32(FAST_MIN)));
        assert_eq!(o.get(MOVE_MAX), Some(&Node::F32(FAST_MAX)));
        assert_eq!(o.get(NO_MOVE), Some(&Node::F32(FAST_NO_MOVE)));
        assert_eq!(o.get("WanderRange"), Some(&Node::F32(3.0)));
    }

    #[test]
    fn the_time_limit_is_twenty_seconds() {
        let Some(mut rf) = game() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let z: Vec<u8> = rf.read(FLOW_FILE).unwrap();
        let raw: Vec<u8> = rf.zs.decompress(&z).unwrap();
        assert_eq!(limit(&raw).unwrap(), Some(1800));
        let out: Vec<u8> = shorten(&raw).unwrap();
        assert_eq!(limit(&out).unwrap(), Some(1200));
    }

    #[test]
    fn five_cuccos_join_and_the_super_cuccos_and_event_actors_are_untouched() {
        let Some(mut rf) = game() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let z: Vec<u8> = rf.read(MAP).unwrap();
        let raw: Vec<u8> = rf.zs.decompress(&z).unwrap();
        let before: Byml = Byml::parse(&raw).unwrap();
        let out: Vec<u8> = crowd(&raw).unwrap();
        let after: Byml = Byml::parse(&out).unwrap();
        assert_eq!(cuccos(&before).len(), ORIGINAL);
        assert_eq!(cuccos(&after).len(), ORIGINAL + COPIES.len());
        let a: &[Node] = after.root.get("Actors").and_then(|a| a.as_arr()).unwrap();
        let b: &[Node] = before.root.get("Actors").and_then(|a| a.as_arr()).unwrap();
        assert_eq!(a.len(), b.len() + COPIES.len());
        assert_eq!(&a[..b.len()], b);
        let mut seen: Set<u64> = Set::new();
        for n in a {
            if let Some(Node::U64(h)) = n.get("Hash") {
                assert!(seen.insert(*h), "duplicate hash");
            }
        }
        assert!(crowd(&out).is_err());
    }

    #[test]
    fn the_numbers_in_talons_lines_become_twenty_and_the_rupee_lines_are_kept() {
        let Some(mut rf) = game() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let langs: Vec<String> = crate::hints::languages(&rf).unwrap();
        assert!(!langs.is_empty());
        let mut english: usize = 0;
        for l in langs {
            let raw: Vec<u8> = rf.zs.decompress(&rf.read(&l).unwrap()).unwrap();
            let out: Vec<u8> = apply_text(&raw).unwrap();
            let before: Msbt = msbt::parse(&sarc::get(&sarc::parse(&raw).unwrap(), TALON_MSG).unwrap().data).unwrap();
            let after: Msbt = msbt::parse(&sarc::get(&sarc::parse(&out).unwrap(), TALON_MSG).unwrap().data).unwrap();
            for (n, _) in before.names() {
                let (a, b): (String, String) = (msbt::to_markup(&before.text(n).unwrap()), msbt::to_markup(&after.text(n).unwrap()));
                if TALON_LINES.contains(&n) {
                    assert_ne!(a, b, "{l}: {n}");
                    english += (l.contains("USen") && b.contains(NEW_SECONDS)) as usize;
                } else {
                    assert_eq!(a, b, "{l}: {n}");
                }
            }
        }
        assert_eq!(english, 2);
    }

    #[test]
    fn only_a_whole_number_is_replaced() {
        assert_eq!(renumber("Ya got 30 seconds."), "Ya got 20 seconds.");
        assert_eq!(renumber("130 or 300 or 30"), "130 or 300 or 20");
    }

    #[test]
    fn the_stage_changes_the_flow_the_map_the_texts_and_the_pack() {
        let Some(mut rf) = game() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let outs: Vec<Out> = stage(&mut rf, &none).unwrap();
        let paths: Vec<&str> = outs.iter().map(|o| o.path.as_str()).collect();
        let langs: Vec<String> = crate::hints::languages(&rf).unwrap();
        assert_eq!(paths[..2], [FLOW_FILE, MAP]);
        assert_eq!(&paths[2..2 + langs.len()], langs.iter().map(|l| l.as_str()).collect::<Vec<&str>>());
        assert_eq!(paths[2 + langs.len()], PACK);
        assert_eq!(outs.len(), 3 + langs.len());
        let es: Vec<Entry> = sarc::parse(&rf.zs.decompress(&outs[outs.len() - 1].z).unwrap()).unwrap();
        let was: Vec<Entry> = sarc::parse(&rf.read_zs(PACK).unwrap()).unwrap();
        let changed: Vec<&str> = was.iter().zip(&es).filter(|(w, n)| w.data != n.data).map(|(_, n)| n.name.as_str()).collect();
        assert_eq!(changed, vec![WANDER]);
    }
}
