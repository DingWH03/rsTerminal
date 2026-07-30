//! Resolved SSH credentials passed into connect (avoids DB access in connection layer).

use crate::persist::types::{AuthMethod, AuthUser, SavedConnection};

/// Authentication material for one SSH connect attempt.
#[derive(Debug, Clone, Default)]
pub struct ResolvedSshAuth {
    pub username: String,
    pub password: Option<String>,
    pub private_key_pem: Option<String>,
    pub key_passphrase: Option<String>,
    /// When true, also try `~/.ssh` default keys and env passwords (legacy connections).
    pub allow_default_keys: bool,
}

impl ResolvedSshAuth {
    /// Build auth from an optional AuthUser + connection fallback fields.
    pub fn from_connection(conn: &SavedConnection, auth_user: Option<&AuthUser>) -> Self {
        if let Some(u) = auth_user {
            return match u.auth_method {
                AuthMethod::Password => Self {
                    username: u.username.clone(),
                    password: u.password.clone(),
                    private_key_pem: None,
                    key_passphrase: None,
                    allow_default_keys: false,
                },
                AuthMethod::PrivateKey => Self {
                    username: u.username.clone(),
                    password: None,
                    private_key_pem: u.private_key.clone(),
                    key_passphrase: u.key_passphrase.clone(),
                    allow_default_keys: false,
                },
            };
        }
        Self {
            username: conn
                .ssh_user
                .clone()
                .unwrap_or_else(|| "root".to_string()),
            password: conn.ssh_password.clone(),
            private_key_pem: None,
            key_passphrase: None,
            allow_default_keys: true,
        }
    }
}
