pub mod ambush;
pub mod gates;
pub mod hook;
pub mod backup;
pub mod battle_bgm;
pub mod bean;
pub mod cucco;
pub mod drops;
pub mod dupes;
pub mod encounters;
pub mod extras;
pub mod ground;
pub mod horde;
pub mod fairy;
pub mod flows;
pub mod lottery;
pub mod minibosses;
pub mod minions;
pub mod regen;
pub mod respawn;
pub mod spawn;
pub mod stats;
pub mod surprises;
pub mod target;
pub mod wake;

use std::fs;

use crate::error::Res;
use crate::qol::{edit_plain, edit_zs, Have};
use crate::romfs::Romfs;
use crate::sarc::{self, Entry};
use crate::settings::Settings;
use crate::staging::Out;
use crate::zs;

pub const HEALTH: &str = "enemy_health";
pub const DAMAGE: &str = "enemy_damage";
pub const SPEED: &str = "enemy_speed";
pub const NO_HEARTS: &str = "no_heart_drops";
pub const NO_FAIRIES: &str = "no_gossip_fairies";
pub const NO_AMMO: &str = "no_ammo_drops";
pub const LOSE_HEART: &str = "lose_heart_on_death";
pub const NO_BOTTLES: &str = "no_bottles_in_combat";
pub const NO_TARGET: &str = "no_z_targeting";
pub const NO_DUPES: &str = "no_dupe_items";
pub const MORE: &str = "more_enemies";
pub const RESPAWN: &str = "enemies_respawn_sooner";
pub const MINIBOSSES: &str = "boss_minibosses";
pub const REGEN: &str = "enemy_regen";
pub const MINIONS: &str = "boss_minions";
pub const BEAN_PITCH: &str = "bean_pitch";
pub const ZORA_AMBUSH: &str = "zora_ambush";
pub const FAIRY_HELPS: &str = "fairy_helps_enemies";
pub const BACKUP: &str = "enemy_backup";
pub const GATES: &str = "enemy_gates";
pub const ENCOUNTERS: &str = "special_encounters";
pub const CUCCO: &str = "cucco_revenge";
pub const FOREST: &str = "open_forest";
pub const ENTRANCES: &str = "shuffle_entrances";
pub const SPAWN_COMBO: &str = "enemy_spawn_combo";
pub const KAKARIKO: &str = "open_kakariko";

pub const HEALTH_NUM: u16 = 110;
pub const DAMAGE_NUM: u16 = 111;
pub const SPEED_NUM: u16 = 112;
pub const NO_HEARTS_NUM: u16 = 113;
pub const NO_FAIRIES_NUM: u16 = 114;
pub const NO_AMMO_NUM: u16 = 115;
pub const LOSE_HEART_NUM: u16 = 116;
pub const NO_BOTTLES_NUM: u16 = 117;
pub const NO_TARGET_NUM: u16 = 124;
pub const NO_DUPES_NUM: u16 = 122;
pub const MORE_NUM: u16 = 119;
pub const RESPAWN_NUM: u16 = 123;
pub const MINIBOSSES_NUM: u16 = 120;
pub const REGEN_NUM: u16 = 118;
pub const MINIONS_NUM: u16 = 121;
pub const BEAN_PITCH_NUM: u16 = 128;
pub const ZORA_AMBUSH_NUM: u16 = 127;
pub const FAIRY_HELPS_NUM: u16 = 125;
pub const BACKUP_NUM: u16 = 126;
pub const GATES_NUM: u16 = 129;
pub const ENCOUNTERS_NUM: u16 = 130;
pub const CUCCO_NUM: u16 = 131;
pub const SPAWN_COMBO_NUM: u16 = 132;

pub const NORMAL: i64 = 100;
pub const STAT_RANGE: (i64, i64) = (50, 500);
pub const SPEED_RANGE: (i64, i64) = (75, 200);
pub const STAT_STEP: i64 = 25;
pub const SPEED_STEP: i64 = 5;
pub const ACTOR_PACKS: &str = "Pack/Actor";
pub const MORE_RANGE: (i64, i64) = (100, 1000);
pub const MORE_STEP: i64 = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opts {
    pub health: i64,
    pub damage: i64,
    pub speed: i64,
    pub no_hearts: bool,
    pub no_fairies: bool,
    pub no_ammo: bool,
    pub lose_heart: bool,
    pub no_bottles: bool,
    pub no_target: bool,
    pub no_dupes: bool,
    pub extra: i64,
    pub respawn: bool,
    pub minibosses: bool,
    pub regen: bool,
    pub minions: bool,
    pub bean_pitch: bool,
    pub zora_ambush: bool,
    pub fairy_helps: bool,
    pub backup: bool,
    pub gates: bool,
    pub encounters: bool,
    pub cucco: bool,
    pub spawn_combo: bool,
    pub forest_open: bool,
    pub shops: bool,
    pub mido_gone: bool,
    pub entrances: bool,
}

impl Default for Opts {
    fn default() -> Opts {
        Opts { health: NORMAL, damage: NORMAL, speed: NORMAL, no_hearts: false, no_fairies: false, no_ammo: false, lose_heart: false, no_bottles: false, no_target: false, no_dupes: false, extra: 0, respawn: false, minibosses: false, regen: false, minions: false, bean_pitch: false, zora_ambush: false, fairy_helps: false, backup: false, gates: false, encounters: false, cucco: false, spawn_combo: false, forest_open: false, shops: false, mido_gone: false, entrances: false }
    }
}

fn times(pct: i64) -> String {
    let s: String = format!("{:.2}", pct as f64 / 100.0);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

impl Opts {
    pub fn cut(&self) -> drops::Cut {
        drops::Cut { hearts: self.no_hearts, ammo: self.no_ammo }
    }

    pub fn packs(&self) -> bool {
        self.health != NORMAL || self.damage != NORMAL || self.speed != NORMAL || self.cut().any() || self.no_target || self.no_bottles
    }

    pub fn gate_list(&self) -> Vec<gates::Gate> {
        if !self.gates {
            return Vec::new();
        }
        gates::GATES.iter().filter(|g| !gates::HELD.contains(&g.id) && gates::active(g, self.forest_open, self.mido_gone, self.entrances)).copied().collect()
    }

    pub fn gates(&self) -> bool {
        !self.gate_list().is_empty()
    }

    pub fn any(&self) -> bool {
        self.packs() || self.no_fairies || self.lose_heart || self.no_bottles || self.no_dupes || self.extra > 0 || self.respawn || self.minibosses || self.regen || self.minions || self.bean_pitch || self.zora_ambush || self.fairy_helps || self.backup || self.encounters || self.cucco || self.spawn_combo || self.gates()
    }

    pub fn notes(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        if self.health != NORMAL {
            out.push(format!("Enemies and bosses have {} times their usual health. Enemies that are beaten by a scripted hit, such as Deku Scrubs, are unchanged. Experimental, untested in the game.", times(self.health)));
        }
        if self.damage != NORMAL {
            out.push(format!("Enemy attacks and enemy projectiles deal {} times their usual damage. Grabs and traps that are not enemies are unchanged. Experimental, untested in the game.", times(self.damage)));
        }
        if self.speed != NORMAL {
            out.push(format!("Enemies move {} times as fast and wait that much less between attacks. Enemies that only move through their animations keep their pace. Experimental, untested in the game.", times(self.speed)));
        }
        if self.regen {
            out.push("Enemies and bosses that are still alive win back one point of health about every five seconds, up to their usual health. Experimental, untested in the game.".to_string());
        }
        if self.backup {
            out.push("An ordinary enemy that has had Link as its target for about 25 seconds without a break calls one helper to its side, once in its life. The helper is a Leever (the small creature that pops out of the desert sand), except for enemies native to Jabu-Jabu's Belly (a Shabom) and to the Ice Cavern and the Shadow and Spirit Temples (a flying pot). Helpers never call anything themselves. Bosses, minibosses, Iron Knuckles, Dead Hand, Dark Link, Phantom Ganon and Leevers never call. Experimental, untested in the game.".to_string());
        }
        if self.no_hearts {
            out.push("Enemies, pots, grass and other random drops never give recovery hearts, also when your health is low. Where the game would hand out a heart because you need one, a green rupee drops. Experimental, untested in the game.".to_string());
        }
        if self.no_ammo {
            let shops: &str = if self.shops { " Shopsanity is on: a special deal can take the place of a shelf that sold arrows, bombs or seeds, and the logic does not check that any ammo can still be bought, so you may find a seed where the only arrows are in the shops you cannot afford or cannot reach." } else { "" };
            out.push(format!("Enemies, pots, grass and other random drops never give arrows, seeds, bombs, Deku Nuts or Deku Sticks.{shops} Experimental, untested in the game."));
        }
        if self.no_fairies {
            out.push("Playing a song to a gossip stone no longer calls a fairy that heals you. Experimental, untested in the game.".to_string());
        }
        if self.lose_heart {
            out.push(match flows::KEEP {
                flows::Keep::CountSave => "Each game over that you continue from takes away one heart container, down to three hearts. The heart is taken right after the game reloads, and the game is saved at that moment so the loss stays. A fairy in a bottle that revives you costs nothing. Experimental, untested in the game.".to_string(),
                flows::Keep::Count => "Each game over that you continue from takes away one heart container, down to three hearts. The heart is taken right after the game reloads, and the game remembers it until you quit to the title, even if you die again before saving. A fairy in a bottle that revives you costs nothing. Experimental, untested in the game.".to_string(),
            });
        }
        if self.no_bottles {
            out.push("While the enemy battle music plays or during a boss battle, Link shakes his head and puts the bottle away without drinking or letting out its contents. Leevers and Moblins, which normally fight without battle music, now start it, because the music is how the game is read as in combat. Experimental, untested in the game.".to_string());
        }
        if self.no_target {
            out.push("Enemies and bosses cannot be locked on to. Aim assist, talking to people, reading signs and locking on to things that are not enemies are unchanged. Experimental, untested in the game.".to_string());
        }
        if self.no_dupes {
            out.push("A bottle is not filled with something that another of your bottles already holds. This covers catching things with a bottle, shop shelves, Deku Scrub sellers, cows, Talon and the potion shops. A refused catch may still use up what you swung at. Bottle contents from chests and other shuffled items are not covered. Experimental, untested in the game.".to_string());
        }
        if self.extra > 0 {
            out.push(format!("About {} in 100 of the ordinary enemies that no door, chest, counter or event is tied to have one extra copy standing next to them. Experimental, untested in the game.", self.extra.min(extras::FULL)));
        }
        if self.extra > horde::FROM {
            out.push(format!("More enemies at {}x: each area is filled toward that many times its usual number of enemies (at most {} in one map), on walkable ground read from the game's navigation mesh, with fliers, water enemies and, from 5x, kinds that never live there, groups and the odd miniboss. Experimental, untested in the game.", (self.extra + 100) as f32 / 100.0, horde::MAP_CAP));
        }
        if self.respawn {
            out.push("Most ordinary enemies come back when you leave their room or area and return, not only when you leave the whole dungeon or region. Bosses, minibosses, Gold Skulltulas, Poes, Deku Scrubs and a few enemies that share their data with a boss are unchanged. Experimental, untested in the game.".to_string());
        }
        if self.minibosses {
            out.push(format!(
                "A strong enemy waits in the boss room of these bosses and leaves when the boss is beaten, in Gohma's room it appears as the boss introduction starts: {}. The other bosses and the dream world fights are unchanged. Experimental, untested in the game.",
                minibosses::covered()
            ));
        }
        if self.minions {
            out.push(minions::NOTE.to_string());
        }
        if self.bean_pitch {
            out.push("One talk in five, the Bean Seller by Zora's River offers you some Difficulty Mods between bites before he gets to his usual business. The line is in English in every language. Experimental, untested in the game.".to_string());
        }
        if self.zora_ambush {
            out.push("The empty hole in Zora's Domain has a wall Skulltula on the far wall of each of its three side caves, and a Skulltula appears at the mouth of a side cave once you have walked into it. They come back when the area is loaded again. Experimental, untested in the game.".to_string());
        }
        if self.encounters {
            out.push(encounters::NOTE.to_string());
            out.push(surprises::NOTE.to_string());
        }
        if self.cucco {
            out.push(cucco::NOTE.to_string());
        }
        if self.spawn_combo {
            out.push(spawn::NOTE.to_string());
        }
        if self.fairy_helps {
            out.push("When you let a fairy out of a bottle, or a bottled fairy revives you, every enemy and boss that is awake at that moment is healed to full health too. Bringing back the enemy you beat last is not possible with file edits and is not done. Experimental, untested in the game.".to_string());
        }
        if self.gates {
            let names: Vec<&str> = self.gate_list().iter().map(|g| g.name).collect();
            if names.is_empty() {
                out.push("Enemies block the way is on, but no gate applies to these settings (shuffled entrances would make it add a requirement), so nothing is gated.".to_string());
            } else {
                out.push(format!("A purple barrier, the one that locks the Wolfos fight in the Sacred Forest Meadow, stands in the way at {} while the enemies guarding it are alive and goes away when they are beaten. At Zora's River the guards are Lizalfos that this setting adds. Nothing is saved: when the area loads again and the guards are back, so is the barrier. Experimental, untested by hand.", names.join(", ")));
            }
        }
        out
    }
}

fn chosen<'a>(r: Res<&'a str>, bad: &mut Vec<String>) -> Option<&'a str> {
    match r {
        Ok(t) => Some(t),
        Err(e) => {
            bad.push(e.to_string());
            None
        }
    }
}

