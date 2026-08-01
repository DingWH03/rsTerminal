//! Lightweight application preferences (JSON).
//!
//! Terminal profiles, connections, and auth users live in SQLite — see [`super::persist`].

mod appearance;
mod chrome;
mod general;
pub(crate) mod io;

pub use appearance::AppearancePrefs;
pub use chrome::ChromePrefs;
pub use general::GeneralPrefs;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Shell-level preferences persisted to `prefs.json`.
#[derive(Debug, Clone, Default)]
pub struct Prefs {
    pub general: GeneralPrefs,
    pub appearance: AppearancePrefs,
    pub chrome: ChromePrefs,
}

impl Prefs {
    pub fn language(&self) -> crate::i18n::Language {
        self.general.language
    }

    pub fn ui_theme(&self) -> crate::i18n::UiTheme {
        self.appearance.ui_theme
    }

    pub fn pane_accent_colors(&self) -> &Vec<[u8; 3]> {
        &self.appearance.pane_accent_colors
    }

    pub fn pane_accent_colors_mut(&mut self) -> &mut Vec<[u8; 3]> {
        &mut self.appearance.pane_accent_colors
    }
}

impl Serialize for Prefs {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("Prefs", 3)?;
        state.serialize_field("general", &self.general)?;
        state.serialize_field("appearance", &self.appearance)?;
        state.serialize_field("chrome", &self.chrome)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Prefs {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(default)]
            general: Option<GeneralPrefs>,
            #[serde(default)]
            language: Option<crate::i18n::Language>,
            #[serde(default)]
            appearance: AppearancePrefs,
            #[serde(default)]
            chrome: Option<ChromePrefs>,
            #[serde(default)]
            function_pane_width: Option<f32>,
            #[serde(default)]
            default_local_connection_id: Option<String>,
        }

        let raw = Raw::deserialize(deserializer)?;
        let general = raw.general.unwrap_or_else(|| GeneralPrefs {
            language: raw.language.unwrap_or_default(),
        });
        let chrome = raw.chrome.unwrap_or(ChromePrefs {
            function_pane_width: raw.function_pane_width,
            default_local_connection_id: raw.default_local_connection_id,
        });
        Ok(Self {
            general,
            appearance: raw.appearance,
            chrome,
        })
    }
}

pub fn load_prefs() -> Prefs {
    io::load_prefs()
}

pub fn save_prefs(prefs: &Prefs) {
    io::save_prefs(prefs)
}
