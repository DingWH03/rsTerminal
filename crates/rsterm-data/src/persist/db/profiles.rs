//! Terminal profiles table CRUD.

use rusqlite::{Connection, OptionalExtension, params};

use rsterm_config::KeyboardMode;
use rsterm_config::{BellStyle, CursorStyle, TerminalTheme, TerminalType};
use crate::persist::types::TerminalProfile;

pub fn list_all(conn: &Connection) -> rusqlite::Result<Vec<TerminalProfile>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, name, description, terminal_font, font_size, line_spacing, cell_width_scale,
               theme_json, cursor_style, bold_is_bright, scrollback_lines, terminal_type, bell,
               enable_bracketed_paste, enable_sgr_mouse, auto_wrap, word_separators,
               keyboard_mode, is_default
        FROM terminal_profiles
        ORDER BY is_default DESC, name COLLATE NOCASE ASC
        "#,
    )?;
    let rows = stmt.query_map([], row_to_profile)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn get(conn: &Connection, id: &str) -> rusqlite::Result<Option<TerminalProfile>> {
    conn.query_row(
        r#"
        SELECT id, name, description, terminal_font, font_size, line_spacing, cell_width_scale,
               theme_json, cursor_style, bold_is_bright, scrollback_lines, terminal_type, bell,
               enable_bracketed_paste, enable_sgr_mouse, auto_wrap, word_separators,
               keyboard_mode, is_default
        FROM terminal_profiles WHERE id = ?1
        "#,
        params![id],
        row_to_profile,
    )
    .optional()
}

pub fn get_default(conn: &Connection) -> rusqlite::Result<Option<TerminalProfile>> {
    conn.query_row(
        r#"
        SELECT id, name, description, terminal_font, font_size, line_spacing, cell_width_scale,
               theme_json, cursor_style, bold_is_bright, scrollback_lines, terminal_type, bell,
               enable_bracketed_paste, enable_sgr_mouse, auto_wrap, word_separators,
               keyboard_mode, is_default
        FROM terminal_profiles WHERE is_default = 1 LIMIT 1
        "#,
        [],
        row_to_profile,
    )
    .optional()
}

