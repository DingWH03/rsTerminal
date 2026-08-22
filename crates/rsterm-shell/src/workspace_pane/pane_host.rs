//! Single workspace pane — session content or empty state.

use std::collections::HashMap;

use crate::PaneChrome;
use crate::layout::{PaneId, PaneState};
use crate::page::home::recent::{SplitPaneChrome, recent_connections_view};
use crate::pane_colors::pane_color;
use crate::shell::layout_preview::PREVIEW_GHOST_PANE;
use crate::shell::messages::{EmptyPaneConnect, WorkspaceAction};
use crate::uiframe::interactive::{self, AccentTone};
use crate::uiframe::style;
use crate::uiframe::tokens;
use rsterm_page_terminal::{ActiveSessionContent, TerminalHostExtras};
use rsterm_session_core::ConnectionViewAction;
use rsterm_workspace::{ContentAction, ContentUiCtx};

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

                if let Some(ref sid) = session_id
                    && let Some(idx) = ctx.sessions.iter().position(|s| s.id() == sid)
                {
                    let show_hamburger = !in_split && ctx.function_pane.show_content_hamburger();
                    let mut hamburger_pending = false;
                    let mut pane_focus_click = false;
                    let mut extras = TerminalHostExtras::new(ctx.profiles, ctx.virtual_keyboard);
                    let mut content_ctx = ContentUiCtx {
                        pane_id: pane_id.0,
                        is_focused,
                        in_split,
                        suppress_terminal_input: ctx.suppress_terminal_input,
                        show_hamburger,
                        hamburger_pending: &mut hamburger_pending,
                        pane_focus_click: &mut pane_focus_click,
                        extras: &mut extras,
                    };
                    let content_action = ctx.sessions[idx].content_mut().ui(ui, &mut content_ctx);
                    if hamburger_pending {
                        ctx.function_pane.hamburger_click();
                    }
                    if pane_focus_click {
                        action.focus_pane = Some(pane_id);
                    }
                    match content_action {
                        ContentAction::None => {}
                        ContentAction::MinimizePane => {
                            action.minimize_pane = Some(pane_id);
                        }
                        ContentAction::Close => {
                            action.terminal = ConnectionViewAction::CloseSession;
                            action.file_manager.close = true;
                            action.terminal_pane = Some(pane_id);
                        }
                    }
                    if let Some(fm) = ctx.sessions[idx]
                        .content_mut()
                        .as_any_mut()
                        .downcast_mut::<rsterm_page_file_manager::FileManagerContent>(
                    ) {
                        if let Some(prefs) = fm.pending_prefs.take() {
                            action.file_manager.prefs = Some(prefs);
                        }
                        if fm.pending_open_settings {
                            fm.pending_open_settings = false;
                            action.file_manager.open_settings = true;
                        }
                    }
                    if let Some(fm) = ctx.sessions[idx]
                        .content_mut()
                        .as_any_mut()
                        .downcast_mut::<rsterm_page_file_manager::FileManagerContent>(
                    ) && let Some(ui_state) = fm.pending_ui_state.take()
                    {
                        action.file_manager.ui_state = Some(ui_state);
                    }
                    if let Some(term) = ctx.sessions[idx]
                        .content_mut()
                        .as_any_mut()
                        .downcast_mut::<ActiveSessionContent>()
                        && let Some(id) = term.pending_reconnect.take()
                    {
                        action.terminal = ConnectionViewAction::Reconnect(id);
                        action.terminal_pane = Some(pane_id);
                    }
                    paint_pane_border(ui, in_split, is_focused, accent);
                    return;
                }

                let mut connect = None;
                let mut more = false;
                let mut close_pane = false;
                let mut hide_pane = false;
                let show_hamburger = !in_split && ctx.function_pane.show_content_hamburger();
                let mut on_hamburger = || ctx.function_pane.hamburger_click();
                let mut chrome = PaneChrome {
                    show_hamburger,
                    on_hamburger: &mut on_hamburger,
                };
                recent_connections_view(
                    ui,
                    &mut chrome,
                    ctx.saved_connections,
                    &mut connect,
                    &mut more,
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
        interactive::accent_tone(accent, AccentTone::Muted)
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
        interactive::accent_tone(style::ACCENT, AccentTone::Faint),
    );
    painter.rect_stroke(
        rect,
        style::CORNER_RADIUS_SM,
        egui::Stroke::new(
            tokens::stroke::EMPHASIS,
            interactive::accent_tone(style::ACCENT, AccentTone::Soft),
        ),
        egui::StrokeKind::Inside,
    );
}
