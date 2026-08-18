//! Terminal-specific font helpers (braille family must match root `fonts` setup).

use std::sync::OnceLock;

use egui::{FontFamily, FontId};

const FALLBACK_BRAILLE_FAMILY: &str = "term_fallback_braille";

/// Host-provided hooks that need root font catalog state.
#[derive(Clone, Copy)]
pub struct FontHooks {
    pub font_generation: fn() -> u32,
}

static FONT_HOOKS: OnceLock<FontHooks> = OnceLock::new();

/// Register font hooks once at app startup (from root `fonts`).
pub fn install_font_hooks(hooks: FontHooks) {
    let _ = FONT_HOOKS.set(hooks);
}

pub fn font_generation() -> u32 {
    FONT_HOOKS.get().map(|h| (h.font_generation)()).unwrap_or(0)
}

pub fn needs_braille_font(ch: char) -> bool {
    matches!(ch as u32, 0x2800..=0x28FF)
}

pub fn terminal_font_id(size: f32) -> FontId {
    FontId::monospace(size)
}

pub fn terminal_font_id_for_char(ch: char, size: f32) -> FontId {
    if needs_braille_font(ch) {
        FontId::new(size, FontFamily::Name(FALLBACK_BRAILLE_FAMILY.into()))
    } else {
        terminal_font_id(size)
    }
}
