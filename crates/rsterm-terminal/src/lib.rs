pub mod parser;
pub mod screen;

pub use screen::{Osc133Kind, SemanticShell};

pub const DEFAULT_GRID_ROWS: usize = 24;
pub const DEFAULT_GRID_COLS: usize = 80;

use parser::{Parser, TermEvent};
use screen::Screen;

pub struct Terminal {
    pub screen: Screen,
    pub title: String,
    parser: Parser,
    raw_mode: bool,
}

impl Terminal {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            screen: Screen::new(rows, cols),
            title: String::new(),
            parser: Parser::new(),
            raw_mode: false,
        }
    }

    pub fn set_raw_mode(&mut self, raw: bool) {
        self.raw_mode = raw;
    }

    pub fn write(&mut self, data: &[u8]) {
        if self.raw_mode {
            for &byte in data {
                if byte == b'\n' || byte == b'\r' {
                    self.screen.newline();
                } else if byte == 0x08 {
                    self.screen.backspace();
                } else if byte == 0x09 {
                    self.screen.advance_tabs();
                } else if (0x20..=0x7e).contains(&byte) {
                    self.screen.put_char(byte as char);
                }
            }
        } else {
            self.parser.process(data, &mut self.screen);
        }
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.screen.resize(rows, cols);
    }

    pub fn set_scrollback_limit(&mut self, limit: usize) {
        self.screen.set_scrollback_limit(limit);
    }

    pub fn drain_pending(&mut self) -> Vec<TermEvent> {
        self.screen.drain_outgoing()
    }
}

#[cfg(test)]
mod tests {
    use super::{TermEvent, Terminal};

