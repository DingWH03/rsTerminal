//! Global UI appearance (not terminal colors).

use serde::{Deserialize, Serialize};

use crate::i18n::UiTheme;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppearancePrefs {
    #[serde(default)]
    pub ui_theme: UiTheme,
    /// Accent colors for split panes (RGB). Empty = theme default palette.
    #[serde(default)]
    pub pane_accent_colors: Vec<[u8; 3]>,
}
