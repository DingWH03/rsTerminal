//! File-manager pane page — dual-pane browse/ops and `WorkspaceContent` adapter.
//!
//! Host (root) registers [`labels::FileManagerLabels`] at startup / language change.

pub mod labels;
pub mod page;

mod content;

pub use content::{
    DetailsPaneSide, FileManagerContent, columns_from_ui_state, dual_split_from_ui_state,
    pane_layout_to_prefs, persist_details_columns, persist_dual_split, persist_file_manager_prefs,
    persist_file_manager_prefs_full, prefs_to_pane_layout, prefs_to_view_mode, view_mode_to_prefs,
    wrap_file_manager,
};
pub use labels::{FileManagerLabels, install_labels, labels, set_labels};
pub use page::{FileManagerAction, file_manager_view};
