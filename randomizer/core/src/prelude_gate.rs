use crate::romfs::Romfs;
use crate::settings::model::Setting;
use crate::settings::Settings;
use crate::staging::Out;
use crate::start_defaults::Edit;

pub const SETTING: &str = "prelude_trigger";
pub const VANILLA: &str = "vanilla";
pub const CLASSIC: &str = "classic";
pub const LOGIC_KEY: &str = "prelude_medallion";

pub fn settings() -> Vec<Setting> {
    vec![Setting::choice(138, SETTING, "goal", "Prelude Trigger", VANILLA)
        .tip("What makes Sheik teach the Prelude of Light to adult Link in the Temple of Time. Untested in the game.")
        .opt(VANILLA, "Vanilla", "The Forest Temple is cleared, as in the remake.")
        .opt(CLASSIC, "Classic", "Hold the Forest Medallion, as in the original game.")]
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

pub fn exp() -> Edit {
    Edit::Exp(crate::medal_gate::PRELUDE_EXP.to_string(), vec![(0, crate::medal_gate::medal("Forest"))])
}

pub fn stage(rf: &mut Romfs, more: &mut Vec<Out>) -> crate::error::Res<()> {
    more.push(crate::medal_gate::module(rf, crate::medal_gate::PRELUDE_MODULE, crate::medal_gate::patch_prelude)?);
    Ok(())
}
