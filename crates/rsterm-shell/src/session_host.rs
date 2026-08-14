//! Workspace session — thin `WorkspaceContent` owner with typed accessors.
//!
//! Terminal content lives in `rsterm-page-terminal`.
//! File-manager content lives in `rsterm-page-file-manager`.

use rsterm_data::persist::types::{AuthUser, ConnectionType, SavedConnection};
use rsterm_page_file_manager::FileManagerContent;
use rsterm_page_terminal::ActiveSessionContent;
use rsterm_session_core::file_manager::FileManagerSession;
use rsterm_session_core::files_cache::tick_session_files;
use rsterm_session_core::terminal::ActiveSession;
use rsterm_workspace::WorkspaceContent;

/// 工作区标签页：终端或文件管理器（内部为 `dyn WorkspaceContent`）。
pub struct WorkspaceSession {
    inner: Box<dyn WorkspaceContent>,
}

impl WorkspaceSession {
    pub fn terminal(s: ActiveSession) -> Self {
        Self {
            inner: rsterm_page_terminal::wrap_terminal(s),
        }
    }

    pub fn file_manager(s: FileManagerSession) -> Self {
        Self {
            inner: rsterm_page_file_manager::wrap_file_manager(s),
        }
    }

    pub fn from_boxed(inner: Box<dyn WorkspaceContent>) -> Self {
        Self { inner }
    }

    pub fn id(&self) -> &str {
        self.inner.id()
    }

    pub fn tab_label(&self) -> String {
        self.inner.tab_label()
    }

    pub fn sidebar_has_new_window(&self) -> bool {
        self.inner.sidebar_has_new_window()
    }

    pub fn content_mut(&mut self) -> &mut dyn WorkspaceContent {
        &mut *self.inner
    }

    pub fn content(&self) -> &dyn WorkspaceContent {
        &*self.inner
    }

    pub fn as_terminal(&self) -> Option<&ActiveSession> {
        self.inner
            .as_any()
            .downcast_ref::<ActiveSessionContent>()
            .map(|c| &c.inner)
    }

    pub fn as_terminal_mut(&mut self) -> Option<&mut ActiveSession> {
        self.inner
            .as_any_mut()
            .downcast_mut::<ActiveSessionContent>()
            .map(|c| &mut c.inner)
    }

    pub fn as_file_manager(&self) -> Option<&FileManagerSession> {
        self.inner
            .as_any()
            .downcast_ref::<FileManagerContent>()
            .map(|c| &c.inner)
    }

    pub fn as_file_manager_mut(&mut self) -> Option<&mut FileManagerSession> {
        self.inner
            .as_any_mut()
            .downcast_mut::<FileManagerContent>()
            .map(|c| &mut c.inner)
    }

    pub fn is_terminal(&self) -> bool {
        self.as_terminal().is_some()
    }

    pub fn terminal_mut(&mut self) -> Option<&mut ActiveSession> {
        self.as_terminal_mut()
    }
}

pub fn terminal_conn_type(session: &WorkspaceSession) -> Option<&ConnectionType> {
    session.as_terminal().map(|s| &s.core.conn_type)
}

/// Keep every terminal session's file cache in sync with cwd / SFTP replies.
pub fn tick_all_session_files(
    sessions: &mut [WorkspaceSession],
    connections: &[SavedConnection],
    auth_users: &[AuthUser],
) {
    for session in sessions {
        if let Some(term) = session.terminal_mut() {
            tick_session_files(term, connections, auth_users);
        }
    }
}
