//! Shell message bus — actions from function pane and workspace pane.

use crate::session::ConnectionViewAction;
use crate::ui::page::file_manager::FileManagerAction;
use crate::ui::shell::layout_state::PaneId;

/// Actions emitted by the left function pane.
#[derive(Debug, Default)]
pub struct FunctionAction {
    pub select_session: Option<String>,
    pub start_session_drag: Option<String>,
    pub duplicate_session: Option<String>,
    pub close_session: Option<String>,
    pub open_connection_mgmt: bool,
    pub toggle_settings: bool,
    pub go_back: bool,
    pub new_connection: bool,
    pub connect_connection: Option<String>,
    pub open_file_mgr: Option<String>,
    pub edit_connection: Option<String>,
    pub delete_connection: Option<String>,
}

impl FunctionAction {
    pub fn empty() -> Self {
        Self::default()
    }
}

/// Target pane + connection id when connecting from an empty split pane.
#[derive(Debug, Clone)]
pub struct EmptyPaneConnect {
    pub pane: PaneId,
    pub connection_id: String,
}

/// Actions emitted by the right workspace pane.
#[derive(Debug)]
pub struct WorkspaceAction {
    pub focus_pane: Option<PaneId>,
    pub minimize_pane: Option<PaneId>,
    pub close_pane_session: Option<PaneId>,
    pub start_pane_drag: Option<PaneId>,
    pub terminal: ConnectionViewAction,
    pub terminal_pane: Option<PaneId>,
    pub file_manager: FileManagerAction,
    pub connect_from_empty: Option<EmptyPaneConnect>,
    pub open_connections_from_empty: Option<PaneId>,
    pub drop_applied: bool,
}

impl WorkspaceAction {
    pub fn empty() -> Self {
        Self::default()
    }
}

impl Default for WorkspaceAction {
    fn default() -> Self {
        Self {
            focus_pane: None,
            minimize_pane: None,
            close_pane_session: None,
            start_pane_drag: None,
            terminal: ConnectionViewAction::default(),
            terminal_pane: None,
            file_manager: FileManagerAction::default(),
            connect_from_empty: None,
            open_connections_from_empty: None,
            drop_applied: false,
        }
    }
}
