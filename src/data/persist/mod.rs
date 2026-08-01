//! SQLite persistence for connections / profiles / users / commands / secrets.
//!
//! Shell prefs live in [`crate::data::prefs`]. Do not open `app.db` outside this module.

pub mod db;
pub mod error;
pub mod secret_backend;
pub mod types;

pub use error::PersistError;

use std::collections::HashMap;
use std::sync::Mutex;

use log::info;
use rusqlite::Connection;

use crate::data::paths::config_dir;
use crate::data::persist::db::schema;
use crate::data::persist::types::{
    default_local_env_vars, default_ssh_env_vars, AuthUser, FavoriteCommand, SavedConnection,
    SecretRecord, TerminalProfile,
};
use crate::data::prefs::io;

pub struct Persist {
    db: Mutex<Connection>,
}

impl Persist {
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
        let persist = Self {
            db: Mutex::new(conn),
        };
        if let Err(e) = persist.migrate_legacy_data() {
            info!("legacy data migrate: {e}");
        }
        persist
    }

    /// Import profiles / env from legacy settings.json; seed default profile; map connections.
    fn migrate_legacy_data(&self) -> Result<(), String> {
        let db = self.db.lock().unwrap();
        let count = db::profiles::count(&db).map_err(|e| e.to_string())?;
        let legacy = io::load_legacy_settings();

        if count == 0 {
            if let Some(legacy) = &legacy {
                let default_name = if legacy.default_profile_name.is_empty() {
                    "Default".to_string()
                } else {
                    legacy.default_profile_name.clone()
                };
                if legacy.profiles.is_empty() {
                    let p = TerminalProfile::default();
                    db::profiles::upsert(&db, &p).map_err(|e| e.to_string())?;
                } else {
                    for lp in &legacy.profiles {
                        let is_default = lp.name == default_name;
                        let p = lp.clone().into_terminal_profile(is_default);
                        db::profiles::upsert(&db, &p).map_err(|e| e.to_string())?;
                    }
                    // Ensure exactly one default.
                    if db::profiles::get_default(&db)
                        .map_err(|e| e.to_string())?
                        .is_none()
                    {
                        if let Some(first) = db::profiles::list_all(&db)
                            .map_err(|e| e.to_string())?
                            .first()
                        {
                            db::profiles::set_default(&db, &first.id)
                                .map_err(|e| e.to_string())?;
                        }
                    }
                }
            } else {
                let p = TerminalProfile::default();
                db::profiles::upsert(&db, &p).map_err(|e| e.to_string())?;
            }
        }

        let default_id = db::profiles::get_default(&db)
            .map_err(|e| e.to_string())?
            .map(|p| p.id)
            .or_else(|| {
                db::profiles::list_all(&db)
                    .ok()
                    .and_then(|v| v.first().map(|p| p.id.clone()))
            });

        // Name → id map for profile_name migration.
        let profiles = db::profiles::list_all(&db).map_err(|e| e.to_string())?;
        let name_to_id: HashMap<String, String> = profiles
            .iter()
            .map(|p| (p.name.clone(), p.id.clone()))
            .collect();

        let ssh_env = legacy
            .as_ref()
            .map(|l| l.ssh_env_vars.clone())
            .unwrap_or_else(default_ssh_env_vars);

        // Apply profile_id from _migrate_profile_names if present.
        let migrate_names: Vec<(String, String)> = {
            let exists: bool = db
                .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='_migrate_profile_names'")
                .and_then(|mut s| s.query_row([], |_| Ok(true)))
                .unwrap_or(false);
            if !exists {
                Vec::new()
            } else {
                let mut stmt = db
                    .prepare("SELECT conn_id, profile_name FROM _migrate_profile_names")
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                    .map_err(|e| e.to_string())?;
                let mut out = Vec::new();
                for r in rows {
                    out.push(r.map_err(|e| e.to_string())?);
                }
                out
            }
        };

        let mut conns = db::connections::list_all(&db).map_err(|e| e.to_string())?;
        for c in &mut conns {
            let mut changed = false;
            if c.profile_id.is_none() {
                if let Some((_, name)) = migrate_names.iter().find(|(id, _)| id == &c.id) {
                    if let Some(pid) = name_to_id.get(name) {
                        c.profile_id = Some(pid.clone());
                        changed = true;
                    }
                }
                if c.profile_id.is_none() {
                    c.profile_id = default_id.clone();
                    changed = true;
                }
            }
            if c.env_vars.is_empty() || c.env_vars == HashMap::new() {
                c.env_vars = match c.conn_type {
                    types::ConnectionType::Local => default_local_env_vars(),
                    types::ConnectionType::Ssh => {
                        let mut e = ssh_env.clone();
                        if e.is_empty() {
                            e = default_ssh_env_vars();
                        }
                        e
                    }
                    _ => HashMap::new(),
                };
                changed = true;
            } else if matches!(c.conn_type, types::ConnectionType::Ssh) {
                for (k, v) in &ssh_env {
                    c.env_vars.entry(k.clone()).or_insert_with(|| v.clone());
                }
                changed = true;
            }
            if changed {
                db::connections::upsert(&db, c).map_err(|e| e.to_string())?;
            }
        }

        let _ = db.execute_batch("DROP TABLE IF EXISTS _migrate_profile_names");
        Ok(())
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

    pub fn list_profiles(&self) -> Vec<TerminalProfile> {
        let db = self.db.lock().unwrap();
        db::profiles::list_all(&db).unwrap_or_default()
    }

    pub fn upsert_profile(&self, profile: &TerminalProfile) -> Result<(), String> {
        let db = self.db.lock().unwrap();
        db::profiles::upsert(&db, profile).map_err(|e| e.to_string())
    }

    pub fn delete_profile(&self, id: &str) -> Result<(), PersistError> {
        let db = self.db.lock().unwrap();
        let n = db::connections::count_using_profile(&db, id).unwrap_or(0);
        if n > 0 {
            return Err(PersistError::ProfileInUse { count: n });
        }
        db::profiles::delete(&db, id).map_err(|e| PersistError::other(e.to_string()))
    }

    pub fn set_default_profile(&self, id: &str) -> Result<(), String> {
        let db = self.db.lock().unwrap();
        db::profiles::set_default(&db, id).map_err(|e| e.to_string())
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

    pub fn list_auth_users(&self) -> Vec<AuthUser> {
        let db = self.db.lock().unwrap();
        db::auth_users::list_all(&db).unwrap_or_default()
    }

    pub fn upsert_auth_user(&self, user: &AuthUser) -> Result<(), String> {
        let db = self.db.lock().unwrap();
        db::auth_users::upsert(&db, user).map_err(|e| e.to_string())
    }

    pub fn delete_auth_user(&self, id: &str) -> Result<(), PersistError> {
        let db = self.db.lock().unwrap();
        let n = db::connections::count_using_auth_user(&db, id).unwrap_or(0);
        if n > 0 {
            return Err(PersistError::AuthUserInUse { count: n });
        }
        match db::auth_users::delete(&db, id) {
            Ok(()) => Ok(()),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("FOREIGN KEY") || msg.contains("constraint") {
                    Err(PersistError::AuthUserInUse { count: n.max(1) })
                } else {
                    Err(PersistError::other(msg))
                }
            }
        }
    }
}

impl Default for Persist {
    fn default() -> Self {
        Self::open()
    }
}
