//! 鼠标事件处理 — 将 egui 指针/触摸/滚轮事件映射为 xterm 鼠标报告发送给 PTY。
//!
//! 功能包括：
//! - 回滚滚动条渲染与交互
//! - xterm SGR/传统鼠标编码
//! - 触摸滚动（单指垂直拖动回滚）
//! - 滚轮事件转发（鼠标跟踪模式或 alt-screen 箭头）
//! - 鼠标按钮事件（左/右/中键）转发

use egui::{CornerRadius, CursorIcon, PointerButton, Pos2, Rect, Response, Sense, TouchPhase, Ui};

use crate::page::selection::TerminalTouchState;
use crate::theme_color::to_egui;
use rsterm_config::TerminalTheme;
use rsterm_terminal::screen::Screen;

/// 回滚滚动条的点击目标宽度（位于面板右侧边距中）。
pub const SCROLLBAR_HIT_WIDTH: f32 = 4.0;
/// 滚动条滑块的可见宽度，紧贴面板右边缘。
pub const SCROLLBAR_THUMB_WIDTH: f32 = 2.0;

/// 计算滚动条轨道矩形区域。
pub fn scrollbar_track_rect(panel_rect: Rect, grid_rect: Rect) -> Rect {
    let right = panel_rect.right();
    Rect::from_min_max(
        egui::pos2(right - SCROLLBAR_HIT_WIDTH, grid_rect.top()),
        egui::pos2(right, grid_rect.bottom()),
    )
}

/// 检查指针是否在滚动条区域内。
pub fn pointer_in_scrollbar(pos: Pos2, panel_rect: Rect, grid_rect: Rect) -> bool {
    scrollbar_track_rect(panel_rect, grid_rect).contains(pos)
}

fn scroll_offset_from_pointer_y(
    y: f32,
    grid_rect: Rect,
    grid_rows: usize,
    max_scroll_offset: usize,
) -> usize {
    let total_rows = max_scroll_offset + grid_rows;
    if total_rows == 0 || grid_rect.height() <= 0.0 {
        return 0;
    }
    let rel = ((y - grid_rect.top()) / grid_rect.height()).clamp(0.0, 1.0);
    ((1.0 - rel) * total_rows as f32)
        .round()
        .clamp(0.0, max_scroll_offset as f32) as usize
}

/// 交互式回滚滚动条（滑块在底部 = 实时尾部 / 偏移量为 0）。
pub fn process_terminal_scrollbar(
    ui: &Ui,
    theme: &TerminalTheme,
    panel_rect: Rect,
    grid_rect: Rect,
    grid_rows: usize,
    max_scroll_offset: usize,
    scroll_offset: &mut usize,
) -> bool {
    if max_scroll_offset == 0 || grid_rows == 0 {
        return false;
    }

    let track_rect = scrollbar_track_rect(panel_rect, grid_rect);
    let total_rows = max_scroll_offset + grid_rows;
    let grid_h = grid_rect.height();
    let sb_visible = grid_rows as f32 / total_rows as f32;
    let sb_pos = 1.0 - (*scroll_offset as f32 / total_rows as f32);
    let thumb_h = (grid_h * sb_visible).max(6.0);
    let thumb_y = grid_rect.top() + grid_h * (sb_pos - sb_visible).max(0.0);
    let thumb_rect = Rect::from_min_size(
        egui::pos2(panel_rect.right() - SCROLLBAR_THUMB_WIDTH, thumb_y),
        egui::vec2(SCROLLBAR_THUMB_WIDTH, thumb_h),
    );

    let track_id = ui.id().with("terminal_scrollbar_track");
    let track_resp = ui.interact(track_rect, track_id, Sense::click_and_drag());

    let active = track_resp.hovered() || track_resp.is_pointer_button_down_on();
    if active {
        ui.ctx().set_cursor_icon(if track_resp.dragged() {
            CursorIcon::Grabbing
        } else {
            CursorIcon::Grab
        });
    }

    let mut changed = false;
    if (track_resp.clicked() || track_resp.dragged())
        && let Some(pos) = track_resp.interact_pointer_pos()
    {
        let new_offset =
            scroll_offset_from_pointer_y(pos.y, grid_rect, grid_rows, max_scroll_offset);
        if new_offset != *scroll_offset {
            *scroll_offset = new_offset;
            changed = true;
        }
    }

    let thumb_color = if active {
        to_egui(theme.scrollbar_thumb_hover)
    } else {
        to_egui(theme.scrollbar_thumb)
    };
    ui.painter()
        .rect_filled(thumb_rect, CornerRadius::same(1), thumb_color);

    changed
}

