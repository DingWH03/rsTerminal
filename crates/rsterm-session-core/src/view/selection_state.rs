//! Terminal selection and touch interaction state.

use egui::Pos2;

use rsterm_terminal::screen::{Screen, cell_display_width};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellPos {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct TerminalSelection {
    pub anchor: CellPos,
    pub cursor: CellPos,
}

impl TerminalSelection {
    pub fn new(anchor: CellPos) -> Self {
        Self {
            anchor,
            cursor: anchor,
        }
    }

    pub fn range(&self) -> (CellPos, CellPos) {
        if self.anchor.line < self.cursor.line
            || (self.anchor.line == self.cursor.line && self.anchor.col <= self.cursor.col)
        {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        }
    }

    pub fn text(&self, screen: &Screen) -> String {
        extract_range_text(screen, self.range())
    }
}

#[derive(Debug, Clone, Default)]
pub struct TerminalTouchState {
    pub touch_select_mode: bool,
    pub scroll_last_pos: Option<Pos2>,
    pub scroll_remainder_rows: f32,
    pub scrolled_this_touch: bool,
    pub show_handles: bool,
    pub long_press_pos: Option<Pos2>,
    pub show_touch_popup: bool,
}

pub fn extract_range_text(screen: &Screen, (start, end): (CellPos, CellPos)) -> String {
    let mut out = String::new();
    for line in start.line..=end.line {
        let Some(cells) = screen.line_at_virtual(line) else {
            continue;
        };
        let cols = screen.cols.min(cells.len());
        let col_start = if line == start.line { start.col } else { 0 };
        let col_end = if line == end.line {
            end.col.min(cols.saturating_sub(1))
        } else {
            cols.saturating_sub(1)
        };
        if line > start.line && !screen.virtual_line_wrapped(line) {
            out.push('\n');
        }
        if col_start <= col_end {
            out.push_str(&line_slice_text(cells, col_start, col_end));
        }
    }
    out
}

fn line_slice_text(
    cells: &[rsterm_terminal::screen::Cell],
    start_col: usize,
    end_col: usize,
) -> String {
    let end_col = end_col.min(cells.len().saturating_sub(1));
    let mut out = String::new();
    let mut col = start_col;
    while col <= end_col {
        if col >= cells.len() {
            break;
        }
        if cells[col].wide_continuation {
            col += 1;
            continue;
        }
        let ch = cells[col].ch;
        if ch != '\0' {
            out.push(ch);
        }
        col += cell_display_width(cells, col).max(1);
    }
    out.trim_end().to_string()
}
