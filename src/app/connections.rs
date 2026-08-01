//! Connect / save-connection CRUD / local terminal settings apply.

use super::RsTerminalApp;
use crate::connection::{ble, serial, ssh};
#[cfg(not(target_os = "android"))]
use crate::connection::local;
use crate::persist::types::ConnectionType;
use crate::prefs::save_prefs;
use crate::ui::function_pane::pages::FunctionPage;

impl RsTerminalApp {
    #[cfg(not(target_os = "android"))]
    pub(crate) fn connect_local(&mut self) {
        let Some(config) = self
            .saved_connections
            .iter()
            .find(|c| c.conn_type == ConnectionType::Local)
            .cloned()
        else {
            self.connection_notice = Some(
                "No saved Local Terminal connection. Add one via the + button.".into(),
            );
            return;
        };
        let profile = self.resolve_profile(config.profile_id.as_deref()).clone();
        match local::connect_local(&config, 24, 80) {
            Ok(handle) => self.open_session(handle, &config, profile.scrollback_lines),
            Err(e) => self.connection_notice = Some(e),
        }
    }

    pub(crate) fn apply_local_terminal_settings(
        &mut self,
        apply: crate::ui::page::dialogs::LocalTerminalSettingsApply,
    ) {
        if self
            .saved_connections
            .iter()
            .any(|c| c.id == apply.config.id)
        {
            if let Some(pos) = self
                .saved_connections
                .iter()
                .position(|c| c.id == apply.config.id)
            {
                self.saved_connections[pos] = apply.config.clone();
            }
            let _ = self.persist.upsert_connection(&apply.config);
            self.prefs.default_local_connection_id = Some(apply.config.id.clone());
            save_prefs(&self.prefs);
        }
        #[cfg(not(target_os = "android"))]
        if let Some(session_id) = &apply.session_id {
            self.reconnect_local_session(session_id, &apply.config);
        }
    }

    pub(crate) fn connect_to(&mut self, conn_id: &str) {
        self.connect_to_pane(conn_id, self.shell.layout.workspace.focused_pane);
    }

    pub(crate) fn connect_to_pane(
        &mut self,
        conn_id: &str,
        pane: crate::ui::shell::layout_state::PaneId,
    ) {
        let config = match self.saved_connections.iter().find(|c| c.id == conn_id) {
            Some(c) => c.clone(),
            None => return,
        };
        let profile = self.resolve_profile(config.profile_id.as_deref()).clone();
        match config.conn_type {
            #[cfg(not(target_os = "android"))]
            ConnectionType::Local => match local::connect_local(&config, 24, 80) {
                Ok(handle) => {
                    self.open_session_in_pane(handle, &config, profile.scrollback_lines, pane, None)
                }
                Err(e) => self.connection_notice = Some(e),
            },
            #[cfg(target_os = "android")]
            ConnectionType::Local => {
                self.connection_notice =
                    Some("Local terminal is not supported on Android".into());
            }
            ConnectionType::Ssh => {
                let auth = config
                    .auth_user_id
                    .as_ref()
                    .and_then(|id| self.auth_users.iter().find(|u| u.id == *id))
                    .cloned();
                match ssh::connect_ssh_session(
                    &config,
                    &config.env_vars,
                    24,
                    80,
                    auth.as_ref(),
                ) {
                    Ok(out) => self.open_session_in_pane(
                        out.handle,
                        &config,
                        profile.scrollback_lines,
                        pane,
                        Some((out.metrics, out.sftp)),
                    ),
                    Err(e) => self.connection_notice = Some(e),
                }
            }
            ConnectionType::Serial => match serial::connect_serial(&config) {
                Ok(handle) => {
                    self.open_session_in_pane(handle, &config, profile.scrollback_lines, pane, None)
                }
                Err(e) => self.connection_notice = Some(e),
            },
            ConnectionType::Ble => match ble::connect_ble(&config) {
                Ok(handle) => {
                    self.open_session_in_pane(handle, &config, profile.scrollback_lines, pane, None)
                }
                Err(e) => self.connection_notice = Some(e),
            },
        }
    }

    pub(crate) fn save_connection(&mut self, new_conn: crate::persist::types::SavedConnection) {
        if let Some(pos) = self
            .saved_connections
            .iter()
            .position(|c| c.id == new_conn.id)
        {
            self.saved_connections[pos] = new_conn.clone();
        } else {
            self.saved_connections.push(new_conn.clone());
        }
        let _ = self.persist.upsert_connection(&new_conn);
    }

    pub(crate) fn delete_connection(&mut self, id: &str) {
        self.saved_connections.retain(|c| c.id != *id);
        let _ = self.persist.delete_connection(id);
    }

    pub(crate) fn open_file_manager_for_connection(&mut self, id: &str) {
        if let Some(conn) = self.saved_connections.iter().find(|c| c.id == *id) {
            match conn.conn_type {
                ConnectionType::Local => self.open_file_manager_local(),
                ConnectionType::Ssh => self.open_file_manager_ssh(id),
                _ => {}
            }
        }
        self.shell.layout.function_page = FunctionPage::Active;
    }
}
