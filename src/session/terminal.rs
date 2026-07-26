//! Runtime terminal session state (no egui painting).

use std::collections::BTreeMap;
use std::time::Instant;

use crate::connection::{ConnectionPort, ConnectionPortKind};
use crate::session::files_cache::SessionFilesCache;
use crate::session::galley_cache::RowGalleyCache;
use crate::session::selection_state::{CellPos, TerminalSelection, TerminalTouchState};
use crate::storage::types::ConnectionType;
use crate::terminal::Terminal;

/// 单个端口（多路复用连接中的子通道）的 UI 状态。
///
/// 用于 BLE 多 UART 等支持多路复用传输的场景，
/// 每个端口拥有独立的终端仿真器、滚动偏移和选择状态。
pub struct PortUiState {
    /// 端口号（0-based）
    pub port: u8,
    /// 端口显示标签
    pub label: String,
    /// 端口类型（如 UART、SPI 等）
    pub kind: ConnectionPortKind,
    /// 该端口的终端仿真器
    pub terminal: Terminal,
    /// 当前滚动偏移量
    pub scroll_offset: usize,
    /// 当前文本选择
    pub selection: Option<TerminalSelection>,
    /// 选择锚点指针位置
    pub selection_pointer: Option<CellPos>,
    /// 触摸交互状态
    pub touch_state: TerminalTouchState,
    /// 行字形缓存（避免重复布局）
    pub row_galley_cache: RowGalleyCache,
    /// 上次鼠标运动位置（用于去重）
    pub mouse_motion_last: Option<(usize, usize)>,
}

impl PortUiState {
    /// 创建新的端口 UI 状态，初始化终端仿真器。
    pub(crate) fn new(
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
            scroll_offset: 0,
            selection: None,
            selection_pointer: None,
            touch_state: TerminalTouchState::default(),
            row_galley_cache: Default::default(),
            mouse_motion_last: None,
        }
    }
}

/// 活跃终端会话 — 管理单个终端标签页的所有状态。
///
/// 包含终端仿真器、连接句柄、滚动、选择、触摸状态、
/// 多端口支持（BLE 多 UART）以及字体/网格尺寸管理。
pub struct ActiveSession {
    /// 会话唯一标识
    pub id: String,
    /// 连接类型（Local/SSH/Serial/BLE）
    pub conn_type: ConnectionType,
    /// 连接断开时的错误消息，显示在终端面板中直到用户关闭标签页
    pub disconnect_message: Option<String>,
    /// 源已保存连接 ID（用于 SSH「新窗口」功能）；本地连接可能为空
    pub saved_conn_id: Option<String>,
    /// 已保存连接的显示名称（Serial/BLE 标签页标题）
    pub name: String,
    /// 空闲标签页标签（Local/SSH 显示为 `user@host`）
    pub user_at_host: String,
    /// 底层连接句柄，用于收发数据
    pub handle: crate::connection::ConnectionHandle,
    /// 活跃端口的终端仿真器
    pub terminal: Terminal,
    /// 当前活跃的逻辑端口号（用于 BLE 多 UART 等多路复用传输）
    pub active_port: u8,
    /// 传输层通告的端口列表。空列表表示经典单流连接。
    pub ports: Vec<ConnectionPort>,
    /// 非活跃端口的终端状态。活跃端口的状态保存在 `terminal` 及相关字段中。
    pub inactive_port_states: BTreeMap<u8, PortUiState>,
    /// 非活跃端口收到的字节计数器（用于显示未读标记）
    pub port_unread: BTreeMap<u8, usize>,
    /// 回滚缓冲区行数
    pub scrollback_lines: usize,
    /// 当前回滚滚动偏移量
    pub scroll_offset: usize,
    /// 当前文本选择
    pub selection: Option<TerminalSelection>,
    /// 选择锚点指针位置
    pub selection_pointer: Option<CellPos>,
    /// Android 触摸状态：回滚拖动、长按选择模式和手势清理
    pub touch_state: TerminalTouchState,
    /// 连接/点击后请求终端区域获得键盘焦点
    pub want_terminal_focus: bool,
    /// 上一帧终端区域是否拥有键盘焦点（用于快捷键路由）
    pub terminal_had_focus: bool,
    /// 行字形缓存（避免重复布局计算）
    pub row_galley_cache: RowGalleyCache,
    /// 上次布局使用的字体大小（检测 A+/A− 变化并立即重排）
    pub layout_font_size: f32,
    /// 上次推送给 PTY 的行数（避免冗余调整大小）
    pub last_pty_rows: u16,
    /// 上次推送给 PTY 的列数
    pub last_pty_cols: u16,
    /// 尺寸叠加层显示的网格尺寸（`cols×rows`）
    pub size_label_dims: (usize, usize),
    /// 尺寸稳定后隐藏叠加层的时间点
    pub size_label_hide_at: Option<Instant>,
    /// 用户是否至少调整过一次尺寸（连接时抑制叠加层显示）
    pub size_label_active: bool,
    /// 仿真器网格行数（第一次布局后与 PTY 匹配）
    pub grid_rows: usize,
    /// 仿真器网格列数
    pub grid_cols: usize,
    /// 上次报告的鼠标位置（用于 xterm 鼠标运动去重）
    pub mouse_motion_last: Option<(usize, usize)>,
    /// 上次应用的字体生成号（字体变化时清除字形缓存）
    pub font_generation: u32,
    /// SSH remote status bus (agent + OSC merger). Empty for non-SSH sessions.
    pub metrics: crate::remote::SessionMetrics,
    /// Shared-session SFTP when connected via `connect_ssh_session`.
    pub session_sftp: Option<std::sync::Arc<crate::fs::sftp::SftpClient>>,
    /// Reactive sidebar file listing for this session (cwd-driven, not UI-driven).
    pub files: SessionFilesCache,
}

