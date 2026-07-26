//! UI framework — reusable chrome, style tokens, icons, and widgets.
//!
//! Business UI (`shell`, `function_pane`, `page`, `workspace_pane`) should
//! *compose* these primitives; avoid putting app-specific session logic here.

pub mod animation;
pub mod clipboard;
pub mod components;
pub mod dialog;
pub mod dialogs;
pub mod file_list;
pub mod keyboard;
pub mod sidebar;
pub mod split_handle;
pub mod style;
pub mod tab_bar;
pub mod top_bar;
pub mod vector_icons;

pub use dialog::{DialogFrame, DialogOutcome};
pub use file_list::{FileListAction, FileListView};
pub use tab_bar::{TabBar, TabBarItem};
pub use top_bar::{TopBar, TopBarAction};
