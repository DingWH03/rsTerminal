

//! 首页 — 已保存连接的展示与操作入口。
//!
//! 提供连接卡片的渲染、筛选（按类型）、排序（收藏优先/最近使用/字母序）、
//! 收藏切换、编辑、删除、SFTP 远程文件管理以及浮动操作按钮（FAB）等功能。

pub mod recent;
pub mod sidebar;

use crate::storage::types::{ConnectionType, SavedConnection};
use crate::ui::uiframe::style;
use crate::ui::uiframe::components::card;
use crate::ui::uiframe::components::empty_state::{self, EmptyStateConfig};
use crate::ui::uiframe::components::filter_chips::{self, CONNECTION_TYPE_FILTERS};
use crate::ui::uiframe::components::icon_widget;

/// 首页连接卡片的工具栏操作（来自卡片上的图标按钮，而非右键菜单）。
#[derive(Default)]
pub struct HomeCardMenuAction {
    /// 打开本地文件管理器
    pub local_fm: bool,
    /// 打开 SSH SFTP 远程文件管理器，值为连接 ID
    pub sftp_id: Option<String>,
    /// 切换收藏状态，值为连接 ID
    pub toggle_favorite: Option<String>,
}

/// 渲染首页主界面。
///
/// 包含筛选标签（All/Local/SSH/Serial/BLE）、已保存连接卡片列表、
/// 右键上下文菜单，以及底部的浮动操作按钮（FAB）。
pub fn home_screen(
    ui: &mut egui::Ui,
    connections: &[SavedConnection],
    selected_conn_id: &mut Option<String>,
    card_menu: &mut HomeCardMenuAction,
    fab_clicked: &mut bool,
    connect_clicked: &mut Option<String>,
    edit_clicked: &mut Option<String>,
    sftp_clicked: &mut Option<String>,
    delete_clicked: &mut Option<String>,
    settings_clicked: &mut bool,
) {
    let _ = settings_clicked;

    // ── 筛选标签 ────────────────────────────────────────────────────────
    let filter: Option<ConnectionType> = filter_chips::paint_filter_chips(
        ui,
        "home_filter",
        CONNECTION_TYPE_FILTERS,
    );
    ui.add_space(2.0);

    // ── 已保存连接列表 ─────────────────────────────────────────────────
    if connections.is_empty() {
        empty_state::paint_empty_state(
            ui,
            EmptyStateConfig {
                icon: "\u{1F4CB}",
                title: &rust_i18n::t!("home_no_connections"),
                subtitle: Some("Tap + to add your first connection"),
                ..Default::default()
            },
        );
    } else {
        // Filter + sort: favorites first, then recent, then alphabetically
        let mut sorted: Vec<&SavedConnection> = match filter {
            Some(ref ft) => connections.iter().filter(|c| c.conn_type == *ft).collect(),
            None => connections.iter().collect(),
        };
        sorted.sort_by(|a, b| {
            b.favorite
                .cmp(&a.favorite)
                .then_with(|| b.last_connected.cmp(&a.last_connected))
                .then_with(|| a.name.cmp(&b.name))
        });

        ui.add_space(2.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut to_delete: Option<usize> = None;

                for (i, conn) in sorted.iter().enumerate() {
                    let selected = selected_conn_id.as_deref() == Some(conn.id.as_str());
                    let (card, file_btn, pencil) = render_connection_card(
                        ui,
                        conn,
                        selected,
                        card_menu,
                        edit_clicked,
                    );

                    if card.clicked() && !file_btn.clicked() && !pencil.clicked() {
                        *selected_conn_id = Some(conn.id.clone());
                        *connect_clicked = Some(conn.id.clone());
                    }

                    card.context_menu(|ui| {
                        if ui.button(rust_i18n::t!("connect")).clicked() {
                            *connect_clicked = Some(conn.id.clone());
                            ui.close();
                        }
                        if ui.button(rust_i18n::t!("edit")).clicked() {
                            *edit_clicked = Some(conn.id.clone());
                            ui.close();
                        }
                        if conn.conn_type == ConnectionType::Ssh
                            && ui.button(rust_i18n::t!("home_remote_files")).clicked()
                        {
                            *sftp_clicked = Some(conn.id.clone());
                            ui.close();
                        }
                        if ui.button(rust_i18n::t!("delete")).clicked() {
                            to_delete = Some(i);
                            ui.close();
                        }
                    });

                    ui.add_space(style::CARD_SPACING);
                }

                if let Some(i) = to_delete {
                    let conn_id = sorted[i].id.clone();
                    *delete_clicked = Some(conn_id);
                }
            });
    }

    // ── Floating Action Button ──────────────────────────────────────────────
    paint_fab(ui, fab_clicked);
}

