use std::collections::BTreeSet as BS;
use std::path::Path as P;

use crate::ainb::start;
use crate::bomb_lift;
use crate::fast;
use crate::flags_build::LIST_PATH;
use crate::flow_locations::path_of;
use crate::hints;
use crate::intro_skip;
use crate::progressive::{ANALYZED_TABLE, BIG_FLOW, SETTING_TABLE, SMALL_FLOW};
use crate::rsizetable::TABLE_PATH;
use crate::slice::data::Data;
use crate::song_models;
use crate::songs;
use crate::special::SPOTS;

pub const LANGUAGE: &str = "Mals/USen.Product.110.sarc.zs";

pub fn list(d: &Data) -> Vec<String> {
    let mut out: BS<String> = BS::new();
    out.extend(d.scan.files.iter().cloned());
    out.extend(d.pots.files.iter().cloned());
    out.extend(d.pots.rock_files.iter().cloned());
    for l in &d.flow.locs {
        for e in &l.edits {
            if crate::cow_clone::of_file(&e.file).is_none() {
                out.insert(path_of(&e.file));
            }
            out.extend(e.also.iter().map(|a| path_of(a)));
        }
    }
    for c in crate::cow_clone::all() {
        out.extend(c.spots.iter().map(|s| s.file.clone()));
    }
    out.extend(crate::cow_clone::stage::sources());
    for s in [BIG_FLOW, SMALL_FLOW, SETTING_TABLE, ANALYZED_TABLE, TABLE_PATH, LIST_PATH, start::MODULE, songs::FLOW, hints::FLOW, LANGUAGE] {
        out.insert(s.to_string());
    }
    out.extend(fast::FILES.iter().map(|s| s.to_string()));
    out.insert(intro_skip::FILE.to_string());
    out.extend(crate::qol::files());
    out.extend(SPOTS.iter().map(|s| s.flow.to_string()));
    out.extend(song_models::SONGS.iter().map(|s| song_models::pack_of(s)));
    out.extend(bomb_lift::PACKS.iter().map(|s| bomb_lift::pack_of(s)));
    out.insert(crate::dungeon_credit::POUCH_TABLE.to_string());
    out.insert("Pack/ZsDic.pack.zs".to_string());
    out.into_iter().collect()
}

/// Folders a build reads whole: it lists them, or picks files in them by names that come
/// from the seed or the data files (actor packs, event flows, map and language files).
pub const TREES: [&str; 12] = [
    "Banc",
    "Event/EventFlow",
    "GameData",
    "Logic",
    "Mals",
    "Pack",
    "RSDB",
    "Sequence",
    "SLink2",
    "Sound/Bgm",
    "System/CombinationDataTableData",
    "System/Resource",
];

/// Music and sound effect shuffle list and size every sound bank and stream.
pub const SOUND: &str = "Sound/Resource";

/// The part of the game a build reads: the files in `files` and everything under the
/// folders in `trees`. Used to unpack only that part of a .nup or .nx (about 300 MB, or
/// about 2.3 GB with music or sound effect shuffle) instead of the whole 25 GB game.
pub struct Unpack {
    pub files: BS<String>,
    pub trees: BS<String>,
}

impl Unpack {
    pub fn wants(&self, r: &str) -> bool {
        self.files.contains(r) || self.trees.iter().any(|t| r.len() > t.len() && r.starts_with(t.as_str()) && r.as_bytes()[t.len()] == b'/')
    }
}

pub fn unpack(d: &Data, sound: bool) -> Unpack {
    let mut files: BS<String> = list(d).into_iter().collect();
    files.extend(crate::colors::files());
    files.extend(crate::gerudo_card::MEMBERS.iter().map(|n| crate::gerudo_card::schedule_path(n)));
    files.extend(crate::open_world::fortress::CARPENTERS.iter().map(|n| crate::open_world::fortress::own_schedule(n)));
    if let Ok(p) = crate::title_logo::patch() {
        files.extend(crate::title_logo::touched(&p));
    }
    let mut trees: BS<String> = TREES.iter().map(|s| s.to_string()).collect();
    if sound {
        trees.insert(SOUND.to_string());
    }
    Unpack { files, trees }
}

pub fn missing(root: &P, d: &Data) -> Vec<String> {
    list(d)
        .into_iter()
        .filter(|r| !r.split('/').fold(root.to_path_buf(), |a, s| a.join(s)).is_file())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_names_the_files_a_default_build_reads() {
        let d: Data = Data::load().unwrap();
        let l: Vec<String> = list(&d);
        assert!(l.iter().any(|x| x == TABLE_PATH));
        assert!(l.iter().any(|x| x.starts_with("Banc/")));
        assert!(l.iter().any(|x| x.contains("Enemy_Scrub_Business_Common")));
        assert!(l.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn the_unpack_set_holds_the_build_list_the_colour_files_and_its_trees() {
        let d: Data = Data::load().unwrap();
        let u: Unpack = unpack(&d, false);
        assert!(list(&d).iter().all(|f| u.wants(f)));
        assert!(crate::colors::files().iter().all(|f| u.wants(f)));
        assert!(u.wants("Pack/Actor/Anything.pack.zs") && u.wants("Banc/X/y.zs"));
        assert!(!u.wants("Packed/x") && !u.wants("Pack") && !u.wants("Sound/Resource/Stream/SE_A.bwav"));
        assert!(unpack(&d, true).wants("Sound/Resource/Stream/SE_A.bwav"));
    }

    #[test]
    fn an_empty_folder_misses_everything() {
        let d: Data = Data::load().unwrap();
        let dir = std::env::temp_dir().join(format!("ootrr_required_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(missing(&dir, &d).len(), list(&d).len());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
