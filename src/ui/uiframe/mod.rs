//! UI framework — reusable chrome, style tokens, icons, and widgets.
//!
//! Business UI (`shell`, `function_pane`, `page`, `workspace_pane`) should
//! *compose* these primitives; avoid putting app-specific session logic here.
//!
//! | Module | Role |
//! |--------|------|
//! | [`menu_bar`] | Declarative menu bar chrome |
//! | [`dialog`] | Centered dialog window frame |
//! | [`tab_bar`] / [`file_list`] / [`components`] | Shared widgets |
//! | [`style`] / [`vector_icons`] | Visual tokens |

pub mod animation;
pub mod clipboard;
pub mod components;
pub mod dialog;
pub mod file_list;
pub mod keyboard;
pub mod menu_bar;
pub mod split_handle;
pub mod style;
pub mod tab_bar;
pub mod vector_icons;

pub use dialog::{
    host_blocked_this_frame, DialogFrame, DialogOutcome, ALERT_HEIGHT, ALERT_WIDTH, DEFAULT_HEIGHT,
    DEFAULT_WIDTH,
};
pub use file_list::{FileListAction, FileListView};
pub use menu_bar::{MenuBar, MenuBarSpec, MenuEntry, MenuEntryId, MenuGroup};
pub use tab_bar::{TabBar, TabBarItem};
