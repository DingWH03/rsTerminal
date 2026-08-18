//! Session-owned terminal view state.

use std::collections::BTreeMap;
use std::time::Instant;

use super::{CellPos, RowGalleyCache, TerminalSelection, TerminalTouchState};

#[derive(Default)]
pub struct PortViewState {
    pub scroll_offset: usize,
    pub selection: Option<TerminalSelection>,
    pub selection_pointer: Option<CellPos>,
    pub touch_state: TerminalTouchState,
    pub row_galley_cache: RowGalleyCache,
    pub mouse_motion_last: Option<(usize, usize)>,
}

pub struct TerminalViewState {
    pub profile_id: String,
    pub live_font_size: f32,
    pub inactive_port_states: BTreeMap<u8, PortViewState>,
    pub scroll_offset: usize,
    pub selection: Option<TerminalSelection>,
    pub selection_pointer: Option<CellPos>,
    pub touch_state: TerminalTouchState,
    pub want_terminal_focus: bool,
    pub terminal_had_focus: bool,
    pub row_galley_cache: RowGalleyCache,
    pub layout_font_size: f32,
    pub last_pty_rows: u16,
    pub last_pty_cols: u16,
    pub size_label_dims: (usize, usize),
    pub size_label_hide_at: Option<Instant>,
    pub size_label_active: bool,
    pub grid_rows: usize,
    pub grid_cols: usize,
    pub mouse_motion_last: Option<(usize, usize)>,
    pub font_generation: u32,
}
