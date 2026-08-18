//! Database schema migration.

use rusqlite::Connection;

const SCHEMA_VERSION: i32 = 4;

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.pragma_update(None, "foreign_keys", true)?;

    let version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version < 1 {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS connections (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                conn_type TEXT NOT NULL,
                favorite INTEGER NOT NULL DEFAULT 0,
                last_connected TEXT,
                shell TEXT,
                working_dir TEXT,
                ssh_host TEXT,
                ssh_port INTEGER,
                ssh_user TEXT,
                ssh_password TEXT,
                serial_port TEXT,
                serial_baud INTEGER,
                ble_device TEXT
            );

            CREATE TABLE IF NOT EXISTS secrets (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                payload TEXT NOT NULL,
                backend TEXT NOT NULL DEFAULT 'local',
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS favorite_commands (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                command TEXT NOT NULL,
                auto_execute INTEGER NOT NULL DEFAULT 0,
                sort_order INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0
            );
            "#,
        )?;
        conn.pragma_update(None, "user_version", 1)?;
    }

    let version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version < 2 {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS auth_users (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                username TEXT NOT NULL,
                auth_method TEXT NOT NULL,
                password TEXT,
                private_key TEXT,
                key_passphrase TEXT,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0
            );
            "#,
        )?;
        if !column_exists(conn, "connections", "auth_user_id")? {
            conn.execute("ALTER TABLE connections ADD COLUMN auth_user_id TEXT", [])?;
        }
        conn.pragma_update(None, "user_version", 2)?;
    }

    let version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version < 3 {
        if !column_exists(conn, "connections", "profile_name")? {
            conn.execute("ALTER TABLE connections ADD COLUMN profile_name TEXT", [])?;
        }
        conn.pragma_update(None, "user_version", 3)?;
    }

    let version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version < 4 {
        migrate_v4(conn)?;
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    }

    conn.pragma_update(None, "foreign_keys", true)?;
    Ok(())
}

fn column_exists(conn: &Connection, table: &str, col: &str) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let exists = stmt
        .query_map([], |row| {
            let name: String = row.get(1)?;
            Ok(name)
        })?
        .filter_map(|r| r.ok())
        .any(|n| n == col);
    Ok(exists)
}