pub fn read(s: &Settings, bad: &mut Vec<String>) -> Opts {
    let mut num = |id: &str, r: (i64, i64)| -> i64 {
        match s.int(id) {
            Ok(v) if (r.0..=r.1).contains(&v) => v,
            Ok(v) => {
                bad.push(format!("{id} is {v}, which is outside {} to {}", r.0, r.1));
                NORMAL
            }
            Err(e) => {
                bad.push(e.to_string());
                NORMAL
            }
        }
    };
    let health: i64 = num(HEALTH, STAT_RANGE);
    let damage: i64 = num(DAMAGE, STAT_RANGE);
    let speed: i64 = num(SPEED, SPEED_RANGE);
    let extra: i64 = num(MORE, MORE_RANGE) - NORMAL;
    let mut flag = |id: &str| -> bool {
        s.flag(id).unwrap_or_else(|e| {
            bad.push(e.to_string());
            false
        })
    };
    let o: Opts = Opts { health, damage, speed, no_hearts: flag(NO_HEARTS), no_fairies: flag(NO_FAIRIES), no_ammo: flag(NO_AMMO), lose_heart: flag(LOSE_HEART), no_bottles: flag(NO_BOTTLES), no_target: flag(NO_TARGET), no_dupes: flag(NO_DUPES), extra: 0, respawn: flag(RESPAWN), minibosses: flag(MINIBOSSES), regen: flag(REGEN), minions: flag(MINIONS), bean_pitch: flag(BEAN_PITCH), zora_ambush: flag(ZORA_AMBUSH), fairy_helps: flag(FAIRY_HELPS), backup: flag(BACKUP), gates: flag(GATES), encounters: false, cucco: flag(CUCCO), spawn_combo: flag(SPAWN_COMBO), forest_open: false, shops: false, mido_gone: false, entrances: false };
    let forest_open: bool = chosen(s.text(FOREST), bad).map(|t| t == "open").unwrap_or(true);
    let entrances: bool = chosen(s.text(ENTRANCES), bad).map(|t| t != "off").unwrap_or(true);
    let shops: bool = chosen(s.text(crate::shops::plan::SETTING), bad).map(|t| t != "off").unwrap_or(false);
    let closed: bool = s.text(FOREST).map(|t| t == "closed").unwrap_or(true);
    let mido_gone: bool = !closed && s.text(KAKARIKO).map(|t| t == "open").unwrap_or(false);
    Opts { extra, encounters: extra > 0, forest_open, mido_gone, entrances, shops, ..o }

}

fn edit_entry(e: &Entry, o: &Opts) -> Res<Option<Vec<u8>>> {
    let n: &str = &e.name;
    if o.minions && n == minions::DODONGO_PARAM {
        return minions::babies(&e.data);
    }
    if o.health != NORMAL
        && let Some(s) = stats::stem(n, stats::LIFE_DIR, stats::LIFE_EXT)
    {
        return if stats::is_enemy(s) { stats::life(&e.data, o.health) } else { Ok(None) };
    }
    if o.damage != NORMAL
        && let Some(s) = stats::stem(n, stats::ATTACK_DIR, stats::ATTACK_EXT)
    {
        return if stats::is_enemy_attack(s) { stats::attack(&e.data, o.damage) } else { Ok(None) };
    }
    if o.speed != NORMAL
        && let Some(s) = stats::stem(n, stats::MOVE_DIR, stats::MOVE_EXT)
    {
        return if stats::is_enemy(s) { stats::movement(&e.data, o.speed) } else { Ok(None) };
    }
    if o.speed != NORMAL
        && let Some(s) = stats::stem(n, stats::PACE_DIR, stats::PACE_EXT)
    {
        return if stats::is_enemy(s) { stats::pace(&e.data, o.speed) } else { Ok(None) };
    }
    if n.starts_with(drops::DIR) {
        return drops::edit(&e.data, o.cut());
    }
    if o.no_target
        && let Some(s) = stats::stem(n, target::DIR, target::EXT)
    {
        return if target::is_foe(s) { target::edit(&e.data) } else { Ok(None) };
    }
    Ok(None)
}

pub fn edit_pack(raw: &[u8], o: &Opts) -> Res<Option<Vec<u8>>> {
    let mut es: Vec<Entry> = sarc::parse(raw)?;
    let mut hit: bool = false;
    for e in es.iter_mut() {
        if let Some(d) = edit_entry(e, o)? {
            e.data = d;
            hit = true;
        }
    }
    if o.no_bottles {
        for (n, d) in battle_bgm::edit(&es)? {
            if let Some(e) = es.iter_mut().find(|e| e.name == n) {
                e.data = d;
                hit = true;
            }
        }
    }
    if !hit {
        return Ok(None);
    }
    Ok(Some(sarc::write(&es)?))
}

fn pack_out(rf: &mut Romfs, have: Have, path: &str, o: &Opts) -> Res<Option<Out>> {
    let z: Vec<u8> = match have(path) {
        Some(b) => b,
        None => rf.read(path)?,
    };
    let id: Option<u32> = zs::dict_of(&z);
    let raw: Vec<u8> = rf.zs.decompress(&z)?;
    let Some(out) = edit_pack(&raw, o)? else {
        return Ok(None);
    };
    Ok(Some(Out { path: path.to_string(), old: raw.len(), new: out.len(), z: zs::compress_raw(&out, id)? }))
}

pub fn pack_files(rf: &Romfs, o: &Opts) -> Res<Vec<String>> {
    if !o.packs() {
        return Ok(if o.minions { vec![minions::DODONGO_PACK.to_string()] } else { Vec::new() });
    }
    let mut out: Vec<String> = rf.list(ACTOR_PACKS)?.into_iter().filter(|p| p.ends_with(".pack.zs")).collect();
    if o.cut().any() {
        out.push(drops::FOLIAGE.to_string());
    }
    out.sort();
    Ok(out)
}

pub fn merge(late: &mut Vec<Out>, outs: Vec<Out>) {
    for x in outs {
        match late.iter_mut().rev().find(|y| y.path == x.path) {
            Some(y) => {
                y.z = x.z;
                y.new = x.new;
            }
            None => late.push(x),
        }
    }
}

fn step(rf: &mut Romfs, outs: &mut Vec<Out>, have: Have, f: impl FnOnce(&mut Romfs, Have) -> Res<Vec<Out>>) -> Res<()> {
    let more: Vec<Out> = {
        let seen = |p: &str| -> Option<Vec<u8>> { outs.iter().find(|x| x.path == p).map(|x| x.z.clone()).or_else(|| have(p)) };
        f(rf, &seen)?
    };
    merge(outs, more);
    Ok(())
}

