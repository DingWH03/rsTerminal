//! Sidebar tab identifiers for the function pane.

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FunctionPage {
    /// Active sessions in the workspace.
    #[default]
    Active,
    /// All saved connections.
    Connections,
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
            Self::Files => 2,
            Self::Monitor => 3,
        }
    }

    pub fn from_tab_id(id: usize) -> Self {
        match id {
            1 => Self::Connections,
            2 => Self::Files,
            3 => Self::Monitor,
            _ => Self::Active,
        }
    }
}
