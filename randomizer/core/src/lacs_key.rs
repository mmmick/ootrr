//! Ganon's Boss Key from the Light Arrows cutscene (Ganon's Boss Key: "LACS - Dungeon Rewards").
//!
//! The cutscene plays once the player holds `lacs_rewards` Stones and Medallions together
//! (crate::medal_gate builds that formula) and gives Ganon's Castle Boss Key in place of the
//! Light Arrows, which join the item pool. The fill locks the key onto the cutscene here; the
//! scene hands it out like any dungeon item in a gift scene (crate::gift_keys): the get window,
//! then DungeonInfo.InsideGanonCastle.IsGetBossKey and the line that names Ganon's Castle.

use crate::error::{Error as E, Res};
use crate::logic_items::ItemId;
use crate::slice::support::{Keep, Plan};
use crate::world::{LocId, Placement, World};

/// The Ganon's Boss Key choice that puts the key on the Light Arrows cutscene.
pub const VALUE: &str = "lacs_dungeons";
pub const LOC: &str = "ToT Light Arrows Cutscene";
pub const ITEM: &str = "Boss Key (Ganons Castle)";
pub const NOTE: &str = "Ganon's Castle Boss Key is given by Zelda in the Light Arrows Cutscene, which plays once you hold the set number of Stones and Medallions together. The Light Arrows are shuffled into the item pool. The scene shows the boss key's get window, sets Ganon's Castle boss key flag and names Ganon's Castle. Experimental and untested in the game.";
pub const NEEDS: &str = "Ganon's Boss Key on the Light Arrows Cutscene needs Story and NPC rewards shuffled, so the cutscene can hold the key, and Chests shuffled in dungeons, so the key's own chest can take another item";

/// The fill lock that puts Ganon's Boss Key on the Light Arrows cutscene. The cutscene and the
/// key's own chest must both be shuffled, so the key is in the pool and the Light Arrows take
/// its place there.
pub fn forced(w: &World, p: &Plan, free: &[LocId], base: &Placement, plan: &[(LocId, ItemId)]) -> Res<Vec<(LocId, ItemId)>> {
    if p.keys.ganon != Keep::Lacs {
        return Ok(Vec::new());
    }
    let (Some(l), Some(k)) = (w.loc(LOC), w.item(ITEM)) else {
        return Err(E::Msg(format!("{LOC} or {ITEM} is not in the logic world")));
    };
    if let Some((_, i)) = plan.iter().find(|x| x.0 == l) {
        return if *i == k { Ok(Vec::new()) } else { Err(E::Msg(format!("plando: {LOC} holds Ganon's Boss Key with these settings, so it cannot hold {}", w.items.names[*i]))) };
    }
    if !free.contains(&l) || !free.iter().any(|x| base.get(*x) == Some(k)) {
        return Err(E::Msg(NEEDS.to_string()));
    }
    Ok(vec![(l, k)])
}
