//! 终端网格尺寸同步 — 管理仿真器网格和 PTY 终端尺寸的协调。
//!
//! 确保 egui 布局尺寸变化时，终端仿真器和底层 PTY 的尺寸保持一致。
//! 在 alt-screen 模式下，先调整仿真器再调整 PTY；普通模式下先调整 PTY 再调整仿真器。

use rsterm_session_core::{ActiveSession, ConnectionViewAction};

/// 同步仿真器网格尺寸。如果尺寸或字体未变化则跳过。
pub fn sync_emulator_grid(session: &mut ActiveSession, rows: usize, cols: usize, font_size: f32) {
    let rows = rows.max(1);
    let cols = cols.max(1);
    if session.view.grid_rows == rows
        && session.view.grid_cols == cols
        && (session.view.layout_font_size - font_size).abs() <= f32::EPSILON
    {
        return;
    }
    session.view.grid_rows = rows;
    session.view.grid_cols = cols;
    session.core.terminal.resize(rows, cols);
    for state in session.core.inactive_port_states.values_mut() {
        state.terminal.resize(rows, cols);
    }
    for state in session.view.inactive_port_states.values_mut() {
        state.scroll_offset = 0;
        state.row_galley_cache.clear();
    }
    session.view.layout_font_size = font_size;
    session.view.row_galley_cache.clear();
    session.view.scroll_offset = 0;
}

/// 同步 PTY 终端尺寸。如果尺寸未变化则跳过。
pub fn sync_pty_size(session: &mut ActiveSession, rows: usize, cols: usize) {
    let rows = rows.max(1) as u16;
    let cols = cols.max(1) as u16;
    if session.view.last_pty_rows == rows && session.view.last_pty_cols == cols {
        return;
    }
    session.view.last_pty_rows = rows;
    session.view.last_pty_cols = cols;
    session.core.handle.resize(rows, cols);
}

/// 应用尺寸调整。在 alt-screen 模式下先调仿真器再调 PTY，否则顺序相反。
pub fn apply_resize(
    session: &mut ActiveSession,
    rows: usize,
    cols: usize,
    font_size: f32,
    in_alt: bool,
) {
    if in_alt {
        sync_emulator_grid(session, rows, cols, font_size);
        sync_pty_size(session, rows, cols);
    } else {
        sync_pty_size(session, rows, cols);
        sync_emulator_grid(session, rows, cols, font_size);
    }
}

/// 网格尺寸变化后排空待处理的 PTY 数据。
///
/// 在 alt-screen 模式下，排空后发送 WINCH 信号并再次排空。
pub fn drain_after_resize(
    session: &mut ActiveSession,
    action: &mut ConnectionViewAction,
    in_alt: bool,
    drain: fn(&mut ActiveSession, &mut ConnectionViewAction) -> bool,
) {
    for _ in 0..256 {
        if !drain(session, action) {
            break;
        }
    }
    if in_alt {
        session.core.handle.signal_winch();
        for _ in 0..128 {
            if !drain(session, action) {
                break;
            }
        }
    }
}
