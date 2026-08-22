//! Shell geometry, navigation, and transient UI state.

use crate::function_pane::pages::FunctionPage;
use crate::page::settings::SettingsTab;

/// Function pane width bounds (shell chrome, not workspace layout).
pub const FUNCTION_MIN_WIDTH: f32 = 200.0;
pub const FUNCTION_MAX_WIDTH: f32 = 360.0;
pub const FUNCTION_DEFAULT_WIDTH: f32 = 220.0;

/// Compatibility exports for callers that have not yet moved to `ui::layout`.
pub use crate::layout::{
    DropEdge, DropZone, MIN_PANE_HEIGHT, MIN_PANE_WIDTH, PaneId, PaneState, SplitAxis, SplitNode,
    WorkspaceLayout,
};

/// Shell-only UI state. Business-page types stay out of the base layout module.
#[derive(Clone, Debug, Default)]
pub struct ShellUiState {
    pub settings_dialog_open: bool,
    pub settings_initial_path: Option<crate::page::settings::SettingsPath>,
    pub help_dialog_open: bool,
    pub connections_dialog_open: bool,
    pub commands_manage_dialog_open: bool,
    pub settings_standalone_tab: Option<SettingsTab>,
}

/// Shell geometry/navigation state composed with transient UI state.
#[derive(Clone, Debug)]
pub struct ShellLayout {
    pub function_width: f32,
    pub function_page: FunctionPage,
    pub workspace: WorkspaceLayout,
    pub ui: ShellUiState,
}

impl Default for ShellLayout {
    fn default() -> Self {
        Self {
            function_width: FUNCTION_DEFAULT_WIDTH,
            function_page: FunctionPage::Active,
            workspace: WorkspaceLayout::new_single(),
            ui: ShellUiState::default(),
        }
    }
}

impl ShellLayout {
    pub fn from_settings(function_width: Option<f32>) -> Self {
        Self {
            function_width: function_width
                .unwrap_or(FUNCTION_DEFAULT_WIDTH)
                .clamp(FUNCTION_MIN_WIDTH, FUNCTION_MAX_WIDTH),
            ..Default::default()
        }
    }
}
