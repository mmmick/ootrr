use std::collections::BTreeSet as BS;
use std::path::PathBuf as PB;

use crate::error::Res;
use crate::romfs::Romfs;
use crate::settings::seed::SeedFile;
use crate::slice::data::Data;
use crate::slice::support::{Plan, Songs};
use crate::staging::Out;

pub struct Ctx<'a> {
    pub d: &'a Data,
    pub seed: &'a SeedFile,
    pub p: &'a Plan,
    pub roam: bool,
    pub keys: &'a [&'static str],
    pub hearts: &'a [(u64, Vec<String>)],
    pub marked: &'a BS<(crate::dungeon_grant::Kind, &'static str)>,
    pub silver: &'a BS<&'static str>,
    pub quiet: &'a [&'a str],
    pub work: &'a PB,
    pub pots: &'a [u64],
    pub scrub_jobs: &'a [crate::scrubs_b::Job],
    pub scrub_rows: usize,
}

pub type Stage = fn(&mut Romfs, &Ctx, &mut Vec<Out>) -> Res<()>;

pub const STAGES: [(&str, Stage); 23] = [
    ("boss heart chests", boss_hearts),
    ("songs", songs),
    ("song models", song_models),
    ("scarecrow song model", scarecrow_model),
    ("hints", hints),
    ("dungeon item text", dungeon_text),
    ("scrub hint text", scrub_hints),
    ("silver rupee text", silver_text),
    ("pouch", pouch),
    ("bomb flowers", bomb_lift),
    ("quiet bomb and arrow pickups", quiet_pickups),
    ("door fix", door_fix),
    ("no tutorials", no_tutorials),
    ("open door of time", door_open),
    ("master sword pedestal", pedestal),
    ("medallion gate", medal_gate),
    ("start reward and flags", start_reward),
    ("starting items", start_items),
    ("pierre at every scarecrow spot", pierre),
    ("dpad items", dpad),
    ("pot rows", pot_rows),
    ("start defaults", start_defaults),
    ("scrub rows", scrub_rows),
];

pub fn run(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    for (_, s) in STAGES {
        s(rf, c, more)?;
    }
    Ok(())
}

fn boss_hearts(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    if !c.hearts.is_empty() {
        more.extend(crate::boss_heart::stage(rf, c.d, c.hearts, true)?);
    }
    Ok(())
}

fn songs(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    if c.p.songs != Songs::Off {
        more.push(crate::songs::stage(rf)?);
    }
    Ok(())
}

fn song_models(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    if c.p.songs == Songs::Anywhere {
        more.extend(crate::song_models::stage(rf, c.p.harp_colors)?);
    }
    Ok(())
}

fn scarecrow_model(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    if crate::slice::scarecrow::pooled(c.p) {
        crate::slice::scarecrow_model::stage(rf, more, Some(c.work))?;
    }
    Ok(())
}

fn hints(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    if c.p.hints || c.roam || c.p.npc.any() || !c.keys.is_empty() {
        more.extend(crate::hints::stage(rf, c.d, c.seed, c.p.hints, c.keys, &c.p.npc)?);
    }
    Ok(())
}

fn dungeon_text(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    crate::dungeon_grant::stage_text(rf, more, c.marked)
}

fn scrub_hints(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    crate::scrub_prices::stage_text(rf, c.d, c.seed, c.p.rewards.scrub_hint, more)
}

fn silver_text(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    crate::slice::silver::stage_text(rf, more, c.silver)
}

fn pouch(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    more.push(crate::dungeon_credit::stage_pouch(rf, c.roam)?);
    Ok(())
}

fn bomb_lift(rf: &mut Romfs, _: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    more.extend(crate::bomb_lift::stage(rf)?);
    Ok(())
}

fn quiet_pickups(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    more.extend(crate::bomb_pickup::stage_all(rf, c.quiet)?);
    Ok(())
}

fn door_fix(rf: &mut Romfs, _: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    more.extend(crate::door_fix::stage(rf)?);
    Ok(())
}

fn no_tutorials(rf: &mut Romfs, _: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    more.push(crate::no_tutorials::stage(rf)?);
    Ok(())
}

fn door_open(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    if c.p.gates.door {
        more.extend(crate::door_open::stage_door(rf)?);
    }
    Ok(())
}

fn pedestal(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    more.extend(crate::door_open::stage_pedestal(rf, Some(c.work))?);
    Ok(())
}

fn medal_gate(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    if c.p.rewards.dungeon_rewards {
        crate::medal_gate::stage(rf, more, Some(c.work))?;
    } else if c.p.lacs.is_some() {
        crate::medal_gate::stage_lacs(rf, more, Some(c.work))?;
    }
    if c.p.rainbow.is_some() {
        crate::rainbow_bridge::stage(rf, more)?;
    }
    Ok(())
}

fn start_reward(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    let reward: Option<&str> = crate::start_reward::pick(&c.seed.seed, c.p.start.reward).map(|(_, a)| a);
    let mut flags: Vec<(String, bool)> = c.p.start_flags();
    flags.extend(crate::no_tutorials::start_flags());
    let mut give: Vec<&str> = c.p.start.pouch.clone();
    if let Some(a) = reward {
        give.push(a);
    }
    if !give.is_empty() || !flags.is_empty() {
        more.push(crate::start_reward::stage_all(rf, &give, &flags)?);
    }
    Ok(())
}

fn start_items(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    crate::start_items::stage::stage(rf, more, &c.p.kit)
}

fn pierre(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    if c.p.scarecrow || c.p.kit.logic().iter().any(|(n, _)| *n == crate::slice::scarecrow::ITEM) {
        crate::pierre::stage(rf, more, Some(c.work))?;
    }
    Ok(())
}

fn dpad(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    crate::start_items::dpad::stage(rf, more, &crate::start_items::dpad::actors(&[&c.p.kit], &c.p.start.pouch))?;
    Ok(())
}

fn pot_rows(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    if c.p.drops.rocks != crate::slice::support::Mode::Off {
        more.push(crate::pot_slice::kf_ring::stage(rf)?);
    }
    crate::pot_slice::rows::stage(rf, more, c.pots)
}

fn start_defaults(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    let mut edits: Vec<crate::start_defaults::Edit> = crate::start_defaults::presets::chosen(&c.p.defaults())?;
    if let Some(n) = c.p.lacs {
        crate::medal_gate::override_lacs(&mut edits, n);
    }
    edits.extend(crate::gerudo_card::exps());
    if let Some(n) = c.p.rainbow {
        edits.push(crate::rainbow_bridge::exp(n));
    }
    crate::start_defaults::stage(rf, more, &edits)
}

fn scrub_rows(rf: &mut Romfs, c: &Ctx, more: &mut Vec<Out>) -> Res<()> {
    if c.scrub_rows == 0 {
        crate::scrubs_b::stage_rows(rf, c.scrub_jobs, more, true)?;
    }
    Ok(())
}
