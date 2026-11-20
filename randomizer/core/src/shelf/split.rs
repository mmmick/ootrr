//! The two Potion Shops get a goods list each.
//!
//! The Market Potion Shop keeper (Npc_Hylia_020_Shop_Child) and the Kakariko one
//! (Npc_Hylia_020_Shop_Adult, whose actor inherits from the child one) read one ShopParam file,
//! Npc_Hylia_020_Shop, so both shops always sold the same goods slot by slot. The Kakariko keeper's
//! own actor file gets a ShopRef of its own that points at a copy of that file inside its pack, so
//! each shop's shelves can sell different goods and special deals. Each keeper already has its own
//! sold counters (Shop_SoldNum.<keeper>.<slot>).

use crate::byml::{Byml, Node};
use crate::error::{Error as E, Res};
use crate::pickup_pack::closure::{actor_file, local, COMPONENTS};
use crate::rsizetable::Rst;
use crate::sarc::{self, Entry};

pub const KEEPER: &str = "Npc_Hylia_020_Shop_Adult";
pub const SHARED: &str = "Component/ShopParam/Npc_Hylia_020_Shop.game__component__ShopParam.bgyml";
pub const OWN: &str = "Component/ShopParam/Npc_Hylia_020_Shop_Adult.game__component__ShopParam.bgyml";
pub const REF: &str = "ShopRef";

/// The keeper's actor file, which gets the new ShopRef.
pub fn actor() -> String {
    actor_file(KEEPER)
}

/// What a split changed inside the keeper's pack: (inner file, old size, new size). Empty when the
/// pack was split before (a seed mod the build reads back).
pub type Grown = Vec<(String, usize, usize)>;

/// Gives the Kakariko Potion Shop keeper's pack its own goods list. Idempotent.
pub fn split(es: &mut Vec<Entry>) -> Res<Grown> {
    if sarc::get(es, OWN).is_some() {
        return Ok(Vec::new());
    }
    let a: String = actor();
    let raw: Vec<u8> = sarc::get(es, &a).map(|e| e.data.clone()).ok_or_else(|| E::Msg(format!("{KEEPER} has no actor file {a}")))?;
    let shared: Vec<u8> = sarc::get(es, SHARED).map(|e| e.data.clone()).ok_or_else(|| E::Msg(format!("{KEEPER} does not hold {SHARED}")))?;
    let mut y: Byml = Byml::parse(&raw)?;
    if y.write()? != raw {
        return Err(E::Format("the Kakariko Potion Shop keeper's actor file does not round trip"));
    }
    let comps: &mut Node = y.root.get_mut(COMPONENTS).ok_or(E::Format("the Kakariko Potion Shop keeper's actor file has no Components"))?;
    if comps.get(REF).is_some() {
        return Err(E::Format("the Kakariko Potion Shop keeper already names its own goods list"));
    }
    comps.set(REF, Node::Str(local(OWN)));
    let out: Vec<u8> = y.write()?;
    sarc::set(es, &a, out.clone());
    sarc::set(es, OWN, shared.clone());
    Ok(vec![(a, raw.len(), out.len())])
}

/// Gives the new goods list the size table entry of the one it copies, so its own edits can grow it.
pub fn size(rst: &mut Rst) -> Res<()> {
    if rst.get(OWN).is_some() {
        return Ok(());
    }
    let n: u32 = rst.get(SHARED).ok_or_else(|| E::Msg(format!("{SHARED} has no size table entry")))?;
    rst.set_size(OWN, n)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pickup_pack::closure::{chain, Pool};
    use crate::pickup_pack::pack_of;
    use crate::shelf::goods::shop_ref;

    #[test]
    fn the_kakariko_potion_keeper_reads_its_own_copy_of_the_goods_list() {
        let p = ootrr_game::test_romfs::path();
        if !p.join("Pack").join("ZsDic.pack.zs").is_file() {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            return;
        }
        let mut rf = crate::romfs::Romfs::open(&p).unwrap();
        let mut es: Vec<Entry> = sarc::parse(&rf.read_zs(&pack_of(KEEPER)).unwrap()).unwrap();
        assert_eq!(shop_ref(&es, KEEPER).unwrap(), SHARED);
        let grown: Grown = split(&mut es).unwrap();
        assert_eq!(grown.len(), 1);
        assert!(grown[0].2 > grown[0].1);
        assert_eq!(shop_ref(&es, KEEPER).unwrap(), OWN);
        assert_eq!(sarc::get(&es, OWN).unwrap().data, sarc::get(&es, SHARED).unwrap().data);
        let back: Vec<Entry> = sarc::parse(&sarc::write(&es).unwrap()).unwrap();
        assert_eq!(shop_ref(&back, KEEPER).unwrap(), OWN);
        let mut pool: Pool = Pool::default();
        pool.add_pack(&back).unwrap();
        let c = chain(&pool, KEEPER).unwrap();
        assert_eq!(c.comps.len(), {
            let mut q: Pool = Pool::default();
            q.add_pack(&sarc::parse(&rf.read_zs(&pack_of(KEEPER)).unwrap()).unwrap()).unwrap();
            chain(&q, KEEPER).unwrap().comps.len()
        });
        assert!(split(&mut es).unwrap().is_empty(), "a second split changes nothing");
        let child: Vec<Entry> = sarc::parse(&rf.read_zs(&pack_of("Npc_Hylia_020_Shop_Child")).unwrap()).unwrap();
        assert_eq!(shop_ref(&child, "Npc_Hylia_020_Shop_Child").unwrap(), SHARED);
        let mut rst: Rst = crate::rsizetable::load(&mut rf).unwrap();
        assert!(rst.get(OWN).is_none());
        size(&mut rst).unwrap();
        assert_eq!(rst.get(OWN), rst.get(SHARED));
    }
}
