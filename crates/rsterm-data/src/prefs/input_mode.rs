//! Pointer vs touch interaction mode preference.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputInteractionMode {
    Pointer,
    Touch,
}

impl InputInteractionMode {
    pub const ALL: [Self; 2] = [Self::Pointer, Self::Touch];
}

pub fn default_input_mode() -> InputInteractionMode {
    #[cfg(target_os = "android")]
    {
        InputInteractionMode::Touch
    }
    #[cfg(not(target_os = "android"))]
    {
        InputInteractionMode::Pointer
    }
}
