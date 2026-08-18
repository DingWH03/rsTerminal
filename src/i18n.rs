//! Internationalization (i18n) module for rsTerminal.
//!
//! Uses `rust-i18n` for translation loading and `sys-locale` for system locale detection.
//! Supports runtime language switching and persists the choice in settings.
//!
//! The `rust_i18n::i18n!("locales")` macro is invoked in `lib.rs` (the crate root).

use rust_i18n::t;
use serde::{Deserialize, Serialize};

// ─── Language ─────────────────────────────────────────────────────────────────

/// Supported languages for the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    /// Follow the system locale (auto-detect).
    System,
    /// Simplified Chinese.
    ZhCN,
    /// English.
    En,
}

impl Language {
    pub const ALL: [Self; 3] = [Self::System, Self::ZhCN, Self::En];

    /// Human-readable label for the language selector.
    pub fn label(self) -> String {
        match self {
            Self::System => t!("language_system").into_owned(),
            Self::ZhCN => t!("language_zh").into_owned(),
            Self::En => t!("language_en").into_owned(),
        }
    }

    /// The locale code used by `rust-i18n`.
    fn locale_code(self) -> &'static str {
        match self {
            Self::System => detect_system_locale(),
            Self::ZhCN => "zh-CN",
            Self::En => "en",
        }
    }

    /// Apply this language setting, making all subsequent `t!()` calls use it.
    pub fn apply(self) {
        let code = self.locale_code();
        rust_i18n::set_locale(code);
    }
}

impl Default for Language {
    fn default() -> Self {
        Self::System
    }
}

// ─── UI Theme ─────────────────────────────────────────────────────────────────

/// UI appearance theme (separate from terminal themes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiTheme {
    System,
    Light,
    Dark,
}

impl UiTheme {
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    pub fn label(self) -> String {
        match self {
            Self::System => t!("ui_theme_system").into_owned(),
            Self::Light => t!("ui_theme_light").into_owned(),
            Self::Dark => t!("ui_theme_dark").into_owned(),
        }
    }

    /// Apply this theme to the egui context.
    pub fn apply(self, ctx: &egui::Context) {
        let dark_mode = match self {
            Self::System => {
                std::env::var("COLORFGBG")
                    .ok()
                    .and_then(|v| v.split(';').last().map(|s| s.trim() == "0"))
                    .unwrap_or(false)
                    || std::env::var("GTK_THEME")
                        .ok()
                        .map(|t| t.contains("dark") || t.contains("Dark"))
                        .unwrap_or(false)
            }
            Self::Light => false,
            Self::Dark => true,
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
        style.spacing.interact_size =
            egui::vec2(tokens::size::TOOLBAR_WIDTH, tokens::size::NAV_ROW);
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
}

impl Default for UiTheme {
    fn default() -> Self {
        Self::System
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

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
