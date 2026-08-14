//! Resolved SSH credentials passed into connect (no persist DTOs).

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
}
