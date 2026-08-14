//! Layout preview geometry — simulated split tree and pane rects for drag-and-drop.

use std::collections::HashMap;

use crate::layout::{DropEdge, DropZone, PaneId, SplitAxis, SplitNode};
use crate::uiframe::split_handle::SPLITTER_SIZE;

/// Placeholder pane id used only during drag preview rendering.
pub const PREVIEW_GHOST_PANE: PaneId = PaneId(u64::MAX);

/// Build a preview split tree with a ghost pane at `zone` (does not mutate layout).
pub fn preview_tree(
    root: &SplitNode,
    zone: DropZone,
    focused: PaneId,
    ratio: f32,
) -> Option<SplitNode> {
    match zone {
        DropZone::Pane { pane_id, edge } => simulate_insert_at_edge(root, pane_id, edge, ratio),
        DropZone::Root { edge } => simulate_insert_at_edge(root, focused, edge, ratio),
        DropZone::PaneCenter { .. } => None,
    }
}

fn simulate_insert_at_edge(
    root: &SplitNode,
    target: PaneId,
    edge: DropEdge,
    ratio: f32,
) -> Option<SplitNode> {
    let mut cloned = root.clone();
    if replace_leaf_with_split_directed(
        &mut cloned,
        target,
        edge.split_axis(),
        ratio,
        PREVIEW_GHOST_PANE,
        edge.new_pane_first(),
    ) {
        Some(cloned)
    } else {
        None
    }
}

/// Compute pane screen rects from a split tree (matches split_widget layout math).
pub fn pane_rects_from_tree(root: &SplitNode, bounds: egui::Rect) -> HashMap<PaneId, egui::Rect> {
    let mut map = HashMap::new();
    collect_pane_rects(root, bounds, &mut map);
    map
}

fn collect_pane_rects(node: &SplitNode, bounds: egui::Rect, out: &mut HashMap<PaneId, egui::Rect>) {
    match node {
        SplitNode::Leaf { pane_id } => {
            out.insert(*pane_id, bounds);
        }
        SplitNode::Split {
            axis,
            ratio,
            first,
            second,
        } => match axis {
            SplitAxis::Horizontal => {
                let total_w = bounds.width();
                let first_w = ((total_w - SPLITTER_SIZE) * *ratio).max(1.0);
                let second_w = (total_w - SPLITTER_SIZE - first_w).max(1.0);
                let first_rect =
                    egui::Rect::from_min_size(bounds.min, egui::vec2(first_w, bounds.height()));
                let second_rect = egui::Rect::from_min_size(
                    egui::pos2(bounds.left() + first_w + SPLITTER_SIZE, bounds.top()),
                    egui::vec2(second_w, bounds.height()),
                );
                collect_pane_rects(first, first_rect, out);
                collect_pane_rects(second, second_rect, out);
            }
            SplitAxis::Vertical => {
                let total_h = bounds.height();
                let first_h = ((total_h - SPLITTER_SIZE) * *ratio).max(1.0);
                let second_h = (total_h - SPLITTER_SIZE - first_h).max(1.0);
                let first_rect =
                    egui::Rect::from_min_size(bounds.min, egui::vec2(bounds.width(), first_h));
                let second_rect = egui::Rect::from_min_size(
                    egui::pos2(bounds.left(), bounds.top() + first_h + SPLITTER_SIZE),
                    egui::vec2(bounds.width(), second_h),
                );
                collect_pane_rects(first, first_rect, out);
                collect_pane_rects(second, second_rect, out);
            }
        },
    }
}

fn replace_leaf_with_split_directed(
    node: &mut SplitNode,
    target: PaneId,
    axis: SplitAxis,
    ratio: f32,
    new_pane: PaneId,
    new_first: bool,
) -> bool {
    match node {
        SplitNode::Leaf { pane_id } if *pane_id == target => {
            let old = *pane_id;
            let (first_id, second_id) = if new_first {
                (new_pane, old)
            } else {
                (old, new_pane)
            };
            *node = SplitNode::Split {
                axis,
                ratio,
                first: Box::new(SplitNode::Leaf { pane_id: first_id }),
                second: Box::new(SplitNode::Leaf { pane_id: second_id }),
            };
            true
        }
        SplitNode::Leaf { .. } => false,
        SplitNode::Split { first, second, .. } => {
            replace_leaf_with_split_directed(first, target, axis, ratio, new_pane, new_first)
                || replace_leaf_with_split_directed(
                    second, target, axis, ratio, new_pane, new_first,
                )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::WorkspaceLayout;

    #[test]
    fn preview_tree_adds_ghost_pane() {
        let layout = WorkspaceLayout::new_single();
        let root = layout.focused_pane;
        let tree = preview_tree(
            &layout.root,
            DropZone::Pane {
                pane_id: root,
                edge: DropEdge::Right,
            },
            root,
            0.35,
        )
        .unwrap();
        let rects = pane_rects_from_tree(
            &tree,
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0)),
        );
        assert!(rects.contains_key(&PREVIEW_GHOST_PANE));
        assert_eq!(rects.len(), 2);
    }
}
