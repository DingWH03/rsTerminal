//! Recursive split tree renderer.

use std::collections::HashMap;

use crate::ui::shell::layout_state::{PaneId, PaneState, SplitAxis, SplitNode, WorkspaceLayout};
use crate::ui::shell::messages::WorkspaceAction;
use crate::ui::uiframe::split_handle::{drag_splitter, SPLITTER_SIZE};

use super::pane_host::render_pane;
use super::WorkspacePaneContext;

pub fn render_split_tree(
    ui: &mut egui::Ui,
    layout: &mut WorkspaceLayout,
    ctx: &mut WorkspacePaneContext<'_>,
    pane_rects: &mut HashMap<PaneId, egui::Rect>,
    render_root: &SplitNode,
    preview_mode: bool,
) -> WorkspaceAction {
    let mut action = WorkspaceAction::empty();
    let available = ui.available_size();
    let panes = &layout.panes;
    let focused = layout.focused_pane;
    let split_enabled = ctx.split_enabled;
    let pane_count = layout.pane_count();
    let dragging = ctx.active_drag.is_some();

    let ratio_overrides = ctx.ratio_overrides.clone();

    if preview_mode {
        render_split_readonly(
            ui,
            render_root,
            available,
            panes,
            focused,
            split_enabled,
            pane_count,
            ctx,
            &mut action,
            pane_rects,
            &ratio_overrides,
        );
    } else {
        render_split(
            ui,
            &mut layout.root,
            available,
            panes,
            focused,
            split_enabled,
            pane_count,
            dragging,
            ctx,
            &mut action,
            pane_rects,
            &ratio_overrides,
        );
    }
    action
}

