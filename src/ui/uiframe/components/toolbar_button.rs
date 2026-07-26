//! Toolbar buttons — fixed-size icon and text controls for pane chrome.

use egui::{Id, Response, Sense, Ui, Vec2, WidgetText};

use crate::ui::uiframe::style;
use crate::ui::uiframe::vector_icons::{self, Icon};

/// Uniform toolbar control size (width × height).
pub const TOOLBAR_BTN_SIZE: Vec2 = Vec2::new(24.0, 22.0);

const ICON_STROKE: f32 = 1.4;

/// Vector-icon toolbar button.
pub fn icon_toolbar_button(ui: &mut Ui, id: Id, icon: Icon) -> Response {
    let (rect, resp) = ui.allocate_exact_size(TOOLBAR_BTN_SIZE, Sense::click());
    paint_icon_btn(ui, rect, &resp, icon, false);
    let _ = id;
    resp
}

/// Vector-icon toolbar button with red hover (close).
pub fn icon_toolbar_danger(ui: &mut Ui, id: Id, icon: Icon) -> Response {
    let (rect, resp) = ui.allocate_exact_size(TOOLBAR_BTN_SIZE, Sense::click());
    paint_icon_btn(ui, rect, &resp, icon, true);
    let _ = id;
    resp
}

/// Vector-icon toolbar button with accent tint when `active`.
pub fn icon_toolbar_toggle(ui: &mut Ui, id: Id, icon: Icon, active: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(TOOLBAR_BTN_SIZE, Sense::click());
    if ui.is_rect_visible(rect) {
        let color = if active {
            style::ACCENT
        } else if resp.hovered() {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().weak_text_color()
        };
        if resp.hovered() && !active {
            ui.painter().rect_filled(rect, style::CORNER_RADIUS_XS, ui.visuals().widgets.hovered.bg_fill);
        }
        vector_icons::paint(ui, rect.shrink(3.0), icon, color, ICON_STROKE);
    }
    let _ = id;
    resp
}

/// Text label in the same fixed toolbar slot (e.g. A-, Sp).
pub fn text_toolbar_button(ui: &mut Ui, id: Id, label: impl Into<WidgetText>) -> Response {
    let (rect, resp) = ui.allocate_exact_size(TOOLBAR_BTN_SIZE, Sense::click());
    if ui.is_rect_visible(rect) {
        if resp.hovered() {
            ui.painter().rect_filled(rect, style::CORNER_RADIUS_XS, ui.visuals().widgets.hovered.bg_fill);
        }
        let text: WidgetText = label.into();
        let galley = text.into_galley(
            ui,
            None,
            f32::INFINITY,
            egui::TextStyle::Button,
        );
        let pos = rect.center() - galley.size() / 2.0;
        let color = if resp.hovered() {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().text_color()
        };
        ui.painter().galley(pos, galley, color);
    }
    let _ = id;
    resp
}

/// Legacy text toolbar button (kept for file manager etc.).
pub fn toolbar_button(ui: &mut Ui, label: impl Into<WidgetText>) -> Response {
    text_toolbar_button(ui, ui.next_auto_id(), label)
}

/// Close button using vector icon.
pub fn close_button(ui: &mut Ui) -> Response {
    icon_toolbar_danger(ui, ui.next_auto_id(), Icon::Close)
        .on_hover_text(rust_i18n::t!("close_pane"))
}

fn paint_icon_btn(ui: &mut Ui, rect: egui::Rect, resp: &Response, icon: Icon, danger: bool) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    if resp.hovered() {
        let fill = if danger {
            style::RED_BG
        } else {
            ui.visuals().widgets.hovered.bg_fill
        };
        ui.painter().rect_filled(rect, style::CORNER_RADIUS_XS, fill);
    }
    let color = if danger {
        if resp.hovered() {
            style::RED
        } else {
            ui.visuals().weak_text_color()
        }
    } else {
        vector_icons::icon_color(ui, resp)
    };
    vector_icons::paint(ui, rect.shrink(3.0), icon, color, ICON_STROKE);
}
