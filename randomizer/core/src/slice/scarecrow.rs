use crate::error::{Error as E, Res};
use crate::logic_json::Json as Lj;
use crate::logic_items::ItemId;
use crate::world::{LocId, Placement, World};

pub const SETTING: &str = "shuffle_scarecrow_song";
pub const KEY: &str = "scarecrow_song_shuffled";
pub const ON: &str = "on";
pub const ITEM: &str = "Scarecrow Song";
pub const LESSON: &str = "Pierre";
pub const REGION: &str = "Lake Hylia";
pub const NEVER: &str = "False";
pub const JUNK: [&str; 4] = ["Rupee (1)", "Rupees (5)", "Rupees (20)", "Deku Nuts (5)"];

pub const ACTOR: &str = "Item_Melody_ScarecrowSong";

pub const NOTE: &str = "One Scarecrow's Song joins the item pool. The lesson from Bonooru still works in the game, but the logic never counts on it, so nothing needed hides behind teaching the scarecrow. Pierre stands at every scarecrow spot once you hold the song and an ocarina.";

pub fn wanted(over: &[(&str, &str)]) -> bool {
    over.iter().any(|(k, v)| *k == KEY && *v == ON)
}

pub fn apply(files: &mut [Lj]) -> Res<()> {
    for f in files.iter_mut() {
        let Lj::Arr(regions) = f else {
            continue;
        };
        for reg in regions.iter_mut() {
            let Lj::Obj(fields) = reg else {
                continue;
            };
            if !fields.iter().any(|(k, v)| k == "region_name" && v.as_str() == Some(REGION)) {
                continue;
            }
            let Some((_, Lj::Obj(list))) = fields.iter_mut().find(|(k, _)| k == "locations") else {
                return Err(E::Msg(format!("scarecrow song: {REGION} has no locations")));
            };
            let Some((_, rule)) = list.iter_mut().find(|(k, _)| k == LESSON) else {
                return Err(E::Msg(format!("scarecrow song: {REGION} has no {LESSON} lesson")));
            };
            *rule = Lj::Str(NEVER.to_string());
            return Ok(());
        }
    }
    Err(E::Msg(format!("scarecrow song: no region {REGION}")))
}

pub fn held(d: &crate::slice::data::Data, loc: &str) -> bool {
    d.flow.by_name(loc).is_some_and(crate::slice::flow_rules::holds_up)
}

pub fn pooled(p: &crate::slice::support::Plan) -> bool {
    p.scarecrow && !p.kit.logic().iter().any(|(n, _)| *n == ITEM)
}

pub fn balance(item: &str, want: usize, acts: &mut Vec<String>) {
    if item == ITEM {
        while acts.len() < want {
            acts.push(ACTOR.to_string());
        }
    } else if JUNK.contains(&item) && acts.len() == want + 1 {
        acts.pop();
    }
}

pub fn add(w: &World, base: &mut Placement, free: &[LocId], fits: &dyn Fn(ItemId, LocId) -> bool) -> Res<()> {
    let song = w.item(ITEM).ok_or_else(|| E::Msg(format!("scarecrow song: {ITEM} is not a logic item")))?;
    for j in JUNK {
        let Some(junk) = w.item(j) else {
            continue;
        };
        if let Some(l) = free.iter().copied().find(|l| base.get(*l) == Some(junk) && fits(song, *l)) {
            base.put(l, song);
            return Ok(());
        }
    }
    Err(E::Msg("scarecrow song: no junk item in the pool to make room for the song".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Settings, Value};
    use crate::slice::data::Data;
    use crate::slice::generate::generate;
    use crate::slice::logic::world_with;
    use crate::slice::support::plan;

    fn on() -> Settings {
        let mut s: Settings = Settings::default();
        s.set(SETTING, Value::Bool(true)).unwrap();
        s
    }

    #[test]
    fn the_lesson_never_counts_when_the_song_is_shuffled() {
        let d: Data = Data::load().unwrap();
        let p = plan(&on()).unwrap();
        assert!(p.scarecrow && p.logic().contains(&(KEY, ON)));
        assert!(!plan(&Settings::default()).unwrap().scarecrow);
        let w: World = world_with(&d, &p.logic()).unwrap();
        let l: LocId = w.loc(LESSON).unwrap();
        assert!(w.locs[l].rule.never);
        let v: World = world_with(&d, &[]).unwrap();
        assert!(!v.locs[v.loc(LESSON).unwrap()].rule.never);
    }

    #[test]
    fn a_shuffled_seed_places_one_song_and_still_finishes() {
        let g = generate(&on(), "crow").unwrap();
        let n: usize = g.seed_file.placements.values().filter(|i| *i == ITEM).count();
        assert_eq!(n, 1, "{:?}", g.seed_file.placements.iter().filter(|(_, i)| *i == ITEM).collect::<Vec<_>>());
        let d: Data = Data::load().unwrap();
        for seed in ["crow", "pierre", "bonooru", "lake"] {
            let g = generate(&on(), seed).unwrap();
            let spot: &String = g.seed_file.placements.iter().find(|(_, i)| *i == ITEM).map(|(l, _)| l).unwrap();
            assert!(!held(&d, spot), "{seed}: {spot}");
        }
        assert!(held(&d, "LLR Talons Chickens") && !held(&d, "KF Midos Top Right Chest"));
        let off = generate(&Settings::default(), "crow").unwrap();
        assert_eq!(off.seed_file.placements.values().filter(|i| *i == ITEM).count(), 0);
    }
}
