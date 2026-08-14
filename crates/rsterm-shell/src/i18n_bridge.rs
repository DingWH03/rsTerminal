//! Runtime i18n bridge — host (root crate) registers a translator; shell calls [`tr`].
//!
//! Avoids a `rust_i18n` dependency in this crate. Root adapts `rust_i18n::t!`.

use std::sync::{Arc, OnceLock};

/// Translator registered by the host application.
pub trait T: Send + Sync {
    fn t(&self, key: &str) -> String;
}

static I18N: OnceLock<Arc<dyn T>> = OnceLock::new();

/// Register the host translator (idempotent: first call wins).
pub fn set_i18n(i: Arc<dyn T>) {
    let _ = I18N.set(i);
}

/// Translate `key`, or return the key itself if no translator is registered.
pub fn tr(key: &str) -> String {
    I18N.get()
        .map(|i| i.t(key))
        .unwrap_or_else(|| key.to_string())
}

/// Translate `key` then substitute `%{name}` placeholders from `args`.
pub fn tr_args(key: &str, args: &[(&str, &str)]) -> String {
    let mut s = tr(key);
    for (name, value) in args {
        s = s.replace(&format!("%{{{name}}}"), value);
    }
    s
}
