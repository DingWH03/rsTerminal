//! Shell chrome preferences (sidebar width, default local connection).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChromePrefs {
    #[serde(default)]
    pub function_pane_width: Option<f32>,
    #[serde(default)]
    pub default_local_connection_id: Option<String>,
}
