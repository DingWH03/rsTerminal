//! 图标绘制组件 — 在指定矩形区域居中绘制文本图标。
//!
//! 提供统一的图标绘制和交互式图标按钮功能，
//! 消除各页面中重复的 `paint_icon`、`paint_edit_icon`、`paint_file_icon` 等函数。

use egui::{Color32, FontId, Pos2, Rect, Response, Sense, Ui};

/// 在指定矩形区域内居中绘制文本图标。
///
/// `font_size` 控制图标大小，`color` 控制颜色。
pub fn paint_icon(ui: &Ui, rect: Rect, icon: &str, font_size: f32, color: Color32) {
    let galley = ui.fonts_mut(|f| {
        f.layout(
            icon.to_string(),
            FontId::proportional(font_size),
            color,
            f32::INFINITY,
        )
    });
    ui.painter_at(rect).galley(
        Pos2::new(
            rect.center().x - galley.size().x / 2.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        color,
    );
}

/// 根据悬停状态返回图标颜色。
pub fn icon_color(ui: &Ui, resp: &Response) -> Color32 {
    if resp.hovered() {
        ui.visuals().selection.stroke.color
    } else {
        ui.visuals().weak_text_color()
    }
}

/// 创建一个可点击的图标按钮。
///
/// 在 `rect` 区域内渲染图标并处理点击交互。
pub fn icon_button(ui: &mut Ui, rect: Rect, id: egui::Id, icon: &str, font_size: f32) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    if ui.is_rect_visible(rect) {
        paint_icon(ui, rect, icon, font_size, icon_color(ui, &resp));
    }
    resp
}
