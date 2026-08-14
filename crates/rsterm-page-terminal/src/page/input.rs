//! 终端键盘输入路由 — 将键盘事件路由到 PTY，防止 egui 默认导航键劫持终端焦点。
//!
//! 核心功能：
//! - 将 egui 键盘事件转换为 PTY 字节序列
//! - 管理终端焦点锁，防止方向键/Tab/Esc 移走焦点
//! - 处理 Android IME 软键盘的显示/隐藏/位置更新
//! - 支持 Ctrl/Alt/Shift 修饰键和 SS3 应用光标键模式

use egui::{Context, Event, EventFilter, Id, Key, Modifiers, Sense, Ui, Vec2};

use rsterm_uiframe::clipboard::read_text;
use rsterm_uiframe::keyboard::ctrl_byte_for_char;

/// 每个窗格使用独立 ID，避免多分屏时 First/Second use 冲突。
pub fn terminal_widget_id(pane: u64) -> Id {
    Id::new(("rsTerminal_terminal_surface", pane))
}

/// 单窗格布局的默认 ID（兼容旧调用）。
pub fn default_terminal_widget_id() -> Id {
    terminal_widget_id(0)
}

/// 阻止方向键/Tab/Esc 将 egui 焦点移出终端区域的事件过滤器。
pub fn terminal_event_filter() -> EventFilter {
    EventFilter {
        tab: true,
        horizontal_arrows: true,
        vertical_arrows: true,
        escape: true,
    }
}

/// 分配终端面板区域：背景铺满可用区域贴边；字符网格左上对齐（余量在右下，同色填充）。
pub fn allocate_terminal_surface(
    ui: &mut Ui,
    available: Vec2,
    grid_size: Vec2,
    sense: Sense,
    widget_id: Id,
) -> (egui::Rect, egui::Rect, egui::Response) {
    let (_, panel_rect) = ui.allocate_space(available);
    let grid_size = Vec2::new(
        grid_size.x.min(panel_rect.width()),
        grid_size.y.min(panel_rect.height()),
    );
    // Top-left align the cell grid; fractional leftover stays inside panel_rect (same bg).
    let grid_rect = egui::Rect::from_min_size(panel_rect.min, grid_size);
    // Hit-test the full panel so clicks in the leftover strip still focus the terminal.
    let response = ui.interact(panel_rect, widget_id, sense);
    (panel_rect, grid_rect, response)
}

/// 为终端画布打开 Android 软键盘。
///
/// 终端网格不是 egui 的 `TextEdit`，所以 egui/winit 在用户通过 Android 返回键关闭键盘后
/// 不会自动重新打开原生 IME。将所有终端特定的 IME 控制集中在此辅助函数中。
#[cfg(target_os = "android")]
pub fn show_android_terminal_ime(ctx: &Context, ime_area: egui::Rect) {
    use egui::viewport::{IMEPurpose, ViewportCommand};
    ctx.send_viewport_cmd(ViewportCommand::IMERect(ime_area));
    ctx.send_viewport_cmd(ViewportCommand::IMEPurpose(IMEPurpose::Terminal));
    ctx.send_viewport_cmd(ViewportCommand::IMEAllowed(true));
    rsterm_platform::android_ime::show_soft_input();
}

/// 仅更新 IME 光标/目标矩形。不能每帧强制显示键盘，
/// 否则 Android 返回键关闭键盘后会立即重新打开。
#[cfg(target_os = "android")]
pub fn update_android_terminal_ime_rect(ctx: &Context, ime_area: egui::Rect) {
    ctx.send_viewport_cmd(egui::viewport::ViewportCommand::IMERect(ime_area));
}

/// 关闭 Android 软键盘，用于终端选择手柄或 rsTerminal 自有虚拟键盘等 UI 状态。
#[cfg(target_os = "android")]
pub fn hide_android_terminal_ime(ctx: &Context) {
    ctx.send_viewport_cmd(egui::viewport::ViewportCommand::IMEAllowed(false));
    rsterm_platform::android_ime::hide_soft_input();
}

