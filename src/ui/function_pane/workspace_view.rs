//! Workspace function page — sessions + settings.

use std::collections::HashSet;

use crate::session::WorkspaceSession;
use crate::settings::AppSettings;
use crate::ui::function_pane::common::{brand_row, nav_button};
use crate::ui::function_pane::session_list::{paint_session_rows, SessionListContext};
use crate::ui::function_pane::{drag_split_enabled, FunctionPane};
use crate::ui::pane_colors::session_accent_map;
use crate::ui::shell::layout_state::WorkspaceLayout;
use crate::ui::shell::messages::FunctionAction;
use crate::ui::widget::vector_icons::Icon;

pub fn render(
    ui: &mut egui::Ui,
    pane: &mut FunctionPane,
    sessions: &[WorkspaceSession],
    workspace: &WorkspaceLayout,
    highlighted_session: Option<&str>,
    settings_open: bool,
    settings: &AppSettings,
) -> FunctionAction {
    let mut action = FunctionAction::empty();

    brand_row(ui, pane, false);
    ui.add_space(1.0);

    if nav_button(ui, None, &rust_i18n::t!("connection_mgmt"), false).clicked() {
        action.open_connection_mgmt = true;
    }
    ui.add_space(1.0);
    ui.separator();
    ui.add_space(1.0);

    let top_used = ui.cursor().min.y - ui.max_rect().top();
    let bottom_reserve = 52.0;
    let scroll_h = (ui.max_rect().height() - top_used - bottom_reserve).max(32.0);

    ui.style_mut().spacing.scroll.bar_width = 6.0;
    ui.style_mut().spacing.scroll.bar_outer_margin = 0.0;

    let visible: HashSet<String> = workspace
        .panes
        .values()
        .filter_map(|p| p.session_id.clone())
        .collect();

    let accents = session_accent_map(workspace, settings);

    let ctx = SessionListContext {
        split_enabled: drag_split_enabled(pane, sessions.len()),
        visible_sessions: &visible,
        session_accents: &accents,
    };

    let sess_action = egui::ScrollArea::vertical()
        .id_salt("function_sessions_scroll")
        .auto_shrink([false; 2])
        .max_height(scroll_h)
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

    ui.add_space(1.0);
    ui.separator();
    ui.add_space(1.0);

    if nav_button(ui, Some(Icon::Settings), &rust_i18n::t!("settings"), settings_open).clicked() {
        action.toggle_settings = true;
    }

    action
}
