//! 工具栏按钮组件 — 无边框、圆角、透明背景的按钮。
//!
//! 消除 `page/terminal/mod.rs` 和 `page/file_manager/mod.rs` 中重复的 `toolbar_button` 函数。

use egui::{Button, Color32, Response, RichText, Stroke, Ui, WidgetText};

use crate::ui::widget::style;

/// 渲染工具栏按钮（无边框、圆角、透明背景）。
///
/// `label` 可以是字符串或 `RichText`，支持自定义颜色和样式。
pub fn toolbar_button(ui: &mut Ui, label: impl Into<WidgetText>) -> Response {
    ui.add(
        Button::new(label)
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
            .corner_radius(style::CORNER_RADIUS_XS)
            .min_size(egui::Vec2::new(26.0, 22.0)),
    )
}

/// 渲染带颜色的关闭按钮（红色悬停背景）。
pub fn close_button(ui: &mut Ui) -> Response {
    ui.scope(|ui| {
        ui.style_mut().visuals.widgets.hovered.bg_fill = style::RED_BG;
        ui.style_mut().visuals.widgets.active.bg_fill = style::RED_BG;
        toolbar_button(ui, RichText::new("✕").size(12.0).color(style::RED))
    })
    .inner
}
