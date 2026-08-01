//! Lightweight application preferences (JSON).
//!
//! Terminal profiles, connections, and auth users live in SQLite — not here.

mod appearance;

pub use appearance::AppearancePrefs;

use serde::{Deserialize, Serialize};

use crate::i18n::Language;

/// Shell-level preferences persisted to `prefs.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefs {
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub appearance: AppearancePrefs,
    #[serde(default)]
    pub function_pane_width: Option<f32>,
    #[serde(default)]
    pub default_local_connection_id: Option<String>,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            language: Language::default(),
            appearance: AppearancePrefs::default(),
            function_pane_width: None,
            default_local_connection_id: None,
        }
    }
}

impl Prefs {
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

pub fn load_prefs() -> Prefs {
    crate::persist::prefs::load_prefs()
}

pub fn save_prefs(prefs: &Prefs) {
    crate::persist::prefs::save_prefs(prefs)
}
