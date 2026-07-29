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
    pub run_favorite_command: Option<String>,
    pub edit_favorite_command: Option<String>,
    pub delete_favorite_command: Option<String>,
    pub new_favorite_command: bool,
}

impl FunctionAction {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Merge another action into this one (non-empty fields from `other` win).
    /// Used so menu-bar actions are not wiped by the function pane render.
    pub fn merge_from(&mut self, other: Self) {
        if other.select_session.is_some() {
            self.select_session = other.select_session;
        }
        if other.start_session_drag.is_some() {
            self.start_session_drag = other.start_session_drag;
        }
        if other.duplicate_session.is_some() {
            self.duplicate_session = other.duplicate_session;
        }
        if other.close_session.is_some() {
            self.close_session = other.close_session;
        }
        self.open_connection_mgmt |= other.open_connection_mgmt;
        self.toggle_settings |= other.toggle_settings;
        self.go_back |= other.go_back;
        self.new_connection |= other.new_connection;
        self.new_favorite_command |= other.new_favorite_command;
        if other.connect_connection.is_some() {
            self.connect_connection = other.connect_connection;
        }
        if other.open_file_mgr.is_some() {
            self.open_file_mgr = other.open_file_mgr;
        }
        if other.edit_connection.is_some() {
            self.edit_connection = other.edit_connection;
        }
        if other.delete_connection.is_some() {
            self.delete_connection = other.delete_connection;
        }
        if other.run_favorite_command.is_some() {
            self.run_favorite_command = other.run_favorite_command;
        }
        if other.edit_favorite_command.is_some() {
            self.edit_favorite_command = other.edit_favorite_command;
        }
        if other.delete_favorite_command.is_some() {
            self.delete_favorite_command = other.delete_favorite_command;
        }
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
