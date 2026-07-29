//! Settings JSON load/save mapped to [`AppSettings`].

use crate::persist::config_dir;
use crate::settings::AppSettings;

fn settings_path() -> Option<std::path::PathBuf> {
    config_dir().map(|p| p.join("settings.json"))
}

pub fn load_settings() -> AppSettings {
    let path = match settings_path() {
        Some(p) => p,
        None => return AppSettings::default(),
    };

    if !path.exists() {
        return AppSettings::default();
    }

    match std::fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => AppSettings::default(),
    }
}

pub fn save_settings(settings: &AppSettings) {
    let path = match settings_path() {
        Some(p) => {
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            p
        }
        None => return,
    };

    if let Ok(data) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(&path, data);
    }
}
