//! Right workspace pane — multi-pane split layout.

pub mod drag_drop;
pub mod pane_host;
pub mod split_tree;
pub mod split_widget;

use std::collections::HashMap;

use crate::data::persist::types::{SavedConnection, TerminalProfile};
use crate::data::prefs::Prefs;
use crate::session::WorkspaceSession;
use crate::ui::function_pane::FunctionPane;
use crate::ui::layout::{DropZone, PaneId, WorkspaceLayout};
use crate::ui::shell::layout_preview::pane_rects_from_tree;
use crate::ui::shell::messages::WorkspaceAction;
use crate::ui::uiframe::keyboard::VirtualKeyboard;

use drag_drop::{commit_drop, hit_test_drop_zone, paint_drag_overlay};
use split_widget::render_split_tree;

pub struct WorkspacePaneContext<'a> {
    pub sessions: &'a mut [WorkspaceSession],
    pub prefs: &'a mut Prefs,
    pub profiles: &'a [TerminalProfile],
    pub saved_connections: &'a [SavedConnection],
    pub virtual_keyboard: &'a mut VirtualKeyboard,
    pub live_font_size: &'a mut f32,
    pub function_pane: &'a mut FunctionPane,
    pub split_enabled: bool,
    pub active_drag: Option<drag_drop::ActiveDrag>,
    pub ratio_overrides: HashMap<u64, f32>,
    pub session_fade: HashMap<PaneId, f32>,
    pub split_layout_active: bool,
    pub last_drop_zone: &'a mut Option<DropZone>,
    /// Modal dialogs (new connection, settings, …) own the keyboard.
    pub suppress_terminal_input: bool,
}

pub struct WorkspaceRenderResult {
    pub action: WorkspaceAction,
    pub pane_rects: HashMap<PaneId, egui::Rect>,
    pub drag_ended: bool,
    pub current_drop_zone: Option<DropZone>,
}

pub fn render(
    ui: &mut egui::Ui,
    layout: &mut WorkspaceLayout,
    ctx: &mut WorkspacePaneContext<'_>,
) -> WorkspaceRenderResult {
    let workspace_rect = ui.max_rect();
    let pointer = ui.input(|i| i.pointer.interact_pos().unwrap_or(egui::Pos2::ZERO));

    ctx.split_layout_active = layout.pane_count() > 1;

    let mut pane_rects = HashMap::new();
    let render_root = layout.root.clone();
    let mut action = render_split_tree(ui, layout, ctx, &mut pane_rects, &render_root, false);

    if pane_rects.is_empty() {
        pane_rects = pane_rects_from_tree(&layout.root, workspace_rect);
    }

    let zone = ctx
        .active_drag
        .as_ref()
        .and_then(|drag| hit_test_drop_zone(pointer, workspace_rect, &pane_rects, drag));

    let mut drag_ended = false;
    let mut current_drop_zone = None;
    if let Some(ref drag) = ctx.active_drag {
        if let Some(z) = zone {
            *ctx.last_drop_zone = Some(z);
            current_drop_zone = Some(z);
        }

        paint_drag_overlay(ui.ctx(), workspace_rect, &pane_rects, drag, pointer, zone);

        if ui.input(|i| i.pointer.any_released()) {
            drag_ended = true;
            let palette_len = crate::ui::pane_colors::resolve_palette(ctx.prefs)
                .len()
                .max(1);
            if let Some(focused) = commit_drop(layout, drag, zone, *ctx.last_drop_zone, palette_len)
            {
                action.drop_applied = true;
                action.focus_pane = Some(focused);
            }
        } else {
            ui.ctx().request_repaint();
        }
    }

    WorkspaceRenderResult {
        action,
        pane_rects,
        drag_ended,
        current_drop_zone,
    }
}
