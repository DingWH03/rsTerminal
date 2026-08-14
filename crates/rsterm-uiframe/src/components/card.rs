//! 卡片组件 — 通用的选择卡片渲染。
//!
//! 提供统一的卡片外观（背景色、边框）和交互状态（选中/悬停/普通），
//! 消除各页面中重复的 `card_fill`、`card_stroke`、`paint_card_chrome` 等函数。

use egui::{Color32, CornerRadius, Rect, Stroke, Ui};

use crate::{interactive, tokens};

/// 卡片圆角大小
pub const CARD_CORNER_RADIUS: CornerRadius = tokens::radius::SM;

/// 获取动态卡片背景色 — 同时适配浅色和深色主题。
pub fn card_fill(ui: &Ui, selected: bool, hovered: bool) -> Color32 {
    interactive::card_chrome(ui, interactive::state(selected, hovered)).fill
}

/// 获取动态卡片边框色 — 同时适配浅色和深色主题。
pub fn card_stroke(ui: &Ui, selected: bool, hovered: bool) -> Stroke {
    interactive::card_chrome(ui, interactive::state(selected, hovered)).stroke
}

/// 绘制卡片的基础外观（填充背景 + 边框描边）。
pub fn paint_card_chrome(ui: &Ui, rect: Rect, fill: Color32, stroke: Stroke) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CARD_CORNER_RADIUS, fill);
    painter.rect_stroke(rect, CARD_CORNER_RADIUS, stroke, egui::StrokeKind::Inside);
}
