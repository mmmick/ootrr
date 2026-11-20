use crate::bfevfl::{Bf, Cont, Event, Val};
use crate::error::{Error as E, Res};
use crate::msbt::{self, Msbt, Tok};
use crate::romfs::Romfs;
use crate::sarc;
use crate::scrub_prices::{price, Mode, MAX};
use crate::settings::seed::SeedFile;
use crate::settings::Settings;
use crate::staging::Out;

pub const SETTING: &str = "fairy_prices";
pub const NUM: u16 = 107;
pub const FIXED: &str = "fairy_price_fixed";
pub const FIXED_NUM: u16 = 108;
const MSG_FILE: &str = "EventFlowMsg/Dm_FairyFountain_Din.msbt";
const TAG: &str = "GreatFairy_Price";

pub struct Fountain {
    pub loc: &'static str,
    pub flow: &'static str,
    pub tag: u8,
    pub vanilla: i32,
    pub check: &'static str,
    pub pay: &'static str,
}

pub const FOUNTAINS: [Fountain; 6] = [
    Fountain { loc: "DMT Great Fairy Reward", flow: "Dm_FairyFountain_Din", tag: 1, vanilla: 25, check: "Event69", pay: "Event123" },
    Fountain { loc: "HC Great Fairy Reward", flow: "Dm_FairyFountain_Power", tag: 2, vanilla: 50, check: "Event14", pay: "Event29" },
    Fountain { loc: "ZF Great Fairy Reward", flow: "Dm_FairyFountain_Frore", tag: 3, vanilla: 100, check: "Event17", pay: "Event24" },
    Fountain { loc: "DMC Great Fairy Reward", flow: "Dm_FairyFountain_Wisdom", tag: 4, vanilla: 200, check: "Event17", pay: "Event23" },
    Fountain { loc: "Colossus Great Fairy Reward", flow: "Dm_FairyFountain_Nayru", tag: 5, vanilla: 300, check: "Event17", pay: "Event23" },
    Fountain { loc: "OGC Great Fairy Reward", flow: "Dm_FairyFountain_Courage", tag: 6, vanilla: 500, check: "Event18", pay: "Event24" },
];

pub fn read(s: &Settings, bad: &mut Vec<String>) -> Mode {
    match s.text(SETTING) {
        Ok("vanilla") => Mode::Vanilla,
        Ok("cheap") => Mode::Cheap,
        Ok("fixed") => match s.int(FIXED) {
            Ok(n) => Mode::Fixed((n as i32).clamp(0, MAX)),
            Err(e) => {
                bad.push(e.to_string());
                Mode::Vanilla
            }
        },
        Ok(v) => {
            bad.push(format!("{SETTING} is {v}, which is not a known choice"));
            Mode::Vanilla
        }
        Err(e) => {
            bad.push(e.to_string());
            Mode::Vanilla
        }
    }
}

pub fn prices(m: Mode, seed: &str) -> Vec<(&'static Fountain, i32)> {
    FOUNTAINS.iter().filter_map(|f| price(m, seed, f.loc).map(|p| (f, p))).collect()
}

fn set_value(b: &mut Bf, name: &str, from: i32, to: i32) -> Res<()> {
    let f = b.flows.first_mut().ok_or(E::Format("fairy prices: a fountain file holds no flow"))?;
    let e: &mut Event = f.events.iter_mut().find(|e| e.name == name).ok_or_else(|| E::Msg(format!("fairy prices: no event {name}, the game files differ")))?;
    let c: &mut Cont = e.kind.params_mut().ok_or_else(|| E::Msg(format!("fairy prices: {name} has no parameters")))?;
    match c.items.iter_mut().find(|(k, _)| k == "Value") {
        Some((_, Val::Int(x))) if *x == from => {
            *x = to;
            Ok(())
        }
        Some((_, Val::Int(x))) => Err(E::Msg(format!("fairy prices: {name} asks {x} and not {from}, the game files differ"))),
        _ => Err(E::Msg(format!("fairy prices: {name} has no number Value"))),
    }
}

pub fn edit_flow(b: &mut Bf, f: &Fountain, p: i32) -> Res<()> {
    set_value(b, f.check, f.vanilla, p)?;
    set_value(b, f.pay, -f.vanilla, -p)
}

pub fn stage(rf: &mut Romfs, late: &mut Vec<Out>, work: &std::path::Path, m: Mode, seed: &SeedFile) -> Res<()> {
    for (f, p) in prices(m, &seed.seed) {
        crate::story_skip::stage_late(rf, late, work, f.flow, |b| edit_flow(b, f, p))?;
    }
    Ok(())
}

fn tag_of(args: &[u8]) -> Option<u8> {
    let at: usize = args.windows(TAG.len()).position(|w| w == TAG.as_bytes())?;
    args.get(at + TAG.len()).filter(|c| c.is_ascii_digit()).map(|c| c - b'0')
}

pub fn literal(t: &[Tok], ps: &[(u8, i32)]) -> (Vec<Tok>, usize) {
    let mut n: usize = 0;
    let out: Vec<Tok> = t
        .iter()
        .map(|x| match x {
            Tok::Code { grp: 2, kind: 1, args } => match tag_of(args).and_then(|k| ps.iter().find(|(i, _)| *i == k)) {
                Some((_, p)) => {
                    n += 1;
                    Tok::Text(p.to_string())
                }
                None => x.clone(),
            },
            _ => x.clone(),
        })
        .collect();
    (out, n)
}

