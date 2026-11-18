use std::collections::BTreeMap as BM;

use crate::bfevfl::Bf;
use crate::error::{Error as E, Res};
use crate::flow_extra::data::{Extra, Spec};
use crate::flow_extra::op::{apply, Op};
use crate::flow_locations::{edit_bytes as plain, set_param, Change, Req};

pub fn always_ops() -> Res<BM<String, Vec<Op>>> {
    let x: Extra = Extra::load()?;
    let mut out: BM<String, Vec<Op>> = BM::new();
    for s in &x.always {
        for o in &s.ops {
            out.entry(o.file.clone()).or_default().push(o.clone());
        }
    }
    Ok(out)
}

pub fn ops_for(changes: &[Change]) -> Res<BM<String, Vec<Op>>> {
    let x: Extra = Extra::load()?;
    let mut seen: Vec<&str> = Vec::new();
    let mut out: BM<String, Vec<Op>> = always_ops()?;
    for c in changes {
        if seen.contains(&c.location.as_str()) {
            continue;
        }
        seen.push(&c.location);
        let Some(s) = x.by_name(&c.location) else {
            continue;
        };
        for o in &s.ops {
            let b: Op = o.bind(&c.new);
            let list: &mut Vec<Op> = out.entry(o.file.clone()).or_default();
            if !list.contains(&b) {
                list.push(b);
            }
        }
    }
    Ok(out)
}

pub fn ops_of(location: &str, item: &str) -> Res<Vec<Op>> {
    let x: Extra = Extra::load()?;
    let s: Option<&Spec> = x.by_name(location);
    Ok(s.map(|s| s.ops.iter().map(|o| o.bind(item)).collect()).unwrap_or_default())
}

pub fn edit_flow(raw: &[u8], rs: &[Req], os: &[Op]) -> Res<Bf> {
    let mut b: Bf = Bf::parse(raw)?;
    for r in rs {
        set_param(&mut b, r)?;
    }
    for o in os {
        apply(&mut b, o).map_err(|e| E::Msg(format!("{} edit on {}: {e}", o.kind(), o.file)))?;
    }
    Ok(b)
}

pub fn edit_bytes(raw: &[u8], rs: &[Req], os: &[Op]) -> Res<Vec<u8>> {
    if os.is_empty() {
        return plain(raw, rs);
    }
    let b: Bf = edit_flow(raw, rs, os)?;
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(E::Format("flow extra: the written flow does not parse back to the edited model"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(loc: &str, file: &str, new: &str) -> Change {
        Change {
            location: loc.to_string(),
            file: file.to_string(),
            index: 0,
            event: "E".to_string(),
            param: "ActorName".to_string(),
            role: "grant".to_string(),
            old: "Old".to_string(),
            new: new.to_string(),
        }
    }

    #[test]
    fn ops_follow_the_changed_locations_and_bind_the_item() {
        let by: BM<String, Vec<Op>> = ops_for(&[ch("GC Medigoron", "Npc_Goron_Medigoron_Adult.bfevfl", "Item_Longshot"), ch("Volvagia", "X.bfevfl", "Item_A")]).unwrap();
        assert_eq!(
            by.keys().cloned().collect::<Vec<String>>(),
            vec![
                "Dm_FairyFountain_Din.bfevfl".to_string(),
                "Dm_SY_Get.bfevfl".to_string(),
                "Npc_Gerudo_006.bfevfl".to_string(),
                "Npc_Goron_Medigoron_Adult.bfevfl".to_string(),
                "Npc_Kaepora_DeathMountainTrail.bfevfl".to_string(),
                "SimpleShop.bfevfl".to_string(),
                "TriForcePoint.bfevfl".to_string()
            ]
        );
        let txt: String = format!("{by:?}");
        assert!(txt.contains("Item_Longshot") && !txt.contains("$item"));
        let always: BM<String, Vec<Op>> = ops_for(&[]).unwrap();
        assert_eq!(always, always_ops().unwrap());
        assert_eq!(
            always.keys().cloned().collect::<Vec<String>>(),
            vec!["Dm_FairyFountain_Din.bfevfl".to_string(), "Dm_SY_Get.bfevfl".to_string(), "Npc_Gerudo_006.bfevfl".to_string(), "Npc_Kaepora_DeathMountainTrail.bfevfl".to_string(), "TriForcePoint.bfevfl".to_string()]
        );
        assert!(always.values().flatten().all(|o| matches!(o.how, crate::flow_extra::op::How::Retarget { .. } | crate::flow_extra::op::How::Backfill(_) | crate::flow_extra::op::How::Next { .. })));
        assert!(ops_of("Volvagia", "Item_A").unwrap().is_empty());
        assert_eq!(ops_of("DMT Biggoron", "Item_A").unwrap().len(), 3);
    }
}
