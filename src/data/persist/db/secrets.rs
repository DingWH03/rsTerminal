//! Secrets table CRUD (reserved for import / system keyring).

use rusqlite::{params, Connection, OptionalExtension};

use crate::data::persist::types::{SecretBackendKind, SecretRecord};

pub fn list_all(conn: &Connection) -> rusqlite::Result<Vec<SecretRecord>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, name, kind, payload, backend, created_at
        FROM secrets
        ORDER BY name COLLATE NOCASE ASC
        "#,
    )?;
    let rows = stmt.query_map([], row_to_secret)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn get(conn: &Connection, id: &str) -> rusqlite::Result<Option<SecretRecord>> {
    conn.query_row(
        r#"
        SELECT id, name, kind, payload, backend, created_at
        FROM secrets WHERE id = ?1
        "#,
        params![id],
        row_to_secret,
    )
    .optional()
}

pub fn upsert(conn: &Connection, s: &SecretRecord) -> rusqlite::Result<()> {
    conn.execute(
        r#"
        INSERT INTO secrets (id, name, kind, payload, backend, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            kind = excluded.kind,
            payload = excluded.payload,
            backend = excluded.backend,
            created_at = excluded.created_at
        "#,
        params![
            s.id,
            s.name,
            s.kind,
            s.payload,
            s.backend.as_str(),
            s.created_at,
        ],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM secrets WHERE id = ?1", params![id])?;
    Ok(())
}

fn row_to_secret(row: &rusqlite::Row<'_>) -> rusqlite::Result<SecretRecord> {
    let backend_s: String = row.get(4)?;
    Ok(SecretRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        kind: row.get(2)?,
        payload: row.get(3)?,
        backend: SecretBackendKind::from_str_db(&backend_s),
        created_at: row.get(5)?,
    })
}
