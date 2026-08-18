//! Workspace session — thin `WorkspaceContent` owner with typed accessors.

use crate::data::persist::types::ConnectionType;
use crate::session::file_manager::FileManagerSession;
use crate::session::terminal::ActiveSession;
use rsterm_workspace::WorkspaceContent;

/// 工作区标签页：终端或文件管理器（内部为 `dyn WorkspaceContent`）。
pub struct WorkspaceSession {
    inner: Box<dyn WorkspaceContent>,
}

impl WorkspaceSession {
    pub fn terminal(s: ActiveSession) -> Self {
        Self { inner: Box::new(s) }
    }

    pub fn file_manager(s: FileManagerSession) -> Self {
        Self { inner: Box::new(s) }
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
        self.inner.as_any().downcast_ref()
    }

    pub fn as_terminal_mut(&mut self) -> Option<&mut ActiveSession> {
        self.inner.as_any_mut().downcast_mut()
    }

    pub fn as_file_manager(&self) -> Option<&FileManagerSession> {
        self.inner.as_any().downcast_ref()
    }

    pub fn as_file_manager_mut(&mut self) -> Option<&mut FileManagerSession> {
        self.inner.as_any_mut().downcast_mut()
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
