//! Persistence facade — settings JSON + SQLite for connections / secrets / commands.
//!
//! Callers should use this module only; do not open `app.db` or settings paths directly.

pub mod db;
pub mod secret_backend;
pub mod settings;
pub mod types;

use std::path::PathBuf;
use std::sync::Mutex;

use directories::ProjectDirs;
use log::info;
use rusqlite::Connection;

use crate::persist::db::schema;
use crate::persist::types::{FavoriteCommand, SavedConnection, SecretRecord};

#[cfg(target_os = "android")]
static ANDROID_BASE_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Initialise the config directory from a platform-provided path
/// (called from `android_main()`).
#[cfg(target_os = "android")]
pub fn init_android_base_dir(path: PathBuf) {
    let _ = ANDROID_BASE_DIR.set(path);
}

/// Resolve the application config directory.
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

/// Opened persistence handle (SQLite + settings path helpers).
pub struct Persist {
    db: Mutex<Connection>,
}

impl Persist {
    /// Open `{config_dir}/app.db`, creating schema as needed.
    pub fn open() -> Self {
        let conn = match config_dir() {
            Some(dir) => {
                let _ = std::fs::create_dir_all(&dir);
                let path = dir.join("app.db");
                match Connection::open(&path) {
                    Ok(c) => {
                        if let Err(e) = schema::migrate(&c) {
                            info!("persist schema migrate failed: {e}");
                        }
                        info!("Opened persist db at {}", path.display());
                        c
                    }
                    Err(e) => {
                        info!("Failed to open app.db ({e}); using in-memory db");
                        let c = Connection::open_in_memory().expect("in-memory sqlite");
                        let _ = schema::migrate(&c);
                        c
                    }
                }
            }
            None => {
                info!("No config dir; using in-memory persist db");
                let c = Connection::open_in_memory().expect("in-memory sqlite");
                let _ = schema::migrate(&c);
                c
            }
        };
        Self {
            db: Mutex::new(conn),
        }
    }

    pub fn list_connections(&self) -> Vec<SavedConnection> {
        let db = self.db.lock().unwrap();
        db::connections::list_all(&db).unwrap_or_default()
    }

    pub fn upsert_connection(&self, conn: &SavedConnection) -> Result<(), String> {
        let db = self.db.lock().unwrap();
        db::connections::upsert(&db, conn).map_err(|e| e.to_string())
    }

    pub fn delete_connection(&self, id: &str) -> Result<(), String> {
        let db = self.db.lock().unwrap();
        db::connections::delete(&db, id).map_err(|e| e.to_string())
    }

    pub fn list_commands(&self) -> Vec<FavoriteCommand> {
        let db = self.db.lock().unwrap();
        db::commands::list_all(&db).unwrap_or_default()
    }

    pub fn upsert_command(&self, cmd: &FavoriteCommand) -> Result<(), String> {
        let db = self.db.lock().unwrap();
        db::commands::upsert(&db, cmd).map_err(|e| e.to_string())
    }

    pub fn delete_command(&self, id: &str) -> Result<(), String> {
        let db = self.db.lock().unwrap();
        db::commands::delete(&db, id).map_err(|e| e.to_string())
    }

    pub fn list_secrets(&self) -> Vec<SecretRecord> {
        let db = self.db.lock().unwrap();
        db::secrets::list_all(&db).unwrap_or_default()
    }

    pub fn upsert_secret(&self, secret: &SecretRecord) -> Result<(), String> {
        let db = self.db.lock().unwrap();
        db::secrets::upsert(&db, secret).map_err(|e| e.to_string())
    }

    pub fn delete_secret(&self, id: &str) -> Result<(), String> {
        let db = self.db.lock().unwrap();
        db::secrets::delete(&db, id).map_err(|e| e.to_string())
    }
}

impl Default for Persist {
    fn default() -> Self {
        Self::open()
    }
}
