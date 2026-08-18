//! Session open/close/duplicate/drain/reconnect/file-manager.

use log::info;

use super::RsTerminalApp;
#[cfg(not(target_os = "android"))]
use crate::connection::local;
use crate::connection::ssh;
use crate::data::persist::types::{ConnectionType, SavedConnection};
use crate::session::{
    ActiveSession, ConnectionViewAction, FileManagerMode, FileManagerSession, TerminalSessionCore,
    WorkspaceSession, drain_connection,
};
use crate::terminal::Terminal;
use crate::terminal::{DEFAULT_GRID_COLS, DEFAULT_GRID_ROWS};
use crate::ui::shell::coordinator::ShellCoordinator;
use crate::session::TerminalViewState;
use crate::ui::uiframe::keyboard::VirtualKeyboard;

impl RsTerminalApp {
    pub(crate) fn push_session(&mut self, session: WorkspaceSession) {
        let id = session.id().to_string();
        self.sessions.push(session);
        ShellCoordinator::assign_new_session(&mut self.shell.layout, id);
    }

    pub(crate) fn open_file_manager_ssh(&mut self, conn_id: &str) {
        let config = match self.saved_connections.iter().find(|c| c.id == conn_id) {
            Some(c) => c.clone(),
            None => return,
        };
        let auth_user = config
            .auth_user_id
            .as_ref()
            .and_then(|id| self.auth_users.iter().find(|u| u.id == *id));
        let auth = crate::session::connect_params::ssh_auth(&config, auth_user);
        let host = config.ssh_host.as_deref().unwrap_or("host");
        let port = config.ssh_port.unwrap_or(22);
        match FileManagerSession::open_ssh(host, port, auth, config.id.clone()) {
            Ok(fm) => self.push_session(WorkspaceSession::file_manager(fm)),
            Err(e) => info!("SFTP failed: {e}"),
        }
    }

    pub(crate) fn open_file_manager_local(&mut self) {
        self.push_session(WorkspaceSession::file_manager(
            FileManagerSession::open_local(),
        ));
    }

    #[cfg(not(target_os = "android"))]
    pub(crate) fn reconnect_local_session(&mut self, session_id: &str, config: &SavedConnection) {
        let Some(idx) = self.sessions.iter().position(|s| s.id() == session_id) else {
            return;
        };
        let profile = self.resolve_profile(config.profile_id.as_deref()).clone();
        let Some(term) = self.sessions[idx].as_terminal_mut() else {
            return;
        };
        if term.core.conn_type != ConnectionType::Local {
            return;
        }
        term.core.handle.close();
        let rows = term.view.last_pty_rows.max(1);
        let cols = term.view.last_pty_cols.max(1);
        match local::connect_local(&crate::session::connect_params::local_params(config), rows, cols) {
            Ok(handle) => {
                term.core.handle = handle;
                term.core.saved_conn_id = Some(config.id.clone());
                term.view.profile_id = profile.id.clone();
                term.view.live_font_size = profile.font_size;
                term.core.name = config.name.clone();
                term.core.user_at_host = crate::platform::get().local_user_at_host();
                term.view.want_terminal_focus = true;
                term.view.selection = None;
                term.view.selection_pointer = None;
            }
            Err(e) => term.core.disconnect_message = Some(e),
        }
    }

