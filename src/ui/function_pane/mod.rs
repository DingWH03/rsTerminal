//! Left function pane — session list, connection management, settings entry.

pub mod common;
pub mod connections;
pub mod pages;
pub mod session_list;
pub mod workspace_view;

use crate::session::WorkspaceSession;
use crate::settings::AppSettings;
use crate::storage::types::SavedConnection;
use crate::ui::function_pane::pages::FunctionPage;
use crate::ui::shell::layout_state::WorkspaceLayout;
use crate::ui::shell::messages::FunctionAction;

/// 宽屏/窄屏切换阈值（像素）
pub const WIDE_THRESHOLD: f32 = 720.0;
/// 停靠功能区默认宽度
pub const DOCK_WIDTH: f32 = 200.0;
/// 覆盖浮动功能区宽度
pub const OVERLAY_WIDTH: f32 = 260.0;

/// Responsive function pane state (dock / overlay).
#[derive(Clone)]
pub struct FunctionPane {
    pub wide: bool,
    docked_open: bool,
    overlay_open: bool,
}

impl FunctionPane {
    pub fn new() -> Self {
        Self {
            wide: false,
            docked_open: true,
            overlay_open: false,
        }
    }

    pub fn sync_width(&mut self, width: f32) {
        let now_wide = width > WIDE_THRESHOLD;
        if now_wide && !self.wide {
            self.overlay_open = false;
        }
        self.wide = now_wide;
    }

    pub fn docked_visible(&self) -> bool {
        self.wide && self.docked_open
    }

    pub fn overlay_visible(&self) -> bool {
        !self.wide && self.overlay_open
    }

    pub fn show_content_hamburger(&self) -> bool {
        true
    }

    pub fn hamburger_click(&mut self) {
        if self.wide {
            self.docked_open = !self.docked_open;
        } else {
            self.overlay_open = !self.overlay_open;
        }
    }

    pub fn open_overlay(&mut self) {
        if !self.wide {
            self.overlay_open = true;
        }
    }

    pub fn close_overlay(&mut self) {
        self.overlay_open = false;
    }

    pub fn hamburger(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 24.0), egui::Sense::click());
        if ui.is_rect_visible(rect) {
            crate::ui::widget::vector_icons::paint(
                ui,
                rect,
                crate::ui::widget::vector_icons::Icon::Hamburger,
                ui.visuals().weak_text_color(),
                1.5,
            );
        }
        resp
    }
}

impl Default for FunctionPane {
    fn default() -> Self {
        Self::new()
    }
}

pub fn split_enabled(pane: &FunctionPane, session_count: usize) -> bool {
    pane.docked_visible() && session_count >= 2
}

/// Sidebar / pane drag to split — allowed with one session (insert at edge).
pub fn drag_split_enabled(pane: &FunctionPane, session_count: usize) -> bool {
    pane.docked_visible() && session_count >= 1
}

pub fn render(
    ui: &mut egui::Ui,
    pane: &mut FunctionPane,
    page: &FunctionPage,
    sessions: &[WorkspaceSession],
    workspace: &WorkspaceLayout,
    highlighted_session: Option<&str>,
    settings_open: bool,
    connections: &[SavedConnection],
    settings: &AppSettings,
    _page_slide: f32,
) -> FunctionAction {
    match page {
        FunctionPage::Workspace => workspace_view::render(
            ui,
            pane,
            sessions,
            workspace,
            highlighted_session,
            settings_open,
            settings,
        ),
        FunctionPage::Connections => connections::render(ui, connections),
    }
}
