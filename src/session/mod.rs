//! Session runtime — terminal and file-manager workspace tabs.
//!
//! Owns per-session state and non-UI advancement (`drain`, files tick).
//! UI modules render and route input against these types.

pub mod drain;
pub mod file_manager;
pub mod files_cache;
pub mod galley_cache;
pub mod selection_state;
pub mod terminal;
pub mod workspace;

pub use drain::{drain_connection, ConnectionViewAction};
pub use file_manager::*;
pub use files_cache::{tick_all_session_files, tick_session_files, SessionFilesCache};
pub use galley_cache::RowGalleyCache;
pub use selection_state::{
    extract_range_text, CellPos, TerminalSelection, TerminalTouchState,
};
pub use terminal::{ActiveSession, PortUiState};
pub use workspace::{terminal_conn_type, WorkspaceSession};
