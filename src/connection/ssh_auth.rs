//! Resolved SSH credentials passed into connect (no persist DTOs).

use crate::persist::types::{AuthMethod, AuthUser};

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
    pub fn from_auth_user(user: &AuthUser) -> Self {
        match user.auth_method {
            AuthMethod::Password => Self {
                username: user.username.clone(),
                password: user.password.clone(),
                private_key_pem: None,
                key_passphrase: None,
                allow_default_keys: false,
            },
            AuthMethod::PrivateKey => Self {
                username: user.username.clone(),
                password: None,
                private_key_pem: user.private_key.clone(),
                key_passphrase: user.key_passphrase.clone(),
                allow_default_keys: false,
            },
        }
    }

    /// Legacy connection fields when no AuthUser is linked.
    pub fn from_legacy(username: Option<&str>, password: Option<&str>) -> Self {
        Self {
            username: username.unwrap_or("root").to_string(),
            password: password.map(|s| s.to_string()),
            private_key_pem: None,
            key_passphrase: None,
            allow_default_keys: true,
        }
    }

    pub fn resolve(auth_user: Option<&AuthUser>, username: Option<&str>, password: Option<&str>) -> Self {
        if let Some(u) = auth_user {
            Self::from_auth_user(u)
        } else {
            Self::from_legacy(username, password)
        }
    }
}
