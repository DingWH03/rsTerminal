//! File-manager UI preferences (view mode / pane layout).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrefsFileViewMode {
    #[default]
    List,
    Details,
    IconsSmall,
    IconsLarge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrefsFilePaneLayout {
    Single,
    #[default]
    Dual,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileManagerPrefs {
    #[serde(default)]
    pub view_mode: PrefsFileViewMode,
    #[serde(default)]
    pub pane_layout: PrefsFilePaneLayout,
    #[serde(default)]
    pub show_hidden: bool,
}