pub fn stage(rf: &mut Romfs, o: &Opts, have: Have, seed: &str) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    for f in pack_files(rf, o)? {
        if let Some(x) = pack_out(rf, have, &f, o)? {
            outs.push(x);
        }
    }
    if o.cut().any() {
        let raw: Vec<u8> = match have(lottery::FILE) {
            Some(b) => b,
            None => rf.read(lottery::FILE)?,
        };
        if let Some(out) = lottery::edit(&raw, o.cut())? {
            outs.push(Out { path: lottery::FILE.to_string(), old: raw.len(), new: out.len(), z: out });
        }
    }
    if o.regen {
        step(rf, &mut outs, have, |rf, have| Ok(regen::stage(rf, have, ACTOR_PACKS)?.1))?;
    }
    if o.fairy_helps {
        step(rf, &mut outs, have, |rf, have| Ok(fairy::stage(rf, have, ACTOR_PACKS)?.1))?;
    }
    if o.backup {
        step(rf, &mut outs, have, |rf, have| Ok(backup::stage(rf, have, ACTOR_PACKS)?.1))?;
    }
    if o.no_fairies {
        step(rf, &mut outs, have, |rf, have| Ok(vec![edit_zs(rf, have, flows::GOSSIP_FILE, flows::no_gossip)?]))?;
    }
    if o.lose_heart {
        step(rf, &mut outs, have, |rf, have| Ok(vec![edit_zs(rf, have, flows::OVER_FILE, flows::lose_heart)?]))?;
        step(rf, &mut outs, have, |rf, have| Ok(vec![edit_zs(rf, have, flows::BOOT_FILE, flows::lose_heart_boot)?]))?;
    }
    if o.fairy_helps {
        step(rf, &mut outs, have, |rf, have| Ok(vec![edit_zs(rf, have, flows::OVER_FILE, |raw| flows::fairy_used(raw, flows::OVER_FLOW, flows::REVIVE))?]))?;
        step(rf, &mut outs, have, |rf, have| Ok(vec![edit_zs(rf, have, flows::OUT_FAIRY_FILE, |raw| flows::fairy_used(raw, flows::OUT_FAIRY_FLOW, flows::OUT_FAIRY))?]))?;
    }
    if o.no_bottles {
        for (file, name) in flows::BOTTLE_FLOWS {
            step(rf, &mut outs, have, |rf, have| Ok(vec![edit_zs(rf, have, file, |raw| flows::no_bottles(raw, name))?]))?;
        }
    }
    if o.no_dupes {
        step(rf, &mut outs, have, |rf, have| Ok(vec![edit_zs(rf, have, dupes::CATCH_FILE, |raw| dupes::entries(raw, dupes::CATCH_FLOW, &dupes::CATCH_ENTRIES, dupes::Ending::Catch))?]))?;
        step(rf, &mut outs, have, |rf, have| Ok(vec![edit_zs(rf, have, dupes::GIVE_FILE, |raw| dupes::entries(raw, dupes::GIVE_FLOW, &dupes::GIVE_ENTRIES, dupes::Ending::Give))?]))?;
        step(rf, &mut outs, have, |rf, have| Ok(vec![edit_zs(rf, have, dupes::SHOP_FILE, |raw| dupes::fronts(raw, dupes::SHOP_FLOW, &dupes::goods()))?]))?;
        for (file, name, item) in dupes::SELLERS {
            step(rf, &mut outs, have, |rf, have| Ok(vec![edit_zs(rf, have, file, |raw| dupes::fronts(raw, name, &[(None, item)]))?]))?;
        }
    }
    if o.bean_pitch {
        step(rf, &mut outs, have, |rf, have| Ok(vec![edit_zs(rf, have, bean::FILE, bean::pitch)?]))?;
        for l in crate::hints::languages(rf)? {
            step(rf, &mut outs, have, |rf, have| Ok(vec![edit_zs(rf, have, &l, bean::text)?]))?;
        }
    }
    if o.respawn {
        for f in respawn::files() {
            step(rf, &mut outs, have, |rf, have| Ok(vec![edit_plain(rf, have, &f, respawn::always)?]))?;
        }
    }
    if o.extra > 0 {
        step(rf, &mut outs, have, |rf, have| extras::stage(rf, have, seed, o.extra.min(extras::FULL)))?;
    }
    if o.minibosses {
        for s in &minibosses::SPOTS {
            step(rf, &mut outs, have, |rf, have| Ok(minibosses::one(rf, have, s)?.to_vec()))?;
        }
    }
    if o.minions {
        step(rf, &mut outs, have, minions::stage)?;
    }
    if o.zora_ambush {
        step(rf, &mut outs, have, ambush::stage)?;
    }
    if o.encounters {
        step(rf, &mut outs, have, encounters::stage)?;
        step(rf, &mut outs, have, |rf, have| surprises::stage(rf, have, seed))?;
    }
    if o.cucco {
        step(rf, &mut outs, have, cucco::stage)?;
    }
    if o.spawn_combo {
        step(rf, &mut outs, have, spawn::stage)?;
    }
    let gl: Vec<gates::Gate> = o.gate_list();
    if !gl.is_empty() {
        step(rf, &mut outs, have, |rf, have| gates::stage(rf, have, &gl))?;
    }
    if o.extra > horde::FROM {
        step(rf, &mut outs, have, |rf, have| horde::stage(rf, have, seed, o.extra, o.encounters))?;
    }
    Ok(outs)
}