    fn row_plaintext(term: &Terminal, row: usize) -> String {
        term.screen.cells[row]
            .iter()
            .map(|c| c.ch)
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    #[test]
    fn utf8_chinese_filename_is_preserved() {
        let mut term = Terminal::new(1, 20);
        term.write("文件.txt".as_bytes());
        let row: String = term.screen.cells[0].iter().map(|c| c.ch).collect();
        assert!(
            row.contains('文'),
            "expected Chinese chars in buffer, got {row:?}"
        );
        assert!(row.contains('件'));
    }

    #[test]
    fn wide_char_uses_two_columns() {
        let mut term = Terminal::new(1, 12);
        term.write("文件".as_bytes());
        assert_eq!(term.screen.cells[0][0].ch, '文');
        assert!(term.screen.cells[0][1].wide_continuation);
        assert_eq!(term.screen.cells[0][2].ch, '件');
        assert!(term.screen.cells[0][3].wide_continuation);
        assert_eq!(term.screen.cursor_x, 4);
    }

    #[test]
    fn utf8_prompt_symbol_is_preserved() {
        let mut term = Terminal::new(1, 40);
        term.write(b"hi \xc2\xbb ");
        let chars: String = term.screen.cells[0].iter().map(|c| c.ch).collect();
        assert!(chars.contains('\u{bb}'));
    }

    #[test]
    fn zsh_prompt_and_typed_char_visible() {
        let prompt = b"%                                                                                                                      \r \r\r\x1b[01;32mdwh@dwh-82sk\x1b[00m \x1b[01;34m~/project\x1b[00m \x1b[33m(master) \x1b[00m\x1b[00m\xc2\xbb \x1b[?2004h";
        let typed = b"a\x08\x08\x1b[31ma\x1b[39m";
        let mut term = Terminal::new(24, 120);
        term.write(prompt);
        term.write(typed);
        let cy = term.screen.cursor_y;
        let line = row_plaintext(&term, cy);
        assert!(
            line.contains("dwh"),
            "prompt text missing on cursor row: {line:?}"
        );
        assert!(
            line.contains('a'),
            "typed char missing on cursor row: {line:?}"
        );
    }

    #[test]
    fn prompt_visible_after_full_screen_ls_output() {
        let prompt = b"\r\n%                                                                                                                      \r \r\r\x1b[01;32mdwh@dwh-82sk\x1b[00m \x1b[01;34m~/project\x1b[00m \x1b[33m(master) \x1b[00m\x1b[00m\xc2\xbb \x1b[?2004h";
        let mut body = String::new();
        for i in 0..34 {
            body.push_str(&format!("line{i:03} file.txt\n"));
        }
        let mut term = Terminal::new(35, 100);
        term.write(body.as_bytes());
        term.write(prompt);
        let cy = term.screen.cursor_y;
        let line = row_plaintext(&term, cy);
        assert!(
            line.contains("dwh"),
            "cursor row {cy} should have prompt, got {line:?}"
        );
        // Also check any row has prompt
        let any: bool = term
            .screen
            .cells
            .iter()
            .any(|row| row.iter().any(|c| c.ch == 'd' || c.ch == '@'));
        assert!(any, "no prompt chars anywhere on screen");
    }

    #[test]
    fn backspace_moves_cursor_without_erasing() {
        let mut term = Terminal::new(1, 20);
        term.write(b"abc");
        term.write(&[0x08]);
        assert_eq!(term.screen.cursor_x, 2);
        let row: String = term.screen.cells[0].iter().map(|c| c.ch).collect();
        assert_eq!(row.chars().filter(|c| *c != ' ').collect::<String>(), "abc");
    }

    #[test]
    fn del_erases_cell_before_cursor() {
        let mut term = Terminal::new(1, 20);
        term.write(b"abc");
        term.write(&[0x7f]);
        let row: String = term.screen.cells[0].iter().map(|c| c.ch).collect();
        assert_eq!(row.chars().filter(|c| *c != ' ').collect::<String>(), "ab");
    }

    #[test]
    fn alternate_screen_1049_save_restore() {
        let mut term = Terminal::new(5, 40);
        term.write(b"saved");
        term.write(b"\x1b[?1049h");
        assert!(term.screen.in_alternate_screen());
        assert_eq!(term.screen.cells[0][0].ch, ' ');
        term.write(b"vim\x1b[?1049l");
        assert!(!term.screen.in_alternate_screen());
        let row: String = term.screen.cells[0].iter().map(|c| c.ch).collect();
        assert!(row.contains('s'), "main screen should be restored: {row:?}");
    }

    #[test]
    fn vim_smcup_sequence_paints_on_alternate() {
        let mut term = Terminal::new(24, 80);
        term.write(b"prompt> ");
        // xterm smcup + stack save + clear + home (typical vim/less entry)
        term.write(b"\x1b[?1049h\x1b[22;0;0t\x1b[2J\x1b[H");
        assert!(term.screen.in_alternate_screen());
        term.write(b"~");
        assert_eq!(term.screen.cells[0][0].ch, '~');
        term.write(b"\x1b[?1049l");
        assert!(!term.screen.in_alternate_screen());
        let row: String = term.screen.cells[0].iter().map(|c| c.ch).collect();
        assert!(
            row.contains('p'),
            "main prompt should return after vim: {row:?}"
        );
    }

    #[test]
    fn csi_esc_aborts_incomplete_sequence_before_1049h() {
        let mut term = Terminal::new(24, 80);
        term.write(b"\x1b[3;1\x1b[?1049h\x1b[2J\x1b[HOK");
        assert!(
            term.screen.in_alternate_screen(),
            "1049h must work after aborted CSI"
        );
        assert_eq!(term.screen.cells[0][0].ch, 'O');
    }

    #[test]
    fn dcs_st_terminator_unblocks_alternate_screen() {
        let mut term = Terminal::new(24, 80);
        term.write(b"\x1bP+q436f\x1b\\\x1b[?1049h\x1b[2J\x1b[Hvim");
        assert!(term.screen.in_alternate_screen());
        assert_eq!(term.screen.cells[0][0].ch, 'v');
    }

    #[test]
    fn xtgettcap_co_reply_then_smcup() {
        let mut term = Terminal::new(24, 80);
        term.write(b"\x1bP+q436f\x1b\\");
        let pending = term.drain_pending();
        assert_eq!(pending.len(), 1);
        let TermEvent::Response(bytes) = pending[0].clone() else {
            panic!("expected xtgettcap response");
        };
        assert!(bytes.starts_with(b"\x1bP1+r436f=323536"));
        assert!(bytes.ends_with(b"\x1b\\"));
        term.write(b"\x1b[?1049h\x1b[2J\x1b[H~");
        assert!(term.screen.in_alternate_screen());
        assert_eq!(term.screen.cells[0][0].ch, '~');
    }

    #[test]
    fn csi_8_resize_window_is_ignored() {
        let mut term = Terminal::new(24, 80);
        term.write(b"\x1b[8;30;100t");
        let pending = term.drain_pending();
        assert!(
            pending.is_empty(),
            "CSI 8 must not resize the grid or PTY (window owned by rsTerminal), got {pending:?}"
        );
        assert_eq!(term.screen.rows, 24);
        assert_eq!(term.screen.cols, 80);
    }

    #[test]
    fn decset_mouse_tracking_modes() {
        let mut term = Terminal::new(24, 80);
        term.write(b"\x1b[?1006h\x1b[?1002h");
        assert!(term.screen.mouse_sgr_encoding());
        assert!(term.screen.mouse_report_drag());
        term.write(b"\x1b[?1003h");
        assert!(term.screen.mouse_report_motion());
        assert!(term.screen.mouse_tracking_active());
    }

    #[test]
    fn window_size_report_18t() {
        let mut term = Terminal::new(24, 80);
        term.write(b"\x1b[18t");
        let pending = term.drain_pending();
        assert_eq!(pending.len(), 1);
        let TermEvent::Response(bytes) = pending[0].clone() else {
            panic!("expected window size report");
        };
        assert_eq!(bytes, b"\x1b[8;24;80t");
    }

    #[test]
    fn bare_lf_then_crlf_on_blank_line_is_skipped() {
        let mut term = Terminal::new(4, 40);
        term.write(b"line1\n");
        assert_eq!(term.screen.cursor_y, 1);
        term.write(b"\r\n");
        assert_eq!(
            term.screen.cursor_y, 1,
            "zsh-style \\r\\n on an already blank line must not add another row"
        );
    }

    #[test]
    fn zsh_pre_prompt_sequence_does_not_leave_percent_only_row() {
        let prompt = b"\r\n%                                                                                                                      \r \r\r\x1b[01;32mdwh@dwh-82sk\x1b[00m \x1b[01;34m~\x1b[00m \xc2\xbb ";
        let mut term = Terminal::new(6, 80);
        term.write(b"last-file-line\n");
        term.write(prompt);
        for (y, _) in term.screen.cells.iter().enumerate() {
            let line = row_plaintext(&term, y);
            if line == "%" {
                panic!("row {y} is lone %% after zsh pre-prompt sequence");
            }
        }
        let cy = term.screen.cursor_y;
        let cursor_line = row_plaintext(&term, cy);
        assert!(
            cursor_line.contains("dwh"),
            "prompt should be on cursor row {cy}, got {cursor_line:?}"
        );
    }

    #[test]
    fn zsh_prompt_sp_not_shown_when_output_ends_with_newline() {
        let mut term = Terminal::new(4, 40);
        // ls-style: line of output ending with LF, then zsh prompt (no PROMPT_SP %).
        term.write(b"file.txt\n");
        term.write(b"\rprompt> ");
        let row0: String = term.screen.cells[0].iter().map(|c| c.ch).collect();
        assert!(
            !row0.contains('%'),
            "row0 should be output, not zsh %% marker: {row0:?}"
        );
        let row1: String = term.screen.cells[1].iter().map(|c| c.ch).collect();
        assert!(
            row1.contains("prompt"),
            "prompt should follow on the next line: {row1:?}"
        );
    }

    #[test]
    fn consecutive_newlines_after_output_are_collapsed() {
        let mut term = Terminal::new(3, 80);
        term.write(b"line1");
        term.write(b"\r\n");
        assert_eq!(term.screen.cursor_y, 1);
        term.write(b"\r\n");
        assert_eq!(
            term.screen.cursor_y, 1,
            "second LF right after first should be skipped"
        );
        term.write(b"next");
        assert_eq!(term.screen.cursor_y, 1);
        term.write(b"\r\n");
        assert_eq!(
            term.screen.cursor_y, 2,
            "LF after printed text should apply"
        );
    }

    #[test]
    fn clear_screen_ed2_works() {
        // Simulate `clear` → \x1b[H\x1b[2J → then new prompt
        let mut term = Terminal::new(4, 40);
        term.write(b"some old content that should disappear\n");
        term.write(b"more content on row 2\n");
        assert_ne!(
            term.screen.cells[0][0].ch, ' ',
            "row 0 should have content before clear"
        );

        term.write(b"\x1b[H\x1b[2J");
        // After clear: all cells should be blank spaces
        for row in 0..term.screen.rows {
            let blank = term.screen.cells[row]
                .iter()
                .all(|c| (c.ch == ' ' || c.ch == '\0') && !c.wide_continuation);
            assert!(blank, "row {row} should be blank after clear");
        }
        assert_eq!(term.screen.cursor_x, 0);
        assert_eq!(term.screen.cursor_y, 0);

        // After clear, cursor at (0,0), shell sends \r\n + prompt →
        // The `\r\n` should NOT be suppressed here because it's a real newline
        // that moves the prompt below the cleared area.
        term.write(b"\r\nprompt> ");
        assert_eq!(
            term.screen.cursor_y, 1,
            "after clear + \\r\\n + prompt, cursor should be on row 1, not {}",
            term.screen.cursor_y
        );
        let row1: String = term.screen.cells[1].iter().map(|c| c.ch).collect();
        assert!(
            row1.contains("prompt"),
            "prompt should appear on row 1, got: {row1:?}"
        );
    }

    #[test]
    fn crlf_is_single_newline() {
        let mut term = Terminal::new(3, 10);
        term.write(b"line1\r\nline2");
        assert_eq!(
            term.screen.cursor_y, 1,
            "LF after CR should advance one row"
        );
        assert_eq!(term.screen.cells[0][0].ch, 'l');
        assert_eq!(term.screen.cells[0][4].ch, '1');
        assert_eq!(term.screen.cells[1][0].ch, 'l');
        assert_eq!(term.screen.cells[1][4].ch, '2');
    }

    #[test]
    fn deferred_cr_applies_before_next_char() {
        let mut term = Terminal::new(1, 10);
        term.write(b"abcde\rxy");
        assert_eq!(term.screen.cells[0][0].ch, 'x');
        assert_eq!(term.screen.cells[0][1].ch, 'y');
        assert_eq!(row_plaintext(&term, 0), "xy");
    }

    #[test]
    fn el_to_eol_fills_with_current_background() {
        let mut term = Terminal::new(1, 16);
        term.write(b"\x1b[44mHEADER\x1b[0K");
        for i in 6..12 {
            assert_eq!(
                term.screen.cells[0][i].bg,
                crate::screen::Color::Indexed(4),
                "column {i} should keep blue background after EL"
            );
        }
    }

    #[test]
    fn cr_on_alternate_screen_does_not_clear_row_for_partial_redraw() {
        let mut term = Terminal::new(1, 20);
        term.write(b"\x1b[?1049h");
        term.write(b"PID 1234  %CPU 15.0\r");
        term.write(b"PID ");
        let row: String = term.screen.cells[0].iter().map(|c| c.ch).collect();
        assert!(
            row.contains('1') && row.contains('%'),
            "partial redraw after CR must keep untouched columns, got {row:?}"
        );
    }

    #[test]
    fn cr_overwrite_clears_trailing_for_progress_bar() {
        let mut term = Terminal::new(1, 80);
        term.write(
            b"Get:48 http://mirrors.example.com/debian bookworm/main amd64 linux-image-6.1.0-48-amd64 amd64 6.1.172-1 [70.2 MB]",
        );
        term.write(b"\rProgress: [ 99%] [#####################################.]");
        let row = row_plaintext(&term, 0);
        assert!(
            row.starts_with("Progress:"),
            "progress should start the line, got {row:?}"
        );
        assert!(
            !row.contains("Get:48") && !row.contains("linux-image"),
            "trailing download text must be cleared, got {row:?}"
        );
    }

    #[test]
    fn zsh_syntax_highlight_patch_after_cr_cub_preserves_line() {
        // zsh-syntax-highlighting: `\r` to column 0, CUF to a token, recolor in place.
        let mut term = Terminal::new(1, 80);
        term.write(b"import base64, io, subprocess, json");
        term.write(b"\r\x1b[7C\x1b[1;33mbase64\x1b[0m");
        let row = row_plaintext(&term, 0);
        assert!(
            row.contains("import") && row.contains("base64") && row.contains("subprocess"),
            "patch highlight must not erase rest of line, got {row:?}"
        );
    }

    #[test]
    fn zsh_backspace_redraw_preserves_char_under_cursor() {
        let mut term = Terminal::new(1, 40);
        term.write(b"hello");
        assert_eq!(term.screen.cursor_x, 5);
        // zsh moves left with BS and re-highlights characters
        term.write(&[0x08, 0x08]);
        assert_eq!(term.screen.cursor_x, 3);
        assert_eq!(term.screen.cells[0][3].ch, 'l');
    }

    #[test]
    fn braille_graph_chars_are_stored_in_cells() {
        let mut term = Terminal::new(1, 8);
        term.write("⣿⢀⡀".as_bytes());
        assert_eq!(term.screen.cells[0][0].ch, '⣿');
        assert_eq!(term.screen.cells[0][1].ch, '⢀');
        assert_eq!(term.screen.cells[0][2].ch, '⡀');
    }

    #[test]
    fn alternate_screen_cub_moves_buffer_columns() {
        let mut term = Terminal::new(10, 60);
        term.write(b"\x1b[?1049h\x1b[2J");
        term.write(b"\x1b[2;2H");
        term.write(b"0123456789012345678901234567890123456789012345");
        assert_eq!(term.screen.cursor_x, 47);
        term.write(b"\x1b[1B");
        assert_eq!(term.screen.cursor_y, 2);
        assert_eq!(term.screen.cursor_x, 47);
        term.write(b"\x1b[47D");
        assert_eq!(
            term.screen.cursor_x, 0,
            "btop-style CUB must use buffer columns in alternate screen"
        );
    }

    #[test]
    fn alternate_screen_preserves_content_on_resize() {
        let mut term = Terminal::new(4, 20);
        term.write(b"\x1b[?1049h\x1b[2J\x1b[H");
        term.write(b"ROW0-OLD-CONTENT-HERE");
        term.screen.resize(6, 30);
        let row0: String = term.screen.cells[0].iter().map(|c| c.ch).collect();
        assert!(
            row0.contains("ROW0-OLD-CONTENT"),
            "alternate buffer should preserve existing content on resize until app redraws, got {row0:?}"
        );
        // After the app redraws (e.g. via SIGWINCH), old data is overwritten.
        term.write(b"\x1b[2J\x1b[H");
        term.write(b"ROW0-FRESH-CONTENT");
        let refreshed: String = term.screen.cells[0].iter().map(|c| c.ch).collect();
        assert!(
            refreshed.contains("ROW0-FRESH"),
            "after alt-screen app redraws, old content must be replaced, got {refreshed:?}"
        );
    }

    #[test]
    fn alternate_screen_keeps_decsctbm_after_resize() {
        let mut term = Terminal::new(24, 80);
        term.write(b"\x1b[?1049h\x1b[2J\x1b[H\x1b[2;24r\x1b[1;1HBAT");
        term.screen.resize(30, 100);
        term.write(b"\x1b[1;1HBAT");
        let top: String = term.screen.cells[0]
            .iter()
            .filter(|c| c.ch != ' ')
            .map(|c| c.ch)
            .collect();
        assert!(
            top.starts_with("BAT"),
            "header row must stay on line 0 after SIGWINCH, got {top:?}"
        );
    }

    #[test]
    fn alternate_screen_cup_1_1_targets_top_row_with_decom_and_scroll_region() {
        let rows = 24;
        let mut term = Terminal::new(rows, 80);
        term.write(b"\x1b[?1049h\x1b[2J\x1b[H");
        term.write(b"\x1b[?6h");
        term.write(format!("\x1b[2;{rows}r").as_bytes());
        term.write(b"\x1b[1;1H");
        term.write(b"cpu menu preset");
        let top: String = term.screen.cells[0]
            .iter()
            .filter(|c| c.ch != ' ')
            .map(|c| c.ch)
            .collect();
        assert!(
            top.starts_with("cpu"),
            "status bar must paint on screen row 0 (Konsole/btop), got row0={top:?}"
        );
        let second: String = term.screen.cells[1]
            .iter()
            .filter(|c| c.ch != ' ')
            .map(|c| c.ch)
            .collect();
        assert!(
            !second.starts_with("cpu"),
            "status bar must not be shifted to row 1, got row1={second:?}"
        );
    }

    #[test]
    fn alternate_screen_uses_full_height_after_decsctbm() {
        let mut term = Terminal::new(5, 10);
        term.write(b"\x1b[2;4r");
        term.write(b"\x1b[?1049h\x1b[5;1HZ");
        assert_eq!(
            term.screen.cells[4][0].ch, 'Z',
            "CUP to last row must work on a full-height alternate buffer"
        );
    }

    #[test]
    fn ss3_cursor_left_and_right() {
        let mut term = Terminal::new(1, 10);
        term.write(b"abcde");
        assert_eq!(term.screen.cursor_x, 5);
        term.write(b"\x1bOD");
        assert_eq!(term.screen.cursor_x, 4);
        term.write(b"\x1bOC");
        assert_eq!(term.screen.cursor_x, 5);
    }

    #[test]
    fn scrollback_virtual_start_maps_history_to_viewport() {
        let mut term = Terminal::new(3, 8);
        term.write(b"AAA\nBBB\nCCC\nDDD\n");
        let sb = term.screen.scrollback_lines();
        assert!(sb >= 1, "expected scrollback after overflow");

        // offset=1: top viewport row is the newest scrollback line
        assert_eq!(
            term.screen.scrollback_row(sb.saturating_sub(1)).unwrap()[0].ch,
            term.screen.scrollback_row(sb - 1).unwrap()[0].ch
        );

        // offset=sb: top viewport row is the oldest scrollback line
        assert_eq!(term.screen.scrollback_row(0).unwrap()[0].ch, 'A');
    }

    #[test]
    fn indexed_color_cell_is_not_default_fg() {
        use crate::screen::Color;
        assert_ne!(Color::Indexed(244), Color::Default);
    }

    #[test]
    fn gray_suggest_chinese_does_not_shift_left() {
        let mut term = Terminal::new(1, 60);
        term.write(b"\x1b[01;32mdwh@dwh-82sk\x1b[00m \x1b[01;34m~\x1b[00m \xc2\xbb ");
        term.write(b"vim ");
        let bb_col = term.screen.cells[0]
            .iter()
            .position(|c| c.ch == '\u{bb}')
            .expect("»");
        term.write(
            b"\x1b[38;5;244m \xe8\x87\xaa\xe5\x8a\xa8\xe4\xbf\x9d\xe5\xad\x98\\ .xmi\x1b[39m",
        );
        assert_eq!(
            term.screen.cells[0][bb_col].ch,
            '\u{bb}',
            "row: {:?}",
            term.screen.cells[0]
                .iter()
                .take(40)
                .map(|c| c.ch)
                .collect::<String>()
        );
    }

    #[test]
    fn gray_suggest_one_space_does_not_shift_left() {
        let mut term = Terminal::new(1, 40);
        term.write(b"\x1b[01;34m~\x1b[00m \xc2\xbb ");
        term.write(b"vim ");
        let bb_col = term.screen.cells[0]
            .iter()
            .position(|c| c.ch == '\u{bb}')
            .expect("» on line");
        let v_col = term.screen.cells[0]
            .iter()
            .position(|c| c.ch == 'v')
            .expect("v on line");
        let cursor_before = term.screen.cursor_x;
        term.write(b"\x1b[38;5;244m");
        assert_eq!(
            term.screen.cursor_x, cursor_before,
            "SGR must not move cursor"
        );
        term.write(b" ");
        assert_eq!(
            term.screen.cells[0][bb_col].ch, '\u{bb}',
            "» must remain after gray space"
        );
        assert_eq!(term.screen.cells[0][v_col].ch, 'v');
        assert_eq!(
            term.screen.cursor_x, cursor_before,
            "POSTDISPLAY leading space does not advance cursor"
        );
    }

    #[test]
    fn zsh_autosuggest_redraw_does_not_corrupt_prompt() {
        let prompt = b"\x1b[01;32mdwh@dwh-82sk\x1b[00m \x1b[01;34m~\x1b[00m \xc2\xbb ";
        let suggest_text =
            b"\x1b[38;5;244m \xe8\x87\xaa\xe5\x8a\xa8\xe4\xbf\x9d\xe5\xad\x98\\ .xmi\x1b[39m";
        let redraw = b"\x1b[14D\x08\x08\x08\x08\x1b[32mv\x1b[32mi\x1b[32mm\x1b[39m\x1b[1C";
        let mut term = Terminal::new(1, 100);
        term.write(prompt);
        term.write(b"vim ");
        let bb_col = term.screen.cells[0]
            .iter()
            .position(|c| c.ch == '\u{bb}')
            .expect("»");
        let v_col = term.screen.cells[0]
            .iter()
            .position(|c| c.ch == 'v')
            .expect("v");
        term.write(suggest_text);
        assert_eq!(term.screen.cells[0][bb_col].ch, '\u{bb}');
        let after_suggest = term.screen.cursor_x;
        assert_eq!(after_suggest, v_col + 4 + 14, "cursor after POSTDISPLAY");
        term.write(redraw);
        let row: String = term.screen.cells[0].iter().map(|c| c.ch).collect();
        assert!(
            term.screen.cells[0][bb_col].ch == '\u{bb}',
            "» corrupted after redraw: {row:?}"
        );
        assert!(!row.contains("~ m"), "stray m in prompt region: {row:?}");
        assert_eq!(term.screen.cells[0][v_col].ch, 'v');
        assert_eq!(term.screen.cursor_x, v_col + 4, "cursor after 'vim '");
    }

    #[test]
    fn postdisplay_leading_space_cursor_matches_zsh_cub() {
        let mut term = Terminal::new(1, 100);
        term.write(b"\x1b[01;32mdwh@dwh-82sk\x1b[00m \x1b[01;34m~\x1b[00m \xc2\xbb ");
        term.write(b"vim ");
        let start = term.screen.cursor_x;
        term.write(
            b"\x1b[38;5;244m \xe8\x87\xaa\xe5\x8a\xa8\xe4\xbf\x9d\xe5\xad\x98\\ .xmi\x1b[39m",
        );
        assert_eq!(
            term.screen.cursor_x,
            start + 14,
            "cursor must advance 14 cols (zsh CUB count), not 15"
        );
    }

    #[test]
    fn vim_space_then_gray_autosuggest() {
        use crate::screen::Color;

        let prompt = b"\x1b[01;32mprompt\x1b[00m \xc2\xbb ";
        let mut term = Terminal::new(1, 60);
        term.write(prompt);
        let start = term.screen.cursor_x;
        term.write(b"vim ");
        assert_eq!(term.screen.cursor_x, start + 4);
        // zsh: clear suffix, print gray suggestion, restore cursor
        term.write(b"\x1b[K\x1b[s\x1b[38;5;244m run\x1b[0m\x1b[u");
        assert_eq!(term.screen.cursor_x, start + 4);
        let row = &term.screen.cells[0];
        assert_eq!(row[start].ch, 'v');
        assert_eq!(row[start + 1].ch, 'i');
        assert_eq!(row[start + 2].ch, 'm');
        assert_eq!(row[start + 3].ch, ' ');
        assert_eq!(row[start + 4].ch, 'r');
        assert_eq!(row[start + 4].fg, Color::Indexed(244));
        assert_eq!(row[start + 5].ch, 'u');
        assert_eq!(row[start + 5].fg, Color::Indexed(244));
    }

    #[test]
    fn scosc_scorc_for_inline_suggestion() {
        use crate::screen::Color;

        let mut term = Terminal::new(1, 30);
        term.write(b"ab");
        assert_eq!(term.screen.cursor_x, 2);
        term.write(b"\x1b[s\x1b[38;5;244mzzz\x1b[0m\x1b[u");
        assert_eq!(term.screen.cursor_x, 2);
        let row = &term.screen.cells[0];
        assert_eq!(row[2].ch, 'z');
        assert_eq!(row[2].fg, Color::Indexed(244));
    }

    #[test]
    fn btop_like_status_bar_on_first_row() {
        // Simulate btop's initialization sequence and UI drawing
        let mut term = Terminal::new(24, 80);

        // Step 1: Enter alternate screen (btop uses CSI ?1049h)
        term.write(b"\x1b[?1049h");
        assert!(term.screen.in_alternate_screen(), "should be in alt screen");

        // Step 2: btop typical initialization
        term.write(b"\x1b[22;0;0t"); // save window title (ignored)
        term.write(b"\x1b[?1l"); // reset cursor keys
        term.write(b"\x1b(B"); // set G0 to US ASCII
        term.write(b"\x1b[m"); // reset attributes
        term.write(b"\x1b[?7h"); // set auto-wrap
        term.write(b"\x1b[?12l"); // reset blink cursor
        term.write(b"\x1b[?25l"); // hide cursor
        term.write(b"\x1b[?1000l"); // disable mouse
        term.write(b"\x1b[?1002l"); // disable mouse drag
        term.write(b"\x1b[?1006l"); // disable SGR mouse

        // Step 3: Clear screen and home cursor
        term.write(b"\x1b[2J"); // clear screen
        term.write(b"\x1b[H"); // home cursor

        // Step 4: Draw first row (status bar with box drawing and battery)
        // btop typically draws with colors, but we test basic positioning
        term.write(b"\x1b[1;1H"); // CUP to row 1, col 1
        term.write(b"\x1b[44m"); // set blue background
        term.write(b"\x1b[37m"); // set white foreground
        term.write(b"\xe2\x94\x8c"); // ┌ (U+250C)
        term.write(b"\xe2\x94\x80"); // ─ (U+2500)
        term.write(b" BAT 100% ");
        term.write(b"\xe2\x94\x80"); // ─
        term.write(b"\xe2\x94\x90"); // ┐ (U+2510)
        term.write(b"\x1b[m"); // reset

        // Step 5: Draw second row (CPU bar)
        term.write(b"\x1b[2;1H"); // CUP to row 2, col 1
        term.write(b"CPU \xe2\x96\x88\xe2\x96\x88\xe2\x96\x88 50%"); // CPU ███ 50%

        // Verify first row content
        let row0: String = term.screen.cells[0].iter().map(|c| c.ch).collect();
        let row0_trimmed = row0.trim_end().to_string();
        assert!(
            row0_trimmed.contains('\u{250c}'),
            "row 0 should start with ┌, got row0={row0:?}"
        );
        assert!(
            row0_trimmed.contains("BAT"),
            "row 0 should contain BAT, got row0={row0:?}"
        );
        assert!(
            row0_trimmed.contains('\u{2510}'),
            "row 0 should end with ┐, got row0={row0:?}"
        );

        // Verify second row content
        let row1: String = term.screen.cells[1].iter().map(|c| c.ch).collect();
        let row1_trimmed = row1.trim_end().to_string();
        assert!(
            row1_trimmed.contains("CPU"),
            "row 1 should contain CPU, got row1={row1:?}"
        );
        assert!(
            row1_trimmed.contains("50%"),
            "row 1 should contain 50%, got row1={row1:?}"
        );
    }

    #[test]
    fn btop_first_row_survives_resize() {
        // Simulate the exact flow: create terminal, resize, run btop-like output
        let mut term = Terminal::new(24, 80);

        // Simulate first frame resize (24x80 -> 35x100)
        term.resize(35, 100);
        assert_eq!(term.screen.rows, 35);
        assert_eq!(term.screen.cols, 100);

        // Now run btop
        term.write(b"\x1b[?1049h"); // Enter alt screen
        assert!(term.screen.in_alternate_screen());
        assert_eq!(term.screen.rows, 35);
        assert_eq!(term.screen.cols, 100);

        term.write(b"\x1b[?25l"); // Hide cursor
        term.write(b"\x1b[0m"); // Reset
        term.write(b"\x1b[38;2;200;200;200m"); // Light gray fg
        term.write(b"\x1b[48;2;0;0;0m"); // Black bg

        // Draw top border row with ─ and corners
        term.write(b"\x1b[1;1f"); // Position (1,1)
        term.write(b"\xe2\x95\xad"); // ╭ (U+256D)
        for _ in 0..30 {
            term.write(b"\xe2\x94\x80"); // ─
        }
        term.write(b"\xe2\x94\xac"); // ┬
        for _ in 0..60 {
            term.write(b"\xe2\x94\x80"); // ─
        }
        term.write(b"\xe2\x95\xae"); // ╮ (U+256E)

        // Draw left box side and text on row 1
        term.write(b"\x1b[2;1f"); // Position (2,1)
        term.write(b"\xe2\x94\x82 \xe2\x96\x88\xe2\x96\x88 50% \xe2\x94\x82");

        // Now resize while in alt screen (simulating window resize)
        term.screen.resize(40, 120);

        // After resize, old alt-screen content is preserved (not cleared) until
        // the app redraws after SIGWINCH.
        let row0: String = term.screen.cells[0].iter().map(|c| c.ch).collect();
        assert!(
            !row0.trim().is_empty(),
            "alt screen should preserve content after resize until app redraws, got row0={row0:?}"
        );

        // Now simulate btop repaint after SIGWINCH
        term.write(b"\x1b[1;1f"); // Re-draw at new size
        term.write(b"\xe2\x95\xad BAT 100% \xe2\x95\xae");
        term.write(b"\x1b[2;1f");
        term.write(b"CPU \xe2\x96\x88\xe2\x96\x88 50%");

        let row0: String = term.screen.cells[0].iter().map(|c| c.ch).collect();
        assert!(
            row0.contains("BAT"),
            "row 0 should contain BAT after repaint, got row0={row0:?}"
        );

        let row1: String = term.screen.cells[1].iter().map(|c| c.ch).collect();
        assert!(
            row1.contains("CPU"),
            "row 1 should contain CPU after repaint, got row1={row1:?}"
        );
    }

    #[test]
    fn btop_real_output_test() {
        let data = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/data/btop_data.bin"
        ));
        let mut term = Terminal::new(35, 100);
        term.write(data);

