use crate::ainb::{audit, Ainb};
use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::qol::Have;
use crate::romfs::Romfs;
use crate::sarc::{self, Entry};
use crate::staging::Out;
use crate::zs;

pub const PACK: &str = "Pack/Actor/Obj_HrlCity_Shop_MovingObstacle_A_01.pack.zs";
pub const GRAPH: &str = "AI/Obj_HrlCity_Shop_MovingObstacle_A_01.root.ainb";
pub const CHEMICAL: &str = "Component/ChemicalParam/Obj_ContactOnly.game__component__ChemicalParam.bgyml";
pub const DAMAGED: &str = "QueryGetLastAttackerInfoFromExternalDamageCalculator";
pub const INPUT: &str = "Input";
pub const BOOL: usize = 1;
pub const OBJECT: &str = "Object";
pub const BURN: &str = "RequestDeathIfBurned";
pub const BLAST: &str = "RequestDeathIfExploded";
pub const NOTE: &str = "The breakable obstacles in the Bombchu Bowling lanes can't be broken any more: Bombchus, bombs, fire and swords leave them standing, so you have to steer around them. Idea by Sabriel.";

pub fn patch_graph(raw: &[u8]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    let hits: Vec<usize> = a.find_els(DAMAGED);
    let mut gates: Vec<usize> = Vec::new();
    for i in 0..a.params.len() {
        if a.el_type(i) == 7 && a.find_in(i, BOOL, INPUT).and_then(|g| a.input(BOOL, g)).is_some_and(|x| hits.contains(&(x.src as usize))) {
            gates.push(i);
        }
    }
    if gates.is_empty() {
        return Err(Error::Format("bowling grass: the obstacle graph has no damage check"));
    }
    for i in gates {
        a.force_bool(i, INPUT, false)?;
    }
    let out: Vec<u8> = a.write()?;
    let back: Ainb = Ainb::parse(&out)?;
    if !audit(&back).is_empty() || !check(&back) {
        return Err(Error::Format("bowling grass: the patched graph does not read back"));
    }
    Ok(out)
}

pub fn check(a: &Ainb) -> bool {
    let hits: Vec<usize> = a.find_els(DAMAGED);
    !hits.is_empty() && (0..a.params.len()).all(|i| a.el_type(i) != 7 || !a.find_in(i, BOOL, INPUT).and_then(|g| a.input(BOOL, g)).is_some_and(|x| hits.contains(&(x.src as usize))))
}

pub fn patch_chemical(raw: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    let list: &mut Vec<Node> = y.root.get_mut(OBJECT).and_then(|o| o.as_arr_mut()).ok_or(Error::Format("bowling grass: the chemical file has no object list"))?;
    let o: &mut Node = list.first_mut().ok_or(Error::Format("bowling grass: the chemical file has an empty object list"))?;
    if o.get(BURN) != Some(&Node::Bool(true)) || o.get(BLAST) != Some(&Node::Bool(true)) {
        return Err(Error::Format("bowling grass: the chemical file is not the one expected"));
    }
    o.set(BURN, Node::Bool(false));
    o.set(BLAST, Node::Bool(false));
    let out: Vec<u8> = y.write()?;
    if out.len() != raw.len() {
        return Err(Error::Format("bowling grass: the chemical file changed size"));
    }
    Ok(out)
}

pub fn patch_entries(es: &mut [Entry]) -> Res<usize> {
    let mut n: usize = 0;
    for e in es.iter_mut() {
        if e.name == GRAPH {
            e.data = patch_graph(&e.data)?;
            n += 1;
        } else if e.name == CHEMICAL {
            e.data = patch_chemical(&e.data)?;
            n += 1;
        }
    }
    if n != 2 {
        return Err(Error::Format("bowling grass: the obstacle pack is missing a file"));
    }
    Ok(n)
}

pub fn stage(rf: &mut Romfs, have: Have) -> Res<Vec<Out>> {
    let z: Vec<u8> = match have(PACK) {
        Some(b) => b,
        None => rf.read(PACK)?,
    };
    let raw: Vec<u8> = rf.zs.decompress(&z)?;
    let mut es: Vec<Entry> = sarc::parse(&raw)?;
    patch_entries(&mut es)?;
    let out: Vec<u8> = sarc::write(&es)?;
    Ok(vec![Out { path: PACK.to_string(), old: raw.len(), new: out.len(), z: zs::compress_raw(&out, zs::dict_of(&z))? }])
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
    fn a_bad_graph_is_refused() {
        assert!(patch_graph(b"not a graph").is_err());
    }

    #[test]
    fn a_bad_chemical_file_is_refused() {
        assert!(patch_chemical(b"not a file").is_err());
        let y: Byml = Byml { root: Node::Map(vec![(OBJECT.to_string(), Node::Arr(vec![Node::Map(vec![(BURN.to_string(), Node::Bool(false)), (BLAST.to_string(), Node::Bool(true))])]))]), version: 7, big: false };
        assert!(patch_chemical(&y.write().unwrap()).is_err());
    }

    #[test]
    fn the_chemical_flags_are_turned_off_and_nothing_else() {
        let obj: Node = Node::Map(vec![("Name".to_string(), Node::Str("x".to_string())), (BURN.to_string(), Node::Bool(true)), (BLAST.to_string(), Node::Bool(true)), ("Other".to_string(), Node::Bool(true))]);
        let y: Byml = Byml { root: Node::Map(vec![(OBJECT.to_string(), Node::Arr(vec![obj]))]), version: 7, big: false };
        let out: Vec<u8> = patch_chemical(&y.write().unwrap()).unwrap();
        let back: Byml = Byml::parse(&out).unwrap();
        let o: &Node = &back.root.get(OBJECT).unwrap().as_arr().unwrap()[0];
        assert_eq!(o.get(BURN), Some(&Node::Bool(false)));
        assert_eq!(o.get(BLAST), Some(&Node::Bool(false)));
        assert_eq!(o.get("Other"), Some(&Node::Bool(true)));
        assert_eq!(o.get("Name"), Some(&Node::Str("x".to_string())));
    }

    #[test]
    fn only_the_obstacle_pack_changes_and_only_two_files_in_it() {
        let Some(mut rf) = game() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let outs: Vec<Out> = stage(&mut rf, &none).unwrap();
        assert_eq!(outs.len(), 1);
        assert_eq!(outs[0].path, PACK);
        assert!(outs[0].new <= outs[0].old);
        let was: Vec<Entry> = sarc::parse(&rf.read_zs(PACK).unwrap()).unwrap();
        let now: Vec<Entry> = sarc::parse(&rf.zs.decompress(&outs[0].z).unwrap()).unwrap();
        assert_eq!(was.len(), now.len());
        let mut changed: Vec<String> = Vec::new();
        for (w, n) in was.iter().zip(&now) {
            assert_eq!(w.name, n.name);
            if w.data != n.data {
                changed.push(n.name.clone());
            }
        }
        changed.sort();
        let mut want: Vec<String> = vec![GRAPH.to_string(), CHEMICAL.to_string()];
        want.sort();
        assert_eq!(changed, want);
        let g: Ainb = Ainb::parse(&sarc::get(&now, GRAPH).unwrap().data).unwrap();
        assert!(check(&g) && audit(&g).is_empty());
        let w: Ainb = Ainb::parse(&sarc::get(&was, GRAPH).unwrap().data).unwrap();
        assert!(!check(&w));
        assert_eq!(g.find_els("OneShotDeathRequestDeath").len(), w.find_els("OneShotDeathRequestDeath").len());
    }
}