/// 编码 xterm SGR 鼠标报告（`CSI < Cb ; Cx ; Cy M|m`）。
pub fn encode_sgr_mouse(button: u8, col: usize, row: usize, release: bool) -> Vec<u8> {
    let suffix = if release { 'm' } else { 'M' };
    format!("\x1b[<{button};{};{}{}", col + 1, row + 1, suffix).into_bytes()
}

/// 传统 xterm 鼠标编码（`CSI M` + 3 字节）。
pub fn encode_legacy_mouse(button: u8, col: usize, row: usize, release: bool) -> Vec<u8> {
    let mut b = button;
    if release {
        b = b.saturating_add(3);
    }
    let cb = b + 32;
    let cx = (col.saturating_add(1) + 32) as u8;
    let cy = (row.saturating_add(1) + 32) as u8;
    vec![0x1b, b'M', cb, cx, cy]
}

/// 根据终端设置自动选择 SGR 或传统鼠标编码。
pub fn encode_mouse(screen: &Screen, button: u8, col: usize, row: usize, release: bool) -> Vec<u8> {
    if screen.mouse_sgr_encoding() {
        encode_sgr_mouse(button, col, row, release)
    } else {
        encode_legacy_mouse(button, col, row, release)
    }
}

fn button_with_mods(base: u8, shift: bool, alt: bool, ctrl: bool) -> u8 {
    let mut b = base;
    if shift {
        b |= 4;
    }
    if alt {
        b |= 8;
    }
    if ctrl {
        b |= 16;
    }
    b
}

/// Viewport cell (0-based row/col in the visible grid), for xterm mouse coordinates.
fn viewport_cell(
    pos: Pos2,
    grid_rect: Rect,
    cell_w: f32,
    cell_h: f32,
    grid_rows: usize,
    grid_cols: usize,
) -> Option<(usize, usize)> {
    if !grid_rect.contains(pos) || cell_w <= 0.0 || cell_h <= 0.0 {
        return None;
    }
    let rel = pos - grid_rect.min;
    let row = ((rel.y / cell_h).floor() as usize).min(grid_rows.saturating_sub(1));
    let col = ((rel.x / cell_w).floor() as usize).min(grid_cols.saturating_sub(1));
    Some((col, row))
}

