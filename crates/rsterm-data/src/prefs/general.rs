//! General preferences (language, input mode).

use serde::{Deserialize, Serialize};

use rsterm_config::Language;

use super::input_mode::{InputInteractionMode, default_input_mode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralPrefs {
    #[serde(default)]
    pub language: Language,
    #[serde(default = "default_input_mode")]
    pub input_mode: InputInteractionMode,
}

impl Default for GeneralPrefs {
    fn default() -> Self {
        Self {
            language: Language::default(),
            input_mode: default_input_mode(),
        }
    }
}
