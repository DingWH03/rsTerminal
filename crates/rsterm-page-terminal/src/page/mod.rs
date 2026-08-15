//! 终端仿真页面 — 核心 UI 模块。
//!
//! 该模块实现了完整的终端仿真器界面，包括：
//! - 终端网格渲染（`paint`）
//! - 键盘输入路由（`input`）
//! - 鼠标事件处理（`mouse`）
//! - 文本选择（`selection`）
//! - 网格大小同步（`grid`）
//!
//! 运行时状态见 [`crate::session`]；本模块负责 `connection_view` 渲染入口。

pub mod grid;
pub mod input;
pub mod mouse;
pub mod paint;
pub mod selection;

mod context_menu;
mod header;
mod overlay;
mod status_overlay;
mod touch;

use crate::page::grid::{apply_resize, drain_after_resize};
use crate::page::input::{
    allocate_terminal_surface, has_any_keyboard_input, lock_terminal_focus, process_keyboard_input,
    terminal_widget_id,
};
#[cfg(target_os = "android")]
use crate::page::input::{show_android_terminal_ime, update_android_terminal_ime_rect};
use crate::page::mouse::{
    process_terminal_mouse, process_terminal_scrollbar, process_terminal_wheel,
    process_touch_scroll,
};
use crate::page::paint::paint_row;
use crate::page::selection::{paint_selection, paint_selection_handles, update_terminal_selection};
use crate::paint_helpers::measure_cell;
use crate::paint_helpers::paint_cursor;
use crate::theme_color::to_egui;
use rsterm_config::{CursorStyle, TerminalTheme};
use rsterm_session_core::{ActiveSession, ConnectionViewAction, drain_connection};
use rsterm_terminal::{DEFAULT_GRID_COLS, DEFAULT_GRID_ROWS};
use rsterm_uiframe::PaneChrome;
use rsterm_uiframe::keyboard::VirtualKeyboard;

use self::context_menu::{
    TerminalMenuAction, apply as apply_terminal_menu_action,
    install as install_terminal_context_menu,
};
use self::overlay::{paint_size_label, size_label_visible};

