//! UI-facing labels/icons for persist connection types.

use crate::data::persist::types::ConnectionType;
use crate::session::file_manager::FileManagerMode;
use crate::session::WorkspaceSession;

pub fn connection_type_label(conn_type: ConnectionType) -> &'static str {
    match conn_type {
        ConnectionType::Local => "Local Terminal",
        ConnectionType::Ssh => "SSH",
        ConnectionType::Serial => "Serial Port",
        ConnectionType::Ble => "BLE Serial",
    }
}

pub fn connection_type_icon(conn_type: ConnectionType) -> &'static str {
    match conn_type {
        ConnectionType::Local => "💻",
        ConnectionType::Ssh => "🌐",
        ConnectionType::Serial => "🔌",
        ConnectionType::Ble => "📶",
    }
}

pub fn workspace_session_icon(session: &WorkspaceSession) -> &str {
    match session {
        WorkspaceSession::Terminal(s) => connection_type_icon(s.conn_type),
        WorkspaceSession::FileManager(s) => match s.mode {
            FileManagerMode::SshSftp => "📁",
            FileManagerMode::LocalDual => "📂",
        },
    }
}
