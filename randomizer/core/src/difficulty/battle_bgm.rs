use crate::byml::{Byml, Node};
use crate::error::Res;
use crate::sarc::{self, Entry};

pub const DIR: &str = "Component/BattlePresentationParam/";
pub const ACTOR_DIR: &str = "Actor/";
pub const ACTOR_EXT: &str = ".engine__actor__ActorParam.bgyml";
pub const COMPONENTS: &str = "Components";
pub const REF: &str = "BattlePresentationRef";
pub const PARENT: &str = "$parent";
pub const BGM: &str = "BgmName";
pub const THEME: &str = "BGM_EnemyBattle";
pub const MAX_DEPTH: usize = 8;

pub fn linked(es: &[Entry]) -> Res<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for a in es.iter().filter(|e| e.name.starts_with(ACTOR_DIR) && e.name.ends_with(ACTOR_EXT)) {
        let y: Byml = Byml::parse(&a.data)?;
        let Some(r) = y.root.get(COMPONENTS).and_then(|c| c.get(REF)).and_then(|v| v.as_str()) else {
            continue;
        };
        let n: &str = r.trim_start_matches('?');
        if n.starts_with(DIR) && !out.iter().any(|x| x == n) {
            out.push(n.to_string());
        }
    }
    Ok(out)
}

pub fn parent_name(p: &str) -> Option<String> {
    let n: &str = p.strip_prefix("Work/")?.strip_suffix(".gyml")?;
    Some(format!("{n}.bgyml"))
}

pub fn silent(es: &[Entry], name: &str, depth: usize) -> Res<bool> {
    let Some(e) = sarc::get(es, name) else {
        return Ok(false);
    };
    let y: Byml = Byml::parse(&e.data)?;
    if y.root.get(BGM).is_some() {
        return Ok(false);
    }
    let Some(p) = y.root.get(PARENT).and_then(|v| v.as_str()) else {
        return Ok(true);
    };
    if depth >= MAX_DEPTH {
        return Ok(false);
    }
    match parent_name(p) {
        Some(n) => silent(es, &n, depth + 1),
        None => Ok(false),
    }
}

