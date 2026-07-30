//! Connections table CRUD.

use rusqlite::{params, Connection, OptionalExtension};

use crate::persist::types::{ConnectionType, SavedConnection};

pub fn list_all(conn: &Connection) -> rusqlite::Result<Vec<SavedConnection>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, name, conn_type, favorite, last_connected, shell, working_dir,
               ssh_host, ssh_port, ssh_user, ssh_password, serial_port, serial_baud, ble_device,
               auth_user_id
        FROM connections
        ORDER BY favorite DESC, name COLLATE NOCASE ASC
        "#,
    )?;
    let rows = stmt.query_map([], row_to_conn)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn get(conn: &Connection, id: &str) -> rusqlite::Result<Option<SavedConnection>> {
    conn.query_row(
        r#"
        SELECT id, name, conn_type, favorite, last_connected, shell, working_dir,
               ssh_host, ssh_port, ssh_user, ssh_password, serial_port, serial_baud, ble_device,
               auth_user_id
        FROM connections WHERE id = ?1
        "#,
        params![id],
        row_to_conn,
    )
    .optional()
}

pub fn upsert(conn: &Connection, c: &SavedConnection) -> rusqlite::Result<()> {
    conn.execute(
        r#"
        INSERT INTO connections (
            id, name, conn_type, favorite, last_connected, shell, working_dir,
            ssh_host, ssh_port, ssh_user, ssh_password, serial_port, serial_baud, ble_device,
            auth_user_id
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15
        )
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            conn_type = excluded.conn_type,
            favorite = excluded.favorite,
            last_connected = excluded.last_connected,
            shell = excluded.shell,
            working_dir = excluded.working_dir,
            ssh_host = excluded.ssh_host,
            ssh_port = excluded.ssh_port,
            ssh_user = excluded.ssh_user,
            ssh_password = excluded.ssh_password,
            serial_port = excluded.serial_port,
            serial_baud = excluded.serial_baud,
            ble_device = excluded.ble_device,
            auth_user_id = excluded.auth_user_id
        "#,
        params![
            c.id,
            c.name,
            c.conn_type.as_str(),
            c.favorite as i64,
            c.last_connected,
            c.shell,
            c.working_dir,
            c.ssh_host,
            c.ssh_port.map(|p| p as i64),
            c.ssh_user,
            c.ssh_password,
            c.serial_port,
            c.serial_baud.map(|b| b as i64),
            c.ble_device,
            c.auth_user_id,
        ],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM connections WHERE id = ?1", params![id])?;
    Ok(())
}

fn row_to_conn(row: &rusqlite::Row<'_>) -> rusqlite::Result<SavedConnection> {
    let conn_type_s: String = row.get(2)?;
    let conn_type = ConnectionType::from_str_db(&conn_type_s).unwrap_or(ConnectionType::Ssh);
    let favorite: i64 = row.get(3)?;
    let ssh_port: Option<i64> = row.get(8)?;
    let serial_baud: Option<i64> = row.get(12)?;
    Ok(SavedConnection {
        id: row.get(0)?,
        name: row.get(1)?,
        conn_type,
        favorite: favorite != 0,
        last_connected: row.get(4)?,
        shell: row.get(5)?,
        working_dir: row.get(6)?,
        ssh_host: row.get(7)?,
        ssh_port: ssh_port.map(|p| p as u16),
        ssh_user: row.get(9)?,
        ssh_password: row.get(10)?,
        serial_port: row.get(11)?,
        serial_baud: serial_baud.map(|b| b as u32),
        ble_device: row.get(13)?,
        auth_user_id: row.get(14)?,
    })
}
