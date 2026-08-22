//! Pixel-accurate single-line text fitting with ellipsis.

use egui::{Color32, FontId, Ui};

/// Truncate `text` to fit `max_w` pixels using `…` when needed (single line).
pub fn truncate_to_width(
    ui: &Ui,
    text: &str,
    font_size: f32,
    max_w: f32,
    color: Color32,
) -> String {
    let font = FontId::proportional(font_size);
    let fits = |s: &str| {
        ui.fonts_mut(|f| {
            f.layout_no_wrap(s.to_string(), font.clone(), color)
                .size()
                .x
        }) <= max_w
    };
    if fits(text) {
        return text.to_string();
    }
    let ellipsis = "…";
    let mut lo = 0usize;
    let mut hi = text.chars().count();
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let prefix: String = text.chars().take(mid).collect();
        if fits(&format!("{prefix}{ellipsis}")) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let prefix: String = text.chars().take(lo).collect();
    format!("{prefix}{ellipsis}")
}

/// Paint single-line fitted text at `pos` (vertically centered on `pos.y`).
pub fn paint_text_fitted(
    ui: &Ui,
    pos: egui::Pos2,
    text: &str,
    max_w: f32,
    font_size: f32,
    color: Color32,
) -> String {
    let fitted = truncate_to_width(ui, text, font_size, max_w, color);
    let galley =
        ui.fonts_mut(|f| f.layout_no_wrap(fitted.clone(), FontId::proportional(font_size), color));
    let y = pos.y - galley.size().y * 0.5;
    ui.painter().galley(egui::pos2(pos.x, y), galley, color);
    fitted
}
