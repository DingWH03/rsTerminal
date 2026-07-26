//! Drain pending connection bytes into a terminal session.

use crate::connection::{ConnIn, ConnectionState};
use crate::session::terminal::ActiveSession;
use crate::terminal::parser::TermEvent;

/// Outcomes from draining / viewing a terminal connection (handled by the app/UI).
#[derive(Debug)]
pub enum ConnectionViewAction {
    /// 无操作
    None,
    /// 关闭当前显示的终端会话
    CloseSession,
    /// 分屏模式下隐藏（最小化）当前窗格
    MinimizePane,
    /// 使用给定的已保存连接 ID 重新连接 SSH 会话
    Reconnect(String),
}

impl Default for ConnectionViewAction {
    fn default() -> Self {
        Self::None
    }
}

/// 从连接中读取待处理的字节并应用到终端仿真器。
///
/// 返回 `true` 表示有数据被处理。
pub fn drain_connection(session: &mut ActiveSession, action: &mut ConnectionViewAction) -> bool {
    let mut updated = false;
    let mut pty_data = Vec::new();
    for ev in session.handle.drain() {
        match ev {
            ConnIn::Data(data) => pty_data.extend(data),
            ConnIn::PortsChanged(ports) => {
                session.set_connection_ports(ports);
                updated = true;
            }
            ConnIn::PortData { port, data } => {
                if port == session.active_port {
                    pty_data.extend(data);
                } else {
                    session.receive_inactive_port_data(port, &data);
                    updated = true;
                }
            }
            ConnIn::StateChanged(s) => match s {
                ConnectionState::Error(e) => {
                    session.disconnect_message = Some(e);
                }
                ConnectionState::Lost(m) => {
                    session.disconnect_message = Some(m);
                }
                ConnectionState::Closed => {
                    session.disconnect_message = None;
                    *action = ConnectionViewAction::CloseSession;
                }
                ConnectionState::Connected => {
                    session.disconnect_message = None;
                }
                ConnectionState::Connecting => {}
            },
        }
    }
    if !pty_data.is_empty() {
        session.terminal.write(&pty_data);
        updated = true;
    }
    session.handle.repaint.clear_repaint_pending();
    for resp in session.terminal.drain_pending() {
        match resp {
            TermEvent::Response(data) => session.send_active(data),
            TermEvent::PtyResize { rows: _, cols: _ } => {}
        }
    }
    if let Some(cwd) = session.terminal.screen.cwd.as_deref() {
        if !cwd.is_empty() {
            session.metrics.note_osc_cwd(Some(cwd));
        }
    }
    for ev in session.metrics.drain_events() {
        let line = crate::remote::format_metrics_event(&ev);
        match &ev {
            crate::remote::MetricsEvent::Status(_) => {
                log::debug!("[remote-metrics] {line}");
            }
            _ => {
                log::info!("[remote-metrics] {line}");
            }
        }
        updated = true;
    }
    updated
}
