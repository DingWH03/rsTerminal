//! 响应式侧边栏：宽屏时停靠（可通过 ☰ 切换），窄屏时覆盖浮动。

pub mod common;
pub mod session_list;
pub mod sidebars;

use crate::ui::widget::style;

/// 宽屏/窄屏切换阈值（像素）
pub const WIDE_THRESHOLD: f32 = 720.0;
/// 停靠侧边栏宽度
pub const DOCK_WIDTH: f32 = 200.0;
/// 覆盖浮动侧边栏宽度
pub const OVERLAY_WIDTH: f32 = 260.0;

/// 侧边栏页面枚举。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SidebarPage {
    /// 工作区
    Workspace,
}

/// 响应式侧边栏 — 管理停靠/覆盖状态和宽度适配。
pub struct Sidebar {
    pub wide: bool,
    /// Wide layout: docked sidebar visible.
    docked_open: bool,
    /// Narrow layout: slide-over panel open.
    overlay_open: bool,
}

impl Sidebar {
    /// 创建新的侧边栏实例，默认停靠打开、覆盖关闭。
    pub fn new() -> Self {
        Self {
            wide: false,
            docked_open: true,
            overlay_open: false,
        }
    }

    /// 根据当前宽度同步侧边栏模式（宽屏/窄屏）。
    pub fn sync_width(&mut self, width: f32) {
        let now_wide = width > WIDE_THRESHOLD;
        if now_wide && !self.wide {
            self.overlay_open = false;
        }
        self.wide = now_wide;
    }

    /// 宽屏布局时停靠侧边栏是否可见。
    pub fn docked_visible(&self) -> bool {
        self.wide && self.docked_open
    }

    /// 窄屏布局时覆盖侧边栏是否可见。
    pub fn overlay_visible(&self) -> bool {
        !self.wide && self.overlay_open
    }

    /// 是否在内容区域显示 ☰ 汉堡菜单按钮。
    pub fn show_content_hamburger(&self) -> bool {
        true
    }

    /// 切换侧边栏可见性（宽屏切换停靠，窄屏切换覆盖）。
    pub fn hamburger_click(&mut self) {
        if self.wide {
            self.docked_open = !self.docked_open;
        } else {
            self.overlay_open = !self.overlay_open;
        }
    }

    /// 打开覆盖侧边栏（窄屏时）。
    pub fn open_overlay(&mut self) {
        if !self.wide {
            self.overlay_open = true;
        }
    }

    /// 关闭覆盖侧边栏。
    pub fn close_overlay(&mut self) {
        self.overlay_open = false;
    }

    /// 渲染汉堡菜单按钮（☰）。
    pub fn hamburger(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(
            egui::Button::new(egui::RichText::new("\u{2630}").size(18.0).color(style::TEXT_SECONDARY))
                .frame(false)
                .corner_radius(style::CORNER_RADIUS_XS),
        )
    }

    /// 渲染变暗的背景遮罩；如果用户点击面板外部则返回 `true`。
    pub fn overlay_backdrop_clicked(ctx: &egui::Context, backdrop_id: egui::Id) -> bool {
        let rect = ctx.content_rect();
        let mut clicked = false;
        egui::Area::new(backdrop_id)
            .order(egui::Order::Background)
            .interactable(true)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                let (_, r) = ui.allocate_exact_size(rect.size(), egui::Sense::click());
                ui.painter()
                    .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(120));
                clicked = r.clicked();
            });
        clicked
    }

    /// 显示覆盖侧边栏面板（窄屏浮动模式）。
    pub fn show_overlay<F>(ctx: &egui::Context, panel_id: &str, mut body: F)
    where
        F: FnMut(&mut egui::Ui),
    {
        let rect = ctx.content_rect();
        // Responsive overlay width: adapts to screen width on narrow devices.
        let w = OVERLAY_WIDTH.min(rect.width() * 0.82).max(180.0);
        let top_inset = {
            #[cfg(target_os = "android")]
            {
                crate::platform::get().top_inset_points(ctx)
            }
            #[cfg(not(target_os = "android"))]
            {
                0.0
            }
        };
        let panel_height = (rect.height() - top_inset).max(1.0);
        egui::Area::new(egui::Id::new(panel_id))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(rect.left(), rect.top() + top_inset))
            .show(ctx, |ui| {
                egui::Frame::side_top_panel(ui.style()).show(ui, |ui| {
                    ui.set_min_width(w);
                    ui.set_max_width(w);
                    ui.set_min_height(panel_height);
                    ui.set_max_height(panel_height);
                    body(ui);
                });
            });
    }
}
