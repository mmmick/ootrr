use std::collections::BTreeMap as BM;
use std::sync::OnceLock;

use ootrr_core::json;
use ootrr_core::slice::data::VANILLA_JSON;

use crate::tracker::targets::Target;

static VANILLA: OnceLock<BM<String, String>> = OnceLock::new();

pub fn vanilla() -> &'static BM<String, String> {
    VANILLA.get_or_init(|| {
        let Ok(j) = json::parse(VANILLA_JSON) else {
            return BM::new();
        };
        j.get("locations")
            .and_then(|l| l.as_obj())
            .map(|o| o.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.to_string(), s.to_string()))).collect())
            .unwrap_or_default()
    })
}

pub fn item_of<'a>(t: &Target, placed: Option<&'a BM<String, String>>) -> Option<&'a str> {
    let o: &str = t.ootr.as_deref()?;
    match placed {
        Some(p) => p.get(o).map(|s| s.as_str()),
        None => vanilla().get(o).map(|s| s.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracker::targets;

    fn named(o: &str) -> &'static Target {
        targets::all().unwrap().iter().find(|t| t.ootr.as_deref() == Some(o)).unwrap()
    }

    #[test]
    fn a_seed_names_the_placed_item_and_without_one_the_vanilla_item() {
        let t: &Target = named("Song from Saria");
        assert_eq!(item_of(t, None), Some("Sarias Song"));
        let mut p: BM<String, String> = BM::new();
        p.insert("Song from Saria".into(), "Progressive Hookshot".into());
        assert_eq!(item_of(t, Some(&p)), Some("Progressive Hookshot"));
        assert_eq!(item_of(t, Some(&BM::new())), None);
        assert!(vanilla().len() > 1000);
    }
}