pub fn worked(rf: &mut Romfs, pk: &crate::modpack::Pack, o: &Opts, seed: &str, late: &mut Vec<Out>) -> Res<()> {
    if !o.any() {
        return Ok(());
    }
    let root: std::path::PathBuf = pk.work.join("romfs");
    let outs: Vec<Out> = {
        let have = |p: &str| -> Option<Vec<u8>> { late.iter().rev().find(|x| x.path == p).map(|x| x.z.clone()).or_else(|| fs::read(p.split('/').fold(root.clone(), |a, s| a.join(s))).ok()) };
        stage(rf, o, &have, seed)?
    };
    merge(late, outs);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byml::{Byml, Node};
    use crate::error::Error;
    use crate::settings::Value;

    #[test]
    fn everything_is_off_by_default_and_the_settings_string_is_unchanged() {
        let mut bad: Vec<String> = Vec::new();
        let o: Opts = read(&Settings::default(), &mut bad);
        assert!(bad.is_empty());
        assert_eq!(o, Opts::default());
        assert!(!o.any() && !o.packs() && o.notes().is_empty());
        let mut s: Settings = Settings::default();
        s.set(HEALTH, Value::Int(100)).unwrap();
        s.set(NO_HEARTS, Value::Bool(false)).unwrap();
        s.set(MORE, Value::Int(100)).unwrap();
        s.set(RESPAWN, Value::Bool(false)).unwrap();
        s.set(MINIBOSSES, Value::Bool(false)).unwrap();
        s.set(REGEN, Value::Bool(false)).unwrap();
        s.set(MINIONS, Value::Bool(false)).unwrap();
        s.set(BEAN_PITCH, Value::Bool(false)).unwrap();
        s.set(ZORA_AMBUSH, Value::Bool(false)).unwrap();
        s.set(FAIRY_HELPS, Value::Bool(false)).unwrap();
        s.set(BACKUP, Value::Bool(false)).unwrap();
        s.set(GATES, Value::Bool(false)).unwrap();
        assert_eq!(s.encode().unwrap(), Settings::default().encode().unwrap());
    }

    #[test]
    fn the_gates_follow_the_forest_and_entrance_settings() {
        let mut s: Settings = Settings::default();
        s.set(GATES, Value::Bool(true)).unwrap();
        let mut bad: Vec<String> = Vec::new();
        let o: Opts = read(&s, &mut bad);
        assert!(bad.is_empty() && o.gates() && o.any() && !o.packs());
        assert_eq!(o.gate_list().len(), gates::GATES.len() - gates::HELD.len());
        assert_eq!(o.notes().len(), 1);
        s.set(FOREST, Value::s("open")).unwrap();
        let o: Opts = read(&s, &mut bad);
        assert!(o.gates() && o.gate_list().len() == gates::GATES.len() - gates::HELD.len() - 1);
        s.set(FOREST, Value::s("closed_deku")).unwrap();
        s.set(KAKARIKO, Value::s("open")).unwrap();
        let o: Opts = read(&s, &mut bad);
        assert!(o.mido_gone && o.gate_list().len() == gates::GATES.len() - gates::HELD.len() - 1);
        s.set(KAKARIKO, Value::s("zelda")).unwrap();
        s.set(ENTRANCES, Value::s("dungeons")).unwrap();
        let o: Opts = read(&s, &mut bad);
        assert!(o.entrances && !o.gates());
        let off: Opts = Opts { gates: false, ..Opts::default() };
        assert!(off.gate_list().is_empty() && !off.any() && off.notes().is_empty());
    }

    #[test]
    fn a_broken_gate_or_shop_setting_is_reported_and_skips_the_gate() {
        let mut bad: Vec<String> = Vec::new();
        assert_eq!(chosen(Ok("open"), &mut bad), Some("open"));
        assert!(bad.is_empty());
        assert_eq!(chosen(Err(Error::Msg("open_forest is gone".to_string())), &mut bad), None);
        assert_eq!(bad, vec!["open_forest is gone".to_string()]);
        let mut s: Settings = Settings::default();
        s.set(NO_AMMO, Value::Bool(true)).unwrap();
        let plain: Opts = read(&s, &mut bad);
        assert!(!plain.shops && !plain.notes()[0].contains("Shopsanity"));
        s.set(crate::shops::plan::SETTING, Value::s("deals_2")).unwrap();
        let shop: Opts = read(&s, &mut bad);
        assert!(shop.shops && shop.notes().len() == 1 && shop.notes()[0].contains("Shopsanity is on"));
    }

    #[test]
    fn each_setting_is_read_and_survives_the_settings_string() {
        let mut s: Settings = Settings::default();
        s.set(HEALTH, Value::Int(250)).unwrap();
        s.set(DAMAGE, Value::Int(50)).unwrap();
        s.set(SPEED, Value::Int(200)).unwrap();
        for id in [NO_HEARTS, NO_FAIRIES, NO_AMMO, LOSE_HEART, NO_BOTTLES, NO_TARGET, NO_DUPES, RESPAWN, MINIBOSSES, REGEN, MINIONS, BEAN_PITCH, ZORA_AMBUSH, FAIRY_HELPS, BACKUP, GATES, CUCCO, SPAWN_COMBO] {
            s.set(id, Value::Bool(true)).unwrap();
        }
        s.set(MORE, Value::Int(150)).unwrap();
        let back: Settings = Settings::decode(&s.encode().unwrap()).unwrap().settings;
        let mut bad: Vec<String> = Vec::new();
        let o: Opts = read(&back, &mut bad);
        assert!(bad.is_empty());
        assert_eq!(o, Opts { health: 250, damage: 50, speed: 200, no_hearts: true, no_fairies: true, no_ammo: true, lose_heart: true, no_bottles: true, no_target: true, no_dupes: true, extra: 50, respawn: true, minibosses: true, regen: true, minions: true, bean_pitch: true, zora_ambush: true, fairy_helps: true, backup: true, gates: true, encounters: true, cucco: true, spawn_combo: true, ..Opts::default() });
        assert!(o.any() && o.packs());
        assert_eq!(o.notes().len(), 24);
        assert_eq!(o.notes().iter().collect::<std::collections::BTreeSet<&String>>().len(), o.notes().len());
        assert!(o.notes().iter().any(|n| n.contains("About 50 in 100")));
        assert!(Opts { extra: 100, ..Opts::default() }.any() && !Opts { extra: 100, ..Opts::default() }.packs());
        assert!(s.set(MORE, Value::Int(1050)).is_err() || s.clone().finish().is_err());
        assert!(o.notes().iter().any(|n| n.contains("Queen Gohma (Stalfos)") && n.contains("untested in the game")));
        assert!(o.notes().iter().any(|n| n == minions::NOTE));
        let fun: Opts = Opts { bean_pitch: true, ..Opts::default() };
        assert!(fun.any() && !fun.packs() && fun.notes().len() == 1);
        assert!(o.notes()[0].contains("2.5 times"));
        assert!(o.notes()[1].contains("0.5 times"));
        assert!(o.notes()[2].contains("2 times"));
        assert!(s.set(HEALTH, Value::Int(49)).is_err() || s.clone().finish().is_err());
        let mut t: Settings = Settings::default();
        assert!(t.set(SPEED, Value::Int(74)).is_err() || t.finish().is_err());
    }

    fn entry(name: &str, items: Vec<(&str, Node)>) -> Entry {
        let y: Byml = Byml { root: Node::Map(items.into_iter().map(|(k, v)| (k.to_string(), v)).collect()), version: 7, big: false };
        Entry { name: name.to_string(), data: y.write().unwrap() }
    }

    fn max_life(es: &[Entry], name: &str) -> Option<Node> {
        Byml::parse(&sarc::get(es, name)?.data).ok()?.root.get("MaxLife").cloned()
    }

    #[test]
    fn a_pack_only_changes_its_enemy_files() {
        let foe: &str = "Life/LifeParameters/Enemy_Stalfos.game__life__LifeParameters.bgyml";
        let hero: &str = "Life/LifeParameters/Player.game__life__LifeParameters.bgyml";
        let es: Vec<Entry> = vec![entry(foe, vec![("MaxLife", Node::I32(18))]), entry(hero, vec![("MaxLife", Node::I32(12))])];
        let raw: Vec<u8> = sarc::write(&es).unwrap();
        assert!(edit_pack(&raw, &Opts::default()).unwrap().is_none());
        assert!(edit_pack(&raw, &Opts { damage: 300, speed: 150, no_hearts: true, ..Opts::default() }).unwrap().is_none());
        let out: Vec<Entry> = sarc::parse(&edit_pack(&raw, &Opts { health: 300, ..Opts::default() }).unwrap().unwrap()).unwrap();
        assert_eq!(max_life(&out, foe), Some(Node::I32(54)));
        assert_eq!(max_life(&out, hero), Some(Node::I32(12)));
    }

    fn up(y: &Byml) -> Option<&str> {
        y.root.get("$parent")?.as_str()?.rsplit('/').next()?.strip_suffix(".game__attention__ClientParam.gyml")
    }

    fn field(es: &[Entry], stem: &str, key: &str) -> Option<u32> {
        let y: Byml = Byml::parse(&sarc::get(es, &format!("{}{stem}{}", target::DIR, target::EXT))?.data).ok()?;
        if let Some(Node::U32(v)) = y.root.get(key) {
            return Some(*v);
        }
        field(es, up(&y)?, key)
    }

    fn origin(es: &[Entry], stem: &str) -> Option<String> {
        let y: Byml = Byml::parse(&sarc::get(es, &format!("{}{stem}{}", target::DIR, target::EXT))?.data).ok()?;
        if y.root.get(target::KIND).is_some() {
            return Some(stem.to_string());
        }
        origin(es, up(&y)?)
    }

    fn kind(es: &[Entry], stem: &str) -> Option<u32> {
        field(es, stem, target::KIND)
    }

    fn action(es: &[Entry], stem: &str) -> Option<u32> {
        field(es, stem, target::ACTION)
    }

    fn client(stem: &str, kind: &str, action: &str) -> Entry {
        entry(&format!("{}{stem}{}", target::DIR, target::EXT), vec![(target::ACTION, Node::U32(target::hash(action))), (target::KIND, Node::U32(target::hash(kind)))])
    }

    #[test]
    fn no_targeting_only_changes_enemy_lock_on_clients() {
        let child: Entry = entry(&format!("{}Enemy_Stalfos_LockOn{}", target::DIR, target::EXT), vec![("$parent", Node::Str("Work/Attention/ClientParam/EnemyCommon_LockOn.game__attention__ClientParam.gyml".to_string()))]);
        let es: Vec<Entry> = vec![
            client("EnemyCommon_LockOn", "LockOn", "None"),
            child,
            client("EnemyCommon_AutoAim", "AutoAim", "None"),
            client("NpcStandard_LockOn", "LockOn", "Talk"),
            client("Object_M_LockOn", "LockOn", "None"),
            client("Enemy_Scrub_Business_LockOn_EventOn", "LockOn", "Talk"),
        ];
        let raw: Vec<u8> = sarc::write(&es).unwrap();
        assert!(edit_pack(&raw, &Opts { health: 300, no_hearts: true, ..Opts::default() }).unwrap().is_none());
        let out: Vec<u8> = edit_pack(&raw, &Opts { no_target: true, ..Opts::default() }).unwrap().unwrap();
        assert_eq!(out.len(), raw.len());
        let got: Vec<Entry> = sarc::parse(&out).unwrap();
        let look: Option<u32> = Some(target::hash(target::LOOK_ONLY));
        let lock: Option<u32> = Some(target::hash(target::LOCK_ON));
        assert_eq!(kind(&got, "EnemyCommon_LockOn"), look);
        assert_eq!(kind(&got, "Enemy_Stalfos_LockOn"), look);
        assert_eq!(kind(&got, "NpcStandard_LockOn"), lock);
        assert_eq!(kind(&got, "Object_M_LockOn"), lock);
        assert_eq!(kind(&got, "Enemy_Scrub_Business_LockOn_EventOn"), lock);
        let same: usize = got.iter().filter(|x| sarc::get(&es, &x.name).is_some_and(|y| y.data == x.data)).count();
        assert_eq!(same, es.len() - 1);
    }

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from)?;
        if p.is_dir() { Some(p) } else { None }
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn the_game_files_take_every_option() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let o: Opts = Opts { health: 200, damage: 150, speed: 150, no_hearts: true, no_fairies: true, no_ammo: true, lose_heart: true, no_bottles: true, no_target: true, no_dupes: true, extra: 100, respawn: true, minibosses: true, regen: true, minions: true, bean_pitch: true, zora_ambush: true, fairy_helps: true, backup: true, gates: true, encounters: true, cucco: true, ..Opts::default() };
        let outs: Vec<Out> = stage(&mut rf, &o, &none, "seed").unwrap();
        let at = |p: &str| -> &Out { outs.iter().find(|x| x.path == p).unwrap_or_else(|| panic!("{p} was not staged")) };
        for f in [fairy::ACTOR_INFO, fairy::GLOBAL, bean::FILE, flows::GOSSIP_FILE, flows::OVER_FILE, flows::BOOT_FILE, drops::FOLIAGE, lottery::FILE, regen::GLOBAL, regen::ACTOR_INFO, "Pack/Actor/Enemy_Stalfos.pack.zs", "Pack/Actor/Enemy_Keese.pack.zs"] {
            at(f);
        }
        for (f, _) in flows::BOTTLE_FLOWS {
            at(f);
        }
        for f in [dupes::CATCH_FILE, dupes::GIVE_FILE, dupes::SHOP_FILE] {
            at(f);
        }
        let added = |rf: &mut Romfs, p: &str| -> usize {
            let b: crate::bfevfl::Bf = crate::bfevfl::Bf::parse(&rf.zs.decompress(&at(p).z).unwrap()).unwrap();
            b.flows.iter().flat_map(|f| f.events.iter()).filter(|e| e.name.starts_with(dupes::MARK)).count()
        };
        assert_eq!(added(&mut rf, dupes::CATCH_FILE), 5 * dupes::CATCH_ENTRIES.len());
        assert_eq!(added(&mut rf, dupes::GIVE_FILE), 3 * dupes::GIVE_ENTRIES.len());
        assert_eq!(added(&mut rf, dupes::SHOP_FILE), 2 * dupes::GOODS.len(), "the shop asks for an empty bottle in one place");
        assert_eq!(dupes::SELLERS.map(|(f, _, _)| added(&mut rf, f)), [3, 1]);
        for f in respawn::files() {
            at(&f);
        }
        let only: Vec<Out> = stage(&mut rf, &Opts { respawn: true, ..Opts::default() }, &none, "seed").unwrap();
        assert_eq!(only.len(), respawn::TABLES.len());
        assert!(only.iter().all(|x| x.path.starts_with(respawn::DIR) && x.new > x.old && x.z == respawn::always(&rf.read(&x.path).unwrap()).unwrap()));
        for m in ambush::MAPS {
            at(&ambush::file(m, "Static"));
        }
        for f in encounters::files() {
            at(&f);
        }
        for f in surprises::drop_packs(&rf).unwrap() {
            at(&f);
        }
        for f in cucco::packs(&rf).unwrap() {
            at(&f);
        }
        assert!(!outs.iter().any(|x| x.path.contains("Player_")), "the player packs are left alone");
        let plain: Vec<Out> = stage(&mut rf, &Opts { health: 200, damage: 150, speed: 150, no_hearts: true, no_ammo: true, no_target: true, ..Opts::default() }, &none, "seed").unwrap();
        for x in plain.iter().filter(|x| x.path.ends_with(".pack.zs")) {
            assert!(x.new <= x.old, "{} grew", x.path);
        }
        let es: Vec<Entry> = sarc::parse(&rf.zs.decompress(&at("Pack/Actor/Enemy_Stalfos.pack.zs").z).unwrap()).unwrap();
        let was: Vec<Entry> = sarc::parse(&rf.read_zs("Pack/Actor/Enemy_Stalfos.pack.zs").unwrap()).unwrap();
        let life: &str = "Life/LifeParameters/Enemy_Stalfos.game__life__LifeParameters.bgyml";
        let Some(Node::I32(a)) = max_life(&was, life) else { panic!() };
        assert_eq!(max_life(&es, life), Some(Node::I32(a * 2)));
        let same: usize = es.iter().filter(|x| sarc::get(&was, &x.name).is_some_and(|y| y.data == x.data)).count();
        assert!(same >= es.len() - 24, "only a handful of files change, {same} of {} match", es.len());
        let look: Node = Node::U32(target::hash(target::LOOK_ONLY));
        let aims = |es: &[Entry]| -> usize { es.iter().filter(|e| stats::stem(&e.name, target::DIR, target::EXT).is_some_and(target::is_foe) && Byml::parse(&e.data).unwrap().root.get(target::KIND) == Some(&look)).count() };
        assert!(aims(&was) == 0 && aims(&es) > 0, "the Stalfos pack carries the lock-on edit next to the others");
        let paths: std::collections::BTreeSet<&str> = outs.iter().map(|x| x.path.as_str()).collect();
        assert_eq!(paths.len(), outs.len(), "no file is staged twice");
        let boss: Vec<Entry> = sarc::parse(&rf.zs.decompress(&at(minions::DODONGO_PACK).z).unwrap()).unwrap();
        let calm: Vec<Entry> = sarc::parse(&rf.read_zs(minions::DODONGO_PACK).unwrap()).unwrap();
        let moved = |n: &str| -> bool { sarc::get(&boss, n).map(|e| &e.data) != sarc::get(&calm, n).map(|e| &e.data) };
        assert!(moved(minions::DODONGO_PARAM), "King Dodongo keeps his extra Baby Dodongos");
        assert!(calm.iter().any(|e| e.name.starts_with(stats::LIFE_DIR) && moved(&e.name)), "King Dodongo keeps his scaled health");
        let count = |y: &Byml| -> usize { y.root.get("Actors").and_then(|a| a.as_arr()).map_or(0, |a| a.len()) };
        let has = |y: &Byml, g: &str| -> usize { y.root.get("Actors").and_then(|a| a.as_arr()).unwrap_or(&[]).iter().filter(|a| a.get("Gyaml").and_then(|v| v.as_str()) == Some(g)).count() };
        let solo: Vec<Out> = extras::stage(&mut rf, &none, "seed", 100).unwrap();
        for r in minions::ROOMS.iter() {
            let now: Byml = Byml::parse(&rf.zs.decompress(&at(r.free).z).unwrap()).unwrap();
            let base: Byml = match solo.iter().find(|x| x.path == r.free) {
                Some(x) => Byml::parse(&rf.zs.decompress(&x.z).unwrap()).unwrap(),
                None => rf.read_byml(r.free).unwrap(),
            };
            let here: usize = if r.start.is_some() { 0 } else { r.foes.len() };
            assert_eq!(count(&now), count(&base) + here, "{} carries the extra enemies and the minions", r.free);
        }
        for s in minibosses::SPOTS.iter() {
            let now: Byml = Byml::parse(&rf.zs.decompress(&at(s.file).z).unwrap()).unwrap();
            let base: Byml = match solo.iter().find(|x| x.path == s.file) {
                Some(x) => Byml::parse(&rf.zs.decompress(&x.z).unwrap()).unwrap(),
                None => rf.read_byml(s.file).unwrap(),
            };
            let signalled: usize = minions::ROOMS.iter().filter(|r| r.fixed == s.file && r.start.is_some()).map(|r| r.foes.len()).sum();
            assert_eq!(count(&now), count(&base) + 1 + signalled, "{} carries the miniboss and the signalled minions", s.file);
            assert_eq!(has(&now, s.mini), has(&base, s.mini) + 1);
        }
        assert!(sarc::get(&es, backup::HELPERS[backup::helper_of("Enemy_Stalfos")].1).is_some());
        let shared: Vec<Entry> = sarc::parse(&rf.zs.decompress(&at(regen::GLOBAL).z).unwrap()).unwrap();
        let vanilla: Vec<Entry> = sarc::parse(&rf.read_zs(regen::GLOBAL).unwrap()).unwrap();
        for g in hook::GRAPHS {
            assert_eq!(sarc::get(&shared, g).map(|e| &e.data), sarc::get(&vanilla, g).map(|e| &e.data), "{g} stays vanilla");
        }
        for name in ["AI/Enemy_Stalfos.utility.root.ainb", "AI/EnemyBase.utility.root.ainb"] {
            let caller: crate::ainb::Ainb = crate::ainb::Ainb::parse(&sarc::get(&shared, name).unwrap().data).unwrap();
            assert_eq!((caller.find_els(regen::HEAL).len(), caller.find_els(fairy::CLEAR).len(), caller.find_els(backup::REQUEST).len()), (2, 1, 2), "{name} carries regen, fairy and backup");
            assert!(crate::ainb::audit(&caller).is_empty());
        }
        assert_eq!(outs.iter().filter(|x| x.path == "Pack/Actor/Enemy_Stalfos.pack.zs").count(), 1);
        let table: Byml = Byml::parse(&rf.zs.decompress(&at(regen::ACTOR_INFO).z).unwrap()).unwrap();
        let before: Byml = rf.read_byml(regen::ACTOR_INFO).unwrap();
        let heap = |y: &Byml, n: &str| -> Option<Node> { crate::pickup_pack::rows::find(y, n)?.get(crate::pickup_pack::rows::HEAP).cloned() };
        let Some(Node::I32(h)) = heap(&before, "Enemy_Stalfos") else { panic!() };
        assert_eq!(heap(&table, "Enemy_Stalfos"), Some(Node::I32(h + regen::HEAP_EXTRA + fairy::HEAP_EXTRA + backup::HEAP_EXTRA)), "the heap growth of each graph edit adds up");
        assert_eq!(heap(&table, "Player_Child"), heap(&before, "Player_Child"));
        for (file, name, used, other) in [(flows::OVER_FILE, flows::OVER_FLOW, flows::REVIVE, flows::HEART_MARK), (flows::OUT_FAIRY_FILE, flows::OUT_FAIRY_FLOW, flows::OUT_FAIRY, flows::BOTTLE_MARK)] {
            let b: crate::bfevfl::Bf = crate::bfevfl::Bf::parse(&rf.zs.decompress(&at(file).z).unwrap()).unwrap();
            let f: &crate::bfevfl::Flow = b.flows.iter().find(|f| f.name == name).unwrap();
            let told: Vec<usize> = (0..f.events.len()).filter(|&i| f.events[i].name.starts_with(flows::FAIRY_MARK)).collect();
            assert_eq!(told.len(), 1, "{name}");
            for e in f.events.iter() {
                let crate::bfevfl::Kind::Action { actor, act, next, .. } = &e.kind else { continue };
                if f.actors[*actor as usize].actions[*act as usize] == used {
                    assert!(told.contains(&(*next as usize)), "{used} is followed by the flag in {name}");
                }
            }
            assert!(f.events.iter().any(|e| e.name.starts_with(other)), "{name} keeps its other edit");
        }
        let only: Vec<Out> = stage(&mut rf, &Opts { fairy_helps: true, ..Opts::default() }, &none, "seed").unwrap();
        let mut hit: Vec<&str> = only.iter().filter(|x| x.path.starts_with("Event/")).map(|x| x.path.as_str()).collect();
        hit.sort();
        assert_eq!(hit, vec![flows::OVER_FILE, flows::OUT_FAIRY_FILE]);
        assert!(stage(&mut rf, &Opts::default(), &none, "seed").unwrap().is_empty());
    }

    #[test]
    fn the_game_files_give_quiet_enemies_the_battle_music() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let quiet: [&str; 5] = ["Enemy_Leever", "Enemy_Leever_Big", "Enemy_Leever_Fixed", "Enemy_Moblin", "Enemy_Moblin_Club"];
        let bgm_of = |es: &[Entry], n: &str| -> Option<String> { Byml::parse(&sarc::get(es, n).unwrap().data).unwrap().root.get(battle_bgm::BGM).and_then(|v| v.as_str()).map(|s| s.to_string()) };
        let on: Opts = Opts { no_bottles: true, ..Opts::default() };
        assert!(on.packs());
        let outs: Vec<Out> = stage(&mut rf, &on, &none, "seed").unwrap();
        let mut names: Vec<String> = outs.iter().filter(|x| x.path.starts_with(ACTOR_PACKS)).map(|x| x.path.clone()).collect();
        names.sort();
        let mut want: Vec<String> = quiet.iter().map(|q| format!("Pack/Actor/{q}.pack.zs")).collect();
        want.sort();
        assert_eq!(names, want, "only the packs whose enemy has no battle music change");
        let mut t: crate::rsizetable::Rst = crate::rsizetable::load(&mut rf).unwrap();
        for x in outs.iter().filter(|x| x.path.starts_with(ACTOR_PACKS)) {
            let was: Vec<Entry> = sarc::parse(&rf.read_zs(&x.path).unwrap()).unwrap();
            let now: Vec<Entry> = sarc::parse(&rf.zs.decompress(&x.z).unwrap()).unwrap();
            let link: String = battle_bgm::linked(&was).unwrap()[0].clone();
            assert!(link.ends_with("EnemyNoBattleBgm.game__component__BattlePresentationParam.bgyml"));
            assert_eq!(bgm_of(&was, &link), None);
            assert_eq!(bgm_of(&now, &link).as_deref(), Some(battle_bgm::THEME), "{}", x.path);
            let changed: Vec<&str> = now.iter().filter(|e| sarc::get(&was, &e.name).is_none_or(|w| w.data != e.data)).map(|e| e.name.as_str()).collect();
            assert_eq!(changed, vec![link.as_str()], "{} changes that one file", x.path);
            assert!(x.new > x.old);
            let inner: Option<u32> = t.get(&link);
            assert!(crate::staging::grow_out(&mut rf, &mut t, x).unwrap() >= 2, "the pack and the file inside it grow in the size table");
            assert!(t.get(&link) > inner);
        }
        let copies: Vec<Vec<u8>> = quiet.iter().map(|q| {
            let es: Vec<Entry> = sarc::parse(&rf.zs.decompress(&outs.iter().find(|x| x.path.contains(&format!("/{q}."))).unwrap().z).unwrap()).unwrap();
            let link: String = battle_bgm::linked(&es).unwrap()[0].clone();
            sarc::get(&es, &link).unwrap().data.clone()
        }).collect();
        assert!(copies.windows(2).all(|w| w[0] == w[1]), "every copy gets the identical edit");
        for keep in ["Enemy_Stalfos", "Enemy_Keese", "Enemy_Dodongo", "Enemy_DungeonBoss_Ganon", "Enemy_DungeonBoss_Dodongo", "Enemy_MiddleBoss_Lizalfos", "Enemy_IronKnuckle_Black", "Enemy_Stalfos_Weak"] {
            let p: String = format!("Pack/Actor/{keep}.pack.zs");
            assert!(!names.contains(&p), "{p} already has its own music");
            let es: Vec<Entry> = sarc::parse(&rf.read_zs(&p).unwrap()).unwrap();
            let link: String = battle_bgm::linked(&es).unwrap()[0].clone();
            assert!(!battle_bgm::silent(&es, &link, 0).unwrap(), "{p}");
        }
        let boss: Vec<Entry> = sarc::parse(&rf.read_zs("Pack/Actor/Enemy_DungeonBoss_Ganon.pack.zs").unwrap()).unwrap();
        let link: String = battle_bgm::linked(&boss).unwrap()[0].clone();
        assert_eq!(bgm_of(&boss, &link).as_deref(), Some("BGM_DungeonBoss_Ganon"));
        let off: Vec<Out> = stage(&mut rf, &Opts { no_target: true, speed: 150, ..Opts::default() }, &none, "seed").unwrap();
        for q in quiet {
            let p: String = format!("Pack/Actor/{q}.pack.zs");
            let es: Vec<Entry> = sarc::parse(&rf.zs.decompress(&off.iter().find(|x| x.path == p).unwrap().z).unwrap()).unwrap();
            let link: String = battle_bgm::linked(&es).unwrap()[0].clone();
            assert_eq!(bgm_of(&es, &link), None, "{p} stays quiet with the setting off");
        }
        assert!(stage(&mut rf, &Opts::default(), &none, "seed").unwrap().is_empty());
    }

    #[test]
    fn the_game_files_lose_enemy_lock_on_and_nothing_else() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let o: Opts = Opts { no_target: true, ..Opts::default() };
        let outs: Vec<Out> = stage(&mut rf, &o, &none, "seed").unwrap();
        let lock: u32 = target::hash(target::LOCK_ON);
        let look: u32 = target::hash(target::LOOK_ONLY);
        let idle: u32 = target::hash(target::NO_ACTION);
        let mut copies: std::collections::BTreeMap<String, Vec<u8>> = std::collections::BTreeMap::new();
        let mut packs: usize = 0;
        let mut kept: usize = 0;
        for f in pack_files(&rf, &o).unwrap() {
            let was: Vec<Entry> = sarc::parse(&rf.read_zs(&f).unwrap()).unwrap();
            let now: Vec<Entry> = match outs.iter().find(|x| x.path == f) {
                Some(x) => {
                    assert_eq!(x.new, x.old, "{f} changed size");
                    packs += 1;
                    sarc::parse(&rf.zs.decompress(&x.z).unwrap()).unwrap()
                }
                None => was.clone(),
            };
            assert_eq!(now.len(), was.len());
            for (a, b) in now.iter().zip(was.iter()) {
                assert_eq!(a.name, b.name);
                let Some(s) = stats::stem(&a.name, target::DIR, target::EXT) else {
                    assert!(a.data == b.data, "{f} {} changed", a.name);
                    continue;
                };
                if a.data != b.data {
                    assert!(target::is_foe(s), "{f} {s} is not an enemy client");
                    assert_eq!(a.data.len(), b.data.len());
                    let seen: &Vec<u8> = copies.entry(s.to_string()).or_insert_with(|| a.data.clone());
                    assert!(*seen == a.data, "{s} differs between packs");
                } else {
                    assert!(!copies.contains_key(s), "{f} keeps an unedited copy of {s}");
                }
                assert_eq!(action(&now, s), action(&was, s), "{f} {s}");
                if kind(&was, s) != Some(lock) {
                    assert_eq!(kind(&now, s), kind(&was, s), "{f} {s}");
                    continue;
                }
                if origin(&was, s).is_some_and(|x| target::is_foe(&x)) && action(&was, s) == Some(idle) {
                    assert_eq!(kind(&now, s), Some(look), "{f} {s} can still be locked on to");
                } else {
                    assert_eq!(kind(&now, s), Some(lock), "{f} {s} lost its lock-on");
                    kept += 1;
                }
            }
        }
        assert_eq!(packs, outs.len());
        assert!(packs > 100, "{packs} packs changed");
        assert!(copies.contains_key("EnemyCommon_LockOn") && copies.contains_key("Ganon_LockOn"));
        assert!(copies.len() < 30, "{} files changed", copies.len());
        assert!(kept > 100, "{kept} lock-on clients of people, signs and objects are kept");
        assert!(!outs.iter().any(|x| x.path.contains("Player") || x.path.contains("Npc_")), "player and people packs are left alone");
    }

    #[test]
    fn the_game_files_take_a_miniboss_in_each_covered_boss_room() {
        use crate::ainb::Ainb;
        use crate::placement::hash_of;
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let o: Opts = Opts { minibosses: true, ..Opts::default() };
        assert!(o.any() && !o.packs());
        let outs: Vec<Out> = stage(&mut rf, &o, &none, "seed").unwrap();
        assert_eq!(outs.len(), 2 * minibosses::SPOTS.len());
        for (s, pair) in minibosses::SPOTS.iter().zip(outs.chunks(2)) {
            assert!(rf.root.join(crate::pickup_pack::pack_of(s.mini)).is_file(), "{} has no actor pack", s.mini);
            assert_eq!(pair[0].path, s.file);
            let was: Byml = Byml::parse(&rf.read_zs(s.file).unwrap()).unwrap();
            let now: Byml = Byml::parse(&rf.zs.decompress(&pair[0].z).unwrap()).unwrap();
            let list = |y: &Byml| -> Vec<Node> { y.root.get("Actors").and_then(|a| a.as_arr()).unwrap().to_vec() };
            let (a, b): (Vec<Node>, Vec<Node>) = (list(&was), list(&now));
            assert_eq!(b.len(), a.len() + 1);
            assert_eq!(&b[..a.len()], &a[..], "{}: the actors of the game are untouched", s.file);
            let new: &Node = &b[a.len()];
            let boss: &Node = a.iter().find(|x| x.get("Gyaml").and_then(|g| g.as_str()) == Some(s.actor)).unwrap();
            assert_eq!(new.get("Gyaml").and_then(|g| g.as_str()), Some(s.mini));
            assert_eq!(new.get("Dynamic").and_then(|d| d.get(minibosses::ROOM)), boss.get("Dynamic").and_then(|d| d.get(minibosses::ROOM)));
            let h: u64 = hash_of(new).unwrap();
            assert!(!a.iter().any(|x| hash_of(x) == Some(h)));
            let heart: &Node = a.iter().find(|x| x.get("Gyaml").and_then(|g| g.as_str()) == Some("Obj_Heart_Container") && x.get("Dynamic").and_then(|d| d.get(minibosses::ROOM)) == boss.get("Dynamic").and_then(|d| d.get(minibosses::ROOM))).unwrap();
            let pos = |n: &Node| -> Vec<f32> { n.get("Translate").and_then(|t| t.as_arr()).unwrap().iter().map(|v| if let Node::F32(f) = v { *f } else { panic!() }).collect() };
            let (m, c): (Vec<f32>, Vec<f32>) = (pos(new), pos(heart));
            let flat: f32 = ((m[0] - c[0]).powi(2) + (m[2] - c[2]).powi(2)).sqrt();
            assert!(flat > 2.0 && flat < 25.0 && (m[1] - c[1]).abs() < 1.0, "{}: {flat} from the heart container, {} above it", s.boss, m[1] - c[1]);
            let raw: Vec<u8> = rf.read(&pair[1].path).unwrap();
            assert!(pair[1].path.starts_with("Logic/") && pair[1].new > pair[1].old && pair[1].old == raw.len());
            let old: Ainb = Ainb::parse(&raw).unwrap();
            let g: Ainb = Ainb::parse(&pair[1].z).unwrap();
            assert_eq!(g.write().unwrap(), pair[1].z);
            assert!(crate::ainb::audit(&g).is_empty());
            let n: usize = old.els.len();
            assert_eq!(g.els.len(), n + 1);
            let plain = |e: &[u8; crate::ainb::EL]| -> Vec<u8> { e.iter().enumerate().map(|(j, v)| if (0x14..0x18).contains(&j) { 0 } else { *v }).collect() };
            assert!(g.els[..n].iter().zip(old.els.iter()).all(|(x, y)| plain(x) == plain(y)), "{}: the nodes of the game are untouched", pair[1].path);
            assert_eq!(&g.params[..n], &old.params[..], "{}: the inputs of the game are untouched", pair[1].path);
            let id: &str = g.prop_str(n, minibosses::NAME).unwrap();
            let groups: Vec<Node> = now.root.get("AiGroups").and_then(|x| x.as_arr()).unwrap().to_vec();
            let named = |r: &Node| r.get("Id").and_then(|v| v.as_str()) == Some(id) && r.get("Reference") == Some(&Node::U64(h));
            let home: Vec<&Node> = groups.iter().filter(|x| x.get("References").and_then(|r| r.as_arr()).is_some_and(|r| r.iter().any(named))).collect();
            assert_eq!(home.len(), 1);
            assert_eq!(minibosses::logic_file(home[0].get("Logic").and_then(|v| v.as_str()).unwrap()).unwrap(), pair[1].path);
            let hn: usize = (0..n).find(|&i| g.prop_str(i, minibosses::NAME).is_some_and(|x| x.starts_with(minibosses::HEART))).unwrap();
            let made: crate::ainb::Input = g.input(1, g.find_in(hn, 1, minibosses::CREATE).unwrap()).unwrap();
            let gone: crate::ainb::Input = g.input(1, g.find_in(n, 1, minibosses::DELETE).unwrap()).unwrap();
            assert!(made.src >= 0 && (gone.src, gone.plug) == (made.src, made.plug), "{}: the miniboss leaves on the flag that brings the heart container", s.boss);
            let born: crate::ainb::Input = g.input(1, g.find_in(n, 1, minibosses::CREATE).unwrap()).unwrap();
            match &s.start {
                Some(st) => {
                    assert_eq!(g.el_name(born.src as usize), st.module, "{}: the miniboss is created by the start of the fight", s.boss);
                    assert_eq!(minibosses::signal(&g, st).unwrap(), (born.src as usize, born.plug));
                    assert_eq!(born.flags & minibosses::UNSET, 0);
                }
                None => assert_eq!((born.src, born.val), (-1, 1u32.to_le_bytes().to_vec()), "{}: no start signal is known, the miniboss is there from the start", s.boss),
            }
            let twin: Vec<u8> = {
                let mut t: Ainb = old.clone();
                minibosses::wire_with(&mut t, Some(g.prop_str(hn, minibosses::NAME).unwrap()), id, s.start.as_ref()).unwrap();
                t.write().unwrap()
            };
            assert_eq!(twin, pair[1].z);
        }
        let again: Vec<Out> = stage(&mut rf, &o, &none, "seed").unwrap();
        assert!(outs.iter().zip(again.iter()).all(|(x, y)| x.path == y.path && x.z == y.z), "the edit is the same on every build");
    }

    fn list(y: &Byml) -> &[Node] {
        y.root.get("Actors").and_then(|a| a.as_arr()).unwrap_or(&[])
    }

    #[test]
    fn the_game_files_take_the_boss_minions() {
        use crate::placement::hash_of;
        use std::collections::BTreeSet as Set;
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let o: Opts = Opts { minions: true, ..Opts::default() };
        assert!(o.any() && !o.packs());
        let outs: Vec<Out> = stage(&mut rf, &o, &none, "seed").unwrap();
        let signalled: usize = minions::ROOMS.iter().filter(|r| r.start.is_some()).count();
        assert!(signalled >= 1);
        assert_eq!(outs.len(), minions::ROOMS.len() + signalled + 1);
        let paths: Set<&str> = outs.iter().map(|x| x.path.as_str()).collect();
        assert_eq!(paths.len(), outs.len());
        for r in minions::ROOMS.iter() {
            if let Some(st) = &r.start {
                use crate::ainb::Ainb;
                assert!(!outs.iter().any(|x| x.path == r.free), "{}: the minions are not placed in the dynamic map", r.boss);
                let x: &Out = outs.iter().find(|x| x.path == r.fixed).unwrap();
                let was: Byml = rf.read_byml(r.fixed).unwrap();
                let now: Byml = Byml::parse(&rf.zs.decompress(&x.z).unwrap()).unwrap();
                let n: usize = list(&was).len();
                assert_eq!(list(&now).len(), n + r.foes.len());
                assert_eq!(&list(&now)[..n], list(&was));
                let room: i32 = minions::room_of(&was, r).unwrap();
                let dyn_map: Byml = rf.read_byml(r.free).unwrap();
                let ids: Set<u64> = list(&now).iter().chain(list(&dyn_map).iter()).filter_map(hash_of).collect();
                assert_eq!(ids.len(), list(&now).len() + list(&dyn_map).len(), "a hash is used twice for {}", r.boss);
                for (i, a) in list(&now)[n..].iter().enumerate() {
                    assert_eq!(a.get("Gyaml").and_then(|g| g.as_str()), Some(r.foes[i]));
                    assert_eq!(a.get("Dynamic").and_then(|d| d.get(minions::ROOM)), Some(&Node::I32(room)));
                    assert!(rf.read(&format!("Pack/Actor/{}.pack.zs", r.foes[i])).is_ok());
                }
                let groups: Vec<Node> = now.root.get("AiGroups").and_then(|g| g.as_arr()).unwrap().to_vec();
                let logic: &Out = outs.iter().find(|x| x.path.starts_with("Logic/")).unwrap();
                let g: Ainb = Ainb::parse(&logic.z).unwrap();
                assert!(crate::ainb::audit(&g).is_empty());
                let (module, plug): (usize, i16) = minibosses::signal(&g, st).unwrap();
                let first: usize = Ainb::parse(&rf.read(&logic.path).unwrap()).unwrap().els.len();
                assert_eq!(g.els.len(), first + r.foes.len());
                for (i, a) in list(&now)[n..].iter().enumerate() {
                    let h: u64 = hash_of(a).unwrap();
                    let home: Vec<&Node> = groups.iter().filter(|x| x.get("References").and_then(|r| r.as_arr()).is_some_and(|rs| rs.iter().any(|q| q.get("Reference") == Some(&Node::U64(h))))).collect();
                    assert_eq!(home.len(), 1, "{}: each minion is in the group of the room graph", r.boss);
                    let id: String = home[0].get("References").and_then(|r| r.as_arr()).unwrap().iter().find(|q| q.get("Reference") == Some(&Node::U64(h))).and_then(|q| q.get("Id")).and_then(|v| v.as_str()).unwrap().to_string();
                    let at: usize = (first..g.els.len()).find(|&e| g.prop_str(e, minibosses::NAME) == Some(id.as_str())).unwrap();
                    let born: crate::ainb::Input = g.input(1, g.find_in(at, 1, minibosses::CREATE).unwrap()).unwrap();
                    assert_eq!((born.src, born.plug), (module as i16, plug), "{}: minion {i} is created by the start signal and not at load", r.boss);
                    assert_eq!(born.flags & minibosses::UNSET, 0);
                    let gone: crate::ainb::Input = g.input(1, g.find_in(at, 1, minibosses::DELETE).unwrap()).unwrap();
                    assert_eq!(gone.src, -1);
                }
                continue;
            }
            let x: &Out = outs.iter().find(|x| x.path == r.free).unwrap();
            let was: Byml = rf.read_byml(r.free).unwrap();
            let now: Byml = Byml::parse(&rf.zs.decompress(&x.z).unwrap()).unwrap();
            let fixed: Byml = rf.read_byml(r.fixed).unwrap();
            let room: i32 = minions::room_of(&fixed, r).unwrap();
            let n: usize = list(&was).len();
            assert_eq!(list(&now).len(), n + r.foes.len());
            assert_eq!(&list(&now)[..n], list(&was));
            assert_eq!(x.old, was.write().unwrap().len(), "{} does not rewrite to its own size", r.free);
            let ids: Set<u64> = list(&now).iter().chain(list(&fixed).iter()).filter_map(hash_of).collect();
            assert_eq!(ids.len(), list(&now).len() + list(&fixed).len(), "a hash is used twice in {}", r.free);
            let peer = |g: &str| list(&was).iter().chain(list(&fixed).iter()).any(|a| a.get("Gyaml").and_then(|v| v.as_str()) == Some(g));
            for (i, a) in list(&now)[n..].iter().enumerate() {
                assert_eq!(a.get("Gyaml").and_then(|g| g.as_str()), Some(r.foes[i]));
                assert_eq!(a.get("Dynamic").and_then(|d| d.get(minions::ROOM)), Some(&Node::I32(room)));
                assert!(rf.read(&format!("Pack/Actor/{}.pack.zs", r.foes[i])).is_ok());
            }
            eprintln!("{}: room {room}, {:?}, already in the dungeon: {:?}", r.boss, r.foes, r.foes.iter().map(|g| peer(g)).collect::<Vec<bool>>());
        }
        let x: &Out = outs.iter().find(|x| x.path == minions::DODONGO_PACK).unwrap();
        let es: Vec<Entry> = sarc::parse(&rf.zs.decompress(&x.z).unwrap()).unwrap();
        let was: Vec<Entry> = sarc::parse(&rf.read_zs(minions::DODONGO_PACK).unwrap()).unwrap();
        let rolls = |es: &[Entry]| -> Vec<i32> {
            let y: Byml = Byml::parse(&sarc::get(es, minions::DODONGO_PARAM).unwrap().data).unwrap();
            let st: &[Node] = y.root.get(minions::STAGES).unwrap().as_arr().unwrap();
            st.iter().map(|s| if let Some(Node::I32(n)) = s.get(minions::AFTER_ROLL) { *n } else { 0 }).collect()
        };
        assert_eq!(rolls(&was), vec![0, 2, 3]);
        assert_eq!(rolls(&es), vec![2, 3, 4]);
        let same: usize = es.iter().zip(was.iter()).filter(|(x, y)| x.name == y.name && x.data == y.data).count();
        assert_eq!(same, es.len() - 1);
        let both: Vec<Out> = stage(&mut rf, &Opts { health: 200, minions: true, ..Opts::default() }, &none, "seed").unwrap();
        assert_eq!(both.iter().filter(|x| x.path == minions::DODONGO_PACK).count(), 1);
        let es: Vec<Entry> = sarc::parse(&rf.zs.decompress(&both.iter().find(|x| x.path == minions::DODONGO_PACK).unwrap().z).unwrap()).unwrap();
        assert_eq!(rolls(&es), vec![2, 3, 4]);
    }

    fn out(path: &str, z: &[u8], old: usize, new: usize) -> Out {
        Out { path: path.to_string(), z: z.to_vec(), old, new }
    }

    #[test]
    fn a_file_staged_twice_is_written_once_with_the_first_size_it_had() {
        let mut late: Vec<Out> = vec![out("a", b"1", 10, 14), out("b", b"2", 5, 5)];
        merge(&mut late, vec![out("a", b"3", 14, 20), out("c", b"4", 7, 9)]);
        let got: Vec<(&str, &[u8], usize, usize)> = late.iter().map(|x| (x.path.as_str(), x.z.as_slice(), x.old, x.new)).collect();
        assert_eq!(got, vec![("a", b"3".as_slice(), 10, 20), ("b", b"2".as_slice(), 5, 5), ("c", b"4".as_slice(), 7, 9)]);
    }

    #[test]
    fn the_bean_seller_makes_his_pitch_in_the_game_files_on_top_of_other_edits() {
        use crate::bfevfl::{Bf, Flow, Kind, Val};
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let o: Opts = Opts { bean_pitch: true, ..Opts::default() };
        let cheap: Vec<u8> = crate::open_world::skips::bean_bytes(&rf.read_zs(bean::FILE).unwrap()).unwrap();
        let cheap_z: Vec<u8> = zs::compress_raw(&cheap, zs::dict_of(&rf.read(bean::FILE).unwrap())).unwrap();
        let have = |p: &str| -> Option<Vec<u8>> { if p == bean::FILE { Some(cheap_z.clone()) } else { None } };
        let outs: Vec<Out> = stage(&mut rf, &o, &have, "seed").unwrap();
        let langs: Vec<String> = crate::hints::languages(&rf).unwrap();
        assert!(!langs.is_empty());
        assert_eq!(outs.len(), 1 + langs.len());
        assert_eq!(outs[0].path, bean::FILE);
        assert!(outs[0].new > outs[0].old);
        let was: Bf = Bf::parse(&cheap).unwrap();
        let b: Bf = Bf::parse(&rf.zs.decompress(&outs[0].z).unwrap()).unwrap();
        let f: &Flow = b.flows.iter().find(|x| x.name == bean::FLOW).unwrap();
        let g: &Flow = was.flows.iter().find(|x| x.name == bean::FLOW).unwrap();
        let n: usize = g.events.len();
        assert_eq!(f.events.len(), n + 2);
        assert_eq!(f.entries, g.entries);
        let moved: Vec<usize> = (0..n).filter(|&i| f.events[i] != g.events[i]).collect();
        let [at] = moved.as_slice() else { panic!("one event changes, got {moved:?}") };
        let Kind::Sub { next: old, entry, .. } = &g.events[*at].kind else { panic!() };
        assert_eq!(entry, bean::GREET);
        let Kind::Sub { next: roll, .. } = &f.events[*at].kind else { panic!() };
        let Kind::Switch { actor, query, cases, .. } = &f.events[*roll as usize].kind else { panic!() };
        assert_eq!(f.actors[*actor as usize].name, bean::SYSTEM);
        assert_eq!(f.actors[*actor as usize].queries[*query as usize], bean::RANDOM);
        assert_eq!(cases.iter().map(|c| c.val).collect::<Vec<u32>>(), vec![0, 1, 2, 3, 4]);
        assert_eq!(cases.iter().filter(|c| c.ev == *old).count(), 4);
        let Kind::Action { next, actor, act, params: Some(c) } = &f.events[cases[0].ev as usize].kind else { panic!() };
        assert_eq!(next, old);
        assert_eq!(f.actors[*actor as usize].name, bean::SELLER);
        assert_eq!(f.actors[*actor as usize].actions[*act as usize], bean::TALK);
        assert_eq!(c.get("MessageId"), Some(&Val::Str(bean::message_id())));
        assert_eq!(c.get("IsWaitEnd"), Some(&Val::Bool(0x8000_0001)));
        assert_eq!(c.get("ChoiceNumber"), Some(&Val::Int(0)));
        let sale = |x: &Flow| x.events.iter().find(|e| e.name == crate::open_world::skips::BEAN_MINUS).and_then(|e| e.kind.params().and_then(|c| c.get("Price").cloned()));
        assert_eq!(sale(f), Some(Val::Int(crate::open_world::skips::BEAN_PRICE)));
        for x in &outs[1..] {
            assert!(langs.contains(&x.path));
            assert!(x.new > x.old);
            let es: Vec<Entry> = sarc::parse(&rf.zs.decompress(&x.z).unwrap()).unwrap();
            let old: Vec<Entry> = sarc::parse(&rf.read_zs(&x.path).unwrap()).unwrap();
            assert_eq!(es.len(), old.len());
            assert_eq!(es.iter().zip(old.iter()).filter(|(a, b)| a.data != b.data).map(|(a, _)| a.name.as_str()).collect::<Vec<&str>>(), vec![bean::MSG]);
            let m: crate::msbt::Msbt = crate::msbt::parse(&sarc::get(&es, bean::MSG).unwrap().data).unwrap();
            let w: crate::msbt::Msbt = crate::msbt::parse(&sarc::get(&old, bean::MSG).unwrap().data).unwrap();
            let said: String = crate::msbt::to_markup(&m.text(bean::LABEL).unwrap());
            assert!(said.ends_with(bean::WORDS), "{} {said}", x.path);
            assert_eq!(m.names().len(), w.names().len() + 1);
            for (l, _) in w.names() {
                assert_eq!(m.raw(l), w.raw(l), "{} {l}", x.path);
            }
            assert!(bean::text(&rf.zs.decompress(&x.z).unwrap()).is_err());
        }
    }

    #[test]
    fn the_game_packs_carry_the_backup_call_beside_the_other_edits() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let o: Opts = Opts { health: 200, backup: true, ..Opts::default() };
        let outs: Vec<Out> = stage(&mut rf, &o, &none, "seed").unwrap();
        let pack: &str = "Pack/Actor/Enemy_Stalfos.pack.zs";
        assert_eq!(outs.iter().filter(|x| x.path == pack).count(), 1);
        for f in [backup::GLOBAL, backup::ACTOR_INFO, pack, "Pack/Actor/Enemy_Keese.pack.zs"] {
            assert!(outs.iter().any(|x| x.path == f), "{f} was not staged");
        }
        let x: &Out = outs.iter().find(|x| x.path == pack).unwrap();
        let es: Vec<Entry> = sarc::parse(&rf.zs.decompress(&x.z).unwrap()).unwrap();
        let was: Vec<Entry> = sarc::parse(&rf.read_zs(pack).unwrap()).unwrap();
        let life: &str = "Life/LifeParameters/Enemy_Stalfos.game__life__LifeParameters.bgyml";
        let Some(Node::I32(a)) = max_life(&was, life) else { panic!() };
        assert_eq!(max_life(&es, life), Some(Node::I32(a * 2)));
        let global: Vec<Entry> = sarc::parse(&rf.zs.decompress(&outs.iter().find(|x| x.path == backup::GLOBAL).unwrap().z).unwrap()).unwrap();
        let graph: crate::ainb::Ainb = crate::ainb::Ainb::parse(&sarc::get(&global, "AI/Enemy_Stalfos.utility.root.ainb").unwrap().data).unwrap();
        assert_eq!(graph.find_els(backup::REQUEST).len(), 2);
        assert!(sarc::get(&es, backup::STALFOS_GRAPH).map(|e| &e.data) == sarc::get(&was, backup::STALFOS_GRAPH).map(|e| &e.data), "the module itself is vanilla");
        assert!(sarc::get(&es, backup::HELPERS[backup::helper_of("Enemy_Stalfos")].1).is_some() && sarc::get(&was, backup::HELPERS[backup::helper_of("Enemy_Stalfos")].1).is_none());
        assert_eq!(backup::holder_of(&es, "Enemy_Stalfos").unwrap().as_deref(), Some(backup::HELPERS[backup::helper_of("Enemy_Stalfos")].1));
        assert!(!outs.iter().any(|x| x.path.contains("Player_")), "the player packs are left alone");
    }

    #[test]
    fn the_horde_keeps_every_actor_id_unique_with_every_other_setting_on() {
        let Some(root) = dump() else {
            eprintln!("skipped: set OOTRR_ROMFS");
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let o: Opts = Opts { no_hearts: true, no_fairies: true, no_ammo: true, lose_heart: true, no_bottles: true, no_target: true, no_dupes: true, extra: 900, respawn: true, minibosses: true, regen: true, minions: true, bean_pitch: true, zora_ambush: true, fairy_helps: true, backup: true, gates: true, encounters: true, cucco: true, spawn_combo: true, ..Opts::default() };
        let outs: Vec<Out> = stage(&mut rf, &o, &none, "seed").unwrap();
        let paths: std::collections::BTreeSet<&str> = outs.iter().map(|x| x.path.as_str()).collect();
        assert_eq!(paths.len(), outs.len(), "no file is staged twice");
        assert_eq!(outs.iter().filter(|x| x.path == wake::PACK).count(), 1);
        let ids = |rf: &mut Romfs, mine: bool| -> (std::collections::HashMap<u64, usize>, usize) {
            let mut m: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
            let mut n: usize = 0;
            for f in extras::files(rf).unwrap() {
                let y: Byml = match outs.iter().find(|x| x.path == f).filter(|_| mine) {
                    Some(x) => Byml::parse(&rf.zs.decompress(&x.z).unwrap()).unwrap(),
                    None => rf.read_byml(&f).unwrap(),
                };
                for a in y.root.get("Actors").and_then(|l| l.as_arr()).unwrap_or(&[]) {
                    if let Some(h) = crate::placement::hash_of(a) {
                        *m.entry(h).or_insert(0) += 1;
                        n += 1;
                    }
                }
            }
            (m, n)
        };
        let (was, before): (std::collections::HashMap<u64, usize>, usize) = ids(&mut rf, false);
        let (now, after): (std::collections::HashMap<u64, usize>, usize) = ids(&mut rf, true);
        assert!(after > before + 1000, "{before} then {after}");
        for (h, n) in &now {
            assert!(*n == 1 || was.get(h) == Some(n), "id {h} is used {n} times");
        }
    }
}
