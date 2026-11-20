use std::collections::BTreeMap as BM;
use std::fs;
use std::path::{Path as P, PathBuf as PB};

use crate::byml::{Byml, Node};
use crate::chest_slot::LIST;
use crate::error::{Error as E, Res};
use crate::pickup_pack::actor::{pouch_file, POUCH_ACTOR};
use crate::pickup_pack::closure::{actor_file, chain, Chain, Pool};
use crate::pickup_pack::pack_of;
use crate::pickup_pack::rows::{self, ACTOR_INFO, GAME_ACTOR_INFO, POUCH_INFO};
use crate::romfs::Romfs;
use crate::rsizetable::{self, Rst, TABLE_PATH};
use crate::sarc::{self, Entry};
use crate::shelf::actor::{int, price_file, table_file};
use crate::shelf::goods::{self, actor_of};
use crate::shelf::{banc, counters, inner, text, Job, What, BASE, KEEPER_PACKS};

fn read(rf: &mut Romfs, root: &P, path: &str) -> Res<Vec<u8>> {
    let p: PB = root.join(path);
    if p.is_file() {
        rf.zs.decompress(&fs::read(p)?)
    } else {
        rf.read_zs(path)
    }
}

fn text_of<'a>(n: &'a Node, k: &str) -> &'a str {
    n.get(k).and_then(|v| v.as_str()).unwrap_or("")
}

