//! Shared function pane UI elements.

use crate::ui::function_pane::FunctionPane;
use crate::ui::uiframe::interactive;
use crate::ui::uiframe::style;
use crate::ui::uiframe::tokens;
use crate::ui::uiframe::vector_icons::{self, Icon};

pub fn nav_button(
    ui: &mut egui::Ui,
    icon: Option<Icon>,
    label: &str,
    selected: bool,
) -> egui::Response {
    let height = tokens::size::NAV_ROW;
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let chrome = interactive::row_chrome(ui, interactive::state(selected, resp.hovered()));
        if chrome.fill != egui::Color32::TRANSPARENT {
            ui.painter()
                .rect_filled(rect, style::CORNER_RADIUS_XS, chrome.fill);
        }

        let color = if selected {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().text_color()
        };

        let mut text_left = rect.left() + 6.0;
        if let Some(icon) = icon {
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(rect.left() + 12.0, rect.center().y),
                egui::vec2(14.0, 14.0),
            );
            vector_icons::paint(ui, icon_rect, icon, color, 1.4);
            text_left = rect.left() + 24.0;
        }

        let galley = ui.fonts_mut(|f| {
            f.layout(
                label.to_string(),
                egui::FontId::proportional(13.0),
                color,
                f32::INFINITY,
            )
        });
        ui.painter().galley(
            egui::pos2(text_left, rect.center().y - galley.size().y / 2.0),
            galley,
            color,
        );
    }

    resp
}

/// Optional hamburger-only row (brand text removed).
pub fn hamburger_row(ui: &mut egui::Ui, pane: &mut FunctionPane) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 24.0), egui::Sense::click());
    if resp.clicked() {
        pane.hamburger_click();
    }
    if ui.is_rect_visible(rect) {
        vector_icons::paint(
            ui,
            rect,
            Icon::Hamburger,
            ui.visuals().weak_text_color(),
            1.5,
        );
    }
}

#[deprecated(note = "use hamburger_row; brand text removed from sidebar")]
pub fn brand_row(ui: &mut egui::Ui, pane: &mut FunctionPane, show_hamburger: bool) {
    if show_hamburger {
        hamburger_row(ui, pane);
    }
}