/// 当 xterm 鼠标跟踪启用时，将指针事件发送到 PTY。
pub fn process_terminal_mouse(
    ui: &Ui,
    term_resp: &Response,
    grid_rect: Rect,
    cell_w: f32,
    cell_h: f32,
    grid_rows: usize,
    grid_cols: usize,
    screen: &Screen,
    pending_input: &mut Vec<Vec<u8>>,
    motion_last: &mut Option<(usize, usize)>,
) {
    if !screen.mouse_tracking_active() {
        *motion_last = None;
        return;
    }

    let shift = ui.input(|i| i.modifiers.shift);
    let alt = ui.input(|i| i.modifiers.alt);
    let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);

    let emit =
        |pending_input: &mut Vec<Vec<u8>>, button: u8, col: usize, row: usize, release: bool| {
            pending_input.push(encode_mouse(screen, button, col, row, release));
        };

    let mut emit_at =
        |pending_input: &mut Vec<Vec<u8>>, pos: Pos2, button: u8, release: bool, motion: bool| {
            let Some((col, row)) =
                viewport_cell(pos, grid_rect, cell_w, cell_h, grid_rows, grid_cols)
            else {
                return;
            };
            let mut b = button_with_mods(button, shift, alt, ctrl);
            if motion && screen.mouse_report_drag() {
                b |= 32;
            }
            if motion && screen.mouse_report_motion() {
                let key = (col, row);
                if motion_last.as_ref() == Some(&key) {
                    return;
                }
                *motion_last = Some(key);
            } else if !motion {
                *motion_last = Some((col, row));
            }
            emit(pending_input, b, col, row, release);
        };

    if ui.input(|i| i.has_touch_screen()) {
        for event in ui.input(|i| i.events.clone()) {
            let egui::Event::Touch { pos, phase, .. } = event else {
                continue;
            };
            if !grid_rect.contains(pos) {
                continue;
            }
            match phase {
                TouchPhase::Start => emit_at(pending_input, pos, 0, false, false),
                TouchPhase::Move => {
                    if screen.mouse_report_motion() || screen.mouse_report_drag() {
                        emit_at(pending_input, pos, 0, false, true);
                    }
                }
                TouchPhase::End | TouchPhase::Cancel => emit_at(pending_input, pos, 0, true, false),
            }
        }
        return;
    }

    if !term_resp.contains_pointer() {
        return;
    }

    let Some(pos) = term_resp.interact_pointer_pos() else {
        return;
    };

    let primary_pressed = ui.input(|i| i.pointer.primary_pressed()) && term_resp.contains_pointer();
    let primary_released = ui.input(|i| i.pointer.primary_released());
    let secondary_pressed =
        ui.input(|i| i.pointer.secondary_pressed()) && term_resp.contains_pointer();
    let secondary_released = ui.input(|i| i.pointer.secondary_released());
    let middle_pressed = ui
        .input(|i| i.pointer.button_pressed(PointerButton::Middle) && term_resp.contains_pointer());
    let middle_released = ui.input(|i| i.pointer.button_released(PointerButton::Middle));

    if primary_pressed {
        emit_at(pending_input, pos, 0, false, false);
    } else if primary_released {
        emit_at(pending_input, pos, 0, true, false);
    }

    if secondary_pressed {
        emit_at(pending_input, pos, 2, false, false);
    } else if secondary_released {
        emit_at(pending_input, pos, 2, true, false);
    }

    if middle_pressed {
        emit_at(pending_input, pos, 1, false, false);
    } else if middle_released {
        emit_at(pending_input, pos, 1, true, false);
    }

    if term_resp.dragged() && (screen.mouse_report_drag() || screen.mouse_report_motion()) {
        emit_at(pending_input, pos, 0, false, true);
    }
}

/// 触摸屏：单指垂直拖动滚动终端回滚缓冲区。
///
/// 触摸设备上的文本选择由 `touch_select_mode` 控制，
/// 以便普通手指拖动能像移动端滚动视图一样工作，而不是立即选择文本。
pub fn process_touch_scroll(
    ui: &Ui,
    term_resp: &Response,
    panel_rect: Rect,
    grid_rect: Rect,
    cell_h: f32,
    screen: &Screen,
    in_alt: bool,
    max_scroll_offset: usize,
    scroll_offset_mut: &mut usize,
    touch_state: &mut TerminalTouchState,
) -> bool {
    if !ui.input(|i| i.has_touch_screen()) || cell_h <= 0.0 {
        touch_state.scroll_last_pos = None;
        touch_state.scroll_remainder_rows = 0.0;
        touch_state.scrolled_this_touch = false;
        return false;
    }

    // In full-screen TUIs or when xterm mouse reporting is active, keep existing mouse/touch
    // reporting semantics. Shell scrollback should be handled by this mobile-scroll path.
    let zooming = ui.ctx().input(|i| (i.zoom_delta() - 1.0).abs() > 0.01);
    if zooming || in_alt || screen.mouse_tracking_active() || touch_state.touch_select_mode {
        touch_state.scroll_last_pos = None;
        touch_state.scroll_remainder_rows = 0.0;
        touch_state.scrolled_this_touch = false;
        return false;
    }

    let mut did_scroll = false;
    for event in ui.input(|i| i.events.clone()) {
        let egui::Event::Touch { pos, phase, .. } = event else {
            continue;
        };

        match phase {
            TouchPhase::Start => {
                if pointer_in_scrollbar(pos, panel_rect, grid_rect) {
                    continue;
                }
                if grid_rect.contains(pos) || term_resp.rect.contains(pos) {
                    touch_state.scroll_last_pos = Some(pos);
                    touch_state.scroll_remainder_rows = 0.0;
                    touch_state.scrolled_this_touch = false;
                }
            }
            TouchPhase::Move => {
                if pointer_in_scrollbar(pos, panel_rect, grid_rect) {
                    continue;
                }
                let Some(last_pos) = touch_state.scroll_last_pos else {
                    continue;
                };
                if !grid_rect.contains(pos) && !term_resp.rect.contains(pos) {
                    touch_state.scroll_last_pos = Some(pos);
                    continue;
                }

                let delta_y = pos.y - last_pos.y;
                touch_state.scroll_last_pos = Some(pos);
                touch_state.scroll_remainder_rows += delta_y / cell_h;

                let whole_rows = touch_state.scroll_remainder_rows.trunc() as isize;
                if whole_rows == 0 {
                    continue;
                }
                touch_state.scroll_remainder_rows -= whole_rows as f32;

                if max_scroll_offset > 0 {
                    let new_offset = (*scroll_offset_mut as isize + whole_rows)
                        .clamp(0, max_scroll_offset as isize)
                        as usize;
                    if new_offset != *scroll_offset_mut {
                        *scroll_offset_mut = new_offset;
                        did_scroll = true;
                        touch_state.scrolled_this_touch = true;
                    }
                } else {
                    *scroll_offset_mut = 0;
                }
            }
            TouchPhase::End | TouchPhase::Cancel => {
                touch_state.scroll_last_pos = None;
                touch_state.scroll_remainder_rows = 0.0;
            }
        }
    }

    did_scroll
}

