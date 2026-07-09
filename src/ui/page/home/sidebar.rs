//! Legacy home sidebar — use [`crate::ui::function_pane`] instead.

use crate::session::WorkspaceSession;
use crate::ui::function_pane::common::{brand_row, nav_button};
use crate::ui::function_pane::session_list::paint_session_rows;
use crate::ui::function_pane::FunctionPane;

pub struct HomeSidebarResult {
    pub nav: HomeSidebarAction,
    pub sessions: SessionRowAction,
}

pub enum HomeSidebarAction {
    None,
    Home,
    Settings,
}

pub struct SessionRowAction {
    pub select_session: Option<String>,
    pub close_session: Option<String>,
    pub new_window_session: Option<String>,
}

pub fn paint_home_sidebar(
    ui: &mut egui::Ui,
    pane: &mut FunctionPane,
    sessions: &[WorkspaceSession],
    active_id: Option<&str>,
    on_settings: bool,
) -> HomeSidebarResult {
    let mut nav_action = HomeSidebarAction::None;

    brand_row(ui, pane, false);
    if nav_button(ui, None, "Home", false).clicked() {
        nav_action = HomeSidebarAction::Home;
    }
    if nav_button(
        ui,
        Some(crate::ui::widget::vector_icons::Icon::Settings),
        &rust_i18n::t!("settings"),
        on_settings,
    )
    .clicked()
    {
        nav_action = HomeSidebarAction::Settings;
    }

    let empty_accents = std::collections::HashMap::new();
    let ctx = crate::ui::function_pane::session_list::SessionListContext {
        split_enabled: false,
        visible_sessions: &std::collections::HashSet::new(),
        session_accents: &empty_accents,
    };
    let sess = paint_session_rows(ui, sessions, active_id, &ctx);

    HomeSidebarResult {
        nav: nav_action,
        sessions: SessionRowAction {
            select_session: sess.select_session,
            close_session: sess.close_session,
            new_window_session: None,
        },
    }
}
