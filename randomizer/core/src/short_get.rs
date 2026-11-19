use crate::bfevfl::{Bf, Flow, Kind, Val};
use crate::error::{Error, Res};
use crate::romfs::Romfs;
use crate::staging::Out;

pub const SETTING: &str = "item_get_short";
pub const FLOW: &str = "GetPouchContent";
pub const NOTE: &str = "Item gets say only \"You got X!\": item descriptions, the pickup window and the other lines after the get message are dropped. Gold Skulltula tokens say the count, and the small key count lines stay.";

const PAIRS: [(&str, &str); 2] = [
    ("Event16", "Event18"), // DefaultGetMessage: GetMessage, GetCaption
    ("Event23", "Event72"), // bottle sold to the ghost shop: GetCaption
];
const GHOST_SECOND: (&str, &str) = ("Event43", "Event71");

const BY_ITEM: &str = "Event60";
const DEFAULT: &str = "Event21";
const KEEP: [u32; 3] = [0x2, 0x9, 0xa];

const BY_BOTTLE: &str = "Event73";
const BOTTLE_DEFAULT: &str = "Event42";

const TOKEN_COUNT: &str = "Event66";
const TOKEN_LINE: (&str, &str) = ("Event64", "ActorMsg/PouchContent:Item_Token_Gold_SecondGetCaption");

const PICKUP_WINDOW: &str = "UISystemActor.EventTriggerRequestOpenPickUpWindow";
pub const WINDOW_FLOWS: [&str; 4] = ["Dm_SY_Get", "Dm_SY_Get_WithoutPlayer", "Dm_SY_Treasurebox", "Dm_SY_TreasureboxSmall"];

fn find(f: &Flow, name: &str) -> Res<usize> {
    f.events.iter().position(|e| e.name == name).ok_or_else(|| Error::Msg(format!("short get: {} has no {name}, the game files differ", f.name)))
}

fn message(f: &Flow, i: usize) -> Option<&str> {
    match f.events[i].kind.params().and_then(|c| c.get("MessageId")) {
        Some(Val::Str(s)) => Some(s.as_str()),
        _ => None,
    }
}

fn action(f: &Flow, i: usize) -> Option<String> {
    let Kind::Action { actor, act, .. } = &f.events[i].kind else {
        return None;
    };
    let a = f.actors.get(*actor as usize)?;
    Some(format!("{}.{}", a.name, a.actions.get(*act as usize)?))
}

fn next_mut(k: &mut Kind) -> Option<&mut u16> {
    match k {
        Kind::Action { next, .. } | Kind::Sub { next, .. } | Kind::Join { next } => Some(next),
        _ => None,
    }
}

fn cut(f: &mut Flow, say: &str, caption: &str) -> Res<()> {
    let s: usize = find(f, say)?;
    let c: usize = find(f, caption)?;
    let after: u16 = match (&f.events[s].kind, &f.events[c].kind) {
        (Kind::Action { next, .. }, Kind::Action { next: then, .. }) if *next as usize == c => *then,
        _ => return Err(Error::Msg(format!("short get: {say} no longer leads to {caption}, the game files differ"))),
    };
    if !message(f, c).is_some_and(|m| m.contains("Caption")) {
        return Err(Error::Msg(format!("short get: {caption} no longer shows a caption, the game files differ")));
    }
    if let Some(n) = next_mut(&mut f.events[s].kind) {
        *n = after;
    }
    Ok(())
}

pub fn patch(b: &mut Bf) -> Res<()> {
    let f: &mut Flow = b.flows.iter_mut().find(|f| f.name == FLOW).ok_or(Error::Format("short get: GetPouchContent has no flow"))?;
    for (say, caption) in PAIRS {
        cut(f, say, caption)?;
    }
    cut(f, GHOST_SECOND.0, GHOST_SECOND.1)?;
    let default: u16 = crate::dungeon_credit::idx(find(f, DEFAULT)?)?;
    if !matches!(&f.events[default as usize].kind, Kind::Sub { entry, .. } if entry == "DefaultGetMessage") {
        return Err(Error::Format("short get: the get window's default path is not where it was, the game files differ"));
    }
    let by: usize = find(f, BY_ITEM)?;
    match &mut f.events[by].kind {
        Kind::Switch { cases, .. } => {
            for c in cases.iter_mut().filter(|c| !KEEP.contains(&c.val)) {
                c.ev = default;
            }
        }
        _ => return Err(Error::Format("short get: the get window no longer picks a path by the item, the game files differ")),
    }
    let bottle: u16 = crate::dungeon_credit::idx(find(f, BOTTLE_DEFAULT)?)?;
    if !matches!(&f.events[bottle as usize].kind, Kind::Sub { entry, .. } if entry == "DefaultGetMessage") {
        return Err(Error::Format("short get: the bottle path's default is not where it was, the game files differ"));
    }
    let by_bottle: usize = find(f, BY_BOTTLE)?;
    match &mut f.events[by_bottle].kind {
        Kind::Switch { cases, .. } => {
            for c in cases.iter_mut() {
                c.ev = bottle;
            }
        }
        _ => return Err(Error::Format("short get: the bottle path no longer picks by the bottle, the game files differ")),
    }
    let line: usize = find(f, TOKEN_LINE.0)?;
    if message(f, line) != Some(TOKEN_LINE.1) {
        return Err(Error::Format("short get: the Gold Skulltula count line is not where it was, the game files differ"));
    }
    let count: usize = find(f, TOKEN_COUNT)?;
    let to: u16 = crate::dungeon_credit::idx(line)?;
    match next_mut(&mut f.events[count].kind) {
        Some(n) => *n = to,
        None => return Err(Error::Format("short get: the Gold Skulltula count step is not an action, the game files differ")),
    }
    Ok(())
}

