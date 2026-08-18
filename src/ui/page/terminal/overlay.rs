use std::time::{Duration, Instant};

use crate::config::TerminalTheme;
use crate::session::ActiveSession;

pub(super) fn size_label_visible(
    session: &mut ActiveSession,
    cols: usize,
    rows: usize,
    ctx: &egui::Context,
) -> bool {
    let dims = (cols, rows);
    let now = Instant::now();

    if dims != session.view.size_label_dims {
        session.view.size_label_dims = dims;
        session.view.size_label_active = true;
        session.view.size_label_hide_at = None;
        return true;
    }
    if !session.view.size_label_active {
        return false;
    }
    if session.view.size_label_hide_at.is_none() {
        session.view.size_label_hide_at = Some(now + Duration::from_secs(1));
        ctx.request_repaint_after(Duration::from_secs(1));
    }

    session
        .view
        .size_label_hide_at
        .is_some_and(|deadline| now < deadline)
}

pub(super) fn paint_size_label(
    painter: &egui::Painter,
    panel_rect: egui::Rect,
    theme: &TerminalTheme,
    cols: usize,
    rows: usize,
) {
    let color = egui::Color32::from_rgba_premultiplied(theme.fg.r, theme.fg.g, theme.fg.b, 140);
    painter.text(
        panel_rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("{cols}×{rows}"),
        egui::FontId::monospace(13.0),
        color,
    );
}