fn migrate_v4(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS terminal_profiles (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            terminal_font TEXT NOT NULL DEFAULT '',
            font_size REAL NOT NULL DEFAULT 14,
            line_spacing REAL NOT NULL DEFAULT 1,
            cell_width_scale REAL NOT NULL DEFAULT 1,
            theme_json TEXT NOT NULL DEFAULT '{}',
            cursor_style TEXT NOT NULL DEFAULT 'bar',
            bold_is_bright INTEGER NOT NULL DEFAULT 1,
            scrollback_lines INTEGER NOT NULL DEFAULT 5000,
            terminal_type TEXT NOT NULL DEFAULT 'xterm_256',
            bell TEXT NOT NULL DEFAULT 'visual',
            enable_bracketed_paste INTEGER NOT NULL DEFAULT 1,
            enable_sgr_mouse INTEGER NOT NULL DEFAULT 1,
            auto_wrap INTEGER NOT NULL DEFAULT 1,
            word_separators TEXT NOT NULL DEFAULT '',
            keyboard_mode TEXT NOT NULL DEFAULT 'full',
            is_default INTEGER NOT NULL DEFAULT 0
        );
        "#,
    )?;

    // Seed will be filled by Persist::migrate_legacy_data if empty.

    // Rebuild connections with FK + env_vars + profile_id.
    conn.execute_batch(
        r#"
        CREATE TABLE connections_v4 (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            conn_type TEXT NOT NULL,
            favorite INTEGER NOT NULL DEFAULT 0,
            last_connected TEXT,
            shell TEXT,
            working_dir TEXT,
            ssh_host TEXT,
            ssh_port INTEGER,
            ssh_user TEXT,
            ssh_password TEXT,
            serial_port TEXT,
            serial_baud INTEGER,
            ble_device TEXT,
            auth_user_id TEXT REFERENCES auth_users(id) ON DELETE RESTRICT,
            profile_id TEXT REFERENCES terminal_profiles(id) ON DELETE RESTRICT,
            env_vars TEXT NOT NULL DEFAULT '{}'
        );
        "#,
    )?;

    // Copy rows — profile_id/env left null/'{}' until legacy migrate fills them.
    let has_profile_name = column_exists(conn, "connections", "profile_name")?;
    let has_auth = column_exists(conn, "connections", "auth_user_id")?;
    let select = if has_profile_name && has_auth {
        r#"
        SELECT id, name, conn_type, favorite, last_connected, shell, working_dir,
               ssh_host, ssh_port, ssh_user, ssh_password, serial_port, serial_baud, ble_device,
               auth_user_id, profile_name
        FROM connections
        "#
    } else if has_auth {
        r#"
        SELECT id, name, conn_type, favorite, last_connected, shell, working_dir,
               ssh_host, ssh_port, ssh_user, ssh_password, serial_port, serial_baud, ble_device,
               auth_user_id, NULL
        FROM connections
        "#
    } else {
        r#"
        SELECT id, name, conn_type, favorite, last_connected, shell, working_dir,
               ssh_host, ssh_port, ssh_user, ssh_password, serial_port, serial_baud, ble_device,
               NULL, NULL
        FROM connections
        "#
    };

    {
        let mut stmt = conn.prepare(select)?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<i64>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, Option<i64>>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<String>>(14)?,
                row.get::<_, Option<String>>(15)?, // profile_name (temp stored)
            ))
        })?;
        // Store profile_name in a side table for later mapping.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _migrate_profile_names (conn_id TEXT PRIMARY KEY, profile_name TEXT)",
        )?;
        for r in rows {
            let (
                id,
                name,
                conn_type,
                favorite,
                last_connected,
                shell,
                working_dir,
                ssh_host,
                ssh_port,
                ssh_user,
                ssh_password,
                serial_port,
                serial_baud,
                ble_device,
                auth_user_id,
                profile_name,
            ) = r?;
            // Clear dangling auth refs so FK insert succeeds.
            let auth_ok: bool = match &auth_user_id {
                Some(aid) => conn
                    .query_row(
                        "SELECT 1 FROM auth_users WHERE id = ?1",
                        rusqlite::params![aid],
                        |_| Ok(true),
                    )
                    .unwrap_or(false),
                None => true,
            };
            let auth_user_id = if auth_ok { auth_user_id } else { None };
            if let Some(ref pn) = profile_name {
                conn.execute(
                    "INSERT OR REPLACE INTO _migrate_profile_names(conn_id, profile_name) VALUES(?1, ?2)",
                    rusqlite::params![id, pn],
                )?;
            }
            conn.execute(
                r#"
                INSERT INTO connections_v4 (
                    id, name, conn_type, favorite, last_connected, shell, working_dir,
                    ssh_host, ssh_port, ssh_user, ssh_password, serial_port, serial_baud, ble_device,
                    auth_user_id, profile_id, env_vars
                ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,NULL,'{}')
                "#,
                rusqlite::params![
                    id,
                    name,
                    conn_type,
                    favorite,
                    last_connected,
                    shell,
                    working_dir,
                    ssh_host,
                    ssh_port,
                    ssh_user,
                    ssh_password,
                    serial_port,
                    serial_baud,
                    ble_device,
                    auth_user_id,
                ],
            )?;
        }
    }

    conn.execute_batch(
        r#"
        DROP TABLE connections;
        ALTER TABLE connections_v4 RENAME TO connections;
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::persist::db::{connections, profiles};
    use crate::data::persist::types::{SavedConnection, TerminalProfile};

    fn open_migrated() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        conn
    }

    #[test]
    fn migrate_creates_v4_tables() {
        let conn = open_migrated();
        let version: i32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert!(column_exists(&conn, "terminal_profiles", "id").unwrap());
        assert!(column_exists(&conn, "connections", "profile_id").unwrap());
        assert!(column_exists(&conn, "connections", "env_vars").unwrap());
        assert!(!column_exists(&conn, "connections", "profile_name").unwrap());
    }

    #[test]
    fn migrate_from_v3_preserves_profile_name_side_table() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        // Downgrade simulation: recreate a v3-shaped DB.
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE connections (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                conn_type TEXT NOT NULL,
                favorite INTEGER NOT NULL DEFAULT 0,
                last_connected TEXT,
                shell TEXT,
                working_dir TEXT,
                ssh_host TEXT,
                ssh_port INTEGER,
                ssh_user TEXT,
                ssh_password TEXT,
                serial_port TEXT,
                serial_baud INTEGER,
                ble_device TEXT,
                auth_user_id TEXT,
                profile_name TEXT
            );
            CREATE TABLE auth_users (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                username TEXT NOT NULL,
                auth_method TEXT NOT NULL,
                password TEXT,
                private_key TEXT,
                key_passphrase TEXT,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0
            );
            "#,
        )
        .unwrap();
        conn.execute(
            "INSERT INTO connections (id, name, conn_type, favorite, profile_name) VALUES ('c1','Local','Local',0,'Work')",
            [],
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 3).unwrap();
        migrate(&conn).unwrap();
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM _migrate_profile_names WHERE conn_id='c1' AND profile_name='Work'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        assert!(column_exists(&conn, "connections", "profile_id").unwrap());
    }

    #[test]
    fn delete_profile_blocked_when_connection_references_it() {
        let conn = open_migrated();
        let profile = TerminalProfile {
            is_default: true,
            ..Default::default()
        };
        profiles::upsert(&conn, &profile).unwrap();
        let mut c = SavedConnection::new_local("t", None);
        c.profile_id = Some(profile.id.clone());
        connections::upsert(&conn, &c).unwrap();
        let n = connections::count_using_profile(&conn, &profile.id).unwrap();
        assert_eq!(n, 1);
        let err = profiles::delete(&conn, &profile.id).unwrap_err();
        assert!(err.to_string().contains("FOREIGN KEY") || err.to_string().contains("constraint"));
    }
}