/// 滚轮事件处理：鼠标跟踪模式下发送 SGR 按钮 64/65；
/// alt-screen 应用中发送箭头键；否则滚动回滚缓冲区。
pub fn process_terminal_wheel(
    term_resp: &Response,
    grid_rect: Rect,
    cell_w: f32,
    cell_h: f32,
    grid_rows: usize,
    grid_cols: usize,
    screen: &Screen,
    in_alt: bool,
    max_scroll_offset: usize,
    scroll_offset_mut: &mut usize,
    pending_input: &mut Vec<Vec<u8>>,
) {
    let delta = term_resp.ctx.input(|i| {
        let mut delta = 0f32;
        for event in &i.events {
            if let egui::Event::MouseWheel {
                unit,
                delta: d,
                modifiers,
                ..
            } = event
            {
                if !modifiers.is_none() {
                    continue;
                }
                let y = match unit {
                    egui::MouseWheelUnit::Line => d.y,
                    egui::MouseWheelUnit::Point => d.y / 50.0,
                    egui::MouseWheelUnit::Page => d.y * 3.0,
                };
                delta += y;
            }
        }
        delta
    });

    if delta.abs() < f32::EPSILON {
        return;
    }

    let steps = (delta.abs() * 3.0).round().max(1.0) as usize;

    if screen.mouse_tracking_active() {
        let pos = term_resp
            .hover_pos()
            .or(term_resp.interact_pointer_pos())
            .unwrap_or_else(|| grid_rect.center());
        let (col, row) = viewport_cell(pos, grid_rect, cell_w, cell_h, grid_rows, grid_cols)
            .unwrap_or((
                screen.cursor_x.min(grid_cols.saturating_sub(1)),
                screen.cursor_y.min(grid_rows.saturating_sub(1)),
            ));
        // egui: positive delta.y = scroll down (content moves up) → xterm wheel down (65)
        let button = if delta > 0.0 { 65 } else { 64 };
        for _ in 0..steps {
            pending_input.push(encode_mouse(screen, button, col, row, false));
        }
        return;
    }

    if in_alt {
        let seq: &[u8] = if delta > 0.0 { b"\x1b[B" } else { b"\x1b[A" };
        for _ in 0..steps {
            pending_input.push(seq.to_vec());
        }
        return;
    }

    if max_scroll_offset > 0 {
        let change = (delta * 3.0).round() as isize;
        let new_offset =
            (*scroll_offset_mut as isize + change).clamp(0, max_scroll_offset as isize) as usize;
        *scroll_offset_mut = new_offset;
    } else {
        *scroll_offset_mut = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sgr_press_and_release() {
        assert_eq!(encode_sgr_mouse(0, 4, 9, false), b"\x1b[<0;5;10M");
        assert_eq!(encode_sgr_mouse(0, 4, 9, true), b"\x1b[<0;5;10m");
    }

    #[test]
    fn legacy_left_click() {
        assert_eq!(
            encode_legacy_mouse(0, 4, 9, false),
            vec![0x1b, b'M', b' ', b'%', b'*']
        );
    }
}