pub fn upsert(conn: &Connection, p: &TerminalProfile) -> rusqlite::Result<()> {
    if p.is_default {
        conn.execute("UPDATE terminal_profiles SET is_default = 0", [])?;
    }
    let theme_json = serde_json::to_string(&p.theme).unwrap_or_else(|_| "{}".into());
    conn.execute(
        r#"
        INSERT INTO terminal_profiles (
            id, name, description, terminal_font, font_size, line_spacing, cell_width_scale,
            theme_json, cursor_style, bold_is_bright, scrollback_lines, terminal_type, bell,
            enable_bracketed_paste, enable_sgr_mouse, auto_wrap, word_separators,
            keyboard_mode, is_default
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19
        )
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            description = excluded.description,
            terminal_font = excluded.terminal_font,
            font_size = excluded.font_size,
            line_spacing = excluded.line_spacing,
            cell_width_scale = excluded.cell_width_scale,
            theme_json = excluded.theme_json,
            cursor_style = excluded.cursor_style,
            bold_is_bright = excluded.bold_is_bright,
            scrollback_lines = excluded.scrollback_lines,
            terminal_type = excluded.terminal_type,
            bell = excluded.bell,
            enable_bracketed_paste = excluded.enable_bracketed_paste,
            enable_sgr_mouse = excluded.enable_sgr_mouse,
            auto_wrap = excluded.auto_wrap,
            word_separators = excluded.word_separators,
            keyboard_mode = excluded.keyboard_mode,
            is_default = excluded.is_default
        "#,
        params![
            p.id,
            p.name,
            p.description,
            p.terminal_font,
            p.font_size as f64,
            p.line_spacing as f64,
            p.cell_width_scale as f64,
            theme_json,
            cursor_to_str(p.cursor_style),
            p.bold_is_bright as i64,
            p.scrollback_lines as i64,
            term_type_to_str(p.terminal_type),
            bell_to_str(p.bell),
            p.enable_bracketed_paste as i64,
            p.enable_sgr_mouse as i64,
            p.auto_wrap as i64,
            p.word_separators,
            keyboard_to_str(p.keyboard_mode),
            p.is_default as i64,
        ],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM terminal_profiles WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn set_default(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute("UPDATE terminal_profiles SET is_default = 0", [])?;
    conn.execute(
        "UPDATE terminal_profiles SET is_default = 1 WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

pub fn count(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM terminal_profiles", [], |r| r.get(0))
}

fn row_to_profile(row: &rusqlite::Row<'_>) -> rusqlite::Result<TerminalProfile> {
    let theme_json: String = row.get(7)?;
    let theme: TerminalTheme = serde_json::from_str(&theme_json).unwrap_or_default();
    let cursor_s: String = row.get(8)?;
    let term_s: String = row.get(11)?;
    let bell_s: String = row.get(12)?;
    let kbd_s: String = row.get(17)?;
    Ok(TerminalProfile {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        terminal_font: row.get(3)?,
        font_size: row.get::<_, f64>(4)? as f32,
        line_spacing: row.get::<_, f64>(5)? as f32,
        cell_width_scale: row.get::<_, f64>(6)? as f32,
        theme,
        cursor_style: cursor_from_str(&cursor_s),
        bold_is_bright: row.get::<_, i64>(9)? != 0,
        scrollback_lines: row.get::<_, i64>(10)? as usize,
        terminal_type: term_type_from_str(&term_s),
        bell: bell_from_str(&bell_s),
        enable_bracketed_paste: row.get::<_, i64>(13)? != 0,
        enable_sgr_mouse: row.get::<_, i64>(14)? != 0,
        auto_wrap: row.get::<_, i64>(15)? != 0,
        word_separators: row.get(16)?,
        keyboard_mode: keyboard_from_str(&kbd_s),
        is_default: row.get::<_, i64>(18)? != 0,
    })
}

fn cursor_to_str(c: CursorStyle) -> &'static str {
    match c {
        CursorStyle::Bar => "bar",
        CursorStyle::Block => "block",
        CursorStyle::Underline => "underline",
        CursorStyle::BarBlink => "bar_blink",
        CursorStyle::BlockBlink => "block_blink",
        CursorStyle::UnderlineBlink => "underline_blink",
    }
}

fn cursor_from_str(s: &str) -> CursorStyle {
    match s {
        "block" => CursorStyle::Block,
        "underline" => CursorStyle::Underline,
        "bar_blink" => CursorStyle::BarBlink,
        "block_blink" => CursorStyle::BlockBlink,
        "underline_blink" => CursorStyle::UnderlineBlink,
        _ => CursorStyle::Bar,
    }
}

fn term_type_to_str(t: TerminalType) -> &'static str {
    match t {
        TerminalType::Xterm256 => "xterm_256",
        TerminalType::Xterm => "xterm",
        TerminalType::Screen256 => "screen_256",
        TerminalType::Screen => "screen",
        TerminalType::Tmux256 => "tmux_256",
        TerminalType::Tmux => "tmux",
    }
}

fn term_type_from_str(s: &str) -> TerminalType {
    match s {
        "xterm" => TerminalType::Xterm,
        "screen_256" => TerminalType::Screen256,
        "screen" => TerminalType::Screen,
        "tmux_256" => TerminalType::Tmux256,
        "tmux" => TerminalType::Tmux,
        _ => TerminalType::Xterm256,
    }
}

fn bell_to_str(b: BellStyle) -> &'static str {
    match b {
        BellStyle::Off => "off",
        BellStyle::Visual => "visual",
        BellStyle::Audible => "audible",
        BellStyle::Both => "both",
    }
}

fn bell_from_str(s: &str) -> BellStyle {
    match s {
        "off" => BellStyle::Off,
        "audible" => BellStyle::Audible,
        "both" => BellStyle::Both,
        _ => BellStyle::Visual,
    }
}

fn keyboard_to_str(k: KeyboardMode) -> &'static str {
    match k {
        KeyboardMode::Special => "special",
        KeyboardMode::Full => "full",
    }
}

fn keyboard_from_str(s: &str) -> KeyboardMode {
    match s {
        "special" => KeyboardMode::Special,
        _ => KeyboardMode::Full,
    }
}