#[cfg(not(target_os = "android"))]
pub fn show_android_terminal_ime(_ctx: &Context, _ime_area: egui::Rect) {}
#[cfg(not(target_os = "android"))]
pub fn update_android_terminal_ime_rect(_ctx: &Context, _ime_area: egui::Rect) {}
#[cfg(not(target_os = "android"))]
pub fn hide_android_terminal_ime(_ctx: &Context) {}

/// 锁定终端焦点，确保方向键/Tab/Esc 事件留在终端而非被 egui 导航劫持。
pub fn lock_terminal_focus(ctx: &Context, widget_id: Id) {
    ctx.memory_mut(|mem| {
        mem.set_focus_lock_filter(widget_id, terminal_event_filter());
    });
}

/// 检查当前帧是否有任何"主动输入"事件（打字、回车、退格等）。
///
/// 用于自动聚焦决策：当用户开始输入时，自动聚焦终端。
/// 注意：排除方向键，因为在回滚历史中方向键用于滚动浏览。
pub fn has_any_keyboard_input(ctx: &Context) -> bool {
    ctx.input(|i| {
        i.events.iter().any(|event| {
            match event {
                // 文本输入 — 用户正在打字
                Event::Text(text) => {
                    !text.is_empty() && !text.chars().all(|c| c.is_ascii_control())
                }
                // IME 提交 — 输入法确认
                Event::Ime(egui::ImeEvent::Commit(text)) => !text.is_empty(),
                // 按键事件 — 排除方向键/功能键/修饰键
                Event::Key {
                    key, pressed: true, ..
                } => {
                    matches!(
                        key,
                        Key::Enter
                            | Key::Backspace
                            | Key::Tab
                            | Key::Space
                            | Key::A
                            | Key::B
                            | Key::C
                            | Key::D
                            | Key::E
                            | Key::F
                            | Key::G
                            | Key::H
                            | Key::I
                            | Key::J
                            | Key::K
                            | Key::L
                            | Key::M
                            | Key::N
                            | Key::O
                            | Key::P
                            | Key::Q
                            | Key::R
                            | Key::S
                            | Key::T
                            | Key::U
                            | Key::V
                            | Key::W
                            | Key::X
                            | Key::Y
                            | Key::Z
                    )
                }
                _ => false,
            }
        })
    })
}

/// 检查当前帧是否有任何会向 PTY 发送数据的按键事件。
///
/// 用于"输入时自动回到实时尾部"的决策。
/// 这比 `has_any_keyboard_input` 范围更广，包含方向键等（因为方向键在
/// 非回滚模式下也会发送 ANSI 转义序列到 PTY）。
pub fn has_terminal_bound_key(ctx: &Context) -> bool {
    ctx.input(|i| {
        i.events.iter().any(|event| {
            match event {
                Event::Text(text) => {
                    !text.is_empty() && !text.chars().all(|c| c.is_ascii_control())
                }
                Event::Ime(egui::ImeEvent::Commit(text)) => !text.is_empty(),
                Event::Key {
                    key, pressed: true, ..
                } => {
                    // 所有会被 key_to_pty 映射为字节序列的键
                    matches!(
                        key,
                        Key::Enter
                            | Key::Backspace
                            | Key::Tab
                            | Key::Escape
                            | Key::Space
                            | Key::A
                            | Key::B
                            | Key::C
                            | Key::D
                            | Key::E
                            | Key::F
                            | Key::G
                            | Key::H
                            | Key::I
                            | Key::J
                            | Key::K
                            | Key::L
                            | Key::M
                            | Key::N
                            | Key::O
                            | Key::P
                            | Key::Q
                            | Key::R
                            | Key::S
                            | Key::T
                            | Key::U
                            | Key::V
                            | Key::W
                            | Key::X
                            | Key::Y
                            | Key::Z
                            | Key::ArrowUp
                            | Key::ArrowDown
                            | Key::ArrowLeft
                            | Key::ArrowRight
                            | Key::Home
                            | Key::End
                            | Key::PageUp
                            | Key::PageDown
                            | Key::Insert
                            | Key::Delete
                            | Key::F1
                            | Key::F2
                            | Key::F3
                            | Key::F4
                            | Key::F5
                            | Key::F6
                            | Key::F7
                            | Key::F8
                            | Key::F9
                            | Key::F10
                            | Key::F11
                            | Key::F12
                    )
                }
                _ => false,
            }
        })
    })
}

