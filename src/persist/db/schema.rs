//! Database schema migration.

use rusqlite::Connection;

const SCHEMA_VERSION: i32 = 1;

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
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
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    }
    Ok(())
}