    pub(crate) fn duplicate_session(&mut self, session_id: &str) {
        enum DupPlan {
            #[cfg(not(target_os = "android"))]
            TerminalLocal,
            TerminalSsh(String),
            FileSsh(String),
            FileLocal,
        }
        let plan = self
            .sessions
            .iter()
            .find(|s| s.id() == session_id)
            .and_then(|s| {
                if let Some(term) = s.as_terminal() {
                    match term.core.conn_type {
                        #[cfg(not(target_os = "android"))]
                        ConnectionType::Local => Some(DupPlan::TerminalLocal),
                        #[cfg(target_os = "android")]
                        ConnectionType::Local => None,
                        ConnectionType::Ssh => {
                            term.core.saved_conn_id.clone().map(DupPlan::TerminalSsh)
                        }
                        ConnectionType::Serial | ConnectionType::Ble => None,
                    }
                } else if let Some(fm) = s.as_file_manager() {
                    match fm.mode {
                        FileManagerMode::SshSftp => fm.saved_conn_id.clone().map(DupPlan::FileSsh),
                        FileManagerMode::LocalDual => Some(DupPlan::FileLocal),
                    }
                } else {
                    None
                }
            });
        match plan {
            #[cfg(not(target_os = "android"))]
            Some(DupPlan::TerminalLocal) => self.connect_local(),
            Some(DupPlan::TerminalSsh(id)) => self.connect_to(&id),
            Some(DupPlan::FileSsh(id)) => self.open_file_manager_ssh(&id),
            Some(DupPlan::FileLocal) => self.open_file_manager_local(),
            None => {}
        }
    }

    pub(crate) fn open_session(
        &mut self,
        handle: crate::connection::ConnectionHandle,
        config: &SavedConnection,
        scrollback_lines: usize,
    ) {
        self.open_session_in_pane(
            handle,
            config,
            scrollback_lines,
            self.shell.layout.workspace.focused_pane,
            None,
        );
    }

    pub(crate) fn open_session_in_pane(
        &mut self,
        handle: crate::connection::ConnectionHandle,
        config: &SavedConnection,
        scrollback_lines: usize,
        pane: crate::ui::layout::PaneId,
        ssh_extras: Option<(
            crate::remote::SessionMetrics,
            std::sync::Arc<crate::fs::sftp::SftpClient>,
        )>,
    ) {
        let profile = self.resolve_profile(config.profile_id.as_deref());
        let profile_id = profile.id.clone();
        let live_font_size = profile.font_size;
        let keyboard_mode = profile.keyboard_mode;
        let mut terminal = Terminal::new(DEFAULT_GRID_ROWS, DEFAULT_GRID_COLS);
        terminal.set_scrollback_limit(scrollback_lines);
        self.live_font_size = live_font_size;
        self.virtual_keyboard = VirtualKeyboard::new(keyboard_mode);

        let user_at_host = match config.conn_type {
            ConnectionType::Local => crate::platform::get().local_user_at_host(),
            ConnectionType::Ssh => {
                let user = config.ssh_user.as_deref().unwrap_or("root");
                let host = config.ssh_host.as_deref().unwrap_or("host");
                crate::platform::get().ssh_user_at_host(user, host)
            }
            _ => String::new(),
        };

        let (metrics, session_sftp) = match ssh_extras {
            Some((m, s)) => (m, Some(s)),
            None => (crate::remote::SessionMetrics::new(), None),
        };

        let id = uuid::Uuid::new_v4().to_string();
        self.sessions
            .push(WorkspaceSession::terminal(ActiveSession::new(
                TerminalSessionCore {
                    id: id.clone(),
                    conn_type: config.conn_type,
                    disconnect_message: None,
                    saved_conn_id: Some(config.id.clone()),
                    name: config.name.clone(),
                    user_at_host,
                    handle,
                    terminal,
                    active_port: 0,
                    ports: Vec::new(),
                    inactive_port_states: Default::default(),
                    port_unread: Default::default(),
                    scrollback_lines,
                    metrics,
                    session_sftp,
                    files: Default::default(),
                },
                TerminalViewState {
                    profile_id,
                    live_font_size,
                    inactive_port_states: Default::default(),
                    scroll_offset: 0,
                    selection: None,
                    selection_pointer: None,
                    touch_state: Default::default(),
                    want_terminal_focus: true,
                    terminal_had_focus: false,
                    row_galley_cache: Default::default(),
                    layout_font_size: live_font_size,
                    grid_rows: DEFAULT_GRID_ROWS,
                    grid_cols: DEFAULT_GRID_COLS,
                    last_pty_rows: DEFAULT_GRID_ROWS as u16,
                    last_pty_cols: DEFAULT_GRID_COLS as u16,
                    size_label_dims: (DEFAULT_GRID_COLS, DEFAULT_GRID_ROWS),
                    size_label_hide_at: None,
                    size_label_active: false,
                    mouse_motion_last: None,
                    font_generation: crate::fonts::font_generation(),
                },
            )));
        ShellCoordinator::assign_session_to_pane(&mut self.shell.layout, pane, id);
    }

