//! Application data layer: JSON prefs + SQLite entities.
//!
//! - [`prefs`]: shell preferences (`prefs.json`)
//! - [`persist`]: connections / profiles / users / commands (`app.db`)

pub mod paths;
pub mod persist;
pub mod prefs;

pub use paths::config_dir;
#[cfg(target_os = "android")]
pub use paths::init_android_base_dir;

pub use persist::{Persist, PersistError};
pub use prefs::{
    AppearancePrefs, ChromePrefs, FileManagerPrefs, FileManagerUiState, GeneralPrefs, Prefs,
    PrefsFilePaneLayout, PrefsFileViewMode, UiStatePrefs, load_prefs, save_prefs,
};
