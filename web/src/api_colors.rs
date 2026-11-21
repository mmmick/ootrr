use ootrr_core::colors::{self, Colors, Pick, Picks};
use ootrr_core::cosmetic::fresh_seed;
use ootrr_core::json::Json;

use crate::body;
use crate::error::{Fail, Hr};

pub fn roll(b: &Json) -> Hr<Json> {
    let c: Colors = match body::opt_text(b, "style")? {
        None | Some("") | Some("random") => Colors::Random,
        Some("shared") => Colors::Shared,
        Some(_) => return Err(Fail::new(422, "Colours: the style is random or shared.")),
    };
    let seed: String = fresh_seed();
    let mut out: Json = Json::obj();
    for (it, to) in colors::plan(c, &seed, &Picks::new()) {
        out.put(it.id, Pick::Rgb(to).id().into());
    }
    Ok(Json::obj().with("colors", out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_part_gets_a_colour_and_bad_styles_are_refused() {
        for s in ["random", "shared"] {
            let j: Json = roll(&Json::obj().with("style", s.into())).unwrap();
            let o: &[(String, Json)] = j.get("colors").and_then(|x| x.as_obj()).unwrap();
            assert_eq!(o.len(), colors::ITEMS.len());
            assert!(o.iter().all(|(_, v)| v.as_str().is_some_and(|t| t.len() == 7 && t.starts_with('#'))));
        }
        assert!(roll(&Json::obj().with("style", "neon".into())).is_err());
    }
}
