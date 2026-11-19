use crate::ainb::{audit, Ainb};
use crate::error::{Error, Res};
use crate::qol::Have;
use crate::romfs::Romfs;
use crate::sarc::{self, Entry};
use crate::staging::Out;
use crate::zs;

pub const NODE: &str = "SelectorBSAKokkoInterruptAction";
pub const KEY: &str = "StartSpecialAttackCount";
pub const VANILLA: i32 = 4;
pub const FIRST_HIT: i32 = 1;
pub const DIR: &str = "Pack/Actor/";
pub const PREFIX: &str = "Animal_Cucco";
pub const SUPER: &str = "Animal_SuperCucco";
pub const QUEST: [&str; 1] = ["Animal_Cucco_FindOutCucco"];
pub const GRAPHS: &str = ".interruptseq.root.ainb";
pub const NOTE: &str = "Hurting a Cucco even once makes the whole flock of Cuccos come at you, where the game normally wants a few hits first. Picking a Cucco up, carrying it and throwing it do not count as hurting it. The Cuccos you catch for Anju in Kakariko Village and the ones the game has you find are left as they were. Experimental, untested in the game.";

pub fn covered(pack: &str) -> bool {
    let Some(name) = pack.strip_prefix(DIR).and_then(|p| p.strip_suffix(".pack.zs")) else {
        return false;
    };
    (name.starts_with(PREFIX) || name == SUPER) && !QUEST.contains(&name) && !name.contains("Dictionary")
}

pub fn patch_graph(raw: &[u8], to: i32) -> Res<Option<Vec<u8>>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    let mut hit: bool = false;
    for i in a.find_els(NODE) {
        let Some(g) = a.find_in(i, 0, KEY) else {
            return Err(Error::Format("cucco revenge: the Cucco hit node has no hit count input"));
        };
        if a.in_s32(i, KEY) != Some(to) {
            a.set_in_val(0, g, &to.to_le_bytes())?;
            hit = true;
        }
    }
    if !hit {
        return Ok(None);
    }
    let out: Vec<u8> = a.write()?;
    let back: Ainb = Ainb::parse(&out)?;
    if !audit(&back).is_empty() || back.find_els(NODE).iter().any(|i| back.in_s32(*i, KEY) != Some(to)) {
        return Err(Error::Format("cucco revenge: the patched graph does not read back"));
    }
    Ok(Some(out))
}

pub fn patch_entries(es: &mut [Entry], to: i32) -> Res<usize> {
    let mut n: usize = 0;
    for e in es.iter_mut().filter(|e| e.name.ends_with(GRAPHS) && e.data.windows(NODE.len()).any(|w| w == NODE.as_bytes())) {
        if let Some(d) = patch_graph(&e.data, to)? {
            e.data = d;
            n += 1;
        }
    }
    Ok(n)
}

pub fn packs(rf: &Romfs) -> Res<Vec<String>> {
    let mut out: Vec<String> = rf.list(DIR.trim_end_matches('/'))?.into_iter().filter(|p| covered(p)).collect();
    out.sort();
    Ok(out)
}