fn text_file(raw: &[u8], ps: &[(u8, i32)]) -> Res<Vec<u8>> {
    let mut es: Vec<sarc::Entry> = sarc::parse(raw)?;
    let e: &sarc::Entry = sarc::get(&es, MSG_FILE).ok_or_else(|| E::Msg(format!("the language archive has no {MSG_FILE}")))?;
    let mut m: Msbt = msbt::parse(&e.data)?;
    let names: Vec<String> = m.names().into_iter().map(|(k, _)| k.to_string()).collect();
    for k in names {
        let (t, n): (Vec<Tok>, usize) = literal(&m.text(&k)?, ps);
        if n > 0 {
            m.put_tokens(&k, &t)?;
        }
    }
    sarc::set(&mut es, MSG_FILE, m.write()?);
    sarc::write(&es)
}

pub fn stage_text(rf: &mut Romfs, m: Mode, seed: &SeedFile, more: &mut Vec<Out>) -> Res<()> {
    let ps: Vec<(u8, i32)> = prices(m, &seed.seed).into_iter().map(|(f, p)| (f.tag, p)).collect();
    if ps.is_empty() {
        return Ok(());
    }
    for l in crate::hints::languages(rf)? {
        crate::dungeon_grant::merge(rf, more, &l, |raw: &[u8]| text_file(raw, &ps))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn romfs() -> Option<Romfs> {
        let p: std::path::PathBuf = ootrr_game::test_romfs::path();
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Romfs::open(&p).ok()
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    fn tag(k: u8) -> Tok {
        let mut args: Vec<u8> = vec![0x11, 0x00];
        args.extend(format!("{TAG}{k}").as_bytes());
        args.extend([0xff, 0xff, 0x00]);
        Tok::Code { grp: 2, kind: 1, args }
    }

    #[test]
    fn each_fountain_has_its_own_price_and_vanilla_keeps_the_game_price() {
        assert!(prices(Mode::Vanilla, "x").is_empty());
        let fixed: Vec<(&Fountain, i32)> = prices(Mode::Fixed(40), "x");
        assert_eq!(fixed.len(), 6);
        assert!(fixed.iter().all(|(_, p)| *p == 40));
        let tags: Vec<u8> = FOUNTAINS.iter().map(|f| f.tag).collect();
        assert_eq!(tags, vec![1, 2, 3, 4, 5, 6]);
        for n in 0..100 {
            for (_, p) in prices(Mode::Cheap, &format!("seed {n}")) {
                assert!((0..=95).contains(&p) && p % 5 == 0, "{p}");
            }
        }
    }

    #[test]
    fn the_price_tag_becomes_the_number_of_its_own_fountain() {
        let t: Vec<Tok> = vec![Tok::Text("Offer me ".into()), tag(2), Tok::Text(" rupees".into()), tag(6)];
        let (out, n): (Vec<Tok>, usize) = literal(&t, &[(2, 15), (4, 30)]);
        assert_eq!(n, 1);
        assert_eq!(out[1], Tok::Text("15".into()));
        assert_eq!(out[3], tag(6));
        assert_eq!(tag_of(&match tag(5) { Tok::Code { args, .. } => args, _ => Vec::new() }), Some(5));
    }

    #[test]
    fn each_fountain_checks_and_takes_the_new_price() {
        let Some(mut rf) = romfs() else {
            return;
        };
        for f in &FOUNTAINS {
            let raw: Vec<u8> = rf.read_zs(&format!("Event/EventFlow/{}.bfevfl.zs", f.flow)).unwrap();
            let mut b: Bf = Bf::parse(&raw).unwrap();
            edit_flow(&mut b, f, 35).unwrap();
            let out: Bf = Bf::parse(&b.write().unwrap()).unwrap();
            let v = |name: &str| out.flows[0].events.iter().find(|e| e.name == name).and_then(|e| e.kind.params()).and_then(|c| c.get("Value").cloned());
            assert_eq!(v(f.check), Some(Val::Int(35)), "{}", f.flow);
            assert_eq!(v(f.pay), Some(Val::Int(-35)), "{}", f.flow);
            assert!(edit_flow(&mut b, f, 35).is_err(), "{}", f.flow);
        }
    }

    #[test]
    fn every_fountain_line_with_a_price_shows_the_number() {
        let Some(mut rf) = romfs() else {
            return;
        };
        let ps: Vec<(u8, i32)> = (1..=6).map(|k| (k, 10 * k as i32 + 3)).collect();
        for l in crate::hints::languages(&rf).unwrap() {
            let raw: Vec<u8> = rf.read_zs(&l).unwrap();
            let out: Vec<u8> = text_file(&raw, &ps).unwrap();
            let es: Vec<sarc::Entry> = sarc::parse(&out).unwrap();
            let m: Msbt = msbt::parse(&sarc::get(&es, MSG_FILE).unwrap().data).unwrap();
            for (k, _) in m.names() {
                assert!(m.text(k).unwrap().iter().all(|t| !matches!(t, Tok::Code { grp: 2, kind: 1, args } if tag_of(args).is_some())), "{l} {k}");
            }
            if l.contains("USen") {
                let s: String = msbt::to_markup(&m.text("Dm_FairyFountain_0306_01").unwrap());
                assert!(s.contains("33 rupees"), "{s}");
            }
        }
    }
}
