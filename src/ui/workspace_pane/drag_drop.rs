//! Drag-and-drop for sidebar sessions and in-workspace pane rearrangement.

use std::collections::HashMap;

use crate::ui::layout::{DropEdge, DropZone, PaneId, WorkspaceLayout};
use crate::ui::shell::layout_preview::PREVIEW_GHOST_PANE;
use crate::ui::uiframe::interactive::{self, AccentTone};
use crate::ui::uiframe::style;
use crate::ui::uiframe::tokens;

/// Ratio allocated to the incoming pane during live insert preview (visual only).
pub const PREVIEW_INSERT_RATIO: f32 = 0.5;

#[derive(Clone, Debug)]
pub struct PaneRect {
    pub pane_id: PaneId,
    pub rect: egui::Rect,
}

#[derive(Clone, Debug)]
pub enum ActiveDrag {
    Session { session_id: String, label: String },
    Pane { pane_id: PaneId, label: String },
}

impl ActiveDrag {
    pub fn label(&self) -> &str {
        match self {
            ActiveDrag::Session { label, .. } | ActiveDrag::Pane { label, .. } => label,
        }
    }
}

/// Collect pane screen rects during tree render (call from leaf nodes).
pub fn register_pane_rect(
    map: &mut HashMap<PaneId, egui::Rect>,
    pane_id: PaneId,
    rect: egui::Rect,
) {
    map.insert(pane_id, rect);
}

/// Hyprland-style hit test: pointer over a pane maps to the nearest edge (L/R/T/B).
/// Pane drags use the inner third as a swap target; session drags are edge-only.
pub fn hit_test_drop_zone(
    pointer: egui::Pos2,
    workspace: egui::Rect,
    panes: &HashMap<PaneId, egui::Rect>,
    drag: &ActiveDrag,
) -> Option<DropZone> {
    if !workspace.contains(pointer) {
        return None;
    }

    if let Some((pane_id, rect)) = pane_under_pointer(pointer, panes) {
        if let ActiveDrag::Pane { pane_id: src, .. } = drag {
            if *src == pane_id {
                return None;
            }
            if pointer_in_center_third(pointer, rect) {
                return Some(DropZone::PaneCenter { pane_id });
            }
        }
        let edge = nearest_edge(pointer, rect);
        return Some(DropZone::Pane { pane_id, edge });
    }

    Some(DropZone::Root {
        edge: nearest_edge(pointer, workspace),
    })
}

/// Smallest pane containing the pointer (ignores preview ghost).
fn pane_under_pointer(
    pointer: egui::Pos2,
    panes: &HashMap<PaneId, egui::Rect>,
) -> Option<(PaneId, egui::Rect)> {
    let mut best: Option<(PaneId, egui::Rect)> = None;
    for (&pane_id, rect) in panes {
        if pane_id == PREVIEW_GHOST_PANE || !rect.contains(pointer) {
            continue;
        }
        let area = rect.width() * rect.height();
        match best {
            None => best = Some((pane_id, *rect)),
            Some((_, prev)) if area < prev.width() * prev.height() => {
                best = Some((pane_id, *rect));
            }
            _ => {}
        }
    }
    best
}

/// Pick the closest edge by distance from pointer to each side.
fn nearest_edge(pointer: egui::Pos2, rect: egui::Rect) -> DropEdge {
    let d_left = pointer.x - rect.left();
    let d_right = rect.right() - pointer.x;
    let d_top = pointer.y - rect.top();
    let d_bottom = rect.bottom() - pointer.y;

    let min = d_left.min(d_right).min(d_top).min(d_bottom);
    if min == d_left {
        DropEdge::Left
    } else if min == d_right {
        DropEdge::Right
    } else if min == d_top {
        DropEdge::Top
    } else {
        DropEdge::Bottom
    }
}

/// Inner third of a pane — drop here swaps panes (Hyprland swap).
fn pointer_in_center_third(pointer: egui::Pos2, rect: egui::Rect) -> bool {
    let w = rect.width().max(1.0);
    let h = rect.height().max(1.0);
    let rel_x = (pointer.x - rect.left()) / w;
    let rel_y = (pointer.y - rect.top()) / h;
    rel_x > 0.33 && rel_x < 0.67 && rel_y > 0.33 && rel_y < 0.67
}

