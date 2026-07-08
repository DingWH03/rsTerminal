//! 卡片组件 — 通用的选择卡片渲染。
//!
//! 提供统一的卡片外观（背景色、边框）和交互状态（选中/悬停/普通），
//! 消除各页面中重复的 `card_fill`、`card_stroke`、`paint_card_chrome` 等函数。

use egui::{Color32, CornerRadius, Rect, Stroke, Ui};

/// 卡片圆角大小
pub const CARD_CORNER_RADIUS: CornerRadius = CornerRadius::same(6);

/// 获取动态卡片背景色 — 同时适配浅色和深色主题。
pub fn card_fill(ui: &Ui, selected: bool, hovered: bool) -> Color32 {
    if selected {
        ui.visuals().selection.bg_fill.gamma_multiply(0.35)
    } else if hovered {
        ui.visuals().widgets.hovered.bg_fill
    } else {
        ui.visuals().extreme_bg_color
    }
}

/// 获取动态卡片边框色 — 同时适配浅色和深色主题。
pub fn card_stroke(ui: &Ui, selected: bool, hovered: bool) -> Stroke {
    if selected {
        Stroke::new(1.5, ui.visuals().selection.stroke.color)
    } else if hovered {
        Stroke::new(1.0, ui.visuals().widgets.hovered.bg_stroke.color)
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke
    }
}

/// 绘制卡片的基础外观（填充背景 + 边框描边）。
pub fn paint_card_chrome(ui: &Ui, rect: Rect, fill: Color32, stroke: Stroke) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CARD_CORNER_RADIUS, fill);
    painter.rect_stroke(rect, CARD_CORNER_RADIUS, stroke, egui::StrokeKind::Inside);
}
