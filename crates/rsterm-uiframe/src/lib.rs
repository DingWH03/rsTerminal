//! UI framework — reusable chrome, style tokens, icons, and widgets.
//!
//! Business UI (`shell`, `function_pane`, `page`, `workspace_pane`) should
//! *compose* these primitives; avoid putting app-specific session logic here.
//!
//! | Module | Role |
//! |--------|------|
//! | [`menu_bar`] | Declarative menu bar chrome |
//! | [`dialog`] | Centered dialog window frame |
//! | [`form`] | Imperative labeled fields / sections / footers |
//! | [`tab_bar`] / [`file_list`] / [`components`] | Shared widgets |
//! | [`style`] / [`vector_icons`] | Visual tokens |

pub mod animation;
pub mod clipboard;
pub mod components;
pub mod dialog;
pub mod file_list;
pub mod form;
pub mod interactive;
pub mod keyboard;
pub mod menu_bar;
pub mod pane_chrome;
pub mod style;
pub mod tab_bar;
pub mod tokens;
pub mod vector_icons;

pub use pane_chrome::PaneChrome;

pub use form::{COMBO_WIDTH, FIELD_GAP, FOOTER_GAP, FooterAction, LABEL_WIDTH, SECTION_GAP};

pub use dialog::{
    ALERT_HEIGHT, ALERT_WIDTH, DEFAULT_HEIGHT, DEFAULT_WIDTH, DialogFrame, DialogOutcome,
    host_blocked_this_frame,
};
pub use file_list::{
    FileBrowserAction, FileBrowserConfig, FileBrowserLabels, FileBrowserState, FileBrowserView,
    FileDetailsColumns, FilePaneLayout, FileRow, FileSortColumn, FileViewMode,
};
pub use menu_bar::{MenuBar, MenuBarSpec, MenuEntry, MenuEntryId, MenuGroup};
pub use tab_bar::{TabBar, TabBarItem};
