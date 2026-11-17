use std::collections::BTreeMap as BM;

use ootrr_core::error::{Error as E, Res};
use ootrr_core::spoiler::unshown;

#[derive(Debug, Clone, PartialEq)]
pub struct SeedLog {
    pub seed: String,
    pub settings_string: String,
    pub checksum: String,
    pub placements: BM<String, String>,
}

fn head<'a>(t: &'a str, k: &str) -> Option<&'a str> {
    t.lines().find_map(|l| l.strip_prefix(k)).map(|v| v.trim())
}

pub fn parse(t: &str) -> Res<SeedLog> {
    let seed: &str = head(t, "Seed:").filter(|s| !s.is_empty()).ok_or_else(|| E::Msg("spoiler: no Seed line".to_string()))?;
    let settings_string: &str =
        head(t, "Settings string:").filter(|s| !s.is_empty()).ok_or_else(|| E::Msg("spoiler: no Settings string line".to_string()))?;
    let checksum: &str = head(t, "Checksum:").unwrap_or("");
    let mut placements: BM<String, String> = BM::new();
    let mut on: bool = false;
    for l in t.lines() {
        if l.starts_with("Locations (") {
            on = true;
            continue;
        }
        if !on || l.starts_with('=') {
            continue;
        }
        let Some(row) = l.strip_prefix("  ") else {
            if !l.trim().is_empty() {
                on = false;
            }
            continue;
        };
        if let Some((k, v)) = row.split_once(": ") {
            placements.insert(k.trim().to_string(), unshown(v.trim()).to_string());
        }
    }
    if placements.is_empty() {
        return Err(E::Msg("spoiler: the Locations section is missing or empty".to_string()));
    }
    Ok(SeedLog { seed: seed.to_string(), settings_string: settings_string.to_string(), checksum: checksum.to_string(), placements })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "OOTRR spoiler log\n====\nSeed:            abc\nSettings string: OOTRR1-AA\nChecksum:        ff\n\nPlaythrough (1 steps)\n====\nStep 1\n  KF Midos Top Left Chest: Bow\n\nLocations (2)\n====\n  KF Midos Top Left Chest: Boomerang\n  KF Kokiri Sword Chest: Rupees (5)\n";

    #[test]
    fn the_locations_section_and_header_are_read() {
        let s: SeedLog = parse(LOG).unwrap();
        assert_eq!((s.seed.as_str(), s.settings_string.as_str(), s.checksum.as_str()), ("abc", "OOTRR1-AA", "ff"));
        assert_eq!(s.placements.len(), 2);
        assert_eq!(s.placements.get("KF Midos Top Left Chest").map(|x| x.as_str()), Some("Boomerang"));
        assert_eq!(s.placements.get("KF Kokiri Sword Chest").map(|x| x.as_str()), Some("Rupees (5)"));
    }

    #[test]
    fn progressive_names_from_newer_spoilers_read_as_logic_names() {
        let t: String = LOG.replace("Boomerang", "Progressive Ocarina").replace("Rupees (5)", "Progressive Slingshot");
        let s: SeedLog = parse(&t).unwrap();
        assert_eq!(s.placements.get("KF Midos Top Left Chest").map(|x| x.as_str()), Some("Ocarina"));
        assert_eq!(s.placements.get("KF Kokiri Sword Chest").map(|x| x.as_str()), Some("Slingshot"));
    }

    #[test]
    fn a_log_without_locations_is_refused() {
        assert!(parse("Seed: a\nSettings string: b\n").is_err());
    }
}
