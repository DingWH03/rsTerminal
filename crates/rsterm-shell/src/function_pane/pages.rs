//! Sidebar tab identifiers for the function pane.

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FunctionPage {
    /// Active sessions in the workspace.
    #[default]
    Active,
    /// All saved connections.
    Connections,
    /// Favorite commands for quick input.
    Commands,
    /// Single-column files for the focused terminal (wide layout only).
    Files,
    /// One-minute remote performance charts (SSH, wide layout).
    Monitor,
}

impl FunctionPage {
    pub fn as_tab_id(self) -> usize {
        match self {
            Self::Active => 0,
            Self::Connections => 1,
            Self::Commands => 2,
            Self::Files => 3,
            Self::Monitor => 4,
        }
    }

    pub fn from_tab_id(id: usize) -> Self {
        match id {
            1 => Self::Connections,
            2 => Self::Commands,
            3 => Self::Files,
            4 => Self::Monitor,
            _ => Self::Active,
        }
    }
}
