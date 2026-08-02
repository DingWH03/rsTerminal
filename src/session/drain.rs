//! Drain pending connection bytes into a terminal session.

use crate::connection::{ConnIn, ConnectionState};
use crate::session::terminal::ActiveSession;
use crate::terminal::parser::TermEvent;

/// Split terminal bytes from connection events without touching session state.
///
/// Untagged data and data for the active logical port share the active terminal
/// stream. All other events retain their original order for stateful handling.
fn collect_terminal_data(events: Vec<ConnIn>, active_port: u8) -> (Vec<u8>, Vec<ConnIn>) {
    let mut terminal_data = Vec::new();
    let mut remaining = Vec::new();

    for event in events {
        match event {
            ConnIn::Data(data) => terminal_data.extend(data),
            ConnIn::PortData { port, data } if port == active_port => {
                terminal_data.extend(data);
            }
            event => remaining.push(event),
        }
    }

    (terminal_data, remaining)
}

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
    let (pty_data, remaining) =
        collect_terminal_data(session.core.handle.drain(), session.core.active_port);
    for ev in remaining {
        match ev {
            ConnIn::PortsChanged(ports) => {
                session.set_connection_ports(ports);
                updated = true;
            }
            ConnIn::PortData { port, data } => {
                session.receive_inactive_port_data(port, &data);
                updated = true;
            }
            ConnIn::StateChanged(s) => match s {
                ConnectionState::Error(e) => {
                    session.core.disconnect_message = Some(e);
                }
                ConnectionState::Lost(m) => {
                    session.core.disconnect_message = Some(m);
                }
                ConnectionState::Closed => {
                    session.core.disconnect_message = None;
                    *action = ConnectionViewAction::CloseSession;
                }
                ConnectionState::Connected => {
                    session.core.disconnect_message = None;
                }
                ConnectionState::Connecting => {}
            },
            ConnIn::Data(_) => unreachable!("terminal data was collected above"),
        }
    }
    if !pty_data.is_empty() {
        session.core.terminal.write(&pty_data);
        updated = true;
    }
    session.core.handle.repaint.clear_repaint_pending();
    for resp in session.core.terminal.drain_pending() {
        match resp {
            TermEvent::Response(data) => session.send_active(data),
            TermEvent::PtyResize { rows: _, cols: _ } => {}
        }
    }
    if let Some(cwd) = session.core.terminal.screen.cwd.as_deref() {
        if !cwd.is_empty() {
            session.core.metrics.note_osc_cwd(Some(cwd));
        }
    }
    for ev in session.core.metrics.drain_events() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::ConnectionPort;

    #[test]
    fn collect_terminal_data_merges_stream_and_active_port_bytes() {
        let events = vec![
            ConnIn::Data(b"hello ".to_vec()),
            ConnIn::PortData {
                port: 2,
                data: b"ignored".to_vec(),
            },
            ConnIn::PortData {
                port: 1,
                data: b"world".to_vec(),
            },
            ConnIn::Data(b"!".to_vec()),
        ];

        let (data, remaining) = collect_terminal_data(events, 1);

        assert_eq!(data, b"hello world!");
        assert_eq!(remaining.len(), 1);
        assert!(matches!(
            &remaining[0],
            ConnIn::PortData { port: 2, data } if data == b"ignored"
        ));
    }

    #[test]
    fn collect_terminal_data_preserves_control_event_order() {
        let ports = vec![ConnectionPort::terminal(3, "shell")];
        let events = vec![
            ConnIn::StateChanged(ConnectionState::Connecting),
            ConnIn::Data(Vec::new()),
            ConnIn::PortsChanged(ports),
            ConnIn::StateChanged(ConnectionState::Connected),
        ];

        let (data, remaining) = collect_terminal_data(events, 3);

        assert!(data.is_empty());
        assert_eq!(remaining.len(), 3);
        assert!(matches!(
            remaining[0],
            ConnIn::StateChanged(ConnectionState::Connecting)
        ));
        assert!(matches!(remaining[1], ConnIn::PortsChanged(_)));
        assert!(matches!(
            remaining[2],
            ConnIn::StateChanged(ConnectionState::Connected)
        ));
    }
}
