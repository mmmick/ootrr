use crate::byml::{Byml, Node};
use crate::error::{Error, Res};

pub const DIR: &str = "System/CombinationDataTableData/";
pub const EXT: &str = "_game__death__DeathReasonProcedures.pp__CombinationDataTableData.bgyml";
pub const CELLS: &str = "CellList";
pub const CELL: &str = "LifeDepleted___Data";
pub const CELL_TYPE: &str = "game__death__DeathReasonProceduresCell";
pub const KEPT: &str = "IsSetRespawnFlagToFalse";
pub const SAVED: &str = "IsSetGmdRespawnFlagToFalse";

pub const TABLES: [&str; 31] = [
    "Amos",
    "Bari",
    "Biri",
    "Bubble",
    "Dekubaba",
    "DekubabaWithered",
    "Dodongo",
    "Enemy_Floormaster",
    "Enemy_Freezard",
    "Enemy_Keese",
    "Enemy_LikeLike",
    "Enemy_Moblin",
    "Enemy_Octorok",
    "Enemy_Peahat",
    "Enemy_Peahat_Larva",
    "Enemy_Shabom",
    "Enemy_Skullwalltula",
    "Enemy_Stalfos",
    "Enemy_Stinger",
    "Enemy_Tektite",
    "Enemy_TorchSlug",
    "Enemy_Wallmaster",
    "Enemy_Wolfos",
    "Floormaster_Little",
    "Guay",
    "Leever",
    "MoblinClub",
    "Redead",
    "ShellBlade",
    "Skulltula",
    "Stalchild",
];

pub fn file(table: &str) -> String {
    format!("{DIR}{table}{EXT}")
}

pub fn files() -> Vec<String> {
    TABLES.iter().map(|t| file(t)).collect()
}