// ─── 浮动操作按钮（FAB）────────────────────────────────────────────────

/// 绘制右下角的浮动操作按钮（"+" 按钮）。
///
/// 包含阴影效果和悬停高亮，点击后触发 `fab_clicked` 标志。
fn paint_fab(ui: &mut egui::Ui, fab_clicked: &mut bool) {
    let fab_size = 56.0;
    let shadow_offset = 2.0;
    let fab_pos = egui::pos2(
        ui.max_rect().right() - fab_size - 20.0,
        ui.max_rect().bottom() - fab_size - 20.0 - shadow_offset,
    );
    let fab_rect = egui::Rect::from_min_size(fab_pos, egui::vec2(fab_size, fab_size));
    let fab_resp = ui.allocate_rect(fab_rect, egui::Sense::click());
    if fab_resp.clicked() {
        *fab_clicked = true;
    }

    if ui.is_rect_visible(fab_rect) {
        let painter = ui.painter_at(fab_rect);

        // 阴影
        let shadow_rect = fab_rect.translate(egui::vec2(0.0, shadow_offset));
        painter.circle_filled(shadow_rect.center(), fab_size / 2.0, egui::Color32::from_black_alpha(60));

        // 主圆形
        let bg = if fab_resp.hovered() {
            style::ACCENT.gamma_multiply(1.15)
        } else {
            style::ACCENT
        };
        painter.circle_filled(fab_rect.center(), fab_size / 2.0, bg);

        // "+" 图标
        icon_widget::paint_icon(ui, fab_rect, "+", 28.0, egui::Color32::WHITE);
    }
}

/// 构建连接副标题，组合连接类型和关键详细信息。
///
/// 根据连接类型显示不同的详细信息：
/// - SSH：user@host:port
/// - Serial：端口 @ 波特率
/// - BLE：设备地址
/// - Local：shell · 工作目录
pub fn conn_subtitle(conn: &SavedConnection) -> String {
    let type_label = conn.conn_type.label();
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
        ConnectionType::Ble => conn
            .ble_device
            .as_deref()
            .unwrap_or("?")
            .to_string(),
        ConnectionType::Local => {
            let wd = conn
                .working_dir
                .as_deref()
                .unwrap_or("~");
            let shell = conn.shell.as_deref().unwrap_or("default");
            format!("{shell} · {wd}")
        }
    };
    format!("{type_label}  ·  {detail}")
}

// ─── 卡片常量 ───────────────────────────────────────────────────────────

/// 卡片图标字体大小
const CARD_ICON_FONT: f32 = 22.0;
/// 收藏星标字体大小
const STAR_ICON_FONT: f32 = 16.0;

/// 文件管理器图标（📁）
const FILE_ICON: &str = "\u{1F4C1}";
/// 编辑图标（✎）
const EDIT_ICON: &str = "\u{270E}";
/// 收藏星标实心（★）
const STAR_FILLED: &str = "\u{2605}";
/// 收藏星标空心（☆）
const STAR_EMPTY: &str = "\u{2606}";

// ─── 连接卡片渲染 ───────────────────────────────────────────────────────

