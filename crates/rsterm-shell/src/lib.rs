//! Application shell UI — menus, function pane, workspace panes, dialogs, settings.
//!
//! Host (root `rsTerminal`) registers [`i18n_bridge`] and [`host_hooks`] at startup.
//! This crate must not depend on the root package or `rust_i18n`.

pub mod actions;
pub mod connection_display;
pub mod filter_chips_conn;
pub mod function_pane;
pub mod host_hooks;
pub mod i18n_bridge;
pub mod page;
pub mod pane_colors;
pub mod session_host;
pub mod shell;
pub mod workspace_pane;

pub use host_hooks::{FontCatalogStatus, FontEntry, HostHooks, install_host_hooks};
pub use i18n_bridge::{T as I18nT, set_i18n, tr, tr_args};
pub use rsterm_uiframe::PaneChrome;
pub use session_host::{WorkspaceSession, terminal_conn_type, tick_all_session_files};
pub use shell::AppShell;

/// Re-export workspace layout types so `rsterm_shell::layout::…` / `crate::layout::…` works.
pub mod layout {
    pub use rsterm_workspace::layout::*;
}

/// Re-export uiframe so `rsterm_shell::uiframe::…` / `crate::uiframe::…` works.
pub mod uiframe {
    pub use rsterm_uiframe::*;

    /// Split handle lives in `rsterm-workspace`; keep the old path for call sites.
    pub mod split_handle {
        pub use rsterm_workspace::split_handle::*;
    }
}

/// Re-export theme helpers (implemented in `rsterm-page-terminal`).
pub mod theme_color {
    pub use rsterm_page_terminal::theme_color::*;
}
