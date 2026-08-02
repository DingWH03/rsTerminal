//! Single workspace pane — session content or empty state.

use std::collections::HashMap;

use crate::session::{ConnectionViewAction, WorkspaceSession};
use crate::ui::layout::{PaneId, PaneState};
use crate::ui::page::file_manager::file_manager_view;
use crate::ui::page::home::recent::{SplitPaneChrome, recent_connections_view};
use crate::ui::page::terminal::connection_view;
use crate::ui::pane_colors::pane_color;
use crate::ui::shell::layout_preview::PREVIEW_GHOST_PANE;
use crate::ui::shell::messages::{EmptyPaneConnect, WorkspaceAction};
use crate::ui::uiframe::style;

use super::WorkspacePaneContext;

pub fn render_pane(
    ui: &mut egui::Ui,
    pane_id: PaneId,
    panes: &HashMap<PaneId, PaneState>,
    focused_pane: PaneId,
    _split_enabled: bool,
    _pane_count: usize,
    ctx: &mut WorkspacePaneContext<'_>,
    action: &mut WorkspaceAction,
    pane_rects: &mut HashMap<PaneId, egui::Rect>,
) {
    let outer_rect = ui.available_rect_before_wrap();

    if pane_id == PREVIEW_GHOST_PANE {
        paint_ghost_pane(ui);
        pane_rects.insert(pane_id, outer_rect);
        return;
    }

    let is_focused = focused_pane == pane_id;
    let session_id = panes.get(&pane_id).and_then(|p| p.session_id.clone());
    let accent = panes
        .get(&pane_id)
        .map(|p| pane_color(ctx.prefs, p.color_index))
        .unwrap_or(style::ACCENT);
    let fade = ctx.session_fade.get(&pane_id).copied().unwrap_or(1.0);
    let in_split = ctx.split_layout_active;

    // Single-pane: flush to workspace edges. Split: no per-pane inset either —
    // inter-pane gap comes solely from SPLITTER_SIZE / PANE_GAP.
    let frame = egui::Frame::NONE;

    frame.show(ui, |ui| {
        ui.set_min_size(ui.available_size());

        let content_h = ui.available_height().max(1.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), content_h),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                if fade < 0.999 {
                    ui.set_opacity(fade);
                }
                if ui
                    .interact(
                        ui.max_rect(),
                        ui.id().with(("pane_body", pane_id.0)),
                        egui::Sense::click(),
                    )
                    .clicked()
                {
                    action.focus_pane = Some(pane_id);
                }

                if let Some(ref sid) = session_id {
                    if let Some(idx) = ctx.sessions.iter().position(|s| s.id() == sid) {
                        match &mut ctx.sessions[idx] {
                            WorkspaceSession::Terminal(term) => {
                                let profile_id = term.view.profile_id.clone();
                                let (theme, cursor_style, cell_width_scale) = {
                                    let profile = crate::data::persist::types::resolve_profile(
                                        ctx.profiles,
                                        Some(profile_id.as_str()),
                                    );
                                    (
                                        profile.theme.clone(),
                                        profile.cursor_style,
                                        profile.cell_width_scale,
                                    )
                                };
                                let mut pane_focus_click = false;
                                let view_action = connection_view(
                                    ui,
                                    Some(term),
                                    ctx.virtual_keyboard,
                                    &theme,
                                    cursor_style,
                                    cell_width_scale,
                                    ctx.function_pane,
                                    pane_id.0,
                                    is_focused,
                                    &mut pane_focus_click,
                                    in_split,
                                    ctx.suppress_terminal_input,
                                );
                                if pane_focus_click {
                                    action.focus_pane = Some(pane_id);
                                }
                                match view_action {
                                    ConnectionViewAction::None => {}
                                    ConnectionViewAction::MinimizePane => {
                                        action.minimize_pane = Some(pane_id);
                                    }
                                    other => {
                                        action.terminal = other;
                                        action.terminal_pane = Some(pane_id);
                                    }
                                }
                            }
                            WorkspaceSession::FileManager(fm) => {
                                let fm_action =
                                    file_manager_view(ui, fm, ctx.function_pane, in_split);
                                if fm_action.close {
                                    action.file_manager = fm_action;
                                    action.terminal_pane = Some(pane_id);
                                }
                            }
                        }
                        paint_pane_border(ui, in_split, is_focused, accent);
                        return;
                    }
                }

                let mut connect = None;
                let mut more = false;
                let mut close_pane = false;
                let mut hide_pane = false;
                recent_connections_view(
                    ui,
                    ctx.function_pane,
                    ctx.saved_connections,
                    &mut connect,
                    &mut more,
                    in_split,
                    in_split.then_some(SplitPaneChrome {
                        hide_pane: Some(&mut hide_pane),
                        close_pane: Some(&mut close_pane),
                    }),
                );
                if let Some(id) = connect {
                    action.connect_from_empty = Some(EmptyPaneConnect {
                        pane: pane_id,
                        connection_id: id,
                    });
                }
                if more {
                    action.open_connections_from_empty = Some(pane_id);
                }
                if close_pane {
                    action.close_pane_session = Some(pane_id);
                }
                if hide_pane {
                    action.minimize_pane = Some(pane_id);
                }

                paint_pane_border(ui, in_split, is_focused, accent);
            },
        );
    });

    pane_rects.insert(pane_id, outer_rect);
}

fn paint_pane_border(ui: &mut egui::Ui, in_split: bool, is_focused: bool, accent: egui::Color32) {
    if !in_split {
        return;
    }
    let rect = ui.max_rect();
    let color = if is_focused {
        accent
    } else {
        accent.gamma_multiply(0.55)
    };
    ui.painter().rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(style::PANE_BORDER_WIDTH, color),
        egui::StrokeKind::Inside,
    );
}

fn paint_ghost_pane(ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    let painter = ui.painter_at(rect);
    painter.rect_filled(
        rect,
        style::CORNER_RADIUS_SM,
        style::ACCENT.gamma_multiply(0.12),
    );
    painter.rect_stroke(
        rect,
        style::CORNER_RADIUS_SM,
        egui::Stroke::new(1.5, style::ACCENT.gamma_multiply(0.5)),
        egui::StrokeKind::Inside,
    );
}