    pub(crate) fn has_open_sessions(&self) -> bool {
        !self.sessions.is_empty()
    }

    pub(crate) fn close_all_sessions(&mut self) {
        let ids: Vec<String> = self.sessions.iter().map(|s| s.id().to_string()).collect();
        for id in ids {
            self.close_session(&id);
        }
    }

    pub(crate) fn close_pane_and_maybe_session(&mut self, pane: crate::ui::layout::PaneId) {
        let sid = self
            .shell
            .layout
            .workspace
            .panes
            .get(&pane)
            .and_then(|p| p.session_id.clone());
        if self.shell.layout.workspace.pane_count() > 1 {
            self.shell.layout.workspace.close_pane(pane);
        }
        if let Some(id) = sid {
            self.close_session(&id);
        }
    }

    pub(crate) fn close_session(&mut self, id: &str) {
        if let Some(pos) = self.sessions.iter().position(|s| s.id() == id) {
            if let Some(s) = self.sessions[pos].as_terminal_mut() {
                s.core.handle.close();
            }
            self.sessions.remove(pos);
        }
        ShellCoordinator::on_sessions_closed(&mut self.shell.layout, id);
        if self.sessions.is_empty() {
            self.save_profile_tweaks();
        }
    }

    pub(crate) fn drain_inactive_sessions(&mut self) {
        let active = self.shell.focused_session_id();
        for session in &mut self.sessions {
            if active == Some(session.id()) {
                continue;
            }
            if let Some(term) = session.terminal_mut() {
                let mut noop = ConnectionViewAction::None;
                drain_connection(term, &mut noop);
            }
        }
    }

    pub(crate) fn focused_session_index(&self) -> Option<usize> {
        self.shell
            .focused_session_id()
            .and_then(|id| self.sessions.iter().position(|s| s.id() == id))
    }

    pub(crate) fn reconnect_ssh_session(&mut self, pane: crate::ui::layout::PaneId, conn_id: &str) {
        let Some(config) = self
            .saved_connections
            .iter()
            .find(|c| c.id == *conn_id)
            .cloned()
        else {
            return;
        };
        let auth_user = config
            .auth_user_id
            .as_ref()
            .and_then(|id| self.auth_users.iter().find(|u| u.id == *id));
        let auth = crate::session::connect_params::ssh_auth(&config, auth_user);
        let params = match crate::session::connect_params::ssh_params(&config) {
            Ok(p) => p,
            Err(e) => {
                self.connection_notice = Some(e);
                return;
            }
        };
        let Some(sid) = self
            .shell
            .layout
            .workspace
            .panes
            .get(&pane)
            .and_then(|p| p.session_id.clone())
        else {
            return;
        };
        let Some(idx) = self.sessions.iter().position(|s| s.id() == sid) else {
            return;
        };
        let Some(session) = self.sessions[idx].as_terminal_mut() else {
            return;
        };
        if !matches!(session.core.conn_type, ConnectionType::Ssh) {
            return;
        }
        match ssh::connect_ssh_session(&params, auth, 24, 80) {
            Ok(out) => {
                session.core.handle = out.handle;
                session.core.metrics = out.metrics;
                session.core.session_sftp = Some(std::sync::Arc::new(
                    crate::fs::sftp::SftpClient::from_endpoint(out.sftp_endpoint),
                ));
                session.core.files.invalidate_pending();
                session.core.disconnect_message = None;
                session.view.want_terminal_focus = true;
            }
            Err(e) => self.connection_notice = Some(e),
        }
    }
}
