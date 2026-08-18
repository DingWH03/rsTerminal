//! Runtime terminal session state and its UI-state facade.

use std::collections::BTreeMap;

use crate::connection::{ConnectionPort, ConnectionPortKind};
use crate::data::persist::types::ConnectionType;
use crate::session::files_cache::SessionFilesCache;
use crate::terminal::Terminal;
use crate::ui::terminal::{PortViewState, TerminalViewState};

pub struct PortCoreState {
    pub port: u8,
    pub label: String,
    pub kind: ConnectionPortKind,
    pub terminal: Terminal,
}

impl PortCoreState {
    fn new(
        port: u8,
        label: impl Into<String>,
        kind: ConnectionPortKind,
        rows: usize,
        cols: usize,
        scrollback_lines: usize,
    ) -> Self {
        let mut terminal = Terminal::new(rows.max(1), cols.max(1));
        terminal.set_scrollback_limit(scrollback_lines);
        Self {
            port,
            label: label.into(),
            kind,
            terminal,
        }
    }
}

/// Compatibility container for callers that previously handled one combined port state.
pub struct PortUiState {
    pub core: PortCoreState,
    pub view: PortViewState,
}

/// UI-independent runtime and transport state for a terminal session.
pub struct TerminalSessionCore {
    pub id: String,
    pub conn_type: ConnectionType,
    pub disconnect_message: Option<String>,
    pub saved_conn_id: Option<String>,
    pub name: String,
    pub user_at_host: String,
    pub handle: crate::connection::ConnectionHandle,
    pub terminal: Terminal,
    pub active_port: u8,
    pub ports: Vec<ConnectionPort>,
    pub inactive_port_states: BTreeMap<u8, PortCoreState>,
    pub port_unread: BTreeMap<u8, usize>,
    pub scrollback_lines: usize,
    pub metrics: crate::remote::SessionMetrics,
    pub session_sftp: Option<std::sync::Arc<crate::fs::sftp::SftpClient>>,
    pub files: SessionFilesCache,
}

/// Public facade kept on hot paths so callers do not need two mutable borrows.
pub struct ActiveSession {
    pub core: TerminalSessionCore,
    pub view: TerminalViewState,
}

impl ActiveSession {
    pub fn new(core: TerminalSessionCore, view: TerminalViewState) -> Self {
        Self { core, view }
    }

    pub fn core(&self) -> &TerminalSessionCore {
        &self.core
    }

    pub fn core_mut(&mut self) -> &mut TerminalSessionCore {
        &mut self.core
    }

    pub fn view(&self) -> &TerminalViewState {
        &self.view
    }

    pub fn view_mut(&mut self) -> &mut TerminalViewState {
        &mut self.view
    }

    pub fn terminal(&self) -> &Terminal {
        &self.core.terminal
    }

    pub fn terminal_mut(&mut self) -> &mut Terminal {
        &mut self.core.terminal
    }

    fn port_info(&self, port: u8) -> Option<&ConnectionPort> {
        self.core.ports.iter().find(|p| p.port == port)
    }

