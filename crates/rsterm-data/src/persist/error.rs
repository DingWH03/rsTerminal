//! Typed persistence errors.

use std::fmt;

#[derive(Debug, Clone)]
pub enum PersistError {
    ProfileInUse { count: i64 },
    AuthUserInUse { count: i64 },
    Other(String),
}

impl PersistError {
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}

impl fmt::Display for PersistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProfileInUse { count } => write!(f, "profile in use by {count} connection(s)"),
            Self::AuthUserInUse { count } => write!(f, "auth user in use by {count} connection(s)"),
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for PersistError {}

impl From<String> for PersistError {
    fn from(value: String) -> Self {
        Self::Other(value)
    }
}

impl From<&str> for PersistError {
    fn from(value: &str) -> Self {
        Self::Other(value.to_string())
    }
}
