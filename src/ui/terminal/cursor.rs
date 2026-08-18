use egui::Painter;

use crate::config::{CursorStyle, Rgba, TerminalTheme};
use crate::terminal::screen::{Screen, cell_display_width};

fn egui_color(c: Rgba) -> egui::Color32 {
    egui::Color32::from_rgba_premultiplied(c.r, c.g, c.b, c.a)
}

pub fn paint_cursor(
    painter: &Painter,
    screen: &Screen,
    theme: &TerminalTheme,
    rect: egui::Rect,
    cell_w: f32,
    cell_h: f32,
    style: CursorStyle,
    now: Option<std::time::Instant>,
    viewport_row: Option<usize>,
) {
    let rows = screen.rows;
    let cols = screen.cols;
    if rows == 0 || cols == 0 || cell_w <= 0.0 || cell_h <= 0.0 {
        return;
    }
    let paint_row = viewport_row.unwrap_or(screen.cursor_y);
    if !screen.cursor_visible
        || screen.cursor_y >= rows
        || screen.cursor_x >= cols
        || paint_row >= rows
    {
        return;
    }

    let is_blink = matches!(
        style,
        CursorStyle::BarBlink | CursorStyle::BlockBlink | CursorStyle::UnderlineBlink
    );
    if is_blink && now.is_some_and(|now| (now.elapsed().as_millis() / 530) % 2 == 1) {
        return;
    }

    let row = screen.cells.get(screen.cursor_y).map(|r| r.as_slice());
    let span = row
        .map(|r| cell_display_width(r, screen.cursor_x))
        .unwrap_or(1)
        .max(1);
    let cx = rect.left() + screen.cursor_x as f32 * cell_w;
    let cy = rect.top() + paint_row as f32 * cell_h;
    let cell_rect =
        egui::Rect::from_min_size(egui::pos2(cx, cy), egui::vec2(cell_w * span as f32, cell_h));

    match style {
        CursorStyle::Bar | CursorStyle::BarBlink => {
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(cx, cy),
                egui::vec2(2.0_f32.min(cell_w), cell_h),
            );
            painter.rect_filled(bar_rect, egui::CornerRadius::ZERO, egui_color(theme.cursor));
        }
        CursorStyle::Block | CursorStyle::BlockBlink => {
            painter.rect_stroke(
                cell_rect,
                egui::CornerRadius::ZERO,
                egui::Stroke::new(1.0_f32, egui_color(theme.cursor)),
                egui::StrokeKind::Inside,
            );
        }
        CursorStyle::Underline | CursorStyle::UnderlineBlink => {
            let line_h = 2.0_f32.min(cell_h * 0.2);
            let line_rect = egui::Rect::from_min_max(
                egui::pos2(cx, cy + cell_h - line_h),
                egui::pos2(cx + cell_w, cy + cell_h),
            );
            painter.rect_filled(
                line_rect,
                egui::CornerRadius::ZERO,
                egui_color(theme.cursor),
            );
        }
    }
}
