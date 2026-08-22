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
pub mod hover_panel;
pub mod interactive;
pub mod keyboard;
pub mod menu_bar;
pub mod pane_chrome;
pub mod style;
pub mod tab_bar;
pub mod text_fit;
pub mod tokens;
pub mod vector_icons;

pub use pane_chrome::PaneChrome;

pub use form::{COMBO_WIDTH, FIELD_GAP, FOOTER_GAP, FooterAction, LABEL_WIDTH, SECTION_GAP};

pub use components::popup_menu::{
    POPUP_MENU_MAX_WIDTH, POPUP_MENU_MIN_WIDTH, PopupMenuOutcome, PopupMenuState,
    install_context_popup, measure_menu_width, menu_action, menu_action_enabled, menu_check,
    menu_heading, menu_separator, popup_body, popup_from_response, popup_menu_content,
    show_anchor_popup,
};
pub use dialog::{
    ALERT_HEIGHT, ALERT_WIDTH, DEFAULT_HEIGHT, DEFAULT_WIDTH, DialogFrame, DialogOutcome,
    host_blocked_this_frame,
};
pub use file_list::{
    FileBrowserAction, FileBrowserConfig, FileBrowserLabels, FileBrowserRowHook,
    FileBrowserRowMenu, FileBrowserState, FileBrowserView, FileDetailsColumns, FilePaneLayout,
    FileRow, FileSortColumn, FileViewMode,
};
pub use hover_panel::{
    HoverDetail, HoverDetailSource, HoverInstallMode, HoverPanelState, file_entry_detail,
    install_hover_detail, paint_hover_panel,
};
pub use menu_bar::{MenuBar, MenuBarSpec, MenuEntry, MenuEntryId, MenuGroup};
pub use tab_bar::{TabBar, TabBarItem};
