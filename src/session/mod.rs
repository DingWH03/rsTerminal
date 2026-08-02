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

pub use drain::{ConnectionViewAction, drain_connection};
pub use file_manager::{
    FileActivePane, FileClipboard, FileClipboardMode, FileManagerMode, FileManagerSession,
    FilePaneState, FileTransferState, InfoDialog, InfoLine, PaneSide, PaneState, RemotePane,
    RenameDialog,
};
pub use files_cache::{SessionFilesCache, tick_all_session_files, tick_session_files};
pub use terminal::{
    ActiveSession, PortCoreState, PortUiState, TerminalSessionCore, normalize_paste_text,
    paste_payload,
};
// Compatibility exports for the pre-phase-4 session paths.
pub use crate::ui::terminal::{
    CellPos, RowGalleyCache, TerminalSelection, TerminalTouchState, extract_range_text,
};
pub use workspace::{WorkspaceSession, terminal_conn_type};
