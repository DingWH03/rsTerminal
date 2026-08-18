//! Secret storage backends (local DB vs system keyring stub).

use crate::persist::types::{SecretBackendKind, SecretRecord};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretError {
    NotFound,
    Unsupported(&'static str),
    Backend(String),
}

pub trait SecretBackend {
    fn kind(&self) -> SecretBackendKind;
    fn get(&self, id: &str) -> Result<SecretRecord, SecretError>;
    fn set(&self, secret: &SecretRecord) -> Result<(), SecretError>;
    fn delete(&self, id: &str) -> Result<(), SecretError>;
}

/// Reads/writes secrets via the SQLite `secrets` table through a callback.
pub struct LocalDbBackend<FGet, FSet, FDel>
where
    FGet: Fn(&str) -> Result<Option<SecretRecord>, String>,
    FSet: Fn(&SecretRecord) -> Result<(), String>,
    FDel: Fn(&str) -> Result<(), String>,
{
    pub get_fn: FGet,
    pub set_fn: FSet,
    pub del_fn: FDel,
}

impl<FGet, FSet, FDel> SecretBackend for LocalDbBackend<FGet, FSet, FDel>
where
    FGet: Fn(&str) -> Result<Option<SecretRecord>, String>,
    FSet: Fn(&SecretRecord) -> Result<(), String>,
    FDel: Fn(&str) -> Result<(), String>,
{
    fn kind(&self) -> SecretBackendKind {
        SecretBackendKind::Local
    }

    fn get(&self, id: &str) -> Result<SecretRecord, SecretError> {
        match (self.get_fn)(id) {
            Ok(Some(s)) => Ok(s),
            Ok(None) => Err(SecretError::NotFound),
            Err(e) => Err(SecretError::Backend(e)),
        }
    }

    fn set(&self, secret: &SecretRecord) -> Result<(), SecretError> {
        (self.set_fn)(secret).map_err(SecretError::Backend)
    }

    fn delete(&self, id: &str) -> Result<(), SecretError> {
        (self.del_fn)(id).map_err(SecretError::Backend)
    }
}

/// Placeholder for OS keyring / secret-service integration.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemKeyringBackend;

impl SecretBackend for SystemKeyringBackend {
    fn kind(&self) -> SecretBackendKind {
        SecretBackendKind::System
    }

    fn get(&self, _id: &str) -> Result<SecretRecord, SecretError> {
        Err(SecretError::Unsupported(
            "system keyring backend is not implemented yet",
        ))
    }

    fn set(&self, _secret: &SecretRecord) -> Result<(), SecretError> {
        Err(SecretError::Unsupported(
            "system keyring backend is not implemented yet",
        ))
    }

    fn delete(&self, _id: &str) -> Result<(), SecretError> {
        Err(SecretError::Unsupported(
            "system keyring backend is not implemented yet",
        ))
    }
}
