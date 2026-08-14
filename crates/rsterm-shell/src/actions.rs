//! Application-facing actions emitted by the UI.
//!
//! Pane-local accumulators remain useful while rendering, but the application
//! consumes this enum instead of interpreting their optional fields.

use rsterm_session_core::ConnectionViewAction;
use crate::layout::PaneId;
use crate::shell::messages::{FunctionAction, WorkspaceAction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    SettingsClosed,
    NewAuthUser,
    EditAuthUser(String),
    DeleteAuthUser(String),
    NewProfile,
    EditProfile(String),
    DeleteProfile(String),
    SetDefaultProfile(String),
    NewConnection,
    Connect(String),
    OpenFileManager(String),
    EditConnection(String),
    DeleteConnection(String),
    CloseSession(String),
    DuplicateSession(String),
    NewFavoriteCommand,
    RunFavoriteCommand(String),
    EditFavoriteCommand(String),
    DeleteFavoriteCommand(String),
    ConnectPane { pane: PaneId, connection_id: String },
    OpenConnectionsForPane(PaneId),
    ClosePane(PaneId),
    ReconnectPane { pane: PaneId, connection_id: String },
    PersistTerminalSettings,
}

impl UiAction {
    pub fn extend_function(actions: &mut Vec<Self>, action: &FunctionAction) {
        if action.new_connection {
            actions.push(Self::NewConnection);
        }
        if action.new_favorite_command {
            actions.push(Self::NewFavoriteCommand);
        }
        if let Some(id) = &action.connect_connection {
            actions.push(Self::Connect(id.clone()));
        }
        if let Some(id) = &action.open_file_mgr {
            actions.push(Self::OpenFileManager(id.clone()));
        }
        if let Some(id) = &action.edit_connection {
            actions.push(Self::EditConnection(id.clone()));
        }
        if let Some(id) = &action.delete_connection {
            actions.push(Self::DeleteConnection(id.clone()));
        }
        if let Some(id) = &action.close_session {
            actions.push(Self::CloseSession(id.clone()));
        }
        if let Some(id) = &action.duplicate_session {
            actions.push(Self::DuplicateSession(id.clone()));
        }
        if let Some(id) = &action.run_favorite_command {
            actions.push(Self::RunFavoriteCommand(id.clone()));
        }
        if let Some(id) = &action.edit_favorite_command {
            actions.push(Self::EditFavoriteCommand(id.clone()));
        }
        if let Some(id) = &action.delete_favorite_command {
            actions.push(Self::DeleteFavoriteCommand(id.clone()));
        }
        if action.toggle_settings {
            actions.push(Self::PersistTerminalSettings);
        }
    }

    pub fn extend_workspace(
        actions: &mut Vec<Self>,
        action: &WorkspaceAction,
        focused_pane: PaneId,
    ) {
        if let Some(request) = &action.connect_from_empty {
            actions.push(Self::ConnectPane {
                pane: request.pane,
                connection_id: request.connection_id.clone(),
            });
        }
        if let Some(pane) = action.open_connections_from_empty {
            actions.push(Self::OpenConnectionsForPane(pane));
        }
        if let Some(pane) = action.close_pane_session {
            actions.push(Self::ClosePane(pane));
        }

        let pane = action.terminal_pane.unwrap_or(focused_pane);
        match &action.terminal {
            ConnectionViewAction::CloseSession => {
                actions.push(Self::ClosePane(pane));
            }
            ConnectionViewAction::Reconnect(connection_id) => {
                actions.push(Self::ReconnectPane {
                    pane,
                    connection_id: connection_id.clone(),
                });
            }
            ConnectionViewAction::None | ConnectionViewAction::MinimizePane => {}
        }
        if action.file_manager.close {
            actions.push(Self::ClosePane(pane));
        }
    }
}
