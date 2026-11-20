use std::collections::BTreeSet as BS;

use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::pickup_pack::kit::AT_MAX;
use crate::pot_slice::targets::Give;
use crate::romfs::Romfs;
use crate::sarc;
use crate::staging::{rewrite, Out};

pub const ACTOR: &str = "Obj_Bomb";
pub const PACK: &str = "Pack/Actor/Obj_Bomb.pack.zs";
pub const PARAM: &str = "Component/PouchContentParam/Obj_Bomb.game__component__PouchContentParam.bgyml";
pub const ACTORS: [&str; 5] = [ACTOR, "Obj_ArrowBundle_Small", "Obj_ArrowBundle_Medium", "Obj_ArrowBundle_Large", "Obj_DekuSeed"];
const PARAMS: &str = "Component/PouchContentParam/";

/// Whether the pickup `actor` (or the skulltula kit copy of one) is ammo whose get event would
/// set up a bag the player does not have.
pub fn quieted(actor: &str) -> bool {
    let base: &str = crate::pickup_pack::kit::parent_of(actor).unwrap_or(actor);
    ACTORS.contains(&base)
}

/// Stops every pouch file of a pack from playing the get event at a full pouch. A skulltula kit
/// copy carries its own copy of the pickup's pouch file, so the kit gets the same edit.
pub fn quiet_all(raw: &[u8]) -> Res<(Vec<u8>, bool)> {
    let names: Vec<String> = sarc::parse(raw)?.into_iter().map(|e| e.name).filter(|n| n.starts_with(PARAMS)).collect();
    let mut out: Vec<u8> = raw.to_vec();
    let mut hit: bool = false;
    for n in &names {
        let (b, h): (Vec<u8>, bool) = quiet_in(&out, n)?;
        out = b;
        hit |= h;
    }
    Ok((out, hit))
}

pub fn pack_of(actor: &str) -> String {
    format!("Pack/Actor/{actor}.pack.zs")
}

pub fn param_of(actor: &str) -> String {
    format!("Component/PouchContentParam/{actor}.game__component__PouchContentParam.bgyml")
}

pub fn needed(ts: &[Give]) -> bool {
    ts.iter().any(|t| ACTORS.contains(&t.actor.as_str()))
}

pub fn wanted<'a>(actors: impl Iterator<Item = &'a str>) -> Vec<&'static str> {
    let got: BS<&str> = actors.collect();
    ACTORS.iter().copied().filter(|a| got.contains(a)).collect()
}

pub fn quiet(raw: &[u8]) -> Res<(Vec<u8>, bool)> {
    quiet_in(raw, PARAM)
}

pub fn quiet_in(raw: &[u8], param: &str) -> Res<(Vec<u8>, bool)> {
    let mut es: Vec<sarc::Entry> = sarc::parse(raw)?;
    let mut hit: bool = false;
    for e in es.iter_mut().filter(|e| e.name == param) {
        let mut y: Byml = Byml::parse(&e.data)?;
        if y.root.get(AT_MAX) == Some(&Node::Bool(true)) {
            y.root.set(AT_MAX, Node::Bool(false));
            e.data = y.write()?;
            hit = true;
        }
    }
    Ok((sarc::write(&es)?, hit))
}

pub fn stage(rf: &mut Romfs) -> Res<Out> {
    stage_actor(rf, ACTOR)
}

pub fn stage_actor(rf: &mut Romfs, actor: &str) -> Res<Out> {
    let param: String = param_of(actor);
    let mut hit: bool = false;
    let o: Out = rewrite(rf, &pack_of(actor), |raw| {
        let (b, h): (Vec<u8>, bool) = quiet_in(raw, &param)?;
        hit = h;
        Ok(b)
    })?;
    if !hit {
        return Err(Error::Msg(format!("bomb pickup: {param} no longer plays the get event at a full pouch, the game files may have changed")));
    }
    Ok(o)
}

