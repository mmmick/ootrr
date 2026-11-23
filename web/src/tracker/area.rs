use ootrr_core::world::World;

pub const PREFIXES: [(&str, &str); 26] = [
    ("KF ", "Kokiri Forest"),
    ("LW ", "Lost Woods"),
    ("Deku Theater ", "Lost Woods"),
    ("SFM ", "Sacred Forest Meadow"),
    ("HF ", "Hyrule Field"),
    ("LLR ", "Lon Lon Ranch"),
    ("Market ", "Market"),
    ("ToT ", "Temple of Time"),
    ("HC ", "Hyrule Castle"),
    ("OGC ", "Outside Ganon's Castle"),
    ("Kak ", "Kakariko Village"),
    ("Graveyard ", "Graveyard"),
    ("DMT ", "Death Mountain Trail"),
    ("GC ", "Goron City"),
    ("DMC ", "Death Mountain Crater"),
    ("ZR ", "Zora's River"),
    ("ZD ", "Zora's Domain"),
    ("ZF ", "Zora's Fountain"),
    ("LH ", "Lake Hylia"),
    ("GV ", "Gerudo Valley"),
    ("GF ", "Gerudo Fortress"),
    ("Hideout ", "Thieves' Hideout"),
    ("Wasteland ", "Haunted Wasteland"),
    ("Colossus ", "Desert Colossus"),
    ("Sheik at Colossus", "Desert Colossus"),
    ("Song from ", ""),
];

pub const MOVED: [(&str, &str); 1] = [("HF Lake Poe Chest", "Lake Hylia")];

fn fold(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect()
}

pub fn canon(area: &str, w: &World) -> String {
    let k: String = fold(area);
    w.regions.iter().filter_map(|r| r.dungeon.as_deref()).find(|d| fold(d) == k).unwrap_or(area).to_string()
}

pub fn of_name(name: &str) -> Option<&'static str> {
    if let Some((_, a)) = MOVED.iter().find(|(n, _)| *n == name) {
        return Some(a);
    }
    PREFIXES.iter().find(|(p, _)| name.starts_with(p)).map(|(_, a)| *a).filter(|a| !a.is_empty())
}

pub fn short(name: &str, area: &str) -> String {
    if let Some(r) = MOVED.iter().find(|(n, a)| *n == name && *a == area).and_then(|_| name.split_once(' ')).map(|(_, r)| r) {
        return r.to_string();
    }
    let cut: Option<&str> = PREFIXES.iter().find(|(p, a)| *a == area && name.starts_with(p) && p.ends_with(' ')).and_then(|(p, _)| name.strip_prefix(p));
    let cut: Option<&str> = cut.or_else(|| name.strip_prefix(area).and_then(|r| r.strip_prefix(' ')));
    cut.filter(|c| !c.is_empty()).unwrap_or(name).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes_name_the_area_and_short_names_drop_them() {
        assert_eq!(of_name("KF Midos Top Left Chest"), Some("Kokiri Forest"));
        assert_eq!(of_name("Song from Impa"), None);
        assert_eq!(of_name("HF Lake Poe Chest"), Some("Lake Hylia"));
        assert_eq!(short("HF Lake Poe Chest", "Lake Hylia"), "Lake Poe Chest");
        assert_eq!(of_name("HF South Poe Chest"), Some("Hyrule Field"));
        assert_eq!(of_name("Deku Tree Map Chest"), None);
        assert_eq!(short("KF Midos Top Left Chest", "Kokiri Forest"), "Midos Top Left Chest");
        assert_eq!(short("Deku Tree Map Chest", "Deku Tree"), "Map Chest");
        assert_eq!(short("Song from Impa", "Hyrule Castle"), "Song from Impa");
        assert_eq!(short("Graveyard Shield Grave Chest", "Graveyard"), "Shield Grave Chest");
    }
}
