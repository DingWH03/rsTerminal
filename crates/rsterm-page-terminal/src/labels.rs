//! Host-provided UI strings so this crate never calls `rust_i18n::t!`.

use std::sync::{OnceLock, RwLock};

#[derive(Clone, Debug)]
pub struct TerminalLabels {
    pub copy: String,
    pub paste: String,
    pub cancel: String,
    pub close_pane: String,
    pub minimize_pane: String,
    pub settings_default_keyboard: String,
    pub clear_selection: String,
    pub connecting: String,
    pub disconnected: String,
    pub connection_failed: String,
    pub reconnect: String,
    pub close: String,
}

impl TerminalLabels {
    pub fn english() -> Self {
        Self {
            copy: "Copy".into(),
            paste: "Paste".into(),
            cancel: "Cancel".into(),
            close_pane: "Close pane".into(),
            minimize_pane: "Minimize pane".into(),
            settings_default_keyboard: "Keyboard".into(),
            clear_selection: "Clear selection".into(),
            connecting: "Connecting…".into(),
            disconnected: "Disconnected".into(),
            connection_failed: "Connection failed".into(),
            reconnect: "Reconnect".into(),
            close: "Close".into(),
        }
    }
}

static LABELS: OnceLock<RwLock<TerminalLabels>> = OnceLock::new();

fn store() -> &'static RwLock<TerminalLabels> {
    LABELS.get_or_init(|| RwLock::new(TerminalLabels::english()))
}

/// Update labels (call after language change).
pub fn set_labels(labels: TerminalLabels) {
    *store().write().expect("terminal labels lock") = labels;
}

/// Snapshot of current labels.
pub fn labels() -> TerminalLabels {
    store().read().expect("terminal labels lock").clone()
}

/// Install alias for startup (same as [`set_labels`]).
pub fn install_labels(labels: TerminalLabels) {
    set_labels(labels);
}