        assert!(term.screen.in_alternate_screen());

        for row in 0..5 {
            let cells: String = term.screen.cells[row].iter().map(|c| c.ch).collect();
            eprintln!("ROW{row}: [{:?}]", cells.trim_end());
        }

        let row0: String = term.screen.cells[0].iter().map(|c| c.ch).collect();
        assert!(!row0.trim().is_empty(), "row 0 empty");

        let row1: String = term.screen.cells[1].iter().map(|c| c.ch).collect();
        let row2: String = term.screen.cells[2].iter().map(|c| c.ch).collect();
        assert_ne!(row1.trim_end(), row2.trim_end(), "rows 1,2 identical");
    }

    #[test]
    fn scrollback_logical_line_preserves_content_on_shrink() {
        let mut term = Terminal::new(3, 40);
        // 65 chars @ 40 cols → row0=40 (wrapped=false), row1=25 (wrapped=true).
        term.write(b"AAAAABBBBBCCCCCDDDDDEEEEEFFFFFGGGGGHHHHHIIIIIJJJJJKKKKKLLLLLMMMMM");
        // Scroll both visible rows into scrollback (3 newlines for 3 rows).
        term.write(b"\nXXXXX\nYYYYY\nZZZZZ");

        let sb_rows = term.screen.scrollback_lines();
        assert!(sb_rows > 0, "expected scrollback content");

        // Shrink width — same logical line produces MORE visual rows.
        let old_rows = sb_rows;
        term.resize(3, 20);
        let new_rows = term.screen.scrollback_lines();
        assert!(
            new_rows >= old_rows,
            "visual rows should not decrease on shrink (was {old_rows}, now {new_rows})"
        );

        let all_text: String = (0..term.screen.scrollback_lines())
            .filter_map(|i| term.screen.scrollback_row(i))
            .flat_map(|row| row.iter().map(|c| c.ch))
            .collect();
        assert!(
            all_text.contains("AAAAA"),
            "should contain start, got {all_text:?}"
        );
        assert!(
            all_text.contains("MMMMM"),
            "should contain end, got {all_text:?}"
        );
    }

    #[test]
    fn scrollback_logical_line_merges_on_widen() {
        let mut term = Terminal::new(3, 20);
        // 52 chars @ 20 cols → row0=20, row1=20, row2=12, all wrapped continuations.
        term.write(b"AAAABBBBCCCCDDDDEEEEFFFFGGGGHHHHIIIIJJJJKKKKLLLLMMMM");
        term.write(b"\nXXXX\nYYYY\nZZZZ");

        let old_rows = term.screen.scrollback_lines();

        term.resize(3, 40);
        let new_rows = term.screen.scrollback_lines();
        assert!(
            new_rows <= old_rows || new_rows == 0,
            "visual rows should not increase on widen (was {old_rows}, now {new_rows})"
        );

        let new_text: String = (0..term.screen.scrollback_lines())
            .filter_map(|i| term.screen.scrollback_row(i))
            .flat_map(|row| row.iter().map(|c| c.ch))
            .collect();
        assert!(
            new_text.contains("AAAA"),
            "{new_text:?} should contain AAAA"
        );
        assert!(
            new_text.contains("MMMM"),
            "{new_text:?} should contain MMMM"
        );
    }

    #[test]
    fn resize_clamps_cursor_to_bounds() {
        let mut term = Terminal::new(4, 30);
        term.write(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        // Cursor is after Z at col 26.
        assert_eq!(term.screen.cursor_y, 0);
        assert_eq!(term.screen.cursor_x, 26);
        // Resize narrower — visible grid is truncated and the cursor
        // lands at its logical position within the last visible segment,
        // not at the rightmost column.
        term.resize(4, 10);
        // Logical line "ABCDEFGHIJKLMNOPQRSTUVWXYZ" reflowed at width 10:
        //   segment 0  cols 0‑9   "ABCDEFGHIJ"
        //   segment 1  cols 0‑9   "KLMNOPQRST"
        //   segment 2  cols 0‑5   "UVWXYZ"      ← cursor after Z = display col 6
        assert_eq!(
            term.screen.cursor_x, 6,
            "cursor at display col 6 in last segment"
        );
        assert_eq!(
            term.screen.cursor_y, 2,
            "cursor on row 2 (third visual segment of the reflowed logical line)"
        );
    }

    #[test]
    fn scrollback_hard_newline_preserves_separate_logical_lines() {
        let mut term = Terminal::new(2, 30);
        // Write two lines with hard newline, then scroll them into scrollback.
        term.write(b"FIRST LINE\nSECOND LINE\nTHIRD LINE\nFOURTH");

        // Resize narrower — logical lines should remain separate in scrollback.
        term.resize(2, 15);

        // No visual row should contain both FIRST and SECOND.
        for row_idx in 0..term.screen.scrollback_lines() {
            if let Some(row) = term.screen.scrollback_row(row_idx) {
                let txt: String = row.iter().map(|c| c.ch).collect();
                assert!(
                    !(txt.contains("FIRST") && txt.contains("SECOND")),
                    "row {row_idx} must not merge FIRST+SECOND, got {txt:?}"
                );
            }
        }

        // Both keywords should be present somewhere in scrollback.
        let all_text: String = (0..term.screen.scrollback_lines())
            .filter_map(|i| term.screen.scrollback_row(i))
            .flat_map(|row| row.iter().map(|c| c.ch))
            .collect();
        assert!(
            all_text.contains("FIRST"),
            "FIRST should be in scrollback, got {all_text:?}"
        );
        assert!(
            all_text.contains("SECOND"),
            "SECOND should be in scrollback, got {all_text:?}"
        );
    }

    #[test]
    fn cjk_resize_no_space_accumulation() {
        // CJK chars are width=2; "文件 视频" is 5 logical chars (文 件 空格 视 频).
        // At width 6: 文(w2)+件(w2)+空格(w1)=5 cols, 视(w2) wraps to row 1.
        // Cell at (row0, col5) is never written → was Cell::default (ch=' ')
        // which reflow interpreted as a real space → extra space each resize.
        let mut term = Terminal::new(5, 6);
        term.write("文件 视频".as_bytes());
        assert_eq!(term.screen.cursor_y, 1, "视 wrapped to row 1");

        // Helper: collect non-null logical chars from row 0.
        fn row0_text(term: &Terminal) -> String {
            term.screen.cells[0]
                .iter()
                .filter(|c| c.ch != '\0' && !c.wide_continuation)
                .map(|c| c.ch)
                .collect::<String>()
        }
        assert_eq!(row0_text(&term), "文件 ", "initial row0: 文 件 space");

        // Shrink to 5 cols then widen back to 6 cols.
        term.resize(5, 5);
        term.resize(5, 6);
        assert_eq!(
            row0_text(&term),
            "文件 ",
            "no extra space after 1 shrink/widen cycle"
        );

        // Repeat 5 more times to guarantee no accumulation.
        for _ in 0..5 {
            term.resize(5, 5);
            term.resize(5, 6);
        }
        assert_eq!(
            row0_text(&term),
            "文件 ",
            "no extra space after 6 shrink/widen cycles"
        );

        assert!(
            term.screen.cursor_y < term.screen.rows,
            "cursor inside screen"
        );
    }

    #[test]
    fn zsh_multiline_heredoc_redraw_preserves_all_lines() {
        // Simulate zsh drawing initial prompt: \r\n%<spaces>\r \r\r<prompt>
        let prompt = b"\r\n%                                                                                                                      \r \r\r\x1b[01;32mdwh@dwh-82sk\x1b[00m \x1b[01;34m~\x1b[00m \xc2\xbb ";
        let mut term = Terminal::new(10, 80);
        term.write(prompt);
        let prompt_row = term.screen.cursor_y;

        // Simulate zsh redrawing a multi-line heredoc after up-arrow:
        // ESC[10D (CUB 10) + first line, then CR CR LF between each subsequent line
        let redraw = concat!(
            "\x1b[10D",                                   // CUB 10: move cursor back past "echo hello"
            "source /tmp/activate && python3 << 'PYEOF'", // overwrite first line
            "\r\r\n",                                     // CR CR LF → go to next line
            "import base64",                              // line 2
            "\x1b[K",                                     // EL: clear rest of line
            "\r\r\n",                                     // CR CR LF
            "import json",                                // line 3
            "\x1b[K",
            "\r\r\n",         // CR CR LF
            "print('hello')", // line 4
            "\x1b[K",
        );
        term.write(redraw.as_bytes());

        // Each CR CR LF should create a real newline (not be suppressed).
        // After redraw, cursor should be on a row containing "import json" or "print".
        let cy = term.screen.cursor_y;
        assert!(
            cy > prompt_row,
            "cursor should have advanced past prompt row {prompt_row}, got {cy}"
        );

        // Verify that the heredoc lines are on separate rows.
        let row0 = row_plaintext(&term, prompt_row);
        assert!(
            row0.contains("source"),
            "first line should contain 'source' on row {prompt_row}, got {row0:?}"
        );
        let row1 = row_plaintext(&term, prompt_row + 1);
        assert!(
            row1.contains("import base64"),
            "second line should contain 'import base64' on row {}, got {row1:?}",
            prompt_row + 1
        );
        let row2 = row_plaintext(&term, prompt_row + 2);
        assert!(
            row2.contains("import json"),
            "third line should contain 'import json' on row {}, got {row2:?}",
            prompt_row + 2
        );
    }

    fn count_prompt_only_rows(term: &Terminal) -> usize {
        term.screen
            .cells
            .iter()
            .filter(|row| {
                let text: String = row.iter().map(|c| c.ch).collect();
                let trimmed = text.trim();
                trimmed.starts_with("dwh@") && trimmed.contains('»') && !trimmed.contains("130")
            })
            .count()
    }

    #[test]
    fn resize_shrink_widen_does_not_duplicate_prompt_lines_with_exit_code() {
        // Start narrow so the exit-code padding soft-wraps, then alternate width.
        let ls = b" 111         Apps   Desktop\n ai-models   code   Documents\n";
        let prompt_prefix =
            b"\r\n%                                                                                                                      \r \r\r\x1b[01;32mdwh@dwh-82sk\x1b[00m \x1b[01;34m~\x1b[00m \xc2\xbb ";
        let exit_pad = b"                                                                                                    130 \xe2\x86\xb5 ";
        let winch_redraw =
            b"\r\r\x1b[0m\x1b[27m\x1b[24m\x1b[J\x1b[01;32mdwh@dwh-82sk\x1b[00m \x1b[01;34m~\x1b[00m \x1b[33m\xc2\xbb\x1b[0m ";
        let exit_redraw = b"                                                                                                    130 \xe2\x86\xb5 ";

        let mut term = Terminal::new(24, 40);
        term.write(ls);
        term.write(prompt_prefix);
        term.write(exit_pad);
        let baseline_prompt_rows = count_prompt_only_rows(&term);
        assert!(
            baseline_prompt_rows >= 1,
            "expected at least one prompt row"
        );

        for cycle in 0..8 {
            term.resize(24, 120);
            term.write(winch_redraw);
            term.write(exit_redraw);
            term.resize(24, 40);
            term.write(winch_redraw);
            term.write(exit_redraw);
            let count = count_prompt_only_rows(&term);
            assert_eq!(
                count, baseline_prompt_rows,
                "cycle {cycle}: prompt row count changed ({count}, expected {baseline_prompt_rows})"
            );
        }
    }

    #[test]
    fn resize_with_zsh_winch_redraw_does_not_duplicate_prompt_lines() {
        let ls = b" 111         Apps   Desktop\n ai-models   code   Documents\n";
        let prompt_prefix =
            b"\r\n%                                                                                                                      \r \r\r\x1b[01;32mdwh@dwh-82sk\x1b[00m \x1b[01;34m~\x1b[00m \xc2\xbb ";
        let exit_pad = b"                                                                                                    130 \xe2\x86\xb5 ";
        let winch_redraw =
            b"\r\r\x1b[0m\x1b[27m\x1b[24m\x1b[J\x1b[01;32mdwh@dwh-82sk\x1b[00m \x1b[01;34m~\x1b[00m \x1b[33m\xc2\xbb\x1b[0m ";
        let exit_redraw = b"                                                                                                    130 \xe2\x86\xb5 ";

        let mut term = Terminal::new(24, 120);
        term.write(ls);
        term.write(prompt_prefix);
        term.write(exit_pad);
        let baseline = count_prompt_only_rows(&term);

        for cycle in 0..8 {
            term.resize(24, 40);
            term.write(winch_redraw);
            term.write(exit_redraw);
            term.resize(24, 120);
            term.write(winch_redraw);
            term.write(exit_redraw);
            let count = count_prompt_only_rows(&term);
            assert!(
                count <= baseline + 1,
                "cycle {cycle}: too many prompt rows ({count}, baseline {baseline}), cursor_y={}",
                term.screen.cursor_y
            );
        }
    }

    #[test]
    fn zsh_consecutive_cr_cr_lf_advances_on_blank_rows() {
        // After the first `\r\r\n`, the cursor sits at col 0 on a blank row.  zsh repeats
        // `\r\r\n` for each additional line in a multiline history entry; those LFs must
        // not be swallowed by the blank-line skip meant for lone `\r\n`.
        let prompt = b"\r\n%                                                                                                                      \r \r\r\x1b[01;32mdwh@dwh-82sk\x1b[00m \x1b[01;34m~\x1b[00m \xc2\xbb ";
        let mut term = Terminal::new(10, 80);
        term.write(prompt);
        let start_y = term.screen.cursor_y;

        for i in 0..4 {
            term.write(b"\r\r\n");
            assert_eq!(
                term.screen.cursor_y,
                start_y + i + 1,
                "after {} consecutive \\r\\r\\n, cursor should be on row {}",
                i + 1,
                start_y + i + 1
            );
        }
    }

    #[test]
    fn zsh_autosuggest_bs_per_display_col_after_cjk() {
        // zsh-autosuggestions: print gray CJK POSTDISPLAY, then one BS per display column.
        let mut term = Terminal::new(1, 80);
        term.write(b"cd 111/");
        let end = term.screen.cursor_x;
        assert_eq!(end, 7);
        // 新项目5 = 2+2+2+1 = 7 display columns
        term.write(b"\x1b[90m\xe6\x96\xb0\xe9\xa1\xb9\xe7\x9b\xae5\x1b[39m");
        assert_eq!(term.screen.cursor_x, end + 7);
        term.write(b"\x08\x08\x08\x08\x08\x08\x08");
        assert_eq!(
            term.screen.cursor_x, end,
            "7 BS over CJK suggestion must return to end of cd 111/"
        );
    }

    #[test]
    fn zsh_menu_complete_then_cjk_autosuggest_restores_cursor() {
        // Reduced capture: after accepting `cd 111/`, omz autosuggest paints gray
        // `新项目5` at CUF 44 and restores with 7×BS.
        let mut term = Terminal::new(3, 100);
        term.write(
            b"\x1b[01;32mdwh@dwh-82sk\x1b[00m \x1b[01;34m/tmp/rsterm-comp-test\x1b[00m \xc2\xbb ",
        );
        term.write(b"cd 111/");
        // autosuggest path from captured omz enter tail
        term.write(b"\r\r\n\x1b[J\x1b[A\x1b[44C\x1b[90m\xe6\x96\xb0\xe9\xa1\xb9\xe7\x9b\xae5\x1b[39m\x08\x08\x08\x08\x08\x08\x08");
        assert_eq!(
            term.screen.cursor_x, 44,
            "cursor must sit after cd 111/, not inside it"
        );
        let row: String = term.screen.cells[term.screen.cursor_y]
            .iter()
            .map(|c| c.ch)
            .collect::<String>()
            .trim_end()
            .to_string();
        assert!(row.contains("cd 111/"), "row={row:?}");
    }
}
