//! Compatibility shim — prefer [`crate::prefs`] and [`crate::persist::types::TerminalProfile`].
//!
//! Kept so gradual call-site migration compiles; new code should not depend on this module.

pub use crate::persist::types::TerminalProfile as Profile;
pub use crate::prefs::{load_prefs as load_settings, save_prefs as save_settings, Prefs as AppSettings};

use crate::config::{CursorStyle, TerminalTheme};
use crate::persist::types::TerminalProfile;

/// Helper used by code still expecting settings-bound profile resolution.
pub trait ProfileResolve {
    fn resolve_profile_id<'a>(
        &'a self,
        profiles: &'a [TerminalProfile],
        id: Option<&str>,
    ) -> &'a TerminalProfile;
}

pub fn resolve_profile<'a>(
    profiles: &'a [TerminalProfile],
    id: Option<&str>,
) -> &'a TerminalProfile {
    if let Some(id) = id {
        if let Some(p) = profiles.iter().find(|p| p.id == id) {
            return p;
        }
    }
    profiles
        .iter()
        .find(|p| p.is_default)
        .or_else(|| profiles.first())
        .expect("at least one terminal profile")
}

pub fn theme_of(profiles: &[TerminalProfile]) -> &TerminalTheme {
    &resolve_profile(profiles, None).theme
}

pub fn cursor_style_of(profiles: &[TerminalProfile]) -> CursorStyle {
    resolve_profile(profiles, None).cursor_style
}
