//! Terminal pane page — grid paint, input, and `WorkspaceContent` adapter.
//!
//! Host (root) registers [`fonts::FontHooks`] and [`labels::TerminalLabels`] at startup.

pub mod fonts;
pub mod host_extras;
pub mod labels;
pub mod page;
pub mod paint_helpers;
pub mod theme_color;

mod content;

pub use content::{ActiveSessionContent, wrap_terminal};
pub use host_extras::TerminalHostExtras;
pub use labels::{TerminalLabels, install_labels, labels, set_labels};
pub use page::connection_view;
pub use paint_helpers::{measure_cell, paint_cursor};
pub use theme_color::{from_egui, to_egui};
