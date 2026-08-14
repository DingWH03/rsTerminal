//! Host-provided UI strings so this crate never calls `rust_i18n::t!`.

use std::sync::{OnceLock, RwLock};

#[derive(Clone, Debug)]
pub struct FileManagerLabels {
    pub close_pane: String,
    pub stop: String,
    pub copy: String,
    pub cut: String,
    pub delete: String,
    pub cancel: String,
    pub paste: String,
    pub loading: String,
    pub empty_folder: String,
    pub parent_folder: String,
    pub multi_select: String,
    pub clipboard_empty: String,
    pub open: String,
    pub rename: String,
    pub file_info: String,
    pub close: String,
    pub original_name: String,
    pub new_name: String,
    pub confirm: String,
}

impl FileManagerLabels {
    pub fn english() -> Self {
        Self {
            close_pane: "Close pane".into(),
            stop: "Stop".into(),
            copy: "Copy".into(),
            cut: "Cut".into(),
            delete: "Delete".into(),
            cancel: "Cancel".into(),
            paste: "Paste".into(),
            loading: "Loading…".into(),
            empty_folder: "Empty folder".into(),
            parent_folder: "Parent folder".into(),
            multi_select: "Multi-select".into(),
            clipboard_empty: "Clipboard empty".into(),
            open: "Open".into(),
            rename: "Rename".into(),
            file_info: "Info".into(),
            close: "Close".into(),
            original_name: "Original:".into(),
            new_name: "New name:".into(),
            confirm: "Confirm".into(),
        }
    }
}

static LABELS: OnceLock<RwLock<FileManagerLabels>> = OnceLock::new();

fn store() -> &'static RwLock<FileManagerLabels> {
    LABELS.get_or_init(|| RwLock::new(FileManagerLabels::english()))
}

/// Update labels (call after language change).
pub fn set_labels(labels: FileManagerLabels) {
    *store().write().expect("file-manager labels lock") = labels;
}

/// Snapshot of current labels.
pub fn labels() -> FileManagerLabels {
    store().read().expect("file-manager labels lock").clone()
}

/// Install alias for startup (same as [`set_labels`]).
pub fn install_labels(labels: FileManagerLabels) {
    set_labels(labels);
}