pub fn patch_pickup_window(b: &mut Bf) -> Res<usize> {
    let mut n: usize = 0;
    for f in b.flows.iter_mut() {
        let windows: Vec<(u16, u16)> = (0..f.events.len())
            .filter(|&i| action(f, i).as_deref() == Some(PICKUP_WINDOW))
            .map(|i| {
                let after: u16 = match &f.events[i].kind {
                    Kind::Action { next, .. } => *next,
                    _ => crate::bfevfl::flow::NONE,
                };
                (i as u16, after)
            })
            .collect();
        for (w, after) in windows {
            for ev in f.events.iter_mut() {
                match &mut ev.kind {
                    Kind::Action { next, .. } | Kind::Sub { next, .. } | Kind::Join { next } if *next == w => *next = after,
                    Kind::Switch { cases, .. } | Kind::SubSwitch { cases, .. } => {
                        for c in cases.iter_mut().filter(|c| c.ev == w) {
                            c.ev = after;
                        }
                    }
                    Kind::Fork { forks, .. } if forks.contains(&w) => {
                        return Err(Error::Msg(format!("short get: a fork in {} starts on the pickup window", f.name)));
                    }
                    _ => {}
                }
            }
            for e in f.entries.iter_mut() {
                if e.main == w {
                    e.main = after;
                }
                e.subs.retain(|&s| s != w);
            }
            n += 1;
        }
    }
    Ok(n)
}

pub fn stage(rf: &mut Romfs, late: &mut Vec<Out>, work: &std::path::Path) -> Res<()> {
    crate::story_skip::stage_late(rf, late, work, FLOW, patch)?;
    for w in WINDOW_FLOWS {
        crate::story_skip::stage_late(rf, late, work, w, |b| {
            if patch_pickup_window(b)? == 0 {
                return Err(Error::Msg(format!("short get: {w} no longer opens the pickup window, the game files differ")));
            }
            Ok(())
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rf() -> Option<Romfs> {
        let p: std::path::PathBuf = ootrr_game::test_romfs::path();
        if !p.join("Pack").join("ZsDic.pack.zs").is_file() {
            eprintln!("skipped, no RomFS dump at {}", p.display());
            return None;
        }
        Some(Romfs::open(&p).unwrap())
    }

    #[test]
    fn the_real_get_window_says_only_the_get_message() {
        let Some(mut rf) = rf() else { return };
        let raw: Vec<u8> = rf.read_zs(&format!("Event/EventFlow/{FLOW}.bfevfl.zs")).unwrap();
        let mut b: Bf = Bf::parse(&raw).unwrap();
        patch(&mut b).unwrap();
        let f: &Flow = &b.flows[0];
        let Kind::Switch { cases, .. } = &f.events[find(f, BY_ITEM).unwrap()].kind else { panic!() };
        let default: u16 = find(f, DEFAULT).unwrap() as u16;
        assert!(cases.iter().filter(|c| !KEEP.contains(&c.val)).all(|c| c.ev == default));
        let Kind::Switch { cases, .. } = &f.events[find(f, BY_BOTTLE).unwrap()].kind else { panic!() };
        let bottle: u16 = find(f, BOTTLE_DEFAULT).unwrap() as u16;
        assert!(cases.iter().all(|c| c.ev == bottle));
        assert!(crate::dungeon_credit::sound(f).is_empty());
    }

    #[test]
    fn the_real_get_flows_skip_the_pickup_window() {
        let Some(mut rf) = rf() else { return };
        for w in WINDOW_FLOWS {
            let raw: Vec<u8> = rf.read_zs(&format!("Event/EventFlow/{w}.bfevfl.zs")).unwrap();
            let mut b: Bf = Bf::parse(&raw).unwrap();
            assert!(patch_pickup_window(&mut b).unwrap() > 0, "{w}");
            assert!(crate::dungeon_credit::sound(&b.flows[0]).is_empty(), "{w}");
        }
    }
}