impl ActiveSession {
    fn port_info(&self, port: u8) -> Option<&ConnectionPort> {
        self.ports.iter().find(|p| p.port == port)
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

    fn blank_port_state(&self, port: u8) -> PortUiState {
        PortUiState::new(
            port,
            self.port_label(port),
            self.port_kind(port),
            self.grid_rows,
            self.grid_cols,
            self.scrollback_lines,
        )
    }

    fn take_current_port_state(&mut self) -> PortUiState {
        let mut placeholder = Terminal::new(self.grid_rows.max(1), self.grid_cols.max(1));
        placeholder.set_scrollback_limit(self.scrollback_lines);
        PortUiState {
            port: self.active_port,
            label: self.port_label(self.active_port),
            kind: self.port_kind(self.active_port),
            terminal: std::mem::replace(&mut self.terminal, placeholder),
            scroll_offset: self.scroll_offset,
            selection: self.selection.take(),
            selection_pointer: self.selection_pointer.take(),
            touch_state: std::mem::take(&mut self.touch_state),
            row_galley_cache: std::mem::take(&mut self.row_galley_cache),
            mouse_motion_last: self.mouse_motion_last.take(),
        }
    }

    fn restore_port_state(&mut self, state: PortUiState) {
        self.active_port = state.port;
        self.terminal = state.terminal;
        self.scroll_offset = state.scroll_offset;
        self.selection = state.selection;
        self.selection_pointer = state.selection_pointer;
        self.touch_state = state.touch_state;
        self.row_galley_cache = state.row_galley_cache;
        self.mouse_motion_last = state.mouse_motion_last;
        self.port_unread.remove(&self.active_port);
    }

    pub fn set_connection_ports(&mut self, ports: Vec<ConnectionPort>) {
        if ports.is_empty() {
            return;
        }
        self.ports = ports;
        if !self.ports.iter().any(|p| p.port == self.active_port) {
            let next = self.ports[0].port;
            self.switch_to_port(next);
        }
        let known: Vec<u8> = self.ports.iter().map(|p| p.port).collect();
        self.inactive_port_states
            .retain(|port, _| known.contains(port));
    }

    fn ensure_port_known(&mut self, port: u8) {
        if self.ports.iter().any(|p| p.port == port) {
            return;
        }
        self.ports.push(ConnectionPort {
            port,
            name: format!("Port {port}"),
            kind: ConnectionPortKind::Unknown,
            read_only: false,
            write_only: false,
        });
        self.ports.sort_by_key(|p| p.port);
    }

    pub fn switch_to_port(&mut self, port: u8) {
        if port == self.active_port {
            self.port_unread.remove(&port);
            return;
        }
        self.ensure_port_known(port);
        let current = self.take_current_port_state();
        self.inactive_port_states.insert(current.port, current);
        let next = self
            .inactive_port_states
            .remove(&port)
            .unwrap_or_else(|| self.blank_port_state(port));
        self.restore_port_state(next);
    }

    pub fn receive_inactive_port_data(&mut self, port: u8, data: &[u8]) {
        self.ensure_port_known(port);
        if !self.inactive_port_states.contains_key(&port) {
            let state = self.blank_port_state(port);
            self.inactive_port_states.insert(port, state);
        }
        if let Some(state) = self.inactive_port_states.get_mut(&port) {
            state.terminal.write(data);
        }
        *self.port_unread.entry(port).or_insert(0) += data.len();
    }

    pub fn send_active(&self, data: Vec<u8>) {
        if self.ports.is_empty() {
            self.handle.send(data);
        } else {
            self.handle.send_to_port(self.active_port, data);
        }
    }

    pub fn clear_all_galley_caches(&mut self) {
        self.row_galley_cache.clear();
        for state in self.inactive_port_states.values_mut() {
            state.row_galley_cache.clear();
        }
    }

    /// Sidebar tab: serial/BLE → connection name; local/SSH → running command or `user@host`.
    pub fn tab_label(&self) -> String {
        match self.conn_type {
            ConnectionType::Serial | ConnectionType::Ble => self.name.clone(),
            ConnectionType::Local | ConnectionType::Ssh => {
                if let Some(cmd) = crate::platform::get().foreground_command(self.handle.shell_pid) {
                    return crate::platform::get().truncate_label(&cmd, 32);
                }
                let title = self.terminal.screen.title.trim();
                if !title.is_empty() && !crate::platform::get().title_is_idle_host(title, &self.user_at_host) {
                    return crate::platform::get().truncate_label(title, 32);
                }
                self.user_at_host.clone()
            }
        }
    }

    /// Sidebar row: local / SSH get「新窗口」; serial / BLE only close.
    pub fn sidebar_has_new_window(&self) -> bool {
        matches!(self.conn_type, ConnectionType::Local | ConnectionType::Ssh)
    }
}

