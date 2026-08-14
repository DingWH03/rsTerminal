//! Draggable split handle for multi-pane layouts.

use rsterm_uiframe::style::PANE_GAP;

use crate::layout::{MIN_PANE_WIDTH, SplitAxis};

/// Gap between split panes (also the splitter hit strip width).
pub const SPLITTER_SIZE: f32 = PANE_GAP;

/// Allocate a draggable splitter between two panes.
/// Returns `Some(new_ratio)` when the user drags the handle.
pub fn drag_splitter(
    ui: &mut egui::Ui,
    axis: SplitAxis,
    ratio: f32,
    available: egui::Vec2,
    id: egui::Id,
) -> Option<f32> {
    let (splitter_size, cursor) = match axis {
        SplitAxis::Horizontal => (
            egui::vec2(SPLITTER_SIZE, available.y),
            egui::CursorIcon::ResizeHorizontal,
        ),
        SplitAxis::Vertical => (
            egui::vec2(available.x, SPLITTER_SIZE),
            egui::CursorIcon::ResizeVertical,
        ),
    };

    let total = match axis {
        SplitAxis::Horizontal => available.x,
        SplitAxis::Vertical => available.y,
    };
    if total <= SPLITTER_SIZE * 2.0 {
        return None;
    }

    let first_size = (total - SPLITTER_SIZE) * ratio;
    let splitter_pos = match axis {
        SplitAxis::Horizontal => egui::pos2(ui.cursor().min.x + first_size, ui.cursor().min.y),
        SplitAxis::Vertical => egui::pos2(ui.cursor().min.x, ui.cursor().min.y + first_size),
    };
    let splitter_rect = egui::Rect::from_min_size(splitter_pos, splitter_size);
    let resp = ui.interact(splitter_rect, id, egui::Sense::drag());

    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(cursor);
    }

    if resp.dragged() {
        let delta = match axis {
            SplitAxis::Horizontal => resp.drag_delta().x,
            SplitAxis::Vertical => resp.drag_delta().y,
        };
        let new_first = (first_size + delta).clamp(
            MIN_PANE_WIDTH.min(total * 0.15),
            total - SPLITTER_SIZE - MIN_PANE_WIDTH.min(total * 0.15),
        );
        let new_ratio = new_first / (total - SPLITTER_SIZE);
        return Some(new_ratio.clamp(0.15, 0.85));
    }

    None
}
