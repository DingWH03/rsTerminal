//! Shell coordinator — applies actions from function and workspace panes.

use crate::ui::shell::layout_state::{PaneId, ShellLayout};
use crate::ui::shell::messages::{FunctionAction, WorkspaceAction};

pub struct ShellCoordinator;

impl ShellCoordinator {
    pub fn apply_function(layout: &mut ShellLayout, action: &FunctionAction, in_overlay: bool) {
        if let Some(ref id) = action.select_session {
            if let Some(pane) = layout.workspace.pane_for_session(id) {
                layout.workspace.focused_pane = pane;
            } else {
                layout
                    .workspace
                    .assign_session(layout.workspace.focused_pane, Some(id.clone()));
            }
            let _ = in_overlay;
        }
        if let Some(ref id) = action.close_session {
            layout.workspace.clear_session_everywhere(id);
        }
        let _ = (
            &action.start_session_drag,
            &action.duplicate_session,
            &action.open_connection_mgmt,
            &action.toggle_settings,
            &action.go_back,
        );
    }

    pub fn apply_workspace(layout: &mut ShellLayout, action: &WorkspaceAction) {
        if let Some(pane) = action.focus_pane {
            layout.workspace.focused_pane = pane;
        }
        if let Some(pane) = action.minimize_pane {
            layout.workspace.hide_pane(pane);
        }
        let _ = action.start_pane_drag;
    }

    pub fn assign_new_session(layout: &mut ShellLayout, session_id: String) {
        Self::assign_session_to_pane(layout, layout.workspace.focused_pane, session_id);
    }

    pub fn assign_session_to_pane(layout: &mut ShellLayout, pane: PaneId, session_id: String) {
        layout.workspace.assign_session(pane, Some(session_id));
        layout.workspace.focused_pane = pane;
    }

    pub fn on_sessions_closed(layout: &mut ShellLayout, session_id: &str) {
        layout.workspace.clear_session_everywhere(session_id);
    }
}
