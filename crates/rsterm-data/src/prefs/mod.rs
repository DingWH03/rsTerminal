//! Lightweight application preferences (JSON).
//!
//! Terminal profiles, connections, and auth users live in SQLite — see [`super::persist`].

mod appearance;
mod chrome;
mod file_manager;
mod general;
mod input_mode;
pub(crate) mod io;
mod ui_state;

pub use appearance::AppearancePrefs;
pub use chrome::ChromePrefs;
pub use file_manager::{FileManagerPrefs, PrefsFilePaneLayout, PrefsFileViewMode};
pub use general::GeneralPrefs;
pub use input_mode::{InputInteractionMode, default_input_mode};
pub use ui_state::{FileManagerUiState, UiStatePrefs};

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Shell-level preferences persisted to `prefs.json`.
#[derive(Debug, Clone, Default)]
pub struct Prefs {
    pub general: GeneralPrefs,
    pub appearance: AppearancePrefs,
    pub chrome: ChromePrefs,
    pub file_manager: FileManagerPrefs,
    /// Silent persistence (no Settings UI): column widths, etc.
    pub ui_state: UiStatePrefs,
}

impl Prefs {
    pub fn language(&self) -> rsterm_config::Language {
        self.general.language
    }

    pub fn ui_theme(&self) -> rsterm_config::UiTheme {
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
        let mut state = serializer.serialize_struct("Prefs", 5)?;
        state.serialize_field("general", &self.general)?;
        state.serialize_field("appearance", &self.appearance)?;
        state.serialize_field("chrome", &self.chrome)?;
        state.serialize_field("file_manager", &self.file_manager)?;
        state.serialize_field("ui_state", &self.ui_state)?;
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
            language: Option<rsterm_config::Language>,
            #[serde(default)]
            appearance: AppearancePrefs,
            #[serde(default)]
            chrome: Option<ChromePrefs>,
            #[serde(default)]
            function_pane_width: Option<f32>,
            #[serde(default)]
            default_local_connection_id: Option<String>,
            #[serde(default)]
            file_manager: FileManagerPrefs,
            #[serde(default)]
            ui_state: UiStatePrefs,
        }

        let raw = Raw::deserialize(deserializer)?;
        let general = raw.general.unwrap_or_else(|| GeneralPrefs {
            language: raw.language.unwrap_or_default(),
            input_mode: default_input_mode(),
        });
        let chrome = raw.chrome.unwrap_or(ChromePrefs {
            function_pane_width: raw.function_pane_width,
            default_local_connection_id: raw.default_local_connection_id,
        });
        Ok(Self {
            general,
            appearance: raw.appearance,
            chrome,
            file_manager: raw.file_manager,
            ui_state: raw.ui_state,
        })
    }
}

pub fn load_prefs() -> Prefs {
    io::load_prefs()
}

pub fn save_prefs(prefs: &Prefs) {
    io::save_prefs(prefs)
}
