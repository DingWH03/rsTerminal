//! Persisted UI geometry / runtime state (not exposed in Settings pages).

use serde::{Deserialize, Serialize};

/// Top-level bucket for silent persistence (column widths, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct UiStatePrefs {
    #[serde(default)]
    pub file_manager: FileManagerUiState,
}

/// File-manager UI geometry that is saved but not edited in Settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct FileManagerUiState {
    /// Legacy shared Details widths (fallback when per-pane values are absent).
    #[serde(default)]
    pub details_name_w: Option<f32>,
    #[serde(default)]
    pub details_size_w: Option<f32>,

    /// Left / remote pane Details column widths.
    #[serde(default)]
    pub left_details_name_w: Option<f32>,
    #[serde(default)]
    pub left_details_size_w: Option<f32>,
    /// Right pane Details column widths.
    #[serde(default)]
    pub right_details_name_w: Option<f32>,
    #[serde(default)]
    pub right_details_size_w: Option<f32>,

    /// Left pane fraction of dual layout width (`0.15..=0.85`).
    #[serde(default)]
    pub dual_split: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefs::Prefs;

    #[test]
    fn prefs_ui_state_roundtrip_and_default_compat() {
        let mut prefs = Prefs::default();
        prefs.ui_state.file_manager = FileManagerUiState {
            details_name_w: Some(180.0),
            details_size_w: Some(96.0),
            left_details_name_w: Some(160.0),
            left_details_size_w: Some(80.0),
            right_details_name_w: Some(200.0),
            right_details_size_w: Some(100.0),
            dual_split: Some(0.4),
        };
        let json = serde_json::to_string(&prefs).unwrap();
        let loaded: Prefs = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.ui_state, prefs.ui_state);

        // Old prefs without ui_state still deserialize.
        let legacy = r#"{"general":{},"appearance":{},"chrome":{},"file_manager":{}}"#;
        let legacy_prefs: Prefs = serde_json::from_str(legacy).unwrap();
        assert_eq!(legacy_prefs.ui_state, UiStatePrefs::default());
    }
}
