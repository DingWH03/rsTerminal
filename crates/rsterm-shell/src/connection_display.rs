//! UI-facing labels/icons for persist connection types.

use crate::session_host::WorkspaceSession;
use rsterm_data::persist::types::ConnectionType;
use rsterm_session_core::file_manager::FileManagerMode;

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
    if let Some(s) = session.as_terminal() {
        return connection_type_icon(s.core.conn_type);
    }
    if let Some(s) = session.as_file_manager() {
        return match s.mode {
            FileManagerMode::SshSftp => "📁",
            FileManagerMode::LocalDual => "📂",
        };
    }
    "💻"
}
