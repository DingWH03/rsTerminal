//! File-manager pane page — dual-pane browse/ops and `WorkspaceContent` adapter.
//!
//! Host (root) registers [`labels::FileManagerLabels`] at startup / language change.

pub mod labels;
pub mod page;

mod content;

pub use content::{FileManagerContent, wrap_file_manager};
pub use labels::{FileManagerLabels, install_labels, labels, set_labels};
pub use page::{FileManagerAction, file_manager_view};
