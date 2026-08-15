//! Session runtime — terminal and file-manager state without workspace UI adapters.
//!
//! Owns per-session state and non-UI advancement (`drain`, files tick).
//! Workspace hosting and `WorkspaceContent` impls live in page crates.

pub mod connect_params;
pub mod drain;
pub mod file_manager;
pub mod files_cache;
pub mod terminal;
pub mod transfer;
pub mod view;

pub use connect_params::{ble_params, local_params, serial_params, ssh_auth, ssh_params};
pub use drain::{ConnectionViewAction, drain_connection};
pub use file_manager::{
    FileActivePane, FileClipboard, FileClipboardMode, FileManagerMode, FileManagerSession,
    FilePaneState, FileTransferState, InfoDialog, InfoLine, PaneSide, PaneState, RemotePane,
    RenameDialog,
};
pub use files_cache::{SessionFilesCache, tick_session_files};
pub use terminal::{
    ActiveSession, PortCoreState, PortUiState, TerminalSessionCore, normalize_paste_text,
    paste_payload,
};
pub use transfer::{PasteTarget, TransferDone, apply_transfer_done};
pub use view::{
    CellPos, PortViewState, RowGalleyCache, TerminalSelection, TerminalTouchState,
    TerminalViewState, extract_range_text,
};
