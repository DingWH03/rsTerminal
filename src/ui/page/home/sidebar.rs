//! 首页侧边栏 — 品牌标识、导航按钮（首页/设置）和会话列表。

use crate::session::WorkspaceSession;
use crate::ui::widget::sidebar::Sidebar;
use crate::ui::widget::sidebar::common::{nav_button, sidebar_brand_row, sidebar_sessions_panel, SidebarSessionAction};

/// 首页侧边栏的渲染结果。
pub struct HomeSidebarResult {
    /// 导航操作（首页/设置）
    pub nav: HomeSidebarAction,
    /// 会话列表操作（选择/关闭/新窗口）
    pub sessions: SidebarSessionAction,
}

/// 首页侧边栏导航操作枚举。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HomeSidebarAction {
    /// 无操作
    None,
    /// 跳转到首页
    Home,
    /// 跳转到设置
    Settings,
}

/// 渲染首页侧边栏。
///
/// 包含品牌标题行、导航按钮（首页/设置）、分隔线和当前会话列表。
/// `in_overlay` 参数控制是否为浮动覆盖模式（窄屏时使用）。
pub fn paint_home_sidebar(
    ui: &mut egui::Ui,
    sidebar: &mut Sidebar,
    in_overlay: bool,
    on_home: bool,
    on_settings: bool,
    sessions: &[WorkspaceSession],
    active_session_id: Option<&str>,
) -> HomeSidebarResult {
    let show_ham = in_overlay && !sidebar.wide;
    sidebar_brand_row(ui, sidebar, show_ham);
    ui.add_space(8.0);

    let mut nav_action = HomeSidebarAction::None;

    ui.add_space(2.0);
    if nav_button(ui, "\u{2302}", &rust_i18n::t!("sidebar_home"), on_home).clicked() {
        nav_action = HomeSidebarAction::Home;
    }
    if nav_button(ui, "\u{2699}", &rust_i18n::t!("settings"), on_settings).clicked() {
        nav_action = HomeSidebarAction::Settings;
    }

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    let sessions_action = sidebar_sessions_panel(ui, sessions, active_session_id);

    HomeSidebarResult {
        nav: nav_action,
        sessions: sessions_action,
    }
}
