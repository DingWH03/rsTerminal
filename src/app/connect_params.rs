//! Map persist DTOs → connection-layer params at the app boundary.

use crate::connection::ssh_auth::ResolvedSshAuth;
use crate::connection::{
    BleConnectParams, LocalConnectParams, SerialConnectParams, SshConnectParams,
};
use crate::data::persist::types::{AuthUser, SavedConnection};

pub fn local_params(conn: &SavedConnection) -> LocalConnectParams {
    LocalConnectParams {
        shell: conn.shell.clone(),
        working_dir: conn.working_dir.clone(),
        env_vars: conn.env_vars.clone(),
    }
}

pub fn ssh_params(conn: &SavedConnection) -> Result<SshConnectParams, String> {
    let host = conn
        .ssh_host
        .clone()
        .ok_or_else(|| "SSH host not configured".to_string())?;
    Ok(SshConnectParams {
        session_tag: conn.id.clone(),
        host,
        port: conn.ssh_port.unwrap_or(22),
        env_vars: conn.env_vars.clone(),
    })
}

pub fn ssh_auth(conn: &SavedConnection, auth_user: Option<&AuthUser>) -> ResolvedSshAuth {
    ResolvedSshAuth::resolve(
        auth_user,
        conn.ssh_user.as_deref(),
        conn.ssh_password.as_deref(),
    )
}

pub fn serial_params(conn: &SavedConnection) -> Result<SerialConnectParams, String> {
    let port = conn
        .serial_port
        .clone()
        .ok_or_else(|| "Serial port not configured".to_string())?;
    Ok(SerialConnectParams {
        port,
        baud: conn.serial_baud.unwrap_or(115200),
    })
}

pub fn ble_params(conn: &SavedConnection) -> Result<BleConnectParams, String> {
    let device = conn
        .ble_device
        .clone()
        .ok_or_else(|| "BLE device not configured".to_string())?;
    Ok(BleConnectParams { device })
}