pub fn run(romfs: &P, root: &P, jobs: &[Job]) -> Res<Vec<String>> {
    let mut rf: Romfs = Romfs::open(romfs)?;
    let mut bad: Vec<String> = Vec::new();
    let mut out: Vec<String> = Vec::new();
    let ts: Vec<Byml> = [ACTOR_INFO, GAME_ACTOR_INFO, POUCH_INFO].iter().map(|t| Byml::parse(&read(&mut rf, root, t)?)).collect::<Res<Vec<Byml>>>()?;
    let ids: Vec<Vec<&str>> = ts[..2].iter().map(|y| y.root.as_arr().unwrap_or(&[]).iter().filter_map(rows::id).collect()).collect();
    if ids[0] != ids[1] {
        bad.push("ActorInfo and GameActorInfo no longer list the same rows in the same order".to_string());
    }
    for (i, y) in ts.iter().enumerate() {
        if !rows::sorted(y)? {
            bad.push(format!("actor table {i} is not sorted"));
        }
    }
    let rst: Rst = Rst::parse(&read(&mut rf, root, TABLE_PATH)?)?;
    let stock: Rst = rsizetable::load(&mut rf)?;
    let mut packs: BM<String, Vec<Entry>> = BM::new();
    for k in KEEPER_PACKS {
        packs.insert(k.to_string(), sarc::parse(&read(&mut rf, root, &pack_of(k))?)?);
    }
    let mut seen: BM<String, Vec<u8>> = BM::new();
    for es in packs.values() {
        for e in es.iter().filter(|e| e.name.contains("ShopParam/")) {
            match seen.get(&e.name) {
                Some(x) if *x != e.data => bad.push(format!("{} differs between keeper packs", e.name)),
                Some(_) => {}
                None => {
                    seen.insert(e.name.clone(), e.data.clone());
                }
            }
        }
    }
    let mut checked: usize = 0;
    for (k, es) in &packs {
        let old: Vec<Entry> = sarc::parse(&rf.read_zs(&pack_of(k))?)?;
        for e in es.iter().filter(|e| e.name.contains("ShopParam/")) {
            let Some(o) = sarc::get(&old, &e.name) else {
                // A goods list the build added (shelf::split) needs a size entry of its own.
                checked += 1;
                if !rst.get(&e.name).is_some_and(|n| n as usize >= e.data.len()) {
                    bad.push(format!("new inner file {} in {k} has no size table entry that fits {} bytes", e.name, e.data.len()));
                }
                continue;
            };
            if o.data == e.data {
                continue;
            }
            let was: u32 = stock.get(&e.name).ok_or_else(|| E::Msg(format!("{} has no vanilla size table entry", e.name)))?;
            checked += 1;
            if !inner::enough(&rst, &e.name, was, o.data.len(), e.data.len()) {
                bad.push(format!("inner file {} in {k}: size table entry {:?} is too small for {} bytes", e.name, rst.get(&e.name), e.data.len()));
            }
        }
    }
    out.push(format!("edited ShopParam copies with a grown inner size entry: {checked}"));
    let bancs: BM<String, Byml> = {
        let mut m: BM<String, Byml> = BM::new();
        for j in jobs {
            for b in &j.bancs {
                if !m.contains_key(b) {
                    m.insert(b.clone(), Byml::parse(&read(&mut rf, root, b)?)?);
                }
            }
        }
        m
    };
    let gdl: Byml = Byml::parse(&read(&mut rf, root, LIST)?)?;
    let mut langs: Vec<String> = rf.list("Mals")?.into_iter().filter(|f| f.ends_with(text::ARCHIVE_TAIL)).collect();
    langs.sort();
    let mals: Vec<Vec<u8>> = if jobs.iter().any(|j| matches!(j.what, What::Deal { .. })) { langs.iter().map(|f| read(&mut rf, root, f)).collect::<Res<Vec<Vec<u8>>>>()? } else { Vec::new() };
    for j in jobs {
        let want: String = j.shelf_actor();
        for k in &j.keepers {
            let es: &Vec<Entry> = packs.get(k).ok_or_else(|| E::Msg(format!("{k} is not a shelf keeper")))?;
            let f: String = goods::shop_ref(es, k)?;
            let g: Vec<Node> = goods::goods(&sarc::get(es, &f).ok_or_else(|| E::Msg(format!("{k} lost {f}")))?.data)?;
            let e: &Node = g.get(j.slot).ok_or(E::Format("a GoodsList lost a slot"))?;
            let ga: &str = actor_of(text_of(e, "Actor"));
            let base: i64 = rows::find(&ts[2], ga).and_then(|r| int(r, "BuyingPrice")).unwrap_or(-1);
            let price: i64 = base + int(e, "PriceOffset").unwrap_or(0);
            let conds: usize = e.get("ConditionalPriceOffsetList").and_then(|l| l.as_arr()).map(|l| l.len()).unwrap_or(0);
            let rel: bool = e.get("Release").is_some_and(|r| r.get("IsCheckReleaseGameData") == Some(&Node::Bool(true)) || r.get("IsCheckSoldOutGameData") == Some(&Node::Bool(true)));
            let stock_ok: bool = match j.what {
                What::Deal { .. } | What::Once { .. } => int(e, "StockNum") == Some(1),
                What::Goods { .. } => int(e, "StockNum").is_none_or(|s| s < 0),
            };
            let shelf: Vec<String> = j.bancs.iter().flat_map(|b| banc::on_shelf(&bancs[b], k, j.slot)).collect();
            let ok: bool = ga == want && g.len() == 8 && price == j.price() as i64 && conds == 0 && !rel && stock_ok && !shelf.is_empty() && shelf.iter().all(|x| *x == want);
            if !ok {
                bad.push(format!("{} {k} slot {}: goods {ga} price {price} stock {:?} gates {rel} discounts {conds} shelf {shelf:?}, want {want} at {}", j.label, j.slot, int(e, "StockNum"), j.price()));
            }
        }
        if j.once() {
            for cn in &j.counters {
                if counters::reset_of(&gdl, cn) != Some(counters::KEPT as i64) {
                    bad.push(format!("{} counter {cn} still resets daily", j.label));
                }
            }
        }
        if let What::Deal { .. } = j.what {
            let es: Vec<Entry> = sarc::parse(&read(&mut rf, root, &pack_of(&want))?)?;
            let mut p: Pool = Pool::default();
            p.add_pack(&es)?;
            let c: Chain = chain(&p, &want)?;
            let parent: bool = c.files.get(1).is_some_and(|f| f == &actor_file(BASE));
            let q: Byml = p.byml(&pouch_file(&want))?;
            let who: String = q.root.get(POUCH_ACTOR).and_then(|v| v.as_str()).map(actor_of).unwrap_or("").to_string();
            let own: Option<i64> = p.byml(&price_file(&want)).ok().and_then(|y| int(&y.root, "BuyingPrice"));
            let rows_ok: bool = ts.iter().all(|y| rows::find(y, &want).is_some());
            let sized: bool = [pack_of(&want), actor_file(&want), pouch_file(&want), price_file(&want), table_file(&want)].iter().all(|f| rst.get(f).is_some());
            let named: usize = mals.iter().filter(|m| text::has(m, &who).unwrap_or(false)).count();
            if !(parent && rows_ok && sized && own == Some(j.price() as i64) && !who.is_empty() && named == mals.len() && !mals.is_empty()) {
                bad.push(format!("{} shelf actor {want}: parent {parent} rows {rows_ok} sizes {sized} own price {own:?} identity {who:?} text in {named} of {} archives", j.label, mals.len()));
            }
        }
    }
    out.push(format!("{} shelf slots read back, {} problems", jobs.len(), bad.len()));
    if !bad.is_empty() {
        return Err(E::Msg(format!("shelf audit failed:\n  {}", bad.join("\n  "))));
    }
    Ok(out)
}