pub fn edit(es: &[Entry]) -> Res<Vec<(String, Vec<u8>)>> {
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    for name in linked(es)? {
        if !silent(es, &name, 0)? {
            continue;
        }
        let Some(e) = sarc::get(es, &name) else {
            continue;
        };
        let mut y: Byml = Byml::parse(&e.data)?;
        y.root.set(BGM, Node::Str(THEME.to_string()));
        out.push((name, y.write()?));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(parent: Option<&str>, bgm: Option<&str>) -> Vec<u8> {
        let mut m: Vec<(String, Node)> = vec![("Other".to_string(), Node::I32(1))];
        if let Some(p) = parent {
            m.push((PARENT.to_string(), Node::Str(p.to_string())));
        }
        if let Some(b) = bgm {
            m.push((BGM.to_string(), Node::Str(b.to_string())));
        }
        m.sort_by(|a, b| a.0.cmp(&b.0));
        Byml { root: Node::Map(m), version: 7, big: false }.write().unwrap()
    }

    fn actor(r: &str) -> Entry {
        let c: Node = Node::Map(vec![(REF.to_string(), Node::Str(r.to_string()))]);
        let y: Byml = Byml { root: Node::Map(vec![(COMPONENTS.to_string(), c)]), version: 7, big: false };
        Entry { name: format!("{ACTOR_DIR}Enemy_X{ACTOR_EXT}"), data: y.write().unwrap() }
    }

    fn bpp(n: &str, parent: Option<&str>, bgm: Option<&str>) -> Entry {
        Entry { name: format!("{DIR}{n}.game__component__BattlePresentationParam.bgyml"), data: file(parent, bgm) }
    }

    fn up(n: &str) -> String {
        format!("Work/{DIR}{n}.game__component__BattlePresentationParam.gyml")
    }

    fn at(n: &str) -> String {
        format!("?{DIR}{n}.game__component__BattlePresentationParam.bgyml")
    }

    fn bgm_of(b: &[u8]) -> Option<String> {
        Byml::parse(b).unwrap().root.get(BGM).and_then(|v| v.as_str()).map(|s| s.to_string())
    }

    #[test]
    fn a_quiet_root_gets_the_battle_theme() {
        let es: Vec<Entry> = vec![actor(&at("Quiet")), bpp("Quiet", None, None)];
        let r: Vec<(String, Vec<u8>)> = edit(&es).unwrap();
        assert_eq!(r.len(), 1);
        let (n, b) = &r[0];
        assert!(n.contains("Quiet"));
        assert_eq!(bgm_of(b).as_deref(), Some(THEME));
        assert!(b.len() > es[1].data.len());
        let y: Byml = Byml::parse(b).unwrap();
        assert_eq!(y.root.get("Other"), Some(&Node::I32(1)));
    }

    #[test]
    fn the_edit_is_identical_for_identical_copies() {
        let a: Vec<Entry> = vec![actor(&at("Quiet")), bpp("Quiet", None, None)];
        let b: Vec<Entry> = vec![bpp("Quiet", None, None), actor(&at("Quiet"))];
        assert_eq!(edit(&a).unwrap()[0].1, edit(&b).unwrap()[0].1);
    }

    #[test]
    fn a_file_that_names_a_bgm_is_left_alone() {
        let es: Vec<Entry> = vec![actor(&at("Boss")), bpp("Boss", None, Some("BGM_DungeonBoss"))];
        assert!(edit(&es).unwrap().is_empty());
    }

    #[test]
    fn a_file_that_inherits_a_bgm_is_left_alone() {
        let es: Vec<Entry> = vec![actor(&at("Mid")), bpp("Mid", Some(&up("Common")), None), bpp("Common", Some(&up("Root")), Some(THEME)), bpp("Root", None, None)];
        assert!(edit(&es).unwrap().is_empty());
    }

    #[test]
    fn a_file_that_inherits_only_silence_is_edited_itself() {
        let es: Vec<Entry> = vec![actor(&at("Mid")), bpp("Mid", Some(&up("Root")), None), bpp("Root", None, None)];
        let r: Vec<(String, Vec<u8>)> = edit(&es).unwrap();
        let (n, b) = &r[0];
        assert!(n.contains("Mid") && bgm_of(b).as_deref() == Some(THEME));
    }

    #[test]
    fn an_unlinked_unknown_or_missing_file_is_left_alone() {
        assert!(edit(&[bpp("Quiet", None, None)]).unwrap().is_empty());
        assert!(edit(&[actor(""), bpp("Quiet", None, None)]).unwrap().is_empty());
        assert!(edit(&[actor(&at("Gone"))]).unwrap().is_empty());
        assert!(edit(&[actor(&at("Mid")), bpp("Mid", Some(&up("Away")), None)]).unwrap().is_empty());
    }

    #[test]
    fn every_distinct_link_in_a_pack_is_followed() {
        let es: Vec<Entry> = vec![actor(&at("Quiet")), Entry { name: format!("{ACTOR_DIR}Enemy_Y{ACTOR_EXT}"), data: actor(&at("Quiet")).data }, bpp("Quiet", None, None)];
        assert_eq!(linked(&es).unwrap().len(), 1);
        assert_eq!(edit(&es).unwrap().len(), 1);
    }

    #[test]
    fn parent_paths_map_to_pack_names() {
        assert_eq!(parent_name("Work/A/B.x.gyml").as_deref(), Some("A/B.x.bgyml"));
        assert_eq!(parent_name("A/B.x.gyml"), None);
    }

    #[test]
    fn the_game_edits_exactly_the_leever_and_moblin_packs() {
        let Some(p) = std::env::var_os("OOTRR_ROMFS") else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: crate::romfs::Romfs = crate::romfs::Romfs::open(std::path::Path::new(&p)).unwrap();
        let mut hit: Vec<(String, Vec<String>)> = Vec::new();
        for f in rf.list("Pack/Actor").unwrap() {
            let es: Vec<Entry> = sarc::parse(&rf.read_zs(&f).unwrap()).unwrap();
            let r: Vec<(String, Vec<u8>)> = edit(&es).unwrap();
            if !r.is_empty() {
                hit.push((f.rsplit('/').next().unwrap().to_string(), r.into_iter().map(|x| x.0).collect()));
            }
        }
        let want: Vec<String> = ["Enemy_Leever", "Enemy_Leever_Big", "Enemy_Leever_Fixed", "Enemy_Moblin", "Enemy_Moblin_Club"].iter().map(|n| format!("{n}.pack.zs")).collect();
        assert_eq!(hit.iter().map(|h| h.0.clone()).collect::<Vec<String>>(), want);
        let file: String = format!("{DIR}EnemyNoBattleBgm.game__component__BattlePresentationParam.bgyml");
        assert!(hit.iter().all(|h| h.1 == vec![file.clone()]), "{hit:?}");
    }
}
