//! Prefs JSON load/save (`prefs.json`), with legacy `settings.json` migration.

use serde::Deserialize;
use std::collections::HashMap;

use super::{
    AppearancePrefs, ChromePrefs, FileManagerPrefs, GeneralPrefs, Prefs, UiStatePrefs,
    default_input_mode,
};
use crate::paths::config_dir;
use crate::persist::types::LegacyProfileJson;
use rsterm_config::{Language, UiTheme};

fn prefs_path() -> Option<std::path::PathBuf> {
    config_dir().map(|p| p.join("prefs.json"))
}

fn legacy_settings_path() -> Option<std::path::PathBuf> {
    config_dir().map(|p| p.join("settings.json"))
}

/// Legacy `settings.json` shape (profiles + shell prefs).
#[derive(Debug, Clone, Deserialize)]
pub struct LegacyAppSettings {
    #[serde(default)]
    pub profiles: Vec<LegacyProfileJson>,
    #[serde(default)]
    pub default_profile_name: String,
    #[serde(default)]
    pub ssh_env_vars: HashMap<String, String>,
    #[serde(default)]
    pub default_local_connection_id: Option<String>,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub ui_theme: UiTheme,
    #[serde(default)]
    pub function_pane_width: Option<f32>,
    #[serde(default)]
    pub pane_accent_colors: Vec<[u8; 3]>,
}

pub(crate) fn load_prefs() -> Prefs {
    if let Some(path) = prefs_path()
        && path.exists()
        && let Ok(data) = std::fs::read_to_string(&path)
        && let Ok(prefs) = serde_json::from_str(&data)
    {
        return prefs;
    }

    if let Some(legacy) = load_legacy_settings() {
        let prefs = Prefs {
            general: GeneralPrefs {
                language: legacy.language,
                input_mode: default_input_mode(),
            },
            appearance: AppearancePrefs {
                ui_theme: legacy.ui_theme,
                pane_accent_colors: legacy.pane_accent_colors,
            },
            chrome: ChromePrefs {
                function_pane_width: legacy.function_pane_width,
                default_local_connection_id: legacy.default_local_connection_id,
            },
            file_manager: FileManagerPrefs::default(),
            ui_state: UiStatePrefs::default(),
        };
        save_prefs(&prefs);
        return prefs;
    }

    Prefs::default()
}

pub(crate) fn save_prefs(prefs: &Prefs) {
    let path = match prefs_path() {
        Some(p) => {
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            p
        }
        None => return,
    };
    if let Ok(data) = serde_json::to_string_pretty(prefs) {
        let _ = std::fs::write(&path, data);
    }
}

pub fn load_legacy_settings() -> Option<LegacyAppSettings> {
    let path = legacy_settings_path()?;
    if !path.exists() {
        return None;
    }
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}
