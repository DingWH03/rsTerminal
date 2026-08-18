//! Workspace session enum — terminal or file-manager tab.

use crate::data::persist::types::ConnectionType;
use crate::session::file_manager::FileManagerSession;
use crate::session::terminal::ActiveSession;

/// 工作区标签页：可以是终端仿真器或文件管理器。
pub enum WorkspaceSession {
    /// 终端仿真会话
    Terminal(ActiveSession),
    /// 文件管理器会话
    FileManager(FileManagerSession),
}

impl WorkspaceSession {
    pub fn id(&self) -> &str {
        match self {
            WorkspaceSession::Terminal(s) => &s.core.id,
            WorkspaceSession::FileManager(s) => &s.id,
        }
    }

    pub fn tab_label(&self) -> String {
        match self {
            WorkspaceSession::Terminal(s) => s.tab_label(),
            WorkspaceSession::FileManager(s) => s.tab_label(),
        }
    }

    pub fn sidebar_has_new_window(&self) -> bool {
        match self {
            WorkspaceSession::Terminal(s) => s.sidebar_has_new_window(),
            WorkspaceSession::FileManager(_) => true,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, WorkspaceSession::Terminal(_))
    }

    pub fn terminal_mut(&mut self) -> Option<&mut ActiveSession> {
        match self {
            WorkspaceSession::Terminal(s) => Some(s),
            _ => None,
        }
    }
}

pub fn terminal_conn_type(session: &WorkspaceSession) -> Option<&ConnectionType> {
    match session {
        WorkspaceSession::Terminal(s) => Some(&s.core.conn_type),
        _ => None,
    }
}
