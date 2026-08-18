//! General preferences (language).

use serde::{Deserialize, Serialize};

use crate::i18n::Language;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeneralPrefs {
    #[serde(default)]
    pub language: Language,
}