/// 终端连接视图的主渲染函数。
///
/// 处理完整的终端 UI 渲染流程：
/// 1. 标题栏（汉堡菜单、端口切换、工具栏按钮）
/// 2. 网格尺寸测量和 PTY 大小同步
/// 3. 连接数据排空和处理
/// 4. 连接状态/错误显示
/// 5. 终端表面（键盘焦点、触摸长按选择、右键菜单）
/// 6. 终端网格绘制（行渲染、光标、选择高亮、滚动条）
/// 7. 虚拟键盘
pub fn connection_view(
    ui: &mut egui::Ui,
    mut session: Option<&mut ActiveSession>,
    keyboard: &mut VirtualKeyboard,
    theme: &TerminalTheme,
    cursor_style: CursorStyle,
    cell_width_scale: f32,
    chrome: &mut PaneChrome<'_>,
    pane_id: u64,
    is_focused_pane: bool,
    pane_focus_click: &mut bool,
    in_split: bool,
    suppress_terminal_input: bool,
) -> ConnectionViewAction {
    let ctx = ui.ctx().clone();
    let term_widget_id = terminal_widget_id(pane_id);
    let mut action = ConnectionViewAction::None;
    let mut font_size = session
        .as_ref()
        .map(|s| s.view.live_font_size)
        .unwrap_or(14.0);

    if let Some(session) = session.as_ref() {
        let wake_ctx = ctx.clone();
        session
            .core
            .handle
            .repaint
            .set_wake(move || wake_ctx.request_repaint());
    }

    let mut copy_requested = false;
    let mut pending_input: Vec<Vec<u8>> = Vec::new();
    let mut paste_texts: Vec<String> = Vec::new();
    let mut terminal_menu_action = TerminalMenuAction::default();

    // 1. Header bar — ☰ + title + selection-action bar + toolbar
    header::render(
        ui,
        &mut session,
        keyboard,
        chrome,
        pane_id,
        in_split,
        &ctx,
        &mut action,
    );

    // 2. Measure and resize terminal
    let available = ui.available_size();
    #[cfg(target_os = "android")]
    let ime_inset = rsterm_platform::get().bottom_inset_points(ui.ctx());
    #[cfg(not(target_os = "android"))]
    let ime_inset = 0.0;
    let kb_enabled = !in_split;
    let kb_total = if kb_enabled {
        keyboard.reserved_height(available.x)
    } else {
        0.0
    };
    let area_w = available.x.max(1.0);
    let area_h = (available.y - kb_total - ime_inset).max(1.0);

    let (cell_w, cell_h) = measure_cell(ui, font_size, cell_width_scale);
    let desired_cols = (area_w / cell_w).floor().max(1.0) as usize;
    let desired_rows = (area_h / cell_h).floor().max(1.0) as usize;
    let mut resize_applied = false;

    if let Some(session) = session.as_mut() {
        let font_changed = (session.view.layout_font_size - font_size).abs() > f32::EPSILON;
        let in_alt = session.core.terminal.screen.in_alternate_screen();

        let pty_rows = session.view.last_pty_rows as usize;
        let pty_cols = session.view.last_pty_cols as usize;
        let size_changed = desired_rows != session.view.grid_rows
            || desired_cols != session.view.grid_cols
            || desired_rows != pty_rows
            || desired_cols != pty_cols
            || font_changed;

        if size_changed {
            apply_resize(session, desired_rows, desired_cols, font_size, in_alt);
            drain_after_resize(session, &mut action, in_alt, drain_connection);
            ctx.request_repaint();
            resize_applied = true;
        }
    }

    let grid_cols = session
        .as_ref()
        .map(|s| s.view.grid_cols)
        .unwrap_or(DEFAULT_GRID_COLS);
    let grid_rows = session
        .as_ref()
        .map(|s| s.view.grid_rows)
        .unwrap_or(DEFAULT_GRID_ROWS);

    // 3. Process connection data
    if let Some(session) = session.as_mut() {
        while drain_connection(session, &mut action) {}
    }

    // 3b. Connection status / error (blocks interaction with the terminal grid)
    if let Some(session) = session.as_mut()
        && let Some(status_action) = status_overlay::render(ui, session, egui::vec2(area_w, area_h))
    {
        if !matches!(status_action, ConnectionViewAction::None) {
            action = status_action;
        }
        session.view.live_font_size = font_size;
        return action;
    }

    // 4. Terminal surface (keyboard focus target; stable id for focus-lock filter)
    let grid_size = egui::vec2(grid_cols as f32 * cell_w, grid_rows as f32 * cell_h);
    let (panel_rect, grid_rect, mut term_resp) = allocate_terminal_surface(
        ui,
        egui::vec2(area_w, area_h),
        grid_size,
        egui::Sense::click_and_drag() | egui::Sense::FOCUSABLE,
        term_widget_id,
    );
    if resize_applied {
        term_resp.mark_changed();
    }
    term_resp = term_resp.on_hover_cursor(egui::CursorIcon::Text);
    if touch::apply_pinch_zoom(&ctx, &mut font_size) {
        if let Some(session) = session.as_mut() {
            session.view.size_label_active = true;
            session.view.size_label_hide_at = None;
        }
        ctx.request_repaint();
    }
    if term_resp.clicked() && !term_resp.long_touched() && !suppress_terminal_input {
        term_resp.request_focus();
        if !is_focused_pane {
            *pane_focus_click = true;
        }
        #[cfg(target_os = "android")]
        {
            keyboard.terminal_ime_enabled = true;
            show_android_terminal_ime(ui.ctx(), grid_rect);
        }
    }
    if !suppress_terminal_input
        && is_focused_pane
        && session.as_ref().is_some_and(|s| s.view.want_terminal_focus)
    {
        ui.ctx().memory_mut(|mem| mem.request_focus(term_widget_id));
    }
    // Reclaim focus if navigation stole it (only the focused pane's terminal).
    if !suppress_terminal_input
        && is_focused_pane
        && session.as_ref().is_some_and(|s| s.view.terminal_had_focus)
        && !term_resp.has_focus()
    {
        term_resp.request_focus();
    }
    let term_focused = !suppress_terminal_input
        && is_focused_pane
        && (term_resp.has_focus() || session.as_ref().is_some_and(|s| s.view.terminal_had_focus));

    let has_touch = touch::handle_selection(
        ui,
        &ctx,
        &term_resp,
        &mut session,
        keyboard,
        grid_rect,
        cell_w,
        cell_h,
        grid_rows,
        grid_cols,
    );

    let has_selection = session
        .as_ref()
        .and_then(|s| s.view.selection.as_ref())
        .is_some();
    let app_cursor_keys = session
        .as_ref()
        .map(|s| s.core.terminal.screen.application_cursor_keys())
        .unwrap_or(false);
    let modifiers = ctx.input(|i| i.modifiers);

    // 自动聚焦：当终端未聚焦但用户开始输入时，自动将焦点还给终端
    // 注意：request_focus 在下一帧生效，但当前帧的事件会被 process_keyboard_input 消费
    let needs_focus = !suppress_terminal_input
        && is_focused_pane
        && !term_focused
        && has_any_keyboard_input(&ctx);
    if needs_focus {
        term_resp.request_focus();
        #[cfg(target_os = "android")]
        {
            keyboard.terminal_ime_enabled = true;
            show_android_terminal_ime(ui.ctx(), grid_rect);
        }
    }

    process_keyboard_input(
        &ctx,
        term_widget_id,
        // 如果本帧需要聚焦，假装终端已聚焦以消费事件
        term_focused || needs_focus,
        has_selection,
        modifiers,
        keyboard.ctrl_active(),
        app_cursor_keys,
        &mut copy_requested,
        &mut pending_input,
        &mut paste_texts,
    );

    if let Some(session) = session.as_mut() {
        if copy_requested {
            context_menu::copy_selection_to_clipboard(session, &ctx);
        }
        for text in paste_texts {
            paste_to_session(session, &text, &ctx, &mut action);
        }
        if !pending_input.is_empty() {
            // 用户输入了内容（打字/回车/退格等），自动回到实时尾部
            session.view.scroll_offset = 0;
            session.view.size_label_active = false;
            for bytes in pending_input {
                session.send_active(bytes);
            }
        }
    }

    // Right-click on desktop opens a context menu; long-press on selected text on
    // touch devices opens the same popup.
    let touch_popup = session
        .as_mut()
        .is_some_and(|s| std::mem::take(&mut s.view.touch_state.show_touch_popup));
    install_terminal_context_menu(
        ui,
        &term_resp,
        has_selection,
        touch_popup,
        &mut terminal_menu_action,
    );

    if let Some(session) = session.as_mut() {
        apply_terminal_menu_action(session, &ctx, &mut action, terminal_menu_action);
    }

    if let Some(session) = session.as_mut()
        && session.view.want_terminal_focus
        && term_resp.has_focus()
    {
        session.view.want_terminal_focus = false;
    }

    // Drain all pending PTY chunks before painting (avoids half-colored history frames).
    let mut terminal_dirty = false;
    if let Some(session) = session.as_mut() {
        while drain_connection(session, &mut action) {
            terminal_dirty = true;
        }
    }
    if terminal_dirty {
        if let Some(session) = session.as_mut() {
            session.view.row_galley_cache.clear();
        }
        term_resp.mark_changed();
    }

    if ui.is_rect_visible(panel_rect) {
        let painter = ui.painter_at(panel_rect);
        painter.rect_filled(panel_rect, egui::CornerRadius::ZERO, to_egui(theme.bg));

        let show_size_label = session
            .as_mut()
            .map(|s| {
                let label_cols = if desired_cols != grid_cols {
                    desired_cols
                } else {
                    grid_cols
                };
                let label_rows = if desired_rows != grid_rows {
                    desired_rows
                } else {
                    grid_rows
                };
                size_label_visible(s, label_cols, label_rows, &ctx)
            })
            .unwrap_or(false);

        if let Some(session) = session.as_mut() {
            let font_gen = crate::fonts::font_generation();
            if session.view.font_generation != font_gen {
                session.view.font_generation = font_gen;
                session.view.row_galley_cache.clear();
            }

            let screen = &session.core.terminal.screen;
            let in_alt = screen.in_alternate_screen();
            if in_alt {
                // vim/htop: do not scroll the shell scrollback behind the alternate buffer.
                session.view.scroll_offset = 0;
            }

            let max_scroll_offset = if in_alt {
                0
            } else {
                screen.max_scroll_offset(grid_rows)
            };
            session.view.scroll_offset = session.view.scroll_offset.min(max_scroll_offset);
            let mouse_to_pty = screen.mouse_tracking_active() && !modifiers.shift;
            if process_touch_scroll(
                ui,
                &term_resp,
                panel_rect,
                grid_rect,
                cell_h,
                screen,
                in_alt,
                max_scroll_offset,
                &mut session.view.scroll_offset,
                &mut session.view.touch_state,
            ) {
                ctx.request_repaint();
            }
            let mut wheel_input: Vec<Vec<u8>> = Vec::new();
            process_terminal_wheel(
                &term_resp,
                grid_rect,
                cell_w,
                cell_h,
                grid_rows,
                grid_cols,
                screen,
                in_alt,
                max_scroll_offset,
                &mut session.view.scroll_offset,
                &mut wheel_input,
            );
            for bytes in wheel_input {
                session.send_active(bytes);
            }

            let mut mouse_input: Vec<Vec<u8>> = Vec::new();
            if mouse_to_pty {
                process_terminal_mouse(
                    ui,
                    &term_resp,
                    grid_rect,
                    cell_w,
                    cell_h,
                    grid_rows,
                    grid_cols,
                    screen,
                    &mut mouse_input,
                    &mut session.view.mouse_motion_last,
                );
            }
            for bytes in mouse_input {
                session.send_active(bytes);
            }

            let offset = session.view.scroll_offset;

            let ppp = ui.ctx().pixels_per_point();
            let row_y = |row: usize| -> f32 {
                let y = grid_rect.top() + row as f32 * cell_h;
                (y * ppp).round() / ppp
            };

            let mut paint_screen_row = |row: usize, cells: &[rsterm_terminal::screen::Cell]| {
                paint_row(
                    &painter,
                    ui,
                    &mut session.view.row_galley_cache,
                    font_size,
                    theme,
                    cells,
                    grid_cols,
                    grid_rect.left(),
                    row_y(row),
                    cell_w,
                    cell_h,
                    in_alt,
                );
            };

            let virtual_start = if in_alt {
                0
            } else {
                screen.viewport_virtual_start(grid_rows, offset)
            };

            for row in 0..grid_rows {
                let virtual_line = if in_alt { row } else { virtual_start + row };
                if let Some(cells) = screen.line_at_virtual(virtual_line) {
                    paint_screen_row(row, cells);
                }
            }

            // Cursor is painted only on the live tail.  Its screen row may differ
            // from screen.cursor_y when the live viewport pulls scrollback rows into
            // view above a shorter active grid after resize growth.
            if let Some(cursor_viewport_row) = screen.cursor_viewport_row(grid_rows, offset)
                && screen.cursor_visible
                && screen.cursor_x < grid_cols
            {
                // Schedule repaint for cursor blink.
                ctx.request_repaint_after(std::time::Duration::from_millis(530));
                paint_cursor(
                    &painter,
                    screen,
                    theme,
                    grid_rect,
                    cell_w,
                    cell_h,
                    cursor_style,
                    Some(std::time::Instant::now()),
                    Some(cursor_viewport_row),
                );
            }

            // Selection highlight
            if let Some(ref sel) = session.view.selection {
                paint_selection(
                    &painter, screen, theme, grid_rect, cell_w, cell_h, offset, sel,
                );
                if session.view.touch_state.show_handles {
                    paint_selection_handles(
                        &painter, screen, grid_rect, cell_w, cell_h, offset, sel,
                    );
                }
            }

            // Selection from mouse/touch (disabled while mouse reporting unless Shift).
            if !mouse_to_pty {
                let touch_selection_enabled = if has_touch {
                    session.view.touch_state.touch_select_mode
                } else {
                    true
                };
                // Save the prior selection so we can restore it if a touch tap
                // inside the existing selection would otherwise collapse it.
                let prev_selection = session.view.selection.clone();
                let finished_touch_selection = update_terminal_selection(
                    &mut session.view.selection,
                    &mut session.view.selection_pointer,
                    screen,
                    &mut session.view.scroll_offset,
                    max_scroll_offset,
                    &ctx,
                    ui,
                    &term_resp,
                    grid_rect,
                    cell_w,
                    cell_h,
                    grid_rows,
                    grid_cols,
                    touch_selection_enabled,
                );
                // Keep touch_select_mode active after the initial long-press so
                // the user can drag to adjust the selection.  Only cleared when
                // tapping outside the selection or explicitly copying / clearing.
                if has_touch && finished_touch_selection && !session.view.touch_state.show_handles {
                    session.view.touch_state.touch_select_mode = false;
                }
                // If we are in touch selection mode with handles, a short tap
                // inside the existing selection must not replace it with a
                // zero-width (single-cell) selection.  Restore the previous one.
                if has_touch
                    && session.view.touch_state.show_handles
                    && session
                        .view
                        .selection
                        .as_ref()
                        .is_some_and(|s| s.anchor == s.cursor)
                    && let Some(prev) = prev_selection
                {
                    session.view.selection = Some(prev);
                }
            }

            if show_size_label {
                let (label_cols, label_rows) =
                    if desired_cols != grid_cols || desired_rows != grid_rows {
                        (desired_cols, desired_rows)
                    } else {
                        (grid_cols, grid_rows)
                    };
                paint_size_label(&painter, panel_rect, theme, label_cols, label_rows);
            }

            // Scrollbar (thumb at bottom when viewing the live tail / offset == 0)
            if process_terminal_scrollbar(
                ui,
                theme,
                panel_rect,
                grid_rect,
                grid_rows,
                max_scroll_offset,
                &mut session.view.scroll_offset,
            ) {
                ctx.request_repaint();
            }
        }
    }

    // 6. Virtual keyboard — fixed-height bottom strip so rows are not pushed/clipped
    if kb_enabled && keyboard.visible {
        ui.separator();
        let kb_h = keyboard.content_height(ui.available_width());
        let kbd_output = ui
            .allocate_ui_with_layout(
                egui::vec2(ui.available_width(), kb_h),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| keyboard.show(ui),
            )
            .inner;
        if let Some(session) = session.as_mut() {
            let mut sent = false;
            for data in &kbd_output {
                if !data.is_empty() {
                    session.send_active(data.clone());
                    sent = true;
                }
            }
            let _ = sent;
        }
    }

    if let Some(session) = session.as_mut() {
        if suppress_terminal_input {
            session.view.terminal_had_focus = false;
            session.view.want_terminal_focus = false;
        } else if is_focused_pane {
            session.view.terminal_had_focus = term_resp.has_focus();
        } else {
            session.view.terminal_had_focus = false;
        }
    }
    if !suppress_terminal_input && is_focused_pane && term_resp.has_focus() {
        lock_terminal_focus(ui.ctx(), term_widget_id);
    } else if suppress_terminal_input {
        // Drop any previous terminal focus lock so TextEdit can receive keys.
        ui.ctx().memory_mut(|mem| {
            mem.surrender_focus(term_widget_id);
        });
        #[cfg(target_os = "android")]
        {
            if keyboard.terminal_ime_enabled {
                keyboard.terminal_ime_enabled = false;
            }
            // Terminal left IMEPurpose::Terminal set; dialog TextEdits need Normal.
            rsterm_platform::android_ime::release_terminal_ime_for_text_fields(ui.ctx());
        }
    }
    #[cfg(target_os = "android")]
    {
        // Keep only a logical "terminal owns the IME" flag. Android Back can
        // hide the keyboard without notifying egui; do not clear this flag just
        // because the inset disappeared. The next terminal tap will call
        // `show_android_terminal_ime` again and reopen it.
        if keyboard.terminal_ime_enabled && term_focused {
            update_android_terminal_ime_rect(ui.ctx(), grid_rect);
        }
    }

    if let Some(session) = session.as_mut() {
        session.view.live_font_size = font_size;
    }

    action
}

/// 向 PTY 粘贴文本。
///
/// 在 shell 提示符下使用原始字节（立即回显）；仅在 alt-screen 应用中启用括号粘贴模式。
pub fn paste_to_session(
    session: &mut ActiveSession,
    text: &str,
    ctx: &egui::Context,
    action: &mut ConnectionViewAction,
) {
    let paste_action = session.paste_text(text);
    if !matches!(paste_action, ConnectionViewAction::None) {
        *action = paste_action;
    }
    ctx.request_repaint();
}