pub fn always(raw: &[u8]) -> Res<Vec<u8>> {
    let mut y: Byml = Byml::parse(raw)?;
    let cells: &mut Node = y.root.get_mut(CELLS).ok_or(Error::Format("respawn: the death table has no cell list"))?;
    if cells.get(CELL).is_none() && !cells.set(CELL, Node::Map(vec![("$type".to_string(), Node::Str(CELL_TYPE.to_string()))])) {
        return Err(Error::Format("respawn: the cell list of the death table is not a map"));
    }
    let c: &mut Node = cells.get_mut(CELL).ok_or(Error::Format("respawn: the death table has no cell for a beaten enemy"))?;
    if !c.set(KEPT, Node::Bool(false)) || !c.set(SAVED, Node::Bool(false)) {
        return Err(Error::Format("respawn: the cell for a beaten enemy is not a map"));
    }
    y.write()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(cells: Vec<(&str, Node)>) -> Vec<u8> {
        let list: Node = Node::Map(cells.into_iter().map(|(k, v)| (k.to_string(), v)).collect());
        let y: Byml = Byml { root: Node::Map(vec![("$parent".to_string(), Node::Str("Work/Parent.gyml".to_string())), (CELLS.to_string(), list)]), version: 7, big: false };
        y.write().unwrap()
    }

    fn cell(items: Vec<(&str, Node)>) -> Node {
        Node::Map(items.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    fn flags(raw: &[u8], name: &str) -> (Option<Node>, Option<Node>) {
        let y: Byml = Byml::parse(raw).unwrap();
        let c: &Node = y.root.get(CELLS).unwrap().get(name).unwrap();
        (c.get(KEPT).cloned(), c.get(SAVED).cloned())
    }

    #[test]
    fn a_beaten_enemy_is_no_longer_written_down() {
        let raw: Vec<u8> = table(vec![
            ("Burned___Data", cell(vec![("$type", Node::Str(CELL_TYPE.to_string())), (KEPT, Node::Bool(true))])),
            (CELL, cell(vec![("$type", Node::Str(CELL_TYPE.to_string())), ("DeathProcedureType", Node::U32(7)), (KEPT, Node::Bool(true))])),
        ]);
        let out: Vec<u8> = always(&raw).unwrap();
        assert_eq!(flags(&out, CELL), (Some(Node::Bool(false)), Some(Node::Bool(false))));
        assert_eq!(flags(&out, "Burned___Data"), (Some(Node::Bool(true)), None));
        let y: Byml = Byml::parse(&out).unwrap();
        assert_eq!(y.root.get(CELLS).unwrap().get(CELL).unwrap().get("DeathProcedureType"), Some(&Node::U32(7)));
        assert_eq!(y.root.get("$parent").and_then(|p| p.as_str()), Some("Work/Parent.gyml"));
        assert_eq!(always(&out).unwrap(), out);
    }

    #[test]
    fn a_table_that_only_inherits_the_cell_gets_its_own() {
        let raw: Vec<u8> = table(vec![("Burned___Data", cell(vec![("$type", Node::Str(CELL_TYPE.to_string()))]))]);
        let out: Vec<u8> = always(&raw).unwrap();
        assert_eq!(flags(&out, CELL), (Some(Node::Bool(false)), Some(Node::Bool(false))));
        let y: Byml = Byml::parse(&out).unwrap();
        assert_eq!(y.root.get(CELLS).unwrap().get(CELL).unwrap().get("$type").and_then(|t| t.as_str()), Some(CELL_TYPE));
    }

    #[test]
    fn a_file_that_is_not_a_death_table_is_refused() {
        let y: Byml = Byml { root: Node::Map(vec![("MaxLife".to_string(), Node::I32(4))]), version: 7, big: false };
        assert!(always(&y.write().unwrap()).is_err());
        assert!(always(b"nope").is_err());
    }

    #[test]
    fn the_list_is_sorted_and_names_no_boss_or_token_spider() {
        let mut t: Vec<&str> = TABLES.to_vec();
        t.sort();
        t.dedup();
        assert_eq!(t, TABLES.to_vec());
        for bad in ["Boss", "Gold", "DeadHand", "IronKnuckle", "Poe", "Scrub", "Tentacle", "EnemyCommon", "Default"] {
            assert!(!TABLES.iter().any(|x| x.contains(bad)), "{bad} is on the list");
        }
        assert_eq!(file("Guay"), "System/CombinationDataTableData/Guay_game__death__DeathReasonProcedures.pp__CombinationDataTableData.bgyml");
        assert_eq!(files().len(), TABLES.len());
    }

    const SPARED: [&str; 14] = ["MiddleBoss", "DungeonBoss", "Skulltula_Gold", "DeadHand", "IronKnuckle", "Poe", "Scrub", "SkullKid", "Anubis", "PhantomGanon", "GhostPicture", "KotakeKoume", "Gohma", "Tentacle"];

    fn ordinary(actor: &str) -> bool {
        actor.starts_with("Enemy_") && !SPARED.iter().any(|s| actor.contains(s))
    }

    fn named(raw: &[u8]) -> Vec<String> {
        let mark: &[u8] = b"CombinationDataTableData/";
        let mut out: Vec<String> = Vec::new();
        let mut i: usize = 0;
        while i + mark.len() <= raw.len() {
            if &raw[i..i + mark.len()] == mark {
                let rest: &[u8] = &raw[i + mark.len()..];
                let end: usize = rest.iter().position(|b| *b == b'.' || *b == 0).unwrap_or(rest.len());
                let s: String = String::from_utf8_lossy(&rest[..end]).to_string();
                out.push(s.strip_suffix("_game__death__DeathReasonProcedures").unwrap_or(&s).to_string());
                i += mark.len();
            } else {
                i += 1;
            }
        }
        out
    }

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        if p.is_dir() { Some(p) } else { None }
    }

    #[test]
    fn the_game_tables_change_only_where_a_beaten_enemy_is_written_down() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: crate::romfs::Romfs = crate::romfs::Romfs::open(&root).unwrap();
        let rst: crate::rsizetable::Rst = crate::rsizetable::load(&mut rf).unwrap();
        for f in files() {
            let raw: Vec<u8> = rf.read(&f).unwrap();
            assert!(rst.get(&f).is_some(), "{f} has no size table entry");
            let out: Vec<u8> = always(&raw).unwrap();
            assert!(out.len() <= raw.len() + 256, "{f} grew by {}", out.len() - raw.len());
            assert_eq!(flags(&out, CELL), (Some(Node::Bool(false)), Some(Node::Bool(false))), "{f}");
            let mut was: Byml = Byml::parse(&raw).unwrap();
            let mut now: Byml = Byml::parse(&out).unwrap();
            for y in [&mut was, &mut now] {
                y.root.get_mut(CELLS).unwrap().remove(CELL);
            }
            assert_eq!(was.root, now.root, "{f} changed outside its cell");
        }
        let model: Vec<u8> = rf.read(&file("Enemy_Shabom_AlwaysRespawn")).unwrap();
        assert_eq!(flags(&model, CELL), (Some(Node::Bool(false)), Some(Node::Bool(false))), "the game's own always respawning enemy is the model");
        let gold: Vec<u8> = rf.read(&file("Enemy_Skulltula_Gold")).unwrap();
        assert_eq!(flags(&gold, CELL).0, None, "the token spider takes its cell from the shared table");
    }

    #[test]
    fn every_actor_that_reads_a_changed_table_is_an_ordinary_enemy() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: crate::romfs::Romfs = crate::romfs::Romfs::open(&root).unwrap();
        let mut hit: std::collections::BTreeSet<String> = TABLES.iter().map(|t| t.to_string()).collect();
        let all: Vec<String> = rf.list(DIR.trim_end_matches('/')).unwrap();
        loop {
            let n: usize = hit.len();
            for f in all.iter().filter(|f| f.ends_with(EXT)) {
                let name: &str = f.rsplit('/').next().unwrap().strip_suffix(EXT).unwrap();
                let y: Byml = Byml::parse(&rf.read(f).unwrap()).unwrap();
                let up: Vec<String> = y.root.get("$parent").and_then(|p| p.as_str()).map(|p| named(p.as_bytes())).unwrap_or_default();
                if up.iter().any(|u| hit.contains(u)) {
                    hit.insert(name.to_string());
                }
            }
            if hit.len() == n {
                break;
            }
        }
        for t in ["Amos_AlwaysRespawn", "Redead_AlwaysRespawn", "Enemy_Stalfos_AlwaysRespawn", "Leever_Fixed"] {
            assert!(hit.contains(t), "{t} follows a changed table");
        }
        for t in ["EnemyCommon", "Default", "DungeonBossCommon", "Enemy_Skulltula_Gold", "BabyDodongo", "EnemyCommonRagdollDie", "Tentacle", "IronKnuckle", "DeadHand"] {
            assert!(!hit.contains(t), "{t} must stay as it is");
        }
        let mut users: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for p in rf.list(crate::difficulty::ACTOR_PACKS).unwrap().into_iter().filter(|p| p.ends_with(".pack.zs")) {
            let actor: &str = p.rsplit('/').next().unwrap().strip_suffix(".pack.zs").unwrap();
            let es: Vec<crate::sarc::Entry> = crate::sarc::parse(&rf.read_zs(&p).unwrap()).unwrap();
            for e in es.iter().filter(|e| e.name.starts_with("Component/DeathParam/")) {
                for t in named(&e.data) {
                    if hit.contains(&t) {
                        assert!(ordinary(actor), "{actor} reads the changed table {t}");
                        users.insert(actor.to_string());
                    }
                }
            }
        }
        for a in ["Enemy_Stalfos", "Enemy_Keese", "Enemy_Wolfos", "Enemy_Tektite_Blue", "Enemy_ReDead", "Enemy_Skulltula"] {
            assert!(users.contains(a), "{a} is covered");
        }
        assert!(!users.iter().any(|a| a.contains("Skulltula_Gold")));
    }
}