pub fn stage_all(rf: &mut Romfs, actors: &[&str]) -> Res<Vec<Out>> {
    actors.iter().map(|a| stage_actor(rf, a)).collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf as PB;

    use super::*;
    use crate::pot_slice::data::Kind;

    fn param(at_max: bool) -> Vec<u8> {
        let y: Byml = Byml {
            root: Node::Map(vec![
                (AT_MAX.to_string(), Node::Bool(at_max)),
                ("PouchActor".to_string(), Node::Str("Work/Actor/Item_Bomb.engine__actor__ActorParam.gyml".to_string())),
            ]),
            version: 7,
            big: false,
        };
        y.write().unwrap()
    }

    fn give(actor: &str) -> Give {
        Give { name: "Pot".to_string(), kind: Kind::Pot, hash: 1, files: vec!["f".to_string()], was: String::new(), item: "Bombs (5)".to_string(), actor: actor.to_string() }
    }

    #[test]
    fn only_the_obj_bomb_pouch_file_stops_playing_the_get_event_at_a_full_pouch() {
        let other: String = "Component/PouchContentParam/Item_Bomb.game__component__PouchContentParam.bgyml".to_string();
        let es: Vec<sarc::Entry> = vec![sarc::Entry { name: PARAM.to_string(), data: param(true) }, sarc::Entry { name: other.clone(), data: param(true) }];
        let (out, hit): (Vec<u8>, bool) = quiet(&sarc::write(&es).unwrap()).unwrap();
        assert!(hit);
        let back: Vec<sarc::Entry> = sarc::parse(&out).unwrap();
        let get = |n: &str| -> Byml { Byml::parse(&sarc::get(&back, n).unwrap().data).unwrap() };
        assert_eq!(get(PARAM).root.get(AT_MAX), Some(&Node::Bool(false)));
        assert!(get(PARAM).root.get("PouchActor").is_some());
        assert_eq!(get(&other).root.get(AT_MAX), Some(&Node::Bool(true)));
        let (_, again): (Vec<u8>, bool) = quiet(&out).unwrap();
        assert!(!again);
    }

    #[test]
    fn every_arrow_pouch_file_is_quieted_on_its_own_and_the_others_stay() {
        for a in &ACTORS[1..] {
            let other: String = param_of(ACTORS.iter().find(|x| *x != a && **x != ACTOR).unwrap());
            let es: Vec<sarc::Entry> = vec![sarc::Entry { name: param_of(a), data: param(true) }, sarc::Entry { name: other.clone(), data: param(true) }];
            let (out, hit): (Vec<u8>, bool) = quiet_in(&sarc::write(&es).unwrap(), &param_of(a)).unwrap();
            assert!(hit, "{a}");
            let back: Vec<sarc::Entry> = sarc::parse(&out).unwrap();
            let get = |n: &str| -> Byml { Byml::parse(&sarc::get(&back, n).unwrap().data).unwrap() };
            assert_eq!(get(&param_of(a)).root.get(AT_MAX), Some(&Node::Bool(false)), "{a}");
            assert_eq!(get(&other).root.get(AT_MAX), Some(&Node::Bool(true)), "{a}");
            assert!(!quiet_in(&out, &param_of(a)).unwrap().1, "{a}");
        }
    }

    #[test]
    fn the_edit_is_needed_only_when_a_breakable_drops_a_bomb_or_an_arrow_bundle() {
        assert!(needed(&[give("Obj_Rupee_Red"), give(ACTOR)]));
        for a in &ACTORS[1..] {
            assert!(needed(&[give("Obj_Rupee_Red"), give(a)]), "{a}");
        }
        assert!(!needed(&[give("Obj_Rupee_Red"), give("Obj_DekuNut")]));
        assert!(!needed(&[]));
    }

    #[test]
    fn wanted_lists_each_quieted_pickup_once_whatever_placed_it() {
        let got: Vec<&str> = wanted(["Obj_Rupee_Red", "Obj_ArrowBundle_Large", "Obj_Bomb", "TBox_ArrowBundle_Large", "Obj_Bomb", "Obj_DekuSeed"].into_iter());
        assert_eq!(got, vec!["Obj_Bomb", "Obj_ArrowBundle_Large", "Obj_DekuSeed"]);
        assert!(quieted("Obj_Bomb_GH") && quieted("Obj_DekuSeed") && !quieted("Obj_Rupee_Red_GH") && !quieted("Obj_DekuNut"));
        assert!(wanted(["Obj_Rupee_Red", "Obj_Bombchu", "Enemy_Skulltula_Gold"].into_iter()).is_empty());
        assert_eq!(pack_of("Obj_ArrowBundle_Small"), "Pack/Actor/Obj_ArrowBundle_Small.pack.zs");
        assert_eq!(param_of(ACTOR), PARAM);
        assert_eq!(pack_of(ACTOR), PACK);
    }

    #[test]
    fn every_bomb_and_arrow_pickup_item_lands_on_an_actor_the_edit_covers() {
        use crate::pot_slice::pickup::PICKUPS;
        let mut n: usize = 0;
        for (item, actor) in PICKUPS {
            if !(actor == ACTOR || actor.starts_with("Obj_ArrowBundle_")) {
                continue;
            }
            assert!(ACTORS.contains(&actor), "{item} drops {actor}");
            assert_eq!(crate::slice::pickups::ground_actor(item), Some(actor), "{item}");
            n += 1;
        }
        assert_eq!(n, 7);
    }

    #[test]
    fn every_bomb_and_arrow_item_on_a_pot_or_a_rock_drops_the_actor_the_edit_covers() {
        use std::collections::BTreeMap as BM;
        use crate::settings::seed::SeedFile;
        use crate::settings::Settings;
        use crate::slice::data::{Data, State};
        let d: Data = Data::load().unwrap();
        let mut pots: Vec<String> = d.pots.state.iter().filter(|(n, s)| **s == State::Eligible && d.pots.kind_of(n) == Some(Kind::Pot)).map(|(n, _)| n.clone()).collect();
        pots.sort();
        for item in ["Bomb (1)", "Bombs (5)", "Bombs (10)", "Bombs (20)"] {
            let m: BM<String, String> = [(pots[0].clone(), item.to_string())].into_iter().collect();
            let s: SeedFile = SeedFile::new("s", &Settings::default(), "g", m).unwrap();
            let ts: Vec<Give> = crate::pot_slice::targets::targets(&d, &s).unwrap();
            assert!(!ts.is_empty() && ts.iter().all(|t| t.actor == ACTOR), "{item}");
            assert!(needed(&ts), "{item}");
        }
        for (item, actor) in [("Arrows (5)", "Obj_ArrowBundle_Small"), ("Arrows (10)", "Obj_ArrowBundle_Medium"), ("Arrows (30)", "Obj_ArrowBundle_Large")] {
            let m: BM<String, String> = [(pots[0].clone(), item.to_string())].into_iter().collect();
            let s: SeedFile = SeedFile::new("s", &Settings::default(), "g", m).unwrap();
            let ts: Vec<Give> = crate::pot_slice::targets::targets(&d, &s).unwrap();
            assert!(!ts.is_empty() && ts.iter().all(|t| t.actor == actor), "{item}");
            assert!(needed(&ts), "{item}");
        }
        let m: BM<String, String> = [(pots[0].clone(), "Rupees (20)".to_string())].into_iter().collect();
        let s: SeedFile = SeedFile::new("s", &Settings::default(), "g", m).unwrap();
        assert!(!needed(&crate::pot_slice::targets::targets(&d, &s).unwrap()));
    }

    #[test]
    fn the_shipped_bomb_pickup_plays_its_get_event_at_a_full_pouch_until_the_edit() {
        let p: PB = ootrr_game::test_romfs::path();
        if !p.join("Pack").join("ZsDic.pack.zs").is_file() {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            return;
        }
        let mut rf: Romfs = Romfs::open(&p).unwrap();
        let raw: Vec<u8> = rf.read_zs(PACK).unwrap();
        let es: Vec<sarc::Entry> = sarc::parse(&raw).unwrap();
        let before: &sarc::Entry = sarc::get(&es, PARAM).unwrap();
        let y: Byml = Byml::parse(&before.data).unwrap();
        assert_eq!(y.root.get(AT_MAX), Some(&Node::Bool(true)));
        assert_eq!(y.root.get("PouchActor").and_then(|v| v.as_str()), Some("Work/Actor/Item_Bomb.engine__actor__ActorParam.gyml"));
        assert!(y.root.get("IsCountCheckOnFirstGet").is_none());
        let o: Out = stage(&mut rf).unwrap();
        let after: Vec<sarc::Entry> = sarc::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap();
        assert_eq!(after.len(), es.len());
        let a: &sarc::Entry = sarc::get(&after, PARAM).unwrap();
        assert!(a.data.len() <= before.data.len());
        assert_eq!(Byml::parse(&a.data).unwrap().root.get(AT_MAX), Some(&Node::Bool(false)));
        for e in &es {
            if e.name != PARAM {
                assert_eq!(sarc::get(&after, &e.name).map(|x| &x.data), Some(&e.data), "{}", e.name);
            }
        }
    }

    #[test]
    fn every_shipped_bomb_and_arrow_pickup_is_quieted_without_growing_any_file() {
        let p: PB = ootrr_game::test_romfs::path();
        if !p.join("Pack").join("ZsDic.pack.zs").is_file() {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            return;
        }
        let mut rf: Romfs = Romfs::open(&p).unwrap();
        for a in ACTORS {
            let es: Vec<sarc::Entry> = sarc::parse(&rf.read_zs(&pack_of(a)).unwrap()).unwrap();
            let name: String = param_of(a);
            let before: &sarc::Entry = sarc::get(&es, &name).unwrap_or_else(|| panic!("{a} has no {name}"));
            assert_eq!(Byml::parse(&before.data).unwrap().root.get(AT_MAX), Some(&Node::Bool(true)), "{a}");
            let o: Out = stage_actor(&mut rf, a).unwrap();
            assert_eq!(o.path, pack_of(a));
            assert!(o.new <= o.old, "{a} grew from {} to {}", o.old, o.new);
            let after: Vec<sarc::Entry> = sarc::parse(&rf.zs.decompress(&o.z).unwrap()).unwrap();
            assert_eq!(after.len(), es.len(), "{a}");
            let q: &sarc::Entry = sarc::get(&after, &name).unwrap();
            assert!(q.data.len() <= before.data.len(), "{a}");
            assert_eq!(Byml::parse(&q.data).unwrap().root.get(AT_MAX), Some(&Node::Bool(false)), "{a}");
            for e in es.iter().filter(|e| e.name != name) {
                assert_eq!(sarc::get(&after, &e.name).map(|x| &x.data), Some(&e.data), "{a} {}", e.name);
            }
        }
    }
}
