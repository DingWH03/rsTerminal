//! General preferences (language).

use serde::{Deserialize, Serialize};

use rsterm_config::Language;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeneralPrefs {
    #[serde(default)]
    pub language: Language,
}
