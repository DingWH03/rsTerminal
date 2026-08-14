//! Internationalization (i18n) module for rsTerminal.
//!
//! Uses `rust-i18n` for translation loading and `sys-locale` for system locale detection.
//! Supports runtime language switching and persists the choice in settings.
//!
//! The `rust_i18n::i18n!("locales")` macro is invoked in `lib.rs` (the crate root).

use rust_i18n::t;

pub use rsterm_config::{CursorStyle, Language, UiTheme};

/// Human-readable label for the language selector.
pub fn language_label(lang: Language) -> String {
    match lang {
        Language::System => t!("language_system").into_owned(),
        Language::ZhCN => t!("language_zh").into_owned(),
        Language::En => t!("language_en").into_owned(),
    }
}

/// The locale code used by `rust-i18n`.
fn locale_code(lang: Language) -> &'static str {
    match lang {
        Language::System => detect_system_locale(),
        Language::ZhCN => "zh-CN",
        Language::En => "en",
    }
}

/// Apply this language setting, making all subsequent `t!()` calls use it.
pub fn apply_language(lang: Language) {
    ensure_shell_i18n();
    rust_i18n::set_locale(locale_code(lang));
    rsterm_page_terminal::set_labels(terminal_labels());
    rsterm_page_file_manager::set_labels(file_manager_labels());
}

struct ShellI18nAdapter;

impl rsterm_shell::I18nT for ShellI18nAdapter {
    fn t(&self, key: &str) -> String {
        rust_i18n::t!(key).into_owned()
    }
}

fn ensure_shell_i18n() {
    use std::sync::Arc;
    rsterm_shell::set_i18n(Arc::new(ShellI18nAdapter));
}

fn terminal_labels() -> rsterm_page_terminal::TerminalLabels {
    rsterm_page_terminal::TerminalLabels {
        copy: t!("copy").into_owned(),
        paste: t!("paste").into_owned(),
        cancel: t!("cancel").into_owned(),
        close_pane: t!("close_pane").into_owned(),
        minimize_pane: t!("minimize_pane").into_owned(),
        settings_default_keyboard: t!("settings_default_keyboard").into_owned(),
        clear_selection: t!("clear_selection").into_owned(),
        connecting: t!("connecting").into_owned(),
        disconnected: t!("disconnected").into_owned(),
        connection_failed: t!("connection_failed").into_owned(),
        reconnect: t!("reconnect").into_owned(),
        close: t!("close").into_owned(),
    }
}

fn file_manager_labels() -> rsterm_page_file_manager::FileManagerLabels {
    rsterm_page_file_manager::FileManagerLabels {
        close_pane: t!("close_pane").into_owned(),
        stop: t!("stop").into_owned(),
        copy: t!("copy").into_owned(),
        cut: t!("cut").into_owned(),
        delete: t!("delete").into_owned(),
        cancel: t!("cancel").into_owned(),
        paste: t!("paste").into_owned(),
        loading: t!("loading").into_owned(),
        empty_folder: t!("empty_folder").into_owned(),
        parent_folder: t!("parent_folder").into_owned(),
        multi_select: t!("multi_select").into_owned(),
        clipboard_empty: t!("clipboard_empty").into_owned(),
        open: t!("open").into_owned(),
        rename: t!("rename").into_owned(),
        file_info: t!("file_info").into_owned(),
        close: t!("close").into_owned(),
        original_name: t!("original_name").into_owned(),
        new_name: t!("new_name").into_owned(),
        confirm: t!("confirm").into_owned(),
    }
}

/// Human-readable label for the UI theme selector.
pub fn ui_theme_label(theme: UiTheme) -> String {
    match theme {
        UiTheme::System => t!("ui_theme_system").into_owned(),
        UiTheme::Light => t!("ui_theme_light").into_owned(),
        UiTheme::Dark => t!("ui_theme_dark").into_owned(),
    }
}

