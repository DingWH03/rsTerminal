use crate::session::ActiveSession;
use crate::ui::uiframe::keyboard::VirtualKeyboard;

use super::selection::{is_pos_in_selection, touch_long_press_selection_from_pos};

pub(super) fn apply_pinch_zoom(ctx: &egui::Context, font_size: &mut f32) -> bool {
    let zoom_delta = ctx.input(|i| i.zoom_delta());
    if !zoom_delta.is_finite() || (zoom_delta - 1.0).abs() < 0.01 {
        return false;
    }
    let next = (*font_size * zoom_delta).clamp(8.0, 32.0);
    if (next - *font_size).abs() < 0.05 {
        return false;
    }
    *font_size = next;
    true
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_selection(
    ui: &egui::Ui,
    ctx: &egui::Context,
    response: &egui::Response,
    session: &mut Option<&mut ActiveSession>,
    _keyboard: &mut VirtualKeyboard,
    grid_rect: egui::Rect,
    cell_width: f32,
    cell_height: f32,
    grid_rows: usize,
    grid_cols: usize,
) -> bool {
    let has_touch = ui.input(|input| input.has_touch_screen());

    if has_touch
        && response.long_touched()
        && let (Some(session), Some(pos)) = (session.as_mut(), response.interact_pointer_pos())
    {
        let inside_selection = session.view.selection.as_ref().is_some_and(|selection| {
            is_pos_in_selection(
                pos,
                selection,
                &session.core.terminal.screen,
                session.view.scroll_offset,
                grid_rect,
                cell_width,
                cell_height,
                grid_rows,
                grid_cols,
            )
        });

        if inside_selection {
            session.view.touch_state.show_touch_popup = true;
            ctx.request_repaint();
        } else if let Some(selection) = touch_long_press_selection_from_pos(
            pos,
            &session.core.terminal.screen,
            session.view.scroll_offset,
            grid_rect,
            cell_width,
            cell_height,
            grid_rows,
            grid_cols,
        ) {
            session.view.selection_pointer = Some(selection.anchor);
            session.view.selection = Some(selection);
            session.view.touch_state.touch_select_mode = true;
            session.view.touch_state.show_handles = true;
            session.view.touch_state.scroll_last_pos = None;
            session.view.touch_state.scroll_remainder_rows = 0.0;
            session.view.touch_state.scrolled_this_touch = false;
            #[cfg(target_os = "android")]
            {
                _keyboard.terminal_ime_enabled = false;
                super::input::hide_android_terminal_ime(ui.ctx());
            }
            ctx.request_repaint();
        }
    }

    if has_touch
        && response.clicked()
        && !response.long_touched()
        && let (Some(session), Some(pos)) = (session.as_mut(), response.interact_pointer_pos())
    {
        let inside = session.view.selection.as_ref().is_some_and(|selection| {
            is_pos_in_selection(
                pos,
                selection,
                &session.core.terminal.screen,
                session.view.scroll_offset,
                grid_rect,
                cell_width,
                cell_height,
                grid_rows,
                grid_cols,
            )
        });
        if !inside {
            session.view.selection = None;
            session.view.selection_pointer = None;
            session.view.touch_state.show_handles = false;
            session.view.touch_state.touch_select_mode = false;
            ctx.request_repaint();
        }
    }

    if !has_touch
        && response.clicked()
        && let Some(session) = session.as_mut()
        && session.view.touch_state.show_handles
    {
        session.view.touch_state.show_handles = false;
        session.view.touch_state.touch_select_mode = false;
    }

    has_touch
}