#[allow(clippy::too_many_arguments)]
fn render_split_readonly(
    ui: &mut egui::Ui,
    node: &SplitNode,
    available: egui::Vec2,
    panes: &HashMap<PaneId, PaneState>,
    focused_pane: PaneId,
    split_enabled: bool,
    pane_count: usize,
    ctx: &mut WorkspacePaneContext<'_>,
    action: &mut WorkspaceAction,
    pane_rects: &mut HashMap<PaneId, egui::Rect>,
    ratio_overrides: &HashMap<u64, f32>,
) {
    match node {
        SplitNode::Leaf { pane_id } => {
            ui.push_id(pane_id.0, |ui| {
                ui.allocate_ui_with_layout(
                    available,
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        render_pane(
                            ui,
                            *pane_id,
                            panes,
                            focused_pane,
                            split_enabled,
                            pane_count,
                            ctx,
                            action,
                            pane_rects,
                        );
                    },
                );
            });
        }
        SplitNode::Split {
            axis,
            ratio,
            first,
            second,
        } => {
            let ratio = ratio_overrides
                .get(&split_key(node))
                .copied()
                .unwrap_or(*ratio);
            match axis {
                SplitAxis::Horizontal => {
                    let total_w = available.x;
                    let first_w = ((total_w - SPLITTER_SIZE) * ratio).max(1.0);
                    let second_w = (total_w - SPLITTER_SIZE - first_w).max(1.0);

                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(first_w, available.y),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                render_split_readonly(
                                    ui,
                                    first,
                                    egui::vec2(first_w, available.y),
                                    panes,
                                    focused_pane,
                                    split_enabled,
                                    pane_count,
                                    ctx,
                                    action,
                                    pane_rects,
                                    ratio_overrides,
                                );
                            },
                        );
                        ui.add_space(SPLITTER_SIZE);
                        ui.allocate_ui_with_layout(
                            egui::vec2(second_w, available.y),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                render_split_readonly(
                                    ui,
                                    second,
                                    egui::vec2(second_w, available.y),
                                    panes,
                                    focused_pane,
                                    split_enabled,
                                    pane_count,
                                    ctx,
                                    action,
                                    pane_rects,
                                    ratio_overrides,
                                );
                            },
                        );
                    });
                }
                SplitAxis::Vertical => {
                    let total_h = available.y;
                    let first_h = ((total_h - SPLITTER_SIZE) * ratio).max(1.0);
                    let second_h = (total_h - SPLITTER_SIZE - first_h).max(1.0);

                    ui.vertical(|ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(available.x, first_h),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                render_split_readonly(
                                    ui,
                                    first,
                                    egui::vec2(available.x, first_h),
                                    panes,
                                    focused_pane,
                                    split_enabled,
                                    pane_count,
                                    ctx,
                                    action,
                                    pane_rects,
                                    ratio_overrides,
                                );
                            },
                        );
                        ui.add_space(SPLITTER_SIZE);
                        ui.allocate_ui_with_layout(
                            egui::vec2(available.x, second_h),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                render_split_readonly(
                                    ui,
                                    second,
                                    egui::vec2(available.x, second_h),
                                    panes,
                                    focused_pane,
                                    split_enabled,
                                    pane_count,
                                    ctx,
                                    action,
                                    pane_rects,
                                    ratio_overrides,
                                );
                            },
                        );
                    });
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_split(
    ui: &mut egui::Ui,
    node: &mut SplitNode,
    available: egui::Vec2,
    panes: &HashMap<PaneId, PaneState>,
    focused_pane: PaneId,
    split_enabled: bool,
    pane_count: usize,
    dragging: bool,
    ctx: &mut WorkspacePaneContext<'_>,
    action: &mut WorkspaceAction,
    pane_rects: &mut HashMap<PaneId, egui::Rect>,
    ratio_overrides: &HashMap<u64, f32>,
) {
    match node {
        SplitNode::Leaf { pane_id } => {
            ui.push_id(pane_id.0, |ui| {
                ui.allocate_ui_with_layout(
                    available,
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        render_pane(
                            ui,
                            *pane_id,
                            panes,
                            focused_pane,
                            split_enabled,
                            pane_count,
                            ctx,
                            action,
                            pane_rects,
                        );
                    },
                );
            });
        }
        SplitNode::Split {
            axis,
            ratio,
            first,
            second,
        } => {
            let key = split_key_from_children(first, second);
            if let Some(&override_ratio) = ratio_overrides.get(&key) {
                *ratio = override_ratio;
            }
            match axis {
                SplitAxis::Horizontal => {
                    let total_w = available.x;
                    let first_w = ((total_w - SPLITTER_SIZE) * *ratio).max(1.0);
                    let second_w = (total_w - SPLITTER_SIZE - first_w).max(1.0);

                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(first_w, available.y),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                render_split(
                                    ui,
                                    first,
                                    egui::vec2(first_w, available.y),
                                    panes,
                                    focused_pane,
                                    split_enabled,
                                    pane_count,
                                    dragging,
                                    ctx,
                                    action,
                                    pane_rects,
                                    ratio_overrides,
                                );
                            },
                        );

                        if !dragging {
                            let split_id = ui.id().with(("hsplit", ratio.to_bits()));
                            if let Some(new_ratio) = drag_splitter(
                                ui,
                                SplitAxis::Horizontal,
                                *ratio,
                                available,
                                split_id,
                            ) {
                                *ratio = new_ratio;
                            } else {
                                ui.add_space(SPLITTER_SIZE);
                            }
                        } else {
                            ui.add_space(SPLITTER_SIZE);
                        }

                        ui.allocate_ui_with_layout(
                            egui::vec2(second_w, available.y),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                render_split(
                                    ui,
                                    second,
                                    egui::vec2(second_w, available.y),
                                    panes,
                                    focused_pane,
                                    split_enabled,
                                    pane_count,
                                    dragging,
                                    ctx,
                                    action,
                                    pane_rects,
                                    ratio_overrides,
                                );
                            },
                        );
                    });
                }
                SplitAxis::Vertical => {
                    let total_h = available.y;
                    let first_h = ((total_h - SPLITTER_SIZE) * *ratio).max(1.0);
                    let second_h = (total_h - SPLITTER_SIZE - first_h).max(1.0);

                    ui.vertical(|ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(available.x, first_h),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                render_split(
                                    ui,
                                    first,
                                    egui::vec2(available.x, first_h),
                                    panes,
                                    focused_pane,
                                    split_enabled,
                                    pane_count,
                                    dragging,
                                    ctx,
                                    action,
                                    pane_rects,
                                    ratio_overrides,
                                );
                            },
                        );

                        if !dragging {
                            let split_id = ui.id().with(("vsplit", ratio.to_bits()));
                            if let Some(new_ratio) = drag_splitter(
                                ui,
                                SplitAxis::Vertical,
                                *ratio,
                                available,
                                split_id,
                            ) {
                                *ratio = new_ratio;
                            } else {
                                ui.add_space(SPLITTER_SIZE);
                            }
                        } else {
                            ui.add_space(SPLITTER_SIZE);
                        }

                        ui.allocate_ui_with_layout(
                            egui::vec2(available.x, second_h),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                render_split(
                                    ui,
                                    second,
                                    egui::vec2(available.x, second_h),
                                    panes,
                                    focused_pane,
                                    split_enabled,
                                    pane_count,
                                    dragging,
                                    ctx,
                                    action,
                                    pane_rects,
                                    ratio_overrides,
                                );
                            },
                        );
                    });
                }
            }
        }
    }
}

fn split_key_from_children(first: &SplitNode, second: &SplitNode) -> u64 {
    split_key(first) ^ split_key(second).rotate_left(17)
}

fn split_key(node: &SplitNode) -> u64 {
    match node {
        SplitNode::Leaf { pane_id } => pane_id.0,
        SplitNode::Split { first, second, .. } => {
            split_key(first) ^ split_key(second).rotate_left(17)
        }
    }
}

pub fn is_preview_insert_zone(
    zone: crate::ui::shell::layout_state::DropZone,
    drag: &super::drag_drop::ActiveDrag,
) -> bool {
    use super::drag_drop::ActiveDrag;
    use crate::ui::shell::layout_state::DropZone;
    match drag {
        ActiveDrag::Session { .. } => matches!(
            zone,
            DropZone::Pane { .. } | DropZone::Root { .. }
        ),
        ActiveDrag::Pane { pane_id: src, .. } => {
            matches!(zone, DropZone::Pane { pane_id: target, .. } if *src != target)
        }
    }
}
