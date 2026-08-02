//! 首页 — 已保存连接的展示与操作入口。
//!
//! 提供连接卡片的渲染、筛选（按类型）、排序（收藏优先/最近使用/字母序）、
//! 收藏切换、编辑、删除、SFTP 远程文件管理以及浮动操作按钮（FAB）等功能。

pub mod recent;

use crate::data::persist::types::{ConnectionType, SavedConnection};
use crate::ui::connection_display::connection_type_label;

/// 构建连接副标题，组合连接类型和关键详细信息。
///
/// 根据连接类型显示不同的详细信息：
/// - SSH：user@host:port
/// - Serial：端口 @ 波特率
/// - BLE：设备地址
/// - Local：shell · 工作目录
pub fn conn_subtitle(conn: &SavedConnection) -> String {
    let type_label = connection_type_label(conn.conn_type);
    let detail = match conn.conn_type {
        ConnectionType::Ssh => {
            let user = conn.ssh_user.as_deref().unwrap_or("root");
            let host = conn.ssh_host.as_deref().unwrap_or("?");
            let port = conn.ssh_port.unwrap_or(22);
            format!("{user}@{host}:{port}")
        }
        ConnectionType::Serial => {
            let port = conn.serial_port.as_deref().unwrap_or("?");
            if let Some(baud) = conn.serial_baud {
                format!("{port} @ {baud} baud")
            } else {
                port.to_string()
            }
        }
        ConnectionType::Ble => conn.ble_device.as_deref().unwrap_or("?").to_string(),
        ConnectionType::Local => {
            let wd = conn.working_dir.as_deref().unwrap_or("~");
            let shell = conn.shell.as_deref().unwrap_or("default");
            format!("{shell} · {wd}")
        }
    };
    format!("{type_label}  ·  {detail}")
}
