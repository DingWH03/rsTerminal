//! Domain types for persisted connections, profiles, secrets, and commands.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::config::{
    BellStyle, CursorStyle, KeyboardMode, SSH_OSC7_PROMPT_COMMAND, TerminalTheme, TerminalType,
};

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
}

/// Default env for a new local connection.
pub fn default_local_env_vars() -> HashMap<String, String> {
    HashMap::from([
        (
            "TERM".to_string(),
            std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string()),
        ),
        (
            "COLORTERM".to_string(),
            std::env::var("COLORTERM").unwrap_or_else(|_| "truecolor".to_string()),
        ),
        (
            "LC_ALL".to_string(),
            std::env::var("LC_ALL").unwrap_or_else(|_| "en_US.UTF-8".to_string()),
        ),
    ])
}

/// Default env for a new SSH connection (includes OSC7 prompt hook).
pub fn default_ssh_env_vars() -> HashMap<String, String> {
    HashMap::from([
        ("TERM".to_string(), "xterm-256color".to_string()),
        ("LANG".to_string(), "en_US.UTF-8".to_string()),
        (
            "PROMPT_COMMAND".to_string(),
            SSH_OSC7_PROMPT_COMMAND.to_string(),
        ),
    ])
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedConnection {
    pub id: String,
    pub name: String,
    pub conn_type: ConnectionType,
    #[serde(default)]
    pub favorite: bool,
    pub last_connected: Option<String>,
    pub shell: Option<String>,
    #[serde(default)]
    pub working_dir: Option<String>,
    pub ssh_host: Option<String>,
    pub ssh_port: Option<u16>,
    pub ssh_user: Option<String>,
    pub ssh_password: Option<String>,
    #[serde(default)]
    pub auth_user_id: Option<String>,
    /// FK → [`TerminalProfile::id`]; `None` → app default profile.
    #[serde(default)]
    pub profile_id: Option<String>,
    /// Per-connection environment variables (Local PTY / SSH `set_env`).
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    pub serial_port: Option<String>,
    pub serial_baud: Option<u32>,
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
            profile_id: None,
            env_vars: default_local_env_vars(),
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
            profile_id: None,
            env_vars: default_ssh_env_vars(),
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
            profile_id: None,
            env_vars: HashMap::new(),
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
            profile_id: None,
            env_vars: HashMap::new(),
            serial_port: None,
            serial_baud: None,
            ble_device: Some(device.to_string()),
        }
    }
}

/// Terminal appearance/behavior profile (SQLite). No environment variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub terminal_font: String,
    pub font_size: f32,
    #[serde(default = "default_line_spacing")]
    pub line_spacing: f32,
    #[serde(default = "default_cell_width_scale")]
    pub cell_width_scale: f32,
    pub theme: TerminalTheme,
    pub cursor_style: CursorStyle,
    #[serde(default = "default_true")]
    pub bold_is_bright: bool,
    pub scrollback_lines: usize,
    #[serde(default)]
    pub terminal_type: TerminalType,
    #[serde(default)]
    pub bell: BellStyle,
    #[serde(default = "default_true")]
    pub enable_bracketed_paste: bool,
    #[serde(default = "default_true")]
    pub enable_sgr_mouse: bool,
    #[serde(default = "default_true")]
    pub auto_wrap: bool,
    #[serde(default)]
    pub word_separators: String,
    pub keyboard_mode: KeyboardMode,
    #[serde(default)]
    pub is_default: bool,
}

fn default_line_spacing() -> f32 {
    1.0
}
fn default_cell_width_scale() -> f32 {
    1.0
}
fn default_true() -> bool {
    true
}

impl Default for TerminalProfile {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Default".to_string(),
            description: String::new(),
            terminal_font: String::new(),
            font_size: 14.0,
            line_spacing: 1.0,
            cell_width_scale: 1.0,
            theme: TerminalTheme::default(),
            cursor_style: CursorStyle::default(),
            bold_is_bright: true,
            scrollback_lines: 5000,
            terminal_type: TerminalType::default(),
            bell: BellStyle::default(),
            enable_bracketed_paste: true,
            enable_sgr_mouse: true,
            auto_wrap: true,
            word_separators: " /\\()\"'-:,.;<>~!@#$%^&*|+=[]{}~?│".to_string(),
            keyboard_mode: KeyboardMode::Full,
            is_default: true,
        }
    }
}

impl TerminalProfile {
    pub fn duplicate(&self, new_name: &str) -> Self {
        let mut copy = self.clone();
        copy.id = uuid::Uuid::new_v4().to_string();
        copy.name = new_name.to_string();
        copy.is_default = false;
        copy
    }
}

/// Resolve a profile by id, or the default / first profile.
pub fn resolve_profile<'a>(
    profiles: &'a [TerminalProfile],
    id: Option<&str>,
) -> &'a TerminalProfile {
    if let Some(id) = id {
        if let Some(p) = profiles.iter().find(|p| p.id == id) {
            return p;
        }
    }
    profiles
        .iter()
        .find(|p| p.is_default)
        .or_else(|| profiles.first())
        .expect("at least one terminal profile")
}

/// Shape of a profile entry inside legacy `settings.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyProfileJson {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub terminal_font: String,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_line_spacing")]
    pub line_spacing: f32,
    #[serde(default = "default_cell_width_scale")]
    pub cell_width_scale: f32,
    #[serde(default)]
    pub theme: TerminalTheme,
    #[serde(default)]
    pub cursor_style: CursorStyle,
    #[serde(default = "default_true")]
    pub bold_is_bright: bool,
    #[serde(default = "default_scrollback")]
    pub scrollback_lines: usize,
    #[serde(default)]
    pub terminal_type: TerminalType,
    #[serde(default)]
    pub bell: BellStyle,
    #[serde(default = "default_true")]
    pub enable_bracketed_paste: bool,
    #[serde(default = "default_true")]
    pub enable_sgr_mouse: bool,
    #[serde(default = "default_true")]
    pub auto_wrap: bool,
    #[serde(default)]
    pub word_separators: String,
    #[serde(default)]
    pub keyboard_mode: KeyboardMode,
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
}

fn default_font_size() -> f32 {
    14.0
}
fn default_scrollback() -> usize {
    5000
}

impl LegacyProfileJson {
    pub fn into_terminal_profile(self, is_default: bool) -> TerminalProfile {
        TerminalProfile {
            id: uuid::Uuid::new_v4().to_string(),
            name: self.name,
            description: self.description,
            terminal_font: self.terminal_font,
            font_size: self.font_size,
            line_spacing: self.line_spacing,
            cell_width_scale: self.cell_width_scale,
            theme: self.theme,
            cursor_style: self.cursor_style,
            bold_is_bright: self.bold_is_bright,
            scrollback_lines: self.scrollback_lines,
            terminal_type: self.terminal_type,
            bell: self.bell,
            enable_bracketed_paste: self.enable_bracketed_paste,
            enable_sgr_mouse: self.enable_sgr_mouse,
            auto_wrap: self.auto_wrap,
            word_separators: if self.word_separators.is_empty() {
                TerminalProfile::default().word_separators
            } else {
                self.word_separators
            },
            keyboard_mode: self.keyboard_mode,
            is_default,
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

    pub fn new_key(
        name: &str,
        username: &str,
        private_key: &str,
        passphrase: Option<&str>,
    ) -> Self {
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
