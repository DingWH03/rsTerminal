//! Auth users (SSH identities) table CRUD.

use rusqlite::{params, Connection, OptionalExtension};

use crate::data::persist::types::{AuthMethod, AuthUser};

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn list_all(conn: &Connection) -> rusqlite::Result<Vec<AuthUser>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, name, username, auth_method, password, private_key, key_passphrase
        FROM auth_users
        ORDER BY name COLLATE NOCASE ASC
        "#,
    )?;
    let rows = stmt.query_map([], row_to_user)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn get(conn: &Connection, id: &str) -> rusqlite::Result<Option<AuthUser>> {
    conn.query_row(
        r#"
        SELECT id, name, username, auth_method, password, private_key, key_passphrase
        FROM auth_users WHERE id = ?1
        "#,
        params![id],
        row_to_user,
    )
    .optional()
}

pub fn upsert(conn: &Connection, u: &AuthUser) -> rusqlite::Result<()> {
    let now = now_secs();
    conn.execute(
        r#"
        INSERT INTO auth_users (
            id, name, username, auth_method, password, private_key, key_passphrase,
            created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            username = excluded.username,
            auth_method = excluded.auth_method,
            password = excluded.password,
            private_key = excluded.private_key,
            key_passphrase = excluded.key_passphrase,
            updated_at = excluded.updated_at
        "#,
        params![
            u.id,
            u.name,
            u.username,
            u.auth_method.as_str(),
            u.password,
            u.private_key,
            u.key_passphrase,
            now,
            now,
        ],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM auth_users WHERE id = ?1", params![id])?;
    Ok(())
}

fn row_to_user(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuthUser> {
    let method_s: String = row.get(3)?;
    Ok(AuthUser {
        id: row.get(0)?,
        name: row.get(1)?,
        username: row.get(2)?,
        auth_method: AuthMethod::from_str_db(&method_s),
        password: row.get(4)?,
        private_key: row.get(5)?,
        key_passphrase: row.get(6)?,
    })
}
