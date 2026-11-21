use crate::error::{Error as E, Res};
use crate::logic_json::Json as Lj;

pub const KEY: &str = "ootrr_tracker_fixes";
pub const ON: &str = "on";

pub const GONE: [&str; 1] = ["KF West Ledge Chest"];

pub const MOVES: [(&str, &str, &str, &str); 1] = [("HF Lake Poe Chest", "Hyrule Field", "Lake Hylia", "is_adult and 'Water Temple Clear'")];

pub fn wanted(over: &[(&str, &str)]) -> bool {
    over.iter().any(|(k, v)| *k == KEY && *v == ON)
}

fn locs<'a>(files: &'a mut [Lj], region: &str) -> Option<&'a mut Vec<(String, Lj)>> {
    for f in files.iter_mut() {
        let Lj::Arr(regions) = f else {
            continue;
        };
        for reg in regions.iter_mut() {
            let Lj::Obj(fields) = reg else {
                continue;
            };
            if !fields.iter().any(|(k, v)| k == "region_name" && v.as_str() == Some(region)) {
                continue;
            }
            if let Some((_, Lj::Obj(list))) = fields.iter_mut().find(|(k, _)| k == "locations") {
                return Some(list);
            }
        }
    }
    None
}

pub fn apply(files: &mut [Lj]) -> Res<()> {
    for (name, from, to, rule) in MOVES {
        let src: &mut Vec<(String, Lj)> = locs(files, from).ok_or_else(|| E::Msg(format!("tracker fixes: region {from} has no locations")))?;
        let n: usize = src.len();
        src.retain(|(k, _)| k != name);
        if src.len() == n {
            return Err(E::Msg(format!("tracker fixes: {name} is not in {from}")));
        }
        let dst: &mut Vec<(String, Lj)> = locs(files, to).ok_or_else(|| E::Msg(format!("tracker fixes: region {to} has no locations")))?;
        dst.push((name.to_string(), Lj::Str(rule.to_string())));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slice::data::Data;
    use crate::slice::logic::world_with;
    use crate::world::World;

    fn region_of(w: &World, n: &str) -> String {
        let l: usize = w.locs.iter().position(|x| x.name == n).unwrap();
        w.regions[w.locs[l].region].name.clone()
    }

    #[test]
    fn only_the_tracker_moves_the_lake_poe_chest() {
        let d: Data = Data::load().unwrap();
        assert_eq!(region_of(&world_with(&d, &[]).unwrap(), "HF Lake Poe Chest"), "Hyrule Field");
        assert_eq!(region_of(&world_with(&d, &[(KEY, ON)]).unwrap(), "HF Lake Poe Chest"), "Lake Hylia");
    }
}
