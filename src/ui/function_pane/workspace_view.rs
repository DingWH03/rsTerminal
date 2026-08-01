//! Workspace function page — active sessions list (no brand / settings).

use std::collections::HashSet;

use crate::data::prefs::Prefs;
use crate::session::WorkspaceSession;
use crate::ui::function_pane::session_list::{paint_session_rows, SessionListContext};
use crate::ui::function_pane::{drag_split_enabled, FunctionPane};
use crate::ui::pane_colors::session_accent_map;
use crate::ui::shell::layout_state::WorkspaceLayout;
use crate::ui::shell::messages::FunctionAction;

pub fn render(
    ui: &mut egui::Ui,
    pane: &mut FunctionPane,
    sessions: &[WorkspaceSession],
    workspace: &WorkspaceLayout,
    highlighted_session: Option<&str>,
    prefs: &Prefs,
) -> FunctionAction {
    let mut action = FunctionAction::empty();

    ui.style_mut().spacing.scroll.bar_width = 6.0;
    ui.style_mut().spacing.scroll.bar_outer_margin = 0.0;

    let visible: HashSet<String> = workspace
        .panes
        .values()
        .filter_map(|p| p.session_id.clone())
        .collect();

    let accents = session_accent_map(workspace, prefs);

    let ctx = SessionListContext {
        split_enabled: drag_split_enabled(pane, sessions.len()),
        visible_sessions: &visible,
        session_accents: &accents,
    };

    let sess_action = egui::ScrollArea::vertical()
        .id_salt("function_sessions_scroll")
        .auto_shrink([false; 2])
        .show(ui, |ui| paint_session_rows(ui, sessions, highlighted_session, &ctx))
        .inner;

    if let Some(id) = sess_action.select_session {
        action.select_session = Some(id);
    }
    if let Some(id) = sess_action.close_session {
        action.close_session = Some(id);
    }
    if let Some(id) = sess_action.start_session_drag {
        action.start_session_drag = Some(id);
    }
    if let Some(id) = sess_action.duplicate_session {
        action.duplicate_session = Some(id);
    }

    action
}
