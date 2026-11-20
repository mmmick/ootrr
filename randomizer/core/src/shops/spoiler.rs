use std::fmt::Write as W;

use crate::shops::data::{table, Slot, Table};
use crate::shops::layout::Layout;
use crate::shops::price::wallets;

fn wallet_word(price: i32) -> &'static str {
    match wallets(price) {
        0 => "",
        1 => ", needs the Adult's Wallet",
        _ => ", needs the Giant's Wallet",
    }
}

pub fn text(l: &Layout) -> String {
    let mut out: String = String::new();
    let Ok(t) = table() else {
        return out;
    };
    let t: &Table = t;
    let _ = writeln!(out, "Shops, {} special deals", l.deal_count());
    for sh in &t.shops {
        let _ = writeln!(out, "  {}", sh.shop);
        let mut ss: Vec<&Slot> = t.slots.iter().filter(|s| s.shop == sh.shop).collect();
        ss.sort_by_key(|s| s.slot);
        for s in ss {
            let Some(p) = l.get(&s.name) else {
                continue;
            };
            let kind: &str = if p.deal { "special deal, sold once" } else { "shop goods" };
            let _ = writeln!(out, "    shelf {} ({}): {}, {} rupees, {kind}{} (vanilla {})", s.slot, s.name, crate::spoiler::shown(&p.item), p.price, wallet_word(p.price), crate::spoiler::shown(&s.item));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shops::layout::make;
    use crate::shops::plan::{Level, Opts, Prices};

    #[test]
    fn every_slot_is_listed_with_its_price() {
        let l: Layout = make(&Opts { level: Level::Shuffle, prices: Prices::Weighted, min: 10, max: 200, kinds: crate::shops::plan::Kinds::default() }, "sp", crate::shops::layout::Room::any(64)).unwrap();
        let s: String = text(&l);
        assert!(s.starts_with("Shops, 0 special deals"));
        assert_eq!(s.lines().filter(|x| x.contains("rupees")).count(), 64);
        assert!(wallet_word(600).contains("Giant's Wallet"));
    }
}
