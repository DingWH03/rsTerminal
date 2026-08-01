//! Favorite commands table CRUD.

use rusqlite::{params, Connection, OptionalExtension};

use crate::data::persist::types::FavoriteCommand;

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn list_all(conn: &Connection) -> rusqlite::Result<Vec<FavoriteCommand>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, name, command, auto_execute, sort_order
        FROM favorite_commands
        ORDER BY sort_order ASC, name COLLATE NOCASE ASC
        "#,
    )?;
    let rows = stmt.query_map([], row_to_cmd)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn get(conn: &Connection, id: &str) -> rusqlite::Result<Option<FavoriteCommand>> {
    conn.query_row(
        r#"
        SELECT id, name, command, auto_execute, sort_order
        FROM favorite_commands WHERE id = ?1
        "#,
        params![id],
        row_to_cmd,
    )
    .optional()
}

pub fn upsert(conn: &Connection, cmd: &FavoriteCommand) -> rusqlite::Result<()> {
    conn.execute(
        r#"
        INSERT INTO favorite_commands (id, name, command, auto_execute, sort_order, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            command = excluded.command,
            auto_execute = excluded.auto_execute,
            sort_order = excluded.sort_order,
            updated_at = excluded.updated_at
        "#,
        params![
            cmd.id,
            cmd.name,
            cmd.command,
            cmd.auto_execute as i64,
            cmd.sort_order,
            now_secs(),
        ],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM favorite_commands WHERE id = ?1", params![id])?;
    Ok(())
}

fn row_to_cmd(row: &rusqlite::Row<'_>) -> rusqlite::Result<FavoriteCommand> {
    let auto_execute: i64 = row.get(3)?;
    Ok(FavoriteCommand {
        id: row.get(0)?,
        name: row.get(1)?,
        command: row.get(2)?,
        auto_execute: auto_execute != 0,
        sort_order: row.get(4)?,
    })
}
