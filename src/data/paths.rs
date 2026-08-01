//! Shared config directory for prefs.json and app.db.

use std::path::PathBuf;

use directories::ProjectDirs;

#[cfg(target_os = "android")]
static ANDROID_BASE_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

#[cfg(target_os = "android")]
pub fn init_android_base_dir(path: PathBuf) {
    let _ = ANDROID_BASE_DIR.set(path);
}

pub fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "android")]
    {
        if let Some(dir) = ANDROID_BASE_DIR.get() {
            return Some(dir.join("config"));
        }
    }
    ProjectDirs::from("io", "rsTerminal", "rsTerminal")
        .map(|d| d.config_dir().to_path_buf())
}