/// 将键盘事件路由到 PTY 并从 egui 事件队列中移除，防止焦点/导航吞噬重复按键。
pub fn process_keyboard_input(
    ctx: &Context,
    widget_id: Id,
    term_focused: bool,
    has_selection: bool,
    modifiers: Modifiers,
    virtual_ctrl: bool,
    app_cursor_keys: bool,
    copy_requested: &mut bool,
    pending_input: &mut Vec<Vec<u8>>,
    paste_texts: &mut Vec<String>,
) {
    if !term_focused {
        // 未聚焦时，只检查复制事件（跨窗口复制）
        ctx.input(|i| {
            for event in &i.events {
                if let Event::Copy = event
                    && has_selection
                {
                    *copy_requested = true;
                }
            }
        });
        return;
    }

    lock_terminal_focus(ctx, widget_id);

    ctx.input_mut(|i| {
        i.events.retain(|event| match event {
            Event::Copy => {
                if modifiers.shift && has_selection {
                    *copy_requested = true;
                } else if !modifiers.shift {
                    pending_input.push(vec![0x03]);
                }
                false
            }
            Event::Cut => {
                if !modifiers.shift {
                    pending_input.push(vec![0x18]);
                }
                false
            }
            Event::Paste(text) => {
                if modifiers.shift {
                    paste_texts.push(text.clone());
                } else {
                    pending_input.push(vec![0x16]);
                }
                false
            }
            Event::Text(text) => {
                route_text_to_terminal(
                    text,
                    modifiers,
                    virtual_ctrl,
                    has_selection,
                    copy_requested,
                    pending_input,
                );
                false
            }
            Event::Ime(egui::ImeEvent::Commit(text)) => {
                route_text_to_terminal(
                    text,
                    modifiers,
                    virtual_ctrl,
                    has_selection,
                    copy_requested,
                    pending_input,
                );
                false
            }
            Event::Key {
                key,
                pressed: true,
                modifiers: key_mods,
                ..
            } => {
                if *key == Key::V && key_mods.command && key_mods.shift {
                    if let Some(t) = read_text() {
                        paste_texts.push(t);
                    }
                    false
                } else if let Some(bytes) = key_to_pty(*key, *key_mods, app_cursor_keys) {
                    pending_input.push(bytes);
                    false
                } else {
                    true
                }
            }
            _ => true,
        });
    });
}

/// 将文本事件路由到终端：Ctrl 组合键转换为控制字节，普通文本直接发送。
fn route_text_to_terminal(
    text: &str,
    modifiers: Modifiers,
    virtual_ctrl: bool,
    has_selection: bool,
    copy_requested: &mut bool,
    pending_input: &mut Vec<Vec<u8>>,
) {
    let ctrl = modifiers.ctrl || modifiers.command || virtual_ctrl;
    if ctrl {
        let mut bytes = Vec::new();
        for c in text.chars() {
            if (c == 'c' || c == 'C') && has_selection && !modifiers.shift {
                *copy_requested = true;
                continue;
            }
            if let Some(b) = ctrl_byte_for_char(c) {
                bytes.push(b);
            }
        }
        if !bytes.is_empty() {
            pending_input.push(bytes);
        }
    } else {
        // Forward printable text and control/DEL chars as-is so the PTY sees
        // backspace/newline etc. when an IME sends them as committed text.
        pending_input.push(text.as_bytes().to_vec());
    }
}

