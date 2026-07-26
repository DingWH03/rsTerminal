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

use std::time::{Duration, Instant};

use crate::config::{CursorStyle, TerminalTheme};
use crate::connection::ConnectionState;
use crate::fonts;
use crate::session::{drain_connection, ActiveSession, ConnectionViewAction};
use crate::terminal::cursor::paint_cursor;
use crate::terminal::metrics::measure_cell;
use crate::terminal::{DEFAULT_GRID_COLS, DEFAULT_GRID_ROWS};
use crate::ui::function_pane::FunctionPane;
use crate::ui::page::terminal::grid::{apply_resize, drain_after_resize};
use crate::ui::page::terminal::input::{
    allocate_terminal_surface, has_any_keyboard_input, lock_terminal_focus,
    process_keyboard_input, terminal_widget_id,
};
#[cfg(target_os = "android")]
use crate::ui::page::terminal::input::{
    hide_android_terminal_ime, show_android_terminal_ime, update_android_terminal_ime_rect,
};
use crate::ui::page::terminal::mouse::{
    process_terminal_mouse, process_terminal_scrollbar, process_terminal_wheel, process_touch_scroll,
};
use crate::ui::page::terminal::paint::paint_row;
use crate::ui::page::terminal::selection::{
    is_pos_in_selection, paint_selection, paint_selection_handles, paste_payload,
    touch_long_press_selection_from_pos, update_terminal_selection,
};
use crate::ui::uiframe::clipboard::{read_text, write_text};
use crate::ui::uiframe::components::toolbar_button::{
    icon_toolbar_button, icon_toolbar_danger, icon_toolbar_toggle, text_toolbar_button,
};
use crate::ui::uiframe::keyboard::VirtualKeyboard;
use crate::ui::uiframe::vector_icons::Icon;

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
    font_size: &mut f32,
    cell_width_scale: f32,
    function_pane: &mut FunctionPane,
    pane_id: u64,
    is_focused_pane: bool,
    pane_focus_click: &mut bool,
    in_split: bool,
    suppress_terminal_input: bool,
) -> ConnectionViewAction {
    let ctx = ui.ctx().clone();
    let term_widget_id = terminal_widget_id(pane_id);
    let mut action = ConnectionViewAction::None;

    if let Some(session) = session.as_ref() {
        session.handle.repaint.set_context(ctx.clone());
    }

    let mut copy_requested = false;
    let mut pending_input: Vec<Vec<u8>> = Vec::new();
    let mut paste_texts: Vec<String> = Vec::new();
    let mut terminal_menu_action = TerminalMenuAction::default();

    let show_hamburger = !in_split && function_pane.show_content_hamburger();

    // 1. Header bar — ☰ + title + selection-action bar + toolbar
    let show_actions = session
        .as_ref()
        .is_some_and(|s| s.touch_state.show_handles);

    // Hide title when the panel is too narrow to fit it comfortably
    let header_total_w = ui.available_width();
    let show_title = header_total_w > 320.0 && !show_actions;

    ui.horizontal(|ui| {
        ui.style_mut().spacing.button_padding = egui::vec2(2.0, 1.0);
        ui.style_mut().spacing.item_spacing.x = 2.0;

        if show_hamburger {
            if icon_toolbar_button(ui, ui.id().with(("hdr_menu", pane_id)), Icon::Hamburger).clicked()
            {
                function_pane.hamburger_click();
            }
        }

        if show_actions {
            // Selection mode: show Copy / Paste / Cancel instead of the title.
            if let Some(session) = session.as_mut() {
                ui.scope(|ui| {
                    ui.style_mut().spacing.button_padding = egui::vec2(5.0, 1.0);
                    if ui
                        .button(egui::RichText::new(rust_i18n::t!("copy")).size(11.0).strong())
                        .clicked()
                    {
                        copy_selection_to_clipboard(session, &ctx);
                        ctx.request_repaint();
                    }
                    if ui
                        .button(egui::RichText::new(rust_i18n::t!("paste")).size(11.0))
                        .clicked()
                    {
                        if let Some(text) = read_text() {
                            paste_to_session(session, &text, &ctx, &mut action);
                        }
                    }
                    if ui
                        .button(egui::RichText::new(rust_i18n::t!("cancel")).size(11.0))
                        .clicked()
                    {
                        session.touch_state.show_handles = false;
                        session.touch_state.touch_select_mode = false;
                        session.selection = None;
                        session.selection_pointer = None;
                        ctx.request_repaint();
                    }
                });
            }
        } else if show_title {
            let title = session.as_ref().map(|s| s.tab_label()).unwrap_or_default();
            ui.label(
                egui::RichText::new(title)
                    .size(12.0)
                    .strong()
                    .color(ui.visuals().text_color()),
            );
        }

        if let Some(session) = session.as_mut() {
            if session.ports.len() > 1 {
                ui.separator();
                let port_buttons: Vec<(u8, String, bool, usize)> = session
                    .ports
                    .iter()
                    .map(|p| {
                        (
                            p.port,
                            p.name.clone(),
                            p.port == session.active_port,
                            *session.port_unread.get(&p.port).unwrap_or(&0),
                        )
                    })
                    .collect();
                for (port, label, selected, unread) in port_buttons {
                    let text = if unread > 0 && !selected {
                        format!("{label} •")
                    } else {
                        label
                    };
                    if ui
                        .selectable_label(selected, egui::RichText::new(text).size(11.0))
                        .clicked()
                    {
                        session.switch_to_port(port);
                        ctx.request_repaint();
                    }
                }
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.style_mut().spacing.item_spacing.x = 2.0;

            if in_split {
                if icon_toolbar_danger(ui, ui.id().with(("hdr_close", pane_id)), Icon::Close)
                    .on_hover_text(rust_i18n::t!("close_pane"))
                    .clicked()
                {
                    action = ConnectionViewAction::CloseSession;
                }
                if icon_toolbar_button(ui, ui.id().with(("hdr_hide", pane_id)), Icon::Minimize)
                    .on_hover_text(rust_i18n::t!("minimize_pane"))
                    .clicked()
                {
                    action = ConnectionViewAction::MinimizePane;
                }
            } else {
                if icon_toolbar_danger(ui, ui.id().with(("hdr_close", pane_id)), Icon::Close)
                    .on_hover_text(rust_i18n::t!("close_pane"))
                    .clicked()
                {
                    action = ConnectionViewAction::CloseSession;
                }

                let mode_label = match keyboard.mode {
                    crate::ui::uiframe::keyboard::KeyboardMode::Special => "Sp",
                    crate::ui::uiframe::keyboard::KeyboardMode::Full => "Full",
                };
                if text_toolbar_button(ui, ui.id().with(("hdr_kbmode", pane_id)), mode_label)
                    .on_hover_text(rust_i18n::t!("settings_default_keyboard"))
                    .clicked()
                {
                    keyboard.toggle_mode();
                }

                if icon_toolbar_toggle(
                    ui,
                    ui.id().with(("hdr_kb", pane_id)),
                    Icon::Keyboard,
                    keyboard.visible,
                )
                .on_hover_text(rust_i18n::t!("settings_default_keyboard"))
                .clicked()
                {
                    keyboard.toggle();
                    #[cfg(target_os = "android")]
                    if keyboard.visible {
                        keyboard.terminal_ime_enabled = false;
                        hide_android_terminal_ime(ui.ctx());
                    }
                }

                #[cfg(not(target_os = "android"))]
                {
                    if icon_toolbar_button(
                        ui,
                        ui.id().with(("hdr_font_dec", pane_id)),
                        Icon::FontSmaller,
                    )
                    .on_hover_text("A-")
                    .clicked()
                    {
                        *font_size = (*font_size - 1.0).max(8.0);
                    }
                    if icon_toolbar_button(
                        ui,
                        ui.id().with(("hdr_font_inc", pane_id)),
                        Icon::FontLarger,
                    )
                    .on_hover_text("A+")
                    .clicked()
                    {
                        *font_size = (*font_size + 1.0).min(32.0);
                    }
                }
            }
        });
    });
    ui.add(egui::Separator::default().spacing(2.0));

    // 2. Measure and resize terminal
    let available = ui.available_size();
    #[cfg(target_os = "android")]
    let ime_inset = crate::platform::get().bottom_inset_points(ui.ctx());
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

    let (cell_w, cell_h) = measure_cell(ui, *font_size, cell_width_scale);
    let desired_cols = (area_w / cell_w).floor().max(1.0) as usize;
    let desired_rows = (area_h / cell_h).floor().max(1.0) as usize;
    let mut resize_applied = false;

    if let Some(session) = session.as_mut() {
        let font_changed = (session.layout_font_size - *font_size).abs() > f32::EPSILON;
        let in_alt = session.terminal.screen.in_alternate_screen();

        let pty_rows = session.last_pty_rows as usize;
        let pty_cols = session.last_pty_cols as usize;
        let size_changed = desired_rows != session.grid_rows
            || desired_cols != session.grid_cols
            || desired_rows != pty_rows
            || desired_cols != pty_cols
            || font_changed;

        if size_changed {
            apply_resize(session, desired_rows, desired_cols, *font_size, in_alt);
            drain_after_resize(session, &mut action, in_alt, drain_connection);
            ctx.request_repaint();
            resize_applied = true;
        }
    }

    let grid_cols = session.as_ref().map(|s| s.grid_cols).unwrap_or(DEFAULT_GRID_COLS);
    let grid_rows = session.as_ref().map(|s| s.grid_rows).unwrap_or(DEFAULT_GRID_ROWS);

    // 3. Process connection data
    if let Some(session) = session.as_mut() {
        while drain_connection(session, &mut action) {}
    }

    // 3b. Connection status / error (blocks interaction with the terminal grid)
    if let Some(session) = session.as_mut() {
        if let Some(msg) = session.disconnect_message.clone() {
            let mut close = false;
            let lost = matches!(session.handle.state, ConnectionState::Lost(_));
            let title: String = if lost {
                "Disconnected".to_string()
            } else {
                rust_i18n::t!("connection_failed").into_owned()
            };
            let mut reconnect = false;
            let can_reconnect = session.saved_conn_id.is_some();
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(20, 20, 20, 240))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(area_w, area_h));
                    ui.vertical_centered(|ui| {
                        ui.add_space(area_h * 0.25);
                        ui.label(
                            egui::RichText::new(title)
                                .size(18.0)
                                .strong()
                                .color(egui::Color32::from_rgb(255, 120, 120)),
                        );
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(msg).size(14.0));
                        ui.add_space(16.0);
                        if can_reconnect {
                            if ui
                                .button(rust_i18n::t!("reconnect"))
                                .clicked()
                            {
                                reconnect = true;
                            }
                            ui.add_space(8.0);
                        }
                        if ui.button(rust_i18n::t!("close")).clicked() {
                            close = true;
                        }
                    });
                });
            if reconnect {
                if let Some(ref id) = session.saved_conn_id {
                    action = ConnectionViewAction::Reconnect(id.clone());
                }
            }
            if close {
                action = ConnectionViewAction::CloseSession;
            }
            return action;
        }
        if matches!(session.handle.state, ConnectionState::Connecting) {
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(20, 20, 20, 200))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(area_w, area_h));
                    ui.vertical_centered(|ui| {
                        ui.add_space(area_h * 0.35);
                        ui.label(egui::RichText::new("Connecting…").size(16.0).weak());
                    });
                });
            return action;
        }
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
    if apply_touch_pinch_zoom(&ctx, font_size) {
        if let Some(session) = session.as_mut() {
            session.size_label_active = true;
            session.size_label_hide_at = None;
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
        && session.as_ref().is_some_and(|s| s.want_terminal_focus)
    {
        ui.ctx()
            .memory_mut(|mem| mem.request_focus(term_widget_id));
    }
    // Reclaim focus if navigation stole it (only the focused pane's terminal).
    if !suppress_terminal_input
        && is_focused_pane
        && session.as_ref().is_some_and(|s| s.terminal_had_focus)
        && !term_resp.has_focus()
    {
        term_resp.request_focus();
    }
    let term_focused = !suppress_terminal_input
        && is_focused_pane
        && (term_resp.has_focus() || session.as_ref().is_some_and(|s| s.terminal_had_focus));

    // Touch long-press behaviour (works on any device with a touch screen):
    //
    //   First long-press on empty text  → select the word under the finger,
    //                                      enter selection mode, show handles.
    //   Second long-press on a word that is already selected → open the copy
    //                                      popup (like a native mobile context menu).
    let has_touch = ui.input(|i| i.has_touch_screen());
    if has_touch && term_resp.long_touched() {
        if let (Some(session), Some(pos)) = (session.as_mut(), term_resp.interact_pointer_pos()) {
            let inside_selection = session.selection.as_ref().is_some_and(|sel| {
                is_pos_in_selection(
                    pos,
                    sel,
                    &session.terminal.screen,
                    session.scroll_offset,
                    grid_rect,
                    cell_w,
                    cell_h,
                    grid_rows,
                    grid_cols,
                )
            });

            if inside_selection {
                // Long-press on already-selected text → show copy popup.
                session.touch_state.show_touch_popup = true;
                ctx.request_repaint();
            } else {
                // First long-press → select a word and show handles.
                if let Some(sel) = touch_long_press_selection_from_pos(
                    pos,
                    &session.terminal.screen,
                    session.scroll_offset,
                    grid_rect,
                    cell_w,
                    cell_h,
                    grid_rows,
                    grid_cols,
                ) {
                    session.selection_pointer = Some(sel.anchor);
                    session.selection = Some(sel);
                    session.touch_state.touch_select_mode = true;
                    session.touch_state.show_handles = true;
                    session.touch_state.scroll_last_pos = None;
                    session.touch_state.scroll_remainder_rows = 0.0;
                    session.touch_state.scrolled_this_touch = false;
                    #[cfg(target_os = "android")]
                    {
                        keyboard.terminal_ime_enabled = false;
                        hide_android_terminal_ime(ui.ctx());
                    }
                    ctx.request_repaint();
                }
            }
        }
    }

    // On touch devices: a short tap (not long-press) outside the current selection
    // clears selection and hides the floating handles.
    if has_touch && term_resp.clicked() && !term_resp.long_touched() {
        if let (Some(session), Some(pos)) = (session.as_mut(), term_resp.interact_pointer_pos()) {
            let inside = session.selection.as_ref().is_some_and(|sel| {
                is_pos_in_selection(
                    pos,
                    sel,
                    &session.terminal.screen,
                    session.scroll_offset,
                    grid_rect,
                    cell_w,
                    cell_h,
                    grid_rows,
                    grid_cols,
                )
            });
            if !inside {
                session.selection = None;
                session.selection_pointer = None;
                session.touch_state.show_handles = false;
                session.touch_state.touch_select_mode = false;
                ctx.request_repaint();
            }
        }
    }

    // When a mouse click happens (non-touch) while touch handles are visible, also
    // clear the touch-selection state so the handles don't persist across input modes.
    if !has_touch && term_resp.clicked() {
        if let Some(session) = session.as_mut() {
            if session.touch_state.show_handles {
                session.touch_state.show_handles = false;
                session.touch_state.touch_select_mode = false;
            }
        }
    }

    let has_selection = session
        .as_ref()
        .and_then(|s| s.selection.as_ref())
        .is_some();
    let app_cursor_keys = session
        .as_ref()
        .map(|s| s.terminal.screen.application_cursor_keys())
        .unwrap_or(false);
    let modifiers = ctx.input(|i| i.modifiers);

    // 自动聚焦：当终端未聚焦但用户开始输入时，自动将焦点还给终端
    // 注意：request_focus 在下一帧生效，但当前帧的事件会被 process_keyboard_input 消费
    let needs_focus =
        !suppress_terminal_input && is_focused_pane && !term_focused && has_any_keyboard_input(&ctx);
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
            if let Some(ref sel) = session.selection {
                let text = sel.text(&session.terminal.screen);
                if !text.is_empty() {
                    write_text(&text);
                    ctx.copy_text(text);
                }
                session.selection = None;
                session.selection_pointer = None;
                session.touch_state.show_handles = false;
                session.touch_state.touch_select_mode = false;
            }
        }
        for text in paste_texts {
            paste_to_session(session, &text, &ctx, &mut action);
        }
        if !pending_input.is_empty() {
            // 用户输入了内容（打字/回车/退格等），自动回到实时尾部
            session.scroll_offset = 0;
            session.size_label_active = false;
            for bytes in pending_input {
                session.send_active(bytes);
            }
        }
    }

    // Right-click on desktop opens a context menu; long-press on selected text on
    // touch devices opens the same popup.
    let touch_popup = session
        .as_mut()
        .is_some_and(|s| std::mem::take(&mut s.touch_state.show_touch_popup));
    install_terminal_context_menu(
        ui,
        &term_resp,
        has_selection,
        touch_popup,
        &mut terminal_menu_action,
    );

    if let Some(session) = session.as_mut() {
        apply_terminal_menu_action(
            session,
            &ctx,
            &mut action,
            terminal_menu_action,
        );
    }

    if let Some(session) = session.as_mut() {
        if session.want_terminal_focus && term_resp.has_focus() {
            session.want_terminal_focus = false;
        }
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
            session.row_galley_cache.clear();
        }
        term_resp.mark_changed();
    }

    if ui.is_rect_visible(panel_rect) {
        let painter = ui.painter_at(panel_rect);
        painter.rect_filled(panel_rect, egui::CornerRadius::ZERO, theme.bg);

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
            let font_gen = fonts::font_generation();
            if session.font_generation != font_gen {
                session.font_generation = font_gen;
                session.row_galley_cache.clear();
            }

            let screen = &session.terminal.screen;
            let in_alt = screen.in_alternate_screen();
            if in_alt {
                // vim/htop: do not scroll the shell scrollback behind the alternate buffer.
                session.scroll_offset = 0;
            }

            let max_scroll_offset = if in_alt {
                0
            } else {
                screen.max_scroll_offset(grid_rows)
            };
            session.scroll_offset = session.scroll_offset.min(max_scroll_offset);
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
                &mut session.scroll_offset,
                &mut session.touch_state,
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
                &mut session.scroll_offset,
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
                    &mut session.mouse_motion_last,
                );
            }
            for bytes in mouse_input {
                session.send_active(bytes);
            }

            let offset = session.scroll_offset;

            let ppp = ui.ctx().pixels_per_point();
            let row_y = |row: usize| -> f32 {
                let y = grid_rect.top() + row as f32 * cell_h;
                (y * ppp).round() / ppp
            };

            let mut paint_screen_row = |row: usize, cells: &[crate::terminal::screen::Cell]| {
                paint_row(
                    &painter,
                    ui,
                    &mut session.row_galley_cache,
                    *font_size,
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
            if let Some(cursor_viewport_row) = screen.cursor_viewport_row(grid_rows, offset) {
                if screen.cursor_visible && screen.cursor_x < grid_cols {
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
            }

            // Selection highlight
            if let Some(ref sel) = session.selection {
                paint_selection(&painter, screen, theme, grid_rect, cell_w, cell_h, offset, sel);
                if session.touch_state.show_handles {
                    paint_selection_handles(
                        &painter,
                        screen,
                        grid_rect,
                        cell_w,
                        cell_h,
                        offset,
                        sel,
                    );
                }
            }

            // Selection from mouse/touch (disabled while mouse reporting unless Shift).
            if !mouse_to_pty {
                let touch_selection_enabled = if has_touch {
                    session.touch_state.touch_select_mode
                } else {
                    true
                };
                // Save the prior selection so we can restore it if a touch tap
                // inside the existing selection would otherwise collapse it.
                let prev_selection = session.selection.clone();
                let finished_touch_selection = update_terminal_selection(
                    &mut session.selection,
                    &mut session.selection_pointer,
                    screen,
                    &mut session.scroll_offset,
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
                if has_touch && finished_touch_selection && !session.touch_state.show_handles {
                    session.touch_state.touch_select_mode = false;
                }
                // If we are in touch selection mode with handles, a short tap
                // inside the existing selection must not replace it with a
                // zero-width (single-cell) selection.  Restore the previous one.
                if has_touch
                    && session.touch_state.show_handles
                    && session
                        .selection
                        .as_ref()
                        .is_some_and(|s| s.anchor == s.cursor)
                {
                    if let Some(prev) = prev_selection {
                        session.selection = Some(prev);
                    }
                }
            }

            if show_size_label {
                let (label_cols, label_rows) = if desired_cols != grid_cols || desired_rows != grid_rows {
                    (desired_cols, desired_rows)
                } else {
                    (grid_cols, grid_rows)
                };
                let dim_label = format!("{label_cols}×{label_rows}");
                let dim_color = egui::Color32::from_rgba_premultiplied(
                    theme.fg.r(),
                    theme.fg.g(),
                    theme.fg.b(),
                    140,
                );
                painter.text(
                    panel_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    dim_label,
                    egui::FontId::monospace(13.0),
                    dim_color,
                );
            }

            // Scrollbar (thumb at bottom when viewing the live tail / offset == 0)
            if process_terminal_scrollbar(
                ui,
                theme,
                panel_rect,
                grid_rect,
                grid_rows,
                max_scroll_offset,
                &mut session.scroll_offset,
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
            session.terminal_had_focus = false;
            session.want_terminal_focus = false;
        } else if is_focused_pane {
            session.terminal_had_focus = term_resp.has_focus();
        } else {
            session.terminal_had_focus = false;
        }
    }
    if !suppress_terminal_input && is_focused_pane && term_resp.has_focus() {
        lock_terminal_focus(ui.ctx(), term_widget_id);
    } else if suppress_terminal_input {
        // Drop any previous terminal focus lock so TextEdit can receive keys.
        ui.ctx().memory_mut(|mem| {
            mem.surrender_focus(term_widget_id);
        });
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

    action
}

/// 判断是否显示网格尺寸叠加层（`cols×rows`）。
///
/// 在网格尺寸变化时显示，稳定后继续显示 1 秒后隐藏。
fn size_label_visible(
    session: &mut ActiveSession,
    cols: usize,
    rows: usize,
    ctx: &egui::Context,
) -> bool {
    let dims = (cols, rows);
    let now = Instant::now();

    if dims != session.size_label_dims {
        session.size_label_dims = dims;
        session.size_label_active = true;
        session.size_label_hide_at = None;
        return true;
    }

    if !session.size_label_active {
        return false;
    }

    if session.size_label_hide_at.is_none() {
        session.size_label_hide_at = Some(now + Duration::from_secs(1));
        ctx.request_repaint_after(Duration::from_secs(1));
    }

    session.size_label_hide_at.is_some_and(|deadline| now < deadline)
}

/// 终端右键菜单操作（复制/粘贴/清除选择）。
#[derive(Default, Clone, Copy)]
struct TerminalMenuAction {
    copy: bool,
    paste: bool,
    clear_selection: bool,
}

/// 安装终端右键上下文菜单（桌面右键 + 触摸长按弹出）。
fn install_terminal_context_menu(
    ui: &egui::Ui,
    resp: &egui::Response,
    has_selection: bool,
    force_popup: bool,
    action: &mut TerminalMenuAction,
) {
    let menu_id = resp.id.with("terminal_ctx_popup");
    let is_touch = ui.input(|i| i.has_touch_screen());

    // Desktop right-click context menu (correctly positioned at cursor).
    // Not registered on touch devices to avoid accidental long-press triggering.
    if !is_touch {
        resp.context_menu(|ui| terminal_context_menu_contents(ui, has_selection, action));
    }

    // Touch long-press on already-selected text.
    let touch_open = force_popup.then_some(egui::SetOpenCommand::Bool(true));
    egui::Popup::from_response(resp)
        .id(menu_id)
        .open_memory(touch_open)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_min_width(150.0);
            terminal_context_menu_contents(ui, has_selection, action);
        });
}

/// 终端上下文菜单内容：复制、粘贴、清除选择。
fn terminal_context_menu_contents(
    ui: &mut egui::Ui,
    has_selection: bool,
    action: &mut TerminalMenuAction,
) {
    if ui
        .add_enabled(has_selection, egui::Button::new(rust_i18n::t!("copy")))
        .clicked()
    {
        action.copy = true;
        ui.close();
    }
    if ui.button(rust_i18n::t!("paste")).clicked() {
        action.paste = true;
        ui.close();
    }
    if ui
        .add_enabled(has_selection, egui::Button::new(rust_i18n::t!("clear_selection")))
        .clicked()
    {
        action.clear_selection = true;
        ui.close();
    }
}

/// 应用终端上下文菜单的操作结果。
fn apply_terminal_menu_action(
    session: &mut ActiveSession,
    ctx: &egui::Context,
    action: &mut ConnectionViewAction,
    menu_action: TerminalMenuAction,
) {
    if menu_action.copy {
        copy_selection_to_clipboard(session, ctx);
    }

    if menu_action.paste {
        if let Some(text) = read_text() {
            paste_to_session(session, &text, ctx, action);
        }
    }

    if menu_action.clear_selection {
        session.selection = None;
        session.selection_pointer = None;
        session.touch_state.show_handles = false;
        session.touch_state.touch_select_mode = false;
    }
}

/// 将当前选择复制到系统剪贴板并清除选择状态。
fn copy_selection_to_clipboard(session: &mut ActiveSession, ctx: &egui::Context) {
    if let Some(ref sel) = session.selection {
        let text = sel.text(&session.terminal.screen);
        if !text.is_empty() {
            write_text(&text);
            ctx.copy_text(text);
        }
    }
    session.selection = None;
    session.selection_pointer = None;
    session.touch_state.show_handles = false;
    session.touch_state.touch_select_mode = false;
}

/// 应用触摸双指缩放手势来调整终端字体大小。
fn apply_touch_pinch_zoom(ctx: &egui::Context, font_size: &mut f32) -> bool {
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

// toolbar_button 已迁移到 crate::ui::uiframe::components::toolbar_button

/// 向 PTY 粘贴文本。
///
/// 在 shell 提示符下使用原始字节（立即回显）；仅在 alt-screen 应用中启用括号粘贴模式。
pub fn paste_to_session(
    session: &mut ActiveSession,
    text: &str,
    ctx: &egui::Context,
    action: &mut ConnectionViewAction,
) {
    let bracketed =
        session.terminal.screen.bracketed_paste_enabled() && session.terminal.screen.in_alternate_screen();
    session.send_active(paste_payload(text, bracketed));
    let _ = drain_connection(session, action);
    ctx.request_repaint();
}