/// Draw drop-zone highlight and ghost drag label.
pub fn paint_drag_overlay(
    ctx: &egui::Context,
    workspace: egui::Rect,
    panes: &HashMap<PaneId, egui::Rect>,
    drag: &ActiveDrag,
    pointer: egui::Pos2,
    zone: Option<DropZone>,
) {
    if let Some(zone) = zone {
        let highlight = match zone {
            DropZone::Root { edge } => insert_highlight_rect(workspace, edge),
            DropZone::Pane { pane_id, edge } => panes
                .get(&pane_id)
                .map(|r| insert_highlight_rect(*r, edge))
                .unwrap_or_else(|| insert_highlight_rect(workspace, edge)),
            DropZone::PaneCenter { pane_id } => panes.get(&pane_id).copied().unwrap_or(workspace),
        };
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("drop_highlight"),
        ));
        let fill = if matches!(zone, DropZone::PaneCenter { .. }) {
            interactive::accent_tone(style::ACCENT, AccentTone::Faint)
        } else {
            interactive::accent_tone(style::ACCENT, AccentTone::Subtle)
        };
        painter.rect_filled(highlight, style::CORNER_RADIUS_XS, fill);
        painter.rect_stroke(
            highlight,
            style::CORNER_RADIUS_XS,
            egui::Stroke::new(tokens::stroke::STRONG, style::ACCENT),
            egui::StrokeKind::Inside,
        );
    }

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("drag_ghost"),
    ));
    let label = drag.label();
    let galley = ctx.fonts_mut(|f| {
        f.layout(
            label.to_string(),
            egui::FontId::proportional(13.0),
            egui::Color32::WHITE,
            f32::INFINITY,
        )
    });
    let pad = egui::vec2(8.0, 4.0);
    let size = galley.size() + pad * 2.0;
    let ghost_rect = egui::Rect::from_center_size(pointer + egui::vec2(12.0, 12.0), size);
    painter.rect_filled(
        ghost_rect,
        style::CORNER_RADIUS_SM,
        egui::Color32::from_black_alpha(180),
    );
    painter.galley(ghost_rect.min + pad, galley, egui::Color32::WHITE);
}

/// Half-pane highlight showing where the dropped session will land.
fn insert_highlight_rect(rect: egui::Rect, edge: DropEdge) -> egui::Rect {
    let w = rect.width();
    let h = rect.height();
    match edge {
        DropEdge::Left => egui::Rect::from_min_size(rect.min, egui::vec2(w * 0.5, h)),
        DropEdge::Right => egui::Rect::from_min_size(
            egui::pos2(rect.left() + w * 0.5, rect.top()),
            egui::vec2(w * 0.5, h),
        ),
        DropEdge::Top => egui::Rect::from_min_size(rect.min, egui::vec2(w, h * 0.5)),
        DropEdge::Bottom => egui::Rect::from_min_size(
            egui::pos2(rect.left(), rect.top() + h * 0.5),
            egui::vec2(w, h * 0.5),
        ),
    }
}

/// Apply a drop zone to the layout; returns new focused pane if changed.
pub fn apply_drop(
    layout: &mut WorkspaceLayout,
    drag: &ActiveDrag,
    zone: DropZone,
    palette_len: usize,
) -> Option<PaneId> {
    let new_color = crate::ui::pane_colors::next_color_index(layout, palette_len);
    match (drag, zone) {
        (ActiveDrag::Session { session_id, .. }, zone) => {
            layout.apply_session_drop(session_id, zone, new_color)
        }
        (ActiveDrag::Pane { pane_id, .. }, DropZone::PaneCenter { pane_id: target })
            if *pane_id != target =>
        {
            layout.swap_panes(*pane_id, target);
            Some(target)
        }
        (
            ActiveDrag::Pane { pane_id, .. },
            DropZone::Pane {
                pane_id: target,
                edge,
            },
        ) if *pane_id != target => layout.move_pane_to_edge(*pane_id, target, edge, new_color),
        (ActiveDrag::Pane { pane_id, .. }, DropZone::Root { edge }) => {
            let target = layout.focused_pane;
            if *pane_id != target {
                layout.move_pane_to_edge(*pane_id, target, edge, new_color)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Commit a drop using the current zone or the most recently observed zone.
///
/// Returns the newly focused pane when the layout changed.
pub fn commit_drop(
    layout: &mut WorkspaceLayout,
    drag: &ActiveDrag,
    zone: Option<DropZone>,
    last_drop_zone: Option<DropZone>,
    palette_len: usize,
) -> Option<PaneId> {
    let zone = zone.or(last_drop_zone)?;
    let focused = apply_drop(layout, drag, zone, palette_len)?;
    layout.focused_pane = focused;
    Some(focused)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_edge_left() {
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 80.0));
        assert_eq!(nearest_edge(egui::pos2(5.0, 40.0), rect), DropEdge::Left);
    }

    #[test]
    fn nearest_edge_right() {
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 80.0));
        assert_eq!(nearest_edge(egui::pos2(95.0, 40.0), rect), DropEdge::Right);
    }

    #[test]
    fn session_drag_always_edge() {
        let mut panes = HashMap::new();
        let pid = PaneId(1);
        panes.insert(
            pid,
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(200.0, 100.0)),
        );
        let ws = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(200.0, 100.0));
        let drag = ActiveDrag::Session {
            session_id: "a".into(),
            label: "a".into(),
        };
        // Center of pane — still maps to an edge, not PaneCenter.
        let zone = hit_test_drop_zone(egui::pos2(100.0, 50.0), ws, &panes, &drag).unwrap();
        assert!(matches!(
            zone,
            DropZone::Pane {
                edge: DropEdge::Right | DropEdge::Left | DropEdge::Top | DropEdge::Bottom,
                ..
            }
        ));
        assert!(!matches!(zone, DropZone::PaneCenter { .. }));
    }
}
