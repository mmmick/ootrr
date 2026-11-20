use std::collections::{BTreeMap as BM, BTreeSet as BS};

use crate::boss_heart::{heart_ids, swap, CHEST, HEART};
use crate::byml::Byml;
use crate::dungeon_credit::{place as place_key, sweep};
use crate::error::{Error as E, Res};
use crate::progressive::{guard_family, guarded, route, Size};
use crate::special;
use crate::slice::banc::{find, set_drop_ev, Found};
use crate::slice::data::Ev;
use crate::slice::pickups as pu;
use crate::slice::chest_targets::Target;



pub struct Edited {
    pub file: String,
    pub old_len: usize,
    pub bytes: Vec<u8>,
    pub hits: Vec<(usize, Found)>,
    pub used: Vec<(String, Size)>,
    pub hearts: Vec<(u64, Vec<String>)>,
    pub wraps: Vec<(u64, String)>,
    pub touched: Vec<u64>,
}

pub fn edit_file(file: &str, raw: &[u8], ts: &[Target]) -> Res<Option<Edited>> {
    edit_file_with(file, raw, ts, false, &BS::new(), &BM::new())
}

/// `small` lists boss heart hashes that become a small chest (chest size matches contents).
/// `lifts` raises a freestanding spot swapped to one of those pickup actors by that many metres,
/// so its item model clears the ground (crate::pickup_pack::lift).
pub fn edit_file_with(file: &str, raw: &[u8], ts: &[Target], keys: bool, small: &BS<u64>, lifts: &BM<String, f32>) -> Res<Option<Edited>> {
    let mut y: Byml = Byml::parse(raw)?;
    let mut hits: Vec<(usize, Found)> = Vec::new();
    let mut used: Vec<(String, Size)> = Vec::new();
    let mut hearts: Vec<(u64, Vec<String>)> = Vec::new();
    let mut wraps: Vec<(u64, String)> = Vec::new();
    let mut touched: Vec<u64> = Vec::new();
    for (i, t) in ts.iter().enumerate() {
        let n: usize = if t.old_actor == HEART && t.heart_chest {
            let chest: &str = if small.contains(&t.hash) { crate::chest_size::PAIRS[0].0 } else { CHEST };
            let n: usize = swap(&mut y, t.hash, &t.new_actor, chest);
            if n > 0 {
                hearts.push((t.hash, heart_ids(&y, t.hash)));
            }
            n
        } else {
            match pu::swap_gyaml(&mut y, t.hash, &t.old_actor, &t.new_actor) {
                0 => set_drop_ev(&mut y, t.hash, &t.old_actor, &t.new_actor, t.ev)?,
                k => {
                    if let Some(dy) = lifts.get(&t.new_actor) {
                        crate::pickup_pack::lift::raise(&mut y, t.hash, *dy);
                    }
                    k
                }
            }
        };
        if n > 0 {
            crate::slice::treasure::set_view(&mut y, t.hash, &t.new_actor);
            if let Some(k) = t.key {
                place_key(&mut y, t.hash, k).ok_or(E::Format("edited key chest vanished"))?;
            }
            if special::of(t.hash).is_some() {
                touched.push(t.hash);
            }
            if let Some(f) = t.family {
                if special::of(t.hash).is_some() {
                    let g: String = find(&y, file, t.hash).into_iter().next().map(|x| x.gyaml).unwrap_or_default();
                    used.push((f.to_string(), Size::of_gyaml(&g)));
                    wraps.push((t.hash, f.to_string()));
                } else {
                    let size: Size = route(&mut y, t.hash, f).ok_or(E::Format("edited actor vanished"))?;
                    used.push((f.to_string(), size));
                }
            } else if t.key.is_none() && guarded(&t.new_actor) && t.ev != Some(Ev::Keep) {
                let fam: String = guard_family(&t.new_actor);
                if special::of(t.hash).is_some() {
                    // A summoned chest starts its open event from its story flow, which the special
                    // stage points at the guard, as it does for an upgrade chain.
                    let g: String = find(&y, file, t.hash).into_iter().next().map(|x| x.gyaml).unwrap_or_default();
                    used.push((fam.clone(), Size::of_gyaml(&g)));
                    wraps.push((t.hash, fam));
                } else {
                    let size: Size = route(&mut y, t.hash, &fam).ok_or(E::Format("edited actor vanished"))?;
                    used.push((fam, size));
                }
            }
            let f: Found = find(&y, file, t.hash).into_iter().next().ok_or(E::Format("edited actor vanished"))?;
            hits.push((i, f));
        }
    }
    let swept: usize = if keys { sweep(&mut y, file)? } else { 0 };
    if hits.is_empty() && swept == 0 {
        return Ok(None);
    }
    Ok(Some(Edited { file: file.to_string(), old_len: raw.len(), bytes: y.write()?, hits, used, hearts, wraps, touched }))
}
