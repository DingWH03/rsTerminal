//! Terminal selection / touch state owned by a session (not UI drawing).

use egui::Pos2;

use crate::terminal::screen::{cell_display_width, Screen};

/// 终端中的单元格位置（回滚感知的行号和列号）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellPos {
    /// Scrollback-aware line index (see [`Screen::line_at_virtual`]).
    pub line: usize,
    pub col: usize,
}

/// 终端文本选择 — 由锚点和光标两个位置定义。
#[derive(Debug, Clone)]
pub struct TerminalSelection {
    /// 选择起始位置
    pub anchor: CellPos,
    /// 选择结束位置
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

/// 终端触摸交互状态。
#[derive(Debug, Clone, Default)]
pub struct TerminalTouchState {
    /// The current direct touch drag should select text instead of scrolling.
    pub touch_select_mode: bool,
    /// Last single-finger position used for scrollback drag.
    pub scroll_last_pos: Option<Pos2>,
    /// Fractional row accumulator so slow drags still scroll smoothly.
    pub scroll_remainder_rows: f32,
    /// True after the current touch sequence moved enough to count as a scroll.
    pub scrolled_this_touch: bool,
    /// Whether to render selection handles at both ends of the selection.
    pub show_handles: bool,
    /// Position where a long-press started.
    pub long_press_pos: Option<Pos2>,
    /// Open the copy popup on the next frame.
    pub show_touch_popup: bool,
}

/// 提取选择范围内的文本，处理换行和宽字符。
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
    cells: &[crate::terminal::screen::Cell],
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