/// 渲染单个已保存连接的卡片。
///
/// 卡片包含：连接类型图标、连接名称、副标题、收藏星标、编辑和文件管理器按钮。
/// 返回 (卡片响应, 文件按钮响应, 编辑按钮响应) 三元组。
fn render_connection_card(
    ui: &mut egui::Ui,
    conn: &SavedConnection,
    selected: bool,
    card_menu: &mut HomeCardMenuAction,
    edit_clicked: &mut Option<String>,
) -> (egui::Response, egui::Response, egui::Response) {
    let show_file = matches!(conn.conn_type, ConnectionType::Local | ConnectionType::Ssh);
    let desired = egui::vec2(ui.available_width(), style::CARD_HEIGHT);
    let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click());

    let noop = ui.interact(
        egui::Rect::NOTHING,
        ui.id().with(("noop", &conn.id)),
        egui::Sense::hover(),
    );
    let mut file_resp = noop.clone();
    let mut pencil_resp = noop;

    if ui.is_rect_visible(rect) {
        card::paint_card_chrome(
            ui,
            rect,
            card::card_fill(ui, selected, resp.hovered()),
            card::card_stroke(ui, selected, resp.hovered()),
        );

        let icon_x = rect.left() + 16.0;
        let icon_y = rect.center().y;

        // 连接类型图标
        let icon = ui.fonts_mut(|f| {
            f.layout(
                conn.conn_type.icon().to_string(),
                egui::FontId::proportional(CARD_ICON_FONT),
                style::ACCENT,
                f32::INFINITY,
            )
        });
        ui.painter_at(rect).galley(
            egui::pos2(icon_x, icon_y - icon.rect.height() / 2.0),
            icon,
            style::ACCENT,
        );

        let text_left = rect.left() + 52.0;
        let name_top = rect.top() + 8.0;
        let sub_top = rect.top() + 27.0;

        // 连接名称
        let name_g = ui.fonts_mut(|f| {
            f.layout(
                conn.name.clone(),
                egui::FontId::proportional(14.0),
                ui.visuals().text_color(),
                f32::INFINITY,
            )
        });
        ui.painter_at(rect).galley(
            egui::pos2(text_left, name_top),
            name_g,
            ui.visuals().text_color(),
        );

        // 副标题（类型 + 关键信息）
        let toolbar_w = style::CardToolbar::reserved_width(show_file, true);
        let max_text_w = (rect.right() - text_left - toolbar_w).max(60.0);
        let sub_g = ui.fonts_mut(|f| {
            f.layout(
                conn_subtitle(conn),
                egui::FontId::proportional(11.0),
                ui.visuals().weak_text_color(),
                max_text_w,
            )
        });
        ui.painter_at(rect).galley(
            egui::pos2(text_left, sub_top),
            sub_g,
            ui.visuals().weak_text_color(),
        );

        // 收藏星标 — 最右侧
        let star_slot = style::ICON_SLOT;
        let star_x = rect.right() - style::TOOLBAR_MARGIN - star_slot;
        let star_rect = egui::Rect::from_center_size(
            egui::pos2(star_x + star_slot / 2.0, rect.center().y),
            egui::vec2(star_slot, star_slot),
        );
        let star_id = ui.id().with(("star", &conn.id));
        let star_resp = ui.interact(star_rect, star_id, egui::Sense::click());
        if star_resp.clicked() {
            card_menu.toggle_favorite = Some(conn.id.clone());
        }
        if ui.is_rect_visible(star_rect) {
            let (star_char, star_color) = if conn.favorite {
                (STAR_FILLED, egui::Color32::from_rgb(255, 200, 0))
            } else {
                (STAR_EMPTY, ui.visuals().weak_text_color())
            };
            icon_widget::paint_icon(ui, star_rect, star_char, STAR_ICON_FONT, star_color);
        }

        // 工具栏图标（编辑、文件管理器）
        let toolbar = style::CardToolbar::layout(rect, show_file, true);

        if let Some(edit_rect) = toolbar.edit {
            pencil_resp = icon_widget::icon_button(
                ui,
                edit_rect,
                ui.id().with(("edit", &conn.id)),
                EDIT_ICON,
                CARD_ICON_FONT,
            );
            if pencil_resp.clicked() {
                *edit_clicked = Some(conn.id.clone());
            }
        }

        if let Some(file_rect) = toolbar.file {
            file_resp = icon_widget::icon_button(
                ui,
                file_rect,
                ui.id().with(("file", &conn.id)),
                FILE_ICON,
                CARD_ICON_FONT,
            );
            if file_resp.clicked() {
                match conn.conn_type {
                    ConnectionType::Local => card_menu.local_fm = true,
                    ConnectionType::Ssh => card_menu.sftp_id = Some(conn.id.clone()),
                    ConnectionType::Serial | ConnectionType::Ble => {}
                }
            }
        }
    }

    (resp, file_resp, pencil_resp)
}
