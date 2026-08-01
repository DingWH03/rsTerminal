//! Left function pane — sessions, connections, commands, sidebar files, and monitor.

pub mod commands_view;
pub mod common;
pub mod connections;
pub mod files_view;
pub mod monitor_view;
pub mod pages;
pub mod session_list;
pub mod workspace_view;

use crate::data::persist::types::{ConnectionType, FavoriteCommand, SavedConnection};
use crate::session::WorkspaceSession;
use crate::data::prefs::Prefs;
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

    /// Whether the docked sidebar is open (independent of wide layout).
    pub fn docked_open(&self) -> bool {
        self.docked_open
    }

    /// Toggle docked sidebar visibility. No-op when not in wide layout.
    pub fn toggle_docked_sidebar(&mut self) {
        if self.wide {
            self.docked_open = !self.docked_open;
        }
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

/// Whether the Monitor tab should be shown (SSH terminal focused, wide layout).
pub fn monitor_tab_visible(
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
    sessions.iter().find(|s| s.id() == id).is_some_and(|s| {
        matches!(
            s,
            WorkspaceSession::Terminal(t) if t.conn_type == ConnectionType::Ssh
        )
    })
}

pub fn render(
    ui: &mut egui::Ui,
    pane: &mut FunctionPane,
    page: &mut FunctionPage,
    sessions: &mut [WorkspaceSession],
    workspace: &WorkspaceLayout,
    highlighted_session: Option<&str>,
    connections: &[SavedConnection],
    favorite_commands: &[FavoriteCommand],
    auth_users: &[crate::data::persist::types::AuthUser],
    prefs: &Prefs,
    _page_slide: f32,
) -> FunctionAction {
    let focused = workspace.focused_session_id();
    let show_files = files_tab_visible(pane, sessions, focused);
    let show_monitor = monitor_tab_visible(pane, sessions, focused);
    if *page == FunctionPage::Files && !show_files {
        *page = FunctionPage::Active;
    }
    if *page == FunctionPage::Monitor && !show_monitor {
        *page = FunctionPage::Active;
    }

    let mut action = FunctionAction::empty();

    // Compact sidebar chrome (outer panel already has 1px inset).
    ui.spacing_mut().item_spacing = egui::vec2(4.0, 2.0);
    ui.spacing_mut().button_padding = egui::vec2(4.0, 2.0);

    let active_tip = rust_i18n::t!("sidebar_tab_active");
    let conn_tip = rust_i18n::t!("sidebar_tab_connections");
    let cmds_tip = rust_i18n::t!("sidebar_tab_commands");
    let files_tip = rust_i18n::t!("sidebar_tab_files");
    let monitor_tip = rust_i18n::t!("sidebar_tab_monitor");

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
        TabBarItem {
            id: FunctionPage::Commands.as_tab_id(),
            icon: Icon::Commands,
            tip: cmds_tip.as_ref(),
        },
    ];
    if show_files {
        items.push(TabBarItem {
            id: FunctionPage::Files.as_tab_id(),
            icon: Icon::Folder,
            tip: files_tip.as_ref(),
        });
    }
    if show_monitor {
        items.push(TabBarItem {
            id: FunctionPage::Monitor.as_tab_id(),
            icon: Icon::Chart,
            tip: monitor_tip.as_ref(),
        });
    }

    let tab_strip_h = TabBar::HEIGHT + 2.0;
    egui::Panel::bottom("function_pane_tabs")
        .exact_size(tab_strip_h)
        .show_separator_line(true)
        .frame(egui::Frame::NONE)
        .show_inside(ui, |ui| {
            let mut selected = page.as_tab_id();
            if TabBar::show(ui, &mut selected, &items) {
                *page = FunctionPage::from_tab_id(selected);
            }
        });

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show_inside(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 2.0);
            let body_action = match *page {
                FunctionPage::Active => workspace_view::render(
                    ui,
                    pane,
                    sessions,
                    workspace,
                    highlighted_session,
                    prefs,
                ),
                FunctionPage::Connections => connections::render(ui, connections),
                FunctionPage::Commands => commands_view::render(ui, favorite_commands),
                FunctionPage::Files => {
                    files_view::render(ui, sessions, focused, connections, auth_users)
                }
                FunctionPage::Monitor => monitor_view::render(ui, sessions, focused),
            };
            action = body_action;
        });

    action
}
