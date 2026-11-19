use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::murmur::murmur3_32;

pub const DIR: &str = "Attention/ClientParam/";
pub const EXT: &str = ".game__attention__ClientParam.bgyml";
pub const KIND: &str = "AttentionType";
pub const ACTION: &str = "ActionType";
pub const LOCK_ON: &str = "LockOn";
pub const LOOK_ONLY: &str = "LookOnly";
pub const NO_ACTION: &str = "None";
pub const FOES: [&str; 4] = ["EnemyCommon_", "Enemy_", "DungeonBoss", "Ganon_"];

pub fn hash(name: &str) -> u32 {
    murmur3_32(name.as_bytes(), 0)
}

pub fn is_foe(stem: &str) -> bool {
    FOES.iter().any(|p| stem.starts_with(p))
}

pub fn locks(y: &Byml) -> bool {
    y.root.get(KIND) == Some(&Node::U32(hash(LOCK_ON))) && y.root.get(ACTION) == Some(&Node::U32(hash(NO_ACTION)))
}

pub fn edit(raw: &[u8]) -> Res<Option<Vec<u8>>> {
    let mut y: Byml = Byml::parse(raw)?;
    if !locks(&y) {
        return Ok(None);
    }
    y.root.set(KIND, Node::U32(hash(LOOK_ONLY)));
    let out: Vec<u8> = y.write()?;
    if out.len() != raw.len() {
        return Err(Error::Format("difficulty: a lock-on parameter file changed size"));
    }
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(items: Vec<(&str, Node)>) -> Vec<u8> {
        let y: Byml = Byml { root: Node::Map(items.into_iter().map(|(k, v)| (k.to_string(), v)).collect()), version: 7, big: false };
        y.write().unwrap()
    }

    #[test]
    fn the_names_hash_to_the_values_in_the_game_files() {
        assert_eq!(hash(LOCK_ON), 3016662870);
        assert_eq!(hash(LOOK_ONLY), 534404998);
        assert_eq!(hash(NO_ACTION), 3069107721);
    }

    #[test]
    fn names_pick_enemy_clients() {
        for n in ["EnemyCommon_LockOn", "Enemy_Moblin_LockOn", "DungeonBossGohma_LockOn", "Ganon_LockOn", "Enemy_DungeonBoss_Twinrova"] {
            assert!(is_foe(n), "{n}");
        }
        for n in ["NpcStandard_LockOn", "Object_M_LockOn", "Check_Signboard_OnlyFront_LockOn", "Bokoblin_LockOn", "EnemyDictionary_LockOn", "Animal_Dog_LockOn", "Npc_Hylia_Sheik_Adult_LockOn_NoAction"] {
            assert!(!is_foe(n), "{n}");
        }
    }

    #[test]
    fn only_a_lock_on_client_with_no_action_changes() {
        let lock: Node = Node::U32(hash(LOCK_ON));
        let none: Node = Node::U32(hash(NO_ACTION));
        let raw: Vec<u8> = file(vec![(ACTION, none.clone()), (KIND, lock.clone()), ("ClientName", Node::Str("LockOn".to_string()))]);
        let out: Vec<u8> = edit(&raw).unwrap().unwrap();
        assert_eq!(out.len(), raw.len());
        let y: Byml = Byml::parse(&out).unwrap();
        assert_eq!(y.root.get(KIND), Some(&Node::U32(hash(LOOK_ONLY))));
        assert_eq!(y.root.get(ACTION), Some(&none));
        assert_eq!(y.root.get("ClientName").and_then(|x| x.as_str()), Some("LockOn"));
        assert!(edit(&out).unwrap().is_none());
        let talk: Vec<u8> = file(vec![(ACTION, Node::U32(hash("Talk"))), (KIND, lock.clone())]);
        assert!(edit(&talk).unwrap().is_none());
        let child: Vec<u8> = file(vec![("$parent", Node::Str("x".to_string())), ("CheckList", Node::Arr(Vec::new()))]);
        assert!(edit(&child).unwrap().is_none());
        let half: Vec<u8> = file(vec![(KIND, lock)]);
        assert!(edit(&half).unwrap().is_none());
        let aim: Vec<u8> = file(vec![(ACTION, none), (KIND, Node::U32(1122297983))]);
        assert!(edit(&aim).unwrap().is_none());
    }
}