/// Apply this theme to the egui context.
pub fn apply_ui_theme(theme: UiTheme, ctx: &egui::Context) {
    let dark_mode = match theme {
        UiTheme::System => {
            std::env::var("COLORFGBG")
                .ok()
                .and_then(|v| v.split(';').next_back().map(|s| s.trim() == "0"))
                .unwrap_or(false)
                || std::env::var("GTK_THEME")
                    .ok()
                    .map(|t| t.contains("dark") || t.contains("Dark"))
                    .unwrap_or(false)
        }
        UiTheme::Light => false,
        UiTheme::Dark => true,
    };

    use crate::ui::uiframe::tokens;

    let palette = tokens::SemanticPalette::for_dark_mode(dark_mode);
    let mut visuals = if dark_mode {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.window_fill = palette.surface_0;
    visuals.panel_fill = palette.surface_1;
    visuals.extreme_bg_color = palette.surface_0;
    visuals.faint_bg_color = palette.surface_2;

    visuals.widgets.noninteractive.bg_fill = palette.surface_2;
    visuals.widgets.noninteractive.weak_bg_fill = palette.surface_1;
    visuals.widgets.noninteractive.fg_stroke =
        egui::Stroke::new(tokens::stroke::HAIRLINE, palette.text_primary);
    visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(tokens::stroke::HAIRLINE, palette.border_subtle);

    visuals.widgets.inactive.bg_fill = palette.surface_2;
    visuals.widgets.inactive.weak_bg_fill = palette.surface_1;
    visuals.widgets.inactive.fg_stroke =
        egui::Stroke::new(tokens::stroke::HAIRLINE, palette.text_primary);
    visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(tokens::stroke::HAIRLINE, palette.border_subtle);

    visuals.widgets.hovered.bg_fill = palette.surface_3;
    visuals.widgets.hovered.weak_bg_fill = palette.surface_3;
    visuals.widgets.hovered.fg_stroke =
        egui::Stroke::new(tokens::stroke::EMPHASIS, palette.text_primary);
    visuals.widgets.hovered.bg_stroke =
        egui::Stroke::new(tokens::stroke::HAIRLINE, palette.border);

    visuals.widgets.active.bg_fill = palette.surface_4;
    visuals.widgets.active.weak_bg_fill = palette.surface_4;
    visuals.widgets.active.fg_stroke =
        egui::Stroke::new(tokens::stroke::EMPHASIS, palette.text_primary);
    visuals.widgets.active.bg_stroke =
        egui::Stroke::new(tokens::stroke::HAIRLINE, palette.border_accent);

    visuals.widgets.open = visuals.widgets.active;
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = tokens::radius::XS;
    }

    visuals.selection.bg_fill = palette.selection;
    visuals.selection.stroke = egui::Stroke::new(tokens::stroke::HAIRLINE, palette.accent);
    visuals.hyperlink_color = palette.accent;
    visuals.override_text_color = Some(palette.text_primary);
    visuals.window_corner_radius = tokens::radius::SM;
    visuals.window_stroke = egui::Stroke::new(tokens::stroke::HAIRLINE, palette.border_subtle);

    // Clone the current style so custom font definitions/families survive the
    // per-frame theme application; only standard UI sizes and spacing change.
    let mut style = (*ctx.global_style()).clone();
    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(tokens::space::MD, tokens::space::SM);
    style.spacing.button_padding = egui::vec2(tokens::space::LG, tokens::space::SM);
    // Keep default control height at the nav/menu row (28px). Primary
    // action buttons opt into 30px via `style::primary_button` min_size —
    // using BUTTON here overflows the fixed top menu panel and leaves a
    // dark seam under the menubar.
    style.spacing.interact_size = egui::vec2(tokens::size::TOOLBAR_WIDTH, tokens::size::NAV_ROW);
    style.spacing.window_margin = egui::Margin::same(tokens::space::XL as i8);

    for (text_style, size) in [
        (egui::TextStyle::Small, tokens::text::SMALL),
        (egui::TextStyle::Body, tokens::text::BODY),
        (egui::TextStyle::Button, tokens::text::BODY),
        (egui::TextStyle::Heading, tokens::text::HEADING),
        (egui::TextStyle::Monospace, tokens::text::BODY),
    ] {
        if let Some(font_id) = style.text_styles.get_mut(&text_style) {
            font_id.size = size;
        }
    }

    ctx.set_global_style(style);
}

/// Human-readable label for cursor style (uses i18n).
pub fn cursor_style_label(style: CursorStyle) -> String {
    match style {
        CursorStyle::Bar => t!("cursor_bar").into_owned(),
        CursorStyle::Block => t!("cursor_block").into_owned(),
        CursorStyle::Underline => t!("cursor_underline").into_owned(),
        CursorStyle::BarBlink => t!("cursor_bar_blink").into_owned(),
        CursorStyle::BlockBlink => t!("cursor_block_blink").into_owned(),
        CursorStyle::UnderlineBlink => t!("cursor_underline_blink").into_owned(),
    }
}

fn detect_system_locale() -> &'static str {
    let locale = sys_locale::get_locale().unwrap_or_else(|| String::from("en"));
    if locale.starts_with("zh") {
        "zh-CN"
    } else {
        "en"
    }
}

/// Convenience wrapper: translate a key, returning the translated string.
/// This is equivalent to `rust_i18n::t!(key)` but can be used as a function.
#[macro_export]
macro_rules! tr {
    ($key:tt) => {
        rust_i18n::t!($key)
    };
    ($key:tt, $($arg:tt)*) => {
        rust_i18n::t!($key, $($arg)*)
    };
}