pub fn stage(rf: &mut Romfs, have: Have) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    for f in packs(rf)? {
        let z: Vec<u8> = match have(&f) {
            Some(b) => b,
            None => rf.read(&f)?,
        };
        let raw: Vec<u8> = rf.zs.decompress(&z)?;
        let mut es: Vec<Entry> = sarc::parse(&raw)?;
        if patch_entries(&mut es, FIRST_HIT)? == 0 {
            continue;
        }
        let out: Vec<u8> = sarc::write(&es)?;
        outs.push(Out { path: f, old: raw.len(), new: out.len(), z: zs::compress_raw(&out, zs::dict_of(&z))? });
    }
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ainb::show::show;

    fn game() -> Option<Romfs> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        Romfs::open(&p).ok()
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn the_quest_cuccos_and_the_compendium_entries_are_left_out() {
        assert!(covered("Pack/Actor/Animal_Cucco.pack.zs"));
        assert!(covered("Pack/Actor/Animal_Cucco_KakarikoVillage_Adult.pack.zs"));
        assert!(covered("Pack/Actor/Animal_Cucco_HrlCity.pack.zs"));
        assert!(covered("Pack/Actor/Animal_SuperCucco.pack.zs"));
        assert!(covered("Pack/Actor/Animal_Cucco_CatchCucco.pack.zs"));
        for p in ["Pack/Actor/Animal_Cucco_FindOutCucco.pack.zs", "Pack/Actor/Animal_FlockCucco.pack.zs", "Pack/Actor/Item_PocketCucco.pack.zs", "Pack/Actor/Enemy_Guay.pack.zs", "Pack/Actor/Animal_Cucco.txt", "Pack/Actor/Animal_Cucco_Dictionary.pack.zs"] {
            assert!(!covered(p), "{p}");
        }
    }

    #[test]
    fn a_bad_graph_is_refused() {
        assert!(patch_graph(b"not a graph", FIRST_HIT).is_err());
    }

    #[test]
    fn the_game_graphs_change_one_number_and_nothing_else() {
        let Some(mut rf) = game() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let es: Vec<Entry> = sarc::parse(&rf.read_zs("Pack/Actor/Animal_Cucco.pack.zs").unwrap()).unwrap();
        let g: &Entry = sarc::get(&es, "AI/Kokko.interruptseq.root.ainb").unwrap();
        let old: Ainb = Ainb::parse(&g.data).unwrap();
        let node: usize = old.find_els(NODE)[0];
        assert_eq!(old.in_s32(node, KEY), Some(VANILLA));
        let out: Vec<u8> = patch_graph(&g.data, FIRST_HIT).unwrap().unwrap();
        assert_eq!(out.len(), g.data.len());
        let new: Ainb = Ainb::parse(&out).unwrap();
        assert_eq!(new.in_s32(node, KEY), Some(FIRST_HIT));
        let (a, b): (String, String) = (show(&old), show(&new));
        let diff: Vec<(&str, &str)> = a.lines().zip(b.lines()).filter(|(x, y)| x != y).collect();
        assert_eq!(diff.len(), 1, "{diff:?}");
        assert!(diff[0].0.contains(KEY) && diff[0].0.contains("= 4") && diff[0].1.contains("= 1"));
        assert_eq!(a.lines().count(), b.lines().count());
        assert!(patch_graph(&out, FIRST_HIT).unwrap().is_none(), "a patched graph is not patched twice");
    }

    #[test]
    fn the_game_cucco_packs_are_patched_and_the_quest_packs_are_not() {
        let Some(mut rf) = game() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let list: Vec<String> = packs(&rf).unwrap();
        assert_eq!(list.len(), 7, "{list:?}");
        let outs: Vec<Out> = stage(&mut rf, &none).unwrap();
        assert_eq!(outs.len(), list.len());
        let mut graphs: usize = 0;
        for x in &outs {
            assert!(covered(&x.path) && x.new == x.old, "{}", x.path);
            let was: Vec<Entry> = sarc::parse(&rf.read_zs(&x.path).unwrap()).unwrap();
            let now: Vec<Entry> = sarc::parse(&rf.zs.decompress(&x.z).unwrap()).unwrap();
            assert_eq!(was.len(), now.len());
            for (w, n) in was.iter().zip(&now) {
                assert_eq!(w.name, n.name);
                if w.data != n.data {
                    graphs += 1;
                    assert!(n.name.ends_with(GRAPHS));
                    let a: Ainb = Ainb::parse(&n.data).unwrap();
                    assert!(a.find_els(NODE).iter().all(|i| a.in_s32(*i, KEY) == Some(FIRST_HIT)));
                }
            }
        }
        assert!(graphs >= 6, "{graphs} graphs changed");
        for q in QUEST {
            assert!(!outs.iter().any(|x| x.path.ends_with(&format!("{q}.pack.zs"))));
        }
    }
}
