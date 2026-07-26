//! Left function pane — sessions, connections, and sidebar files.

pub mod common;
pub mod connections;
pub mod files_view;
pub mod pages;
pub mod session_list;
pub mod workspace_view;

use crate::session::WorkspaceSession;
use crate::settings::AppSettings;
use crate::storage::types::SavedConnection;
use crate::ui::function_pane::files_view::SidebarFilesState;
use crate::ui::function_pane::pages::FunctionPage;
use crate::ui::shell::layout_state::WorkspaceLayout;
use crate::ui::shell::messages::FunctionAction;
use crate::ui::uiframe::{TabBar, TabBarItem};

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
            crate::ui::uiframe::vector_icons::paint(
                ui,
                rect,
                crate::ui::uiframe::vector_icons::Icon::Hamburger,
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

/// Whether the Files tab should be shown.
pub fn files_tab_visible(
    pane: &FunctionPane,
    sessions: &[WorkspaceSession],
    focused_session_id: Option<&str>,
) -> bool {
    if !pane.wide {
        return false;
    }
    let Some(id) = focused_session_id else {
        return false;
    };
    sessions
        .iter()
        .find(|s| s.id() == id)
        .is_some_and(|s| s.is_terminal())
}

pub fn render(
    ui: &mut egui::Ui,
    pane: &mut FunctionPane,
    page: &mut FunctionPage,
    sessions: &[WorkspaceSession],
    workspace: &WorkspaceLayout,
    highlighted_session: Option<&str>,
    connections: &[SavedConnection],
    settings: &AppSettings,
    files_state: &mut SidebarFilesState,
    _page_slide: f32,
) -> FunctionAction {
    let show_files = files_tab_visible(pane, sessions, workspace.focused_session_id());
    if *page == FunctionPage::Files && !show_files {
        *page = FunctionPage::Active;
    }

    let mut action = FunctionAction::empty();

    let active_tip = rust_i18n::t!("sidebar_tab_active");
    let conn_tip = rust_i18n::t!("sidebar_tab_connections");
    let files_tip = rust_i18n::t!("sidebar_tab_files");

    use crate::ui::uiframe::vector_icons::Icon;
    let mut items = vec![
        TabBarItem {
            id: FunctionPage::Active.as_tab_id(),
            icon: Icon::Sessions,
            tip: active_tip.as_ref(),
        },
        TabBarItem {
            id: FunctionPage::Connections.as_tab_id(),
            icon: Icon::Connections,
            tip: conn_tip.as_ref(),
        },
    ];
    if show_files {
        items.push(TabBarItem {
            id: FunctionPage::Files.as_tab_id(),
            icon: Icon::Folder,
            tip: files_tip.as_ref(),
        });
    }

    // Pin tab strip to the bottom of the function pane.
    let tab_strip_h = TabBar::HEIGHT + 6.0;
    egui::Panel::bottom("function_pane_tabs")
        .exact_size(tab_strip_h)
        .show_separator_line(true)
        .show_inside(ui, |ui| {
            let mut selected = page.as_tab_id();
            if TabBar::show(ui, &mut selected, &items) {
                *page = FunctionPage::from_tab_id(selected);
            }
        });

    egui::CentralPanel::default().show_inside(ui, |ui| {
        let body_action = match *page {
            FunctionPage::Active => workspace_view::render(
                ui,
                pane,
                sessions,
                workspace,
                highlighted_session,
                settings,
            ),
            FunctionPage::Connections => connections::render(ui, connections),
            FunctionPage::Files => files_view::render(
                ui,
                files_state,
                sessions,
                workspace.focused_session_id(),
                connections,
            ),
        };
        action = body_action;
    });

    action
}
