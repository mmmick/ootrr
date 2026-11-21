use crate::settings::model::Setting;
use crate::settings::Settings;
use crate::start_defaults::Edit;

pub const SETTING: &str = "nocturne_trigger";
pub const VANILLA: &str = "vanilla";
pub const CLASSIC: &str = "classic";
pub const EXP: &str = "SageOfShadow_Kakariko_Burning";
pub const MEDALS: [&str; 3] = ["IsGet.Item_Medal_Forest", "IsGet.Item_Medal_Fire", "IsGet.Item_Medal_Water"];
pub const NOTE: &str = "Nocturne Trigger, Classic: Kakariko burns and Sheik teaches the Nocturne of Shadow once adult Link holds the Forest, Fire and Water Medallions (SageOfShadow_Kakariko_Burning is rewritten to the three medallions).";

pub fn settings() -> Vec<Setting> {
    vec![Setting::choice(137, SETTING, "goal", "Nocturne Trigger", VANILLA)
        .tip("What makes Kakariko burn, where Sheik teaches the Nocturne of Shadow to adult Link. Untested in the game.")
        .opt(VANILLA, "Vanilla", "The Forest, Fire and Water Temples are cleared, as in the remake.")
        .opt(CLASSIC, "Classic", "Hold the Forest, Fire and Water Medallions, as in the original game.")]
}

pub fn read(s: &Settings, bad: &mut Vec<String>) -> bool {
    match s.text(SETTING) {
        Ok(v) => v == CLASSIC,
        Err(e) => {
            bad.push(e.to_string());
            false
        }
    }
}

pub fn exp() -> Option<Edit> {
    crate::medal_gate::at_least(&MEDALS, MEDALS.len()).map(|t| Edit::Exp(EXP.to_string(), t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_rewrites_the_burning_to_the_three_medallions() {
        let Some(Edit::Exp(name, t)) = exp() else { panic!("no edit") };
        assert_eq!(name, EXP);
        for m in MEDALS {
            assert!(t.iter().any(|(_, f)| f == m), "{m} missing from {t:?}");
        }
    }
}
