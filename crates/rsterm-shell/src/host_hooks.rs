//! Host-provided hooks that cannot live in this crate (fonts catalog, theme apply).

use std::sync::{Arc, OnceLock};

use egui::Context;
use rsterm_config::{CursorStyle, Language, UiTheme};

/// Monospace font entry for the profile dialog picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontEntry {
    pub path: String,
    pub label: String,
}

/// Status of the async monospace font catalog scan.
pub enum FontCatalogStatus {
    Loading,
    Ready(Arc<Vec<FontEntry>>),
}

/// Host callbacks for fonts + applying language/theme from settings UI.
#[derive(Clone, Copy)]
pub struct HostHooks {
    pub monospace_catalog_status: fn() -> FontCatalogStatus,
    pub apply_terminal_fonts: fn(&Context, &str),
    pub apply_language: fn(Language),
    pub apply_ui_theme: fn(UiTheme, &Context),
    pub language_label: fn(Language) -> String,
    pub ui_theme_label: fn(UiTheme) -> String,
    pub cursor_style_label: fn(CursorStyle) -> String,
}

static HOST: OnceLock<HostHooks> = OnceLock::new();

/// Register host hooks once at app startup.
pub fn install_host_hooks(hooks: HostHooks) {
    let _ = HOST.set(hooks);
}

fn hooks() -> Option<&'static HostHooks> {
    HOST.get()
}

pub fn monospace_catalog_status() -> FontCatalogStatus {
    hooks()
        .map(|h| (h.monospace_catalog_status)())
        .unwrap_or(FontCatalogStatus::Loading)
}

pub fn apply_terminal_fonts(ctx: &Context, path: &str) {
    if let Some(h) = hooks() {
        (h.apply_terminal_fonts)(ctx, path);
    }
}

pub fn apply_language(lang: Language) {
    if let Some(h) = hooks() {
        (h.apply_language)(lang);
    }
}

pub fn apply_ui_theme(theme: UiTheme, ctx: &Context) {
    if let Some(h) = hooks() {
        (h.apply_ui_theme)(theme, ctx);
    }
}

pub fn language_label(lang: Language) -> String {
    hooks()
        .map(|h| (h.language_label)(lang))
        .unwrap_or_else(|| format!("{lang:?}"))
}

pub fn ui_theme_label(theme: UiTheme) -> String {
    hooks()
        .map(|h| (h.ui_theme_label)(theme))
        .unwrap_or_else(|| format!("{theme:?}"))
}

pub fn cursor_style_label(style: CursorStyle) -> String {
    hooks()
        .map(|h| (h.cursor_style_label)(style))
        .unwrap_or_else(|| format!("{style:?}"))
}
