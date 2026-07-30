//! Domain types for persisted connections, secrets, and favorite commands.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ConnectionType {
    Local,
    Ssh,
    Serial,
    Ble,
}

impl ConnectionType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Ssh => "ssh",
            Self::Serial => "serial",
            Self::Ble => "ble",
        }
    }

    pub fn from_str_db(s: &str) -> Option<Self> {
        match s {
            "local" | "Local" => Some(Self::Local),
            "ssh" | "Ssh" => Some(Self::Ssh),
            "serial" | "Serial" => Some(Self::Serial),
            "ble" | "Ble" => Some(Self::Ble),
            _ => None,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            ConnectionType::Local => "Local Terminal",
            ConnectionType::Ssh => "SSH",
            ConnectionType::Serial => "Serial Port",
            ConnectionType::Ble => "BLE Serial",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            ConnectionType::Local => "💻",
            ConnectionType::Ssh => "🌐",
            ConnectionType::Serial => "🔌",
            ConnectionType::Ble => "📶",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedConnection {
    pub id: String,
    pub name: String,
    pub conn_type: ConnectionType,
    #[serde(default)]
    pub favorite: bool,
    pub last_connected: Option<String>,
    /// Local: shell path
    pub shell: Option<String>,
    /// Local: initial working directory
    #[serde(default)]
    pub working_dir: Option<String>,
    /// SSH
    pub ssh_host: Option<String>,
    pub ssh_port: Option<u16>,
    pub ssh_user: Option<String>,
    /// Optional password (legacy; prefer auth_user_id).
    pub ssh_password: Option<String>,
    /// Reference to [`AuthUser`] for SSH credentials.
    #[serde(default)]
    pub auth_user_id: Option<String>,
    /// Serial
    pub serial_port: Option<String>,
    pub serial_baud: Option<u32>,
    /// BLE
    pub ble_device: Option<String>,
}

impl SavedConnection {
    pub fn new_local(name: &str, shell: Option<&str>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            conn_type: ConnectionType::Local,
            favorite: false,
            last_connected: None,
            shell: shell.map(|s| s.to_string()),
            working_dir: None,
            ssh_host: None,
            ssh_port: None,
            ssh_user: None,
            ssh_password: None,
            auth_user_id: None,
            serial_port: None,
            serial_baud: None,
            ble_device: None,
        }
    }

    pub fn new_ssh(name: &str, host: &str, port: u16, user: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            conn_type: ConnectionType::Ssh,
            favorite: false,
            last_connected: None,
            shell: None,
            working_dir: None,
            ssh_host: Some(host.to_string()),
            ssh_port: Some(port),
            ssh_user: Some(user.to_string()),
            ssh_password: None,
            auth_user_id: None,
            serial_port: None,
            serial_baud: None,
            ble_device: None,
        }
    }

    pub fn new_serial(name: &str, port: &str, baud: u32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            conn_type: ConnectionType::Serial,
            favorite: false,
            last_connected: None,
            shell: None,
            working_dir: None,
            ssh_host: None,
            ssh_port: None,
            ssh_user: None,
            ssh_password: None,
            auth_user_id: None,
            serial_port: Some(port.to_string()),
            serial_baud: Some(baud),
            ble_device: None,
        }
    }

    pub fn new_ble(name: &str, device: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            conn_type: ConnectionType::Ble,
            favorite: false,
            last_connected: None,
            shell: None,
            working_dir: None,
            ssh_host: None,
            ssh_port: None,
            ssh_user: None,
            ssh_password: None,
            auth_user_id: None,
            serial_port: None,
            serial_baud: None,
            ble_device: Some(device.to_string()),
        }
    }
}

/// How an [`AuthUser`] authenticates to SSH.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AuthMethod {
    #[default]
    Password,
    PrivateKey,
}

impl AuthMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::PrivateKey => "private_key",
        }
    }

    pub fn from_str_db(s: &str) -> Self {
        match s {
            "private_key" => Self::PrivateKey,
            _ => Self::Password,
        }
    }
}

/// SSH identity managed in Preferences → Users.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: String,
    pub name: String,
    pub username: String,
    pub auth_method: AuthMethod,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub key_passphrase: Option<String>,
}

impl AuthUser {
    pub fn new_password(name: &str, username: &str, password: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            username: username.to_string(),
            auth_method: AuthMethod::Password,
            password: Some(password.to_string()),
            private_key: None,
            key_passphrase: None,
        }
    }

    pub fn new_key(name: &str, username: &str, private_key: &str, passphrase: Option<&str>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            username: username.to_string(),
            auth_method: AuthMethod::PrivateKey,
            password: None,
            private_key: Some(private_key.to_string()),
            key_passphrase: passphrase.map(|s| s.to_string()),
        }
    }
}

/// Favorite / quick-input command stored in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoriteCommand {
    pub id: String,
    pub name: String,
    pub command: String,
    pub auto_execute: bool,
    pub sort_order: i64,
}

impl FavoriteCommand {
    pub fn new(name: &str, command: &str, auto_execute: bool) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            command: command.to_string(),
            auto_execute,
            sort_order: 0,
        }
    }
}

/// Which store holds the secret payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SecretBackendKind {
    #[default]
    Local,
    System,
}

impl SecretBackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::System => "system",
        }
    }

    pub fn from_str_db(s: &str) -> Self {
        match s {
            "system" => Self::System,
            _ => Self::Local,
        }
    }
}

/// Reserved secret record (importable / system keyring capable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretRecord {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub payload: String,
    pub backend: SecretBackendKind,
    pub created_at: i64,
}

impl SecretRecord {
    pub fn new_local(name: &str, kind: &str, payload: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            kind: kind.to_string(),
            payload: payload.to_string(),
            backend: SecretBackendKind::Local,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
        }
    }
}
