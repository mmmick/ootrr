pub const SETTING: &str = "skip_first_pickup_text";
pub const NUM: u16 = 136;
pub const NOTE: &str = "Rupees, recovery hearts, magic jars, arrow bundles and Deku seeds are picked up without the first time \"You got X!\" box. Chests, quest items, equipment and every other item keep their box.";

pub const PICKUPS: [&str; 11] = [
    "Obj_Rupee_Green",
    "Obj_Rupee_Blue",
    "Obj_Rupee_Red",
    "Obj_Rupee_Purple",
    "Obj_Heart",
    "Obj_MagicJar_Small",
    "Obj_MagicJar_Large",
    "Obj_ArrowBundle_Small",
    "Obj_ArrowBundle_Medium",
    "Obj_ArrowBundle_Large",
    "Obj_DekuSeed",
];

pub const PREFIX: &str = "HavePlayedGetEvent.";

pub fn start_flags() -> Vec<(String, bool)> {
    PICKUPS.iter().map(|a| (format!("{PREFIX}{a}"), true)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> String {
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/flags.json")).unwrap()
    }

    #[test]
    fn a_new_game_marks_each_listed_pickup_as_already_shown() {
        let f: Vec<(String, bool)> = start_flags();
        assert_eq!(f.len(), PICKUPS.len());
        assert!(f.iter().all(|(n, v)| *v && n.starts_with(PREFIX)));
        assert!(f.contains(&("HavePlayedGetEvent.Obj_Rupee_Green".to_string(), true)));
    }

    #[test]
    fn every_flag_is_a_game_save_bool() {
        let r: String = rows();
        for (n, _) in start_flags() {
            let row: &str = r.split('\n').find(|l| l.contains(&format!("\"name\":\"{n}\""))).unwrap_or_else(|| panic!("{n} is not a game flag"));
            assert!(row.contains("\"type\":\"Bool\"") && row.contains("\"save\":0"), "{n}: {row}");
        }
    }

    #[test]
    fn only_repeatable_pickups_are_listed() {
        for a in PICKUPS {
            assert!(a.starts_with("Obj_") && !a.contains("Heart_") && !a.contains("Gold") && !a.contains("Silver"), "{a}");
        }
        let f: Vec<(String, bool)> = start_flags();
        for keep in ["Obj_Heart_Piece", "Obj_Heart_Container", "Obj_Rupee_Gold", "Obj_Rupee_Silver", "Item_Bomb", "Item_DekuNut", "Obj_Fairy"] {
            assert!(!f.iter().any(|(n, _)| n == &format!("{PREFIX}{keep}")), "{keep}");
        }
    }

    #[test]
    fn every_listed_pickup_is_a_real_actor_with_a_pouch_file() {
        let p: std::path::PathBuf = ootrr_game::test_romfs::path();
        if !p.is_dir() {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            return;
        }
        let mut rf: crate::romfs::Romfs = crate::romfs::Romfs::open(&p).unwrap();
        for a in PICKUPS {
            let raw: Vec<u8> = rf.read_zs(&format!("Pack/Actor/{a}.pack.zs")).unwrap();
            let es: Vec<crate::sarc::Entry> = crate::sarc::parse(&raw).unwrap();
            assert!(es.iter().any(|e| e.name.starts_with("Component/PouchContentParam/")), "{a} has no pouch file");
        }
    }
}
