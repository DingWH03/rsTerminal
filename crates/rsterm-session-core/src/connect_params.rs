//! Map persist DTOs → connection-layer params at the app boundary.

use rsterm_connection::ssh_auth::ResolvedSshAuth;
use rsterm_connection::{
    BleConnectParams, LocalConnectParams, SerialConnectParams, SshConnectParams,
};
use rsterm_data::persist::types::{AuthMethod, AuthUser, SavedConnection};

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
    if let Some(user) = auth_user {
        match user.auth_method {
            AuthMethod::Password => ResolvedSshAuth {
                username: user.username.clone(),
                password: user.password.clone(),
                private_key_pem: None,
                key_passphrase: None,
                allow_default_keys: false,
            },
            AuthMethod::PrivateKey => ResolvedSshAuth {
                username: user.username.clone(),
                password: None,
                private_key_pem: user.private_key.clone(),
                key_passphrase: user.key_passphrase.clone(),
                allow_default_keys: false,
            },
        }
    } else {
        ResolvedSshAuth::from_legacy(conn.ssh_user.as_deref(), conn.ssh_password.as_deref())
    }
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
