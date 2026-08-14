//! Session runtime — re-exports `rsterm-session-core` plus shell workspace hosting.
//!
//! Core types live in `rsterm-session-core`. Terminal / file-manager
//! `WorkspaceContent` adapters live in their page crates.
//! [`WorkspaceSession`] lives in `rsterm-shell` (depends on both page crates).

pub use rsterm_page_file_manager::FileManagerContent;
pub use rsterm_page_terminal::ActiveSessionContent;
pub use rsterm_session_core::*;
pub use rsterm_shell::{WorkspaceSession, terminal_conn_type, tick_all_session_files};