/// 将 egui 按键映射为 PTY 字节序列。
///
/// 支持 Ctrl 组合键、SS3 应用光标键模式、功能键和编辑键。
pub fn key_to_pty(key: Key, modifiers: Modifiers, app_cursor_keys: bool) -> Option<Vec<u8>> {
    let ctrl = modifiers.ctrl || modifiers.command;
    let shift = modifiers.shift;
    let alt = modifiers.alt;
    let use_ss3 = app_cursor_keys && !ctrl && !shift && !alt;
    let result = match key {
        Key::Enter => b"\r".to_vec(),
        Key::Backspace => b"\x7f".to_vec(),
        Key::Tab => b"\t".to_vec(),
        Key::Escape => b"\x1b".to_vec(),
        Key::A if ctrl => vec![0x01],
        Key::B if ctrl => vec![0x02],
        Key::C if ctrl => vec![0x03],
        Key::D if ctrl => vec![0x04],
        Key::E if ctrl => vec![0x05],
        Key::F if ctrl => vec![0x06],
        Key::G if ctrl => vec![0x07],
        Key::H if ctrl => vec![0x08],
        Key::I if ctrl => vec![0x09],
        Key::J if ctrl => vec![0x0a],
        Key::K if ctrl => vec![0x0b],
        Key::L if ctrl => vec![0x0c],
        Key::M if ctrl => vec![0x0d],
        Key::N if ctrl => vec![0x0e],
        Key::O if ctrl => vec![0x0f],
        Key::P if ctrl => vec![0x10],
        Key::Q if ctrl => vec![0x11],
        Key::R if ctrl => vec![0x12],
        Key::S if ctrl => vec![0x13],
        Key::T if ctrl => vec![0x14],
        Key::U if ctrl => vec![0x15],
        Key::V if ctrl => vec![0x16],
        Key::W if ctrl => vec![0x17],
        Key::X if ctrl => vec![0x18],
        Key::Y if ctrl => vec![0x19],
        Key::Z if ctrl => vec![0x1a],
        Key::ArrowUp if ctrl => b"\x1b[1;5A".to_vec(),
        Key::ArrowDown if ctrl => b"\x1b[1;5B".to_vec(),
        Key::ArrowLeft if ctrl => b"\x1b[1;5D".to_vec(),
        Key::ArrowRight if ctrl => b"\x1b[1;5C".to_vec(),
        Key::ArrowUp if use_ss3 => b"\x1bOA".to_vec(),
        Key::ArrowDown if use_ss3 => b"\x1bOB".to_vec(),
        Key::ArrowRight if use_ss3 => b"\x1bOC".to_vec(),
        Key::ArrowLeft if use_ss3 => b"\x1bOD".to_vec(),
        Key::ArrowUp => b"\x1b[A".to_vec(),
        Key::ArrowDown => b"\x1b[B".to_vec(),
        Key::ArrowRight => b"\x1b[C".to_vec(),
        Key::ArrowLeft => b"\x1b[D".to_vec(),
        Key::Home if use_ss3 => b"\x1bOH".to_vec(),
        Key::End if use_ss3 => b"\x1bOF".to_vec(),
        Key::Home => b"\x1b[H".to_vec(),
        Key::End => b"\x1b[F".to_vec(),
        Key::PageUp => b"\x1b[5~".to_vec(),
        Key::PageDown => b"\x1b[6~".to_vec(),
        Key::Insert => b"\x1b[2~".to_vec(),
        Key::Delete => b"\x1b[3~".to_vec(),
        Key::F1 => b"\x1bOP".to_vec(),
        Key::F2 => b"\x1bOQ".to_vec(),
        Key::F3 => b"\x1bOR".to_vec(),
        Key::F4 => b"\x1bOS".to_vec(),
        Key::F5 => b"\x1b[15~".to_vec(),
        Key::F6 => b"\x1b[17~".to_vec(),
        Key::F7 => b"\x1b[18~".to_vec(),
        Key::F8 => b"\x1b[19~".to_vec(),
        Key::F9 => b"\x1b[20~".to_vec(),
        Key::F10 => b"\x1b[21~".to_vec(),
        Key::F11 => b"\x1b[23~".to_vec(),
        Key::F12 => b"\x1b[24~".to_vec(),
        _ => return None,
    };
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Modifiers;

    #[test]
    fn app_cursor_keys_use_ss3_arrows() {
        let mods = Modifiers::default();
        assert_eq!(
            key_to_pty(Key::ArrowUp, mods, true),
            Some(b"\x1bOA".to_vec())
        );
        assert_eq!(
            key_to_pty(Key::ArrowUp, mods, false),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(key_to_pty(Key::Home, mods, true), Some(b"\x1bOH".to_vec()));
    }
}