    fn port_label(&self, port: u8) -> String {
        self.port_info(port)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| format!("Port {port}"))
    }

    fn port_kind(&self, port: u8) -> ConnectionPortKind {
        self.port_info(port)
            .map(|p| p.kind)
            .unwrap_or(ConnectionPortKind::Unknown)
    }

    fn blank_port_core(&self, port: u8) -> PortCoreState {
        PortCoreState::new(
            port,
            self.port_label(port),
            self.port_kind(port),
            self.view.grid_rows,
            self.view.grid_cols,
            self.core.scrollback_lines,
        )
    }

    fn take_current_port_core(&mut self) -> PortCoreState {
        let mut placeholder = Terminal::new(self.view.grid_rows.max(1), self.view.grid_cols.max(1));
        placeholder.set_scrollback_limit(self.core.scrollback_lines);
        PortCoreState {
            port: self.core.active_port,
            label: self.port_label(self.core.active_port),
            kind: self.port_kind(self.core.active_port),
            terminal: std::mem::replace(&mut self.core.terminal, placeholder),
        }
    }

    fn take_current_port_view(&mut self) -> PortViewState {
        PortViewState {
            scroll_offset: self.view.scroll_offset,
            selection: self.view.selection.take(),
            selection_pointer: self.view.selection_pointer.take(),
            touch_state: std::mem::take(&mut self.view.touch_state),
            row_galley_cache: std::mem::take(&mut self.view.row_galley_cache),
            mouse_motion_last: self.view.mouse_motion_last.take(),
        }
    }

    fn restore_port(&mut self, core: PortCoreState, view: PortViewState) {
        self.core.active_port = core.port;
        self.core.terminal = core.terminal;
        self.view.scroll_offset = view.scroll_offset;
        self.view.selection = view.selection;
        self.view.selection_pointer = view.selection_pointer;
        self.view.touch_state = view.touch_state;
        self.view.row_galley_cache = view.row_galley_cache;
        self.view.mouse_motion_last = view.mouse_motion_last;
        self.core.port_unread.remove(&self.core.active_port);
    }

    pub fn set_connection_ports(&mut self, ports: Vec<ConnectionPort>) {
        if ports.is_empty() {
            return;
        }
        self.core.ports = ports;
        if !self
            .core
            .ports
            .iter()
            .any(|p| p.port == self.core.active_port)
        {
            self.switch_to_port(self.core.ports[0].port);
        }
        let known: Vec<u8> = self.core.ports.iter().map(|p| p.port).collect();
        self.core
            .inactive_port_states
            .retain(|port, _| known.contains(port));
        self.view
            .inactive_port_states
            .retain(|port, _| known.contains(port));
    }

    fn ensure_port_known(&mut self, port: u8) {
        if self.core.ports.iter().any(|p| p.port == port) {
            return;
        }
        self.core.ports.push(ConnectionPort {
            port,
            name: format!("Port {port}"),
            kind: ConnectionPortKind::Unknown,
            read_only: false,
            write_only: false,
        });
        self.core.ports.sort_by_key(|p| p.port);
    }

    pub fn switch_to_port(&mut self, port: u8) {
        if port == self.core.active_port {
            self.core.port_unread.remove(&port);
            return;
        }
        self.ensure_port_known(port);
        let current_port = self.core.active_port;
        let current_core = self.take_current_port_core();
        let current_view = self.take_current_port_view();
        self.core
            .inactive_port_states
            .insert(current_port, current_core);
        self.view
            .inactive_port_states
            .insert(current_port, current_view);
        let next_core = self
            .core
            .inactive_port_states
            .remove(&port)
            .unwrap_or_else(|| self.blank_port_core(port));
        let next_view = self
            .view
            .inactive_port_states
            .remove(&port)
            .unwrap_or_default();
        self.restore_port(next_core, next_view);
    }

    pub fn receive_inactive_port_data(&mut self, port: u8, data: &[u8]) {
        self.ensure_port_known(port);
        if !self.core.inactive_port_states.contains_key(&port) {
            let state = self.blank_port_core(port);
            self.core.inactive_port_states.insert(port, state);
        }
        self.view.inactive_port_states.entry(port).or_default();
        if let Some(state) = self.core.inactive_port_states.get_mut(&port) {
            state.terminal.write(data);
        }
        *self.core.port_unread.entry(port).or_insert(0) += data.len();
    }

    pub fn send_active(&self, data: Vec<u8>) {
        if self.core.ports.is_empty() {
            self.core.handle.send(data);
        } else {
            self.core.handle.send_to_port(self.core.active_port, data);
        }
    }

    pub fn paste_text(&mut self, text: &str) -> crate::session::ConnectionViewAction {
        let bracketed = self.core.terminal.screen.bracketed_paste_enabled()
            && self.core.terminal.screen.in_alternate_screen();
        self.send_active(paste_payload(text, bracketed));
        let mut action = crate::session::ConnectionViewAction::None;
        let _ = crate::session::drain_connection(self, &mut action);
        action
    }

    pub fn clear_all_galley_caches(&mut self) {
        self.view.row_galley_cache.clear();
        for state in self.view.inactive_port_states.values_mut() {
            state.row_galley_cache.clear();
        }
    }

    pub fn tab_label(&self) -> String {
        match self.core.conn_type {
            ConnectionType::Serial | ConnectionType::Ble => self.core.name.clone(),
            ConnectionType::Local | ConnectionType::Ssh => {
                if let Some(cmd) =
                    crate::platform::get().foreground_command(self.core.handle.shell_pid)
                {
                    return crate::platform::get().truncate_label(&cmd, 32);
                }
                let title = self.core.terminal.screen.title.trim();
                if !title.is_empty()
                    && !crate::platform::get().title_is_idle_host(title, &self.core.user_at_host)
                {
                    return crate::platform::get().truncate_label(title, 32);
                }
                self.core.user_at_host.clone()
            }
        }
    }

    pub fn sidebar_has_new_window(&self) -> bool {
        matches!(
            self.core.conn_type,
            ConnectionType::Local | ConnectionType::Ssh
        )
    }
}

pub fn normalize_paste_text(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

pub fn paste_payload(text: &str, bracketed: bool) -> Vec<u8> {
    let normalized = normalize_paste_text(text);
    if bracketed {
        let mut bytes = Vec::with_capacity(normalized.len() + 14);
        bytes.extend_from_slice(b"\x1b[200~");
        bytes.extend_from_slice(normalized.as_bytes());
        bytes.extend_from_slice(b"\x1b[201~");
        bytes
    } else {
        normalized.into_bytes()
    }
}
