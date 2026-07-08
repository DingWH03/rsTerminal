//! 最近连接视图 — 在侧边栏中显示最近使用的连接列表。
//!
//! 按最后连接时间排序，最多显示 `MAX_RECENT_CONNECTIONS` 条，
//! 每条显示连接图标、名称和类型详情，底部提供"查看全部"按钮跳转到首页。

use crate::storage::types::SavedConnection;
use crate::ui::widget::sidebar::Sidebar;
use crate::ui::widget::components::empty_state::{self, EmptyStateConfig};

/// 最近连接最大显示数量
const MAX_RECENT_CONNECTIONS: usize = 20;
/// 每行高度
const RECENT_ROW_HEIGHT: f32 = 34.0;
/// 行间距
const RECENT_ROW_GAP: f32 = 2.0;
/// 底部"查看全部"按钮区域高度
const RECENT_FOOTER_HEIGHT: f32 = 30.0;

/// 渲染最近连接列表视图。
///
/// 按 `last_connected` 降序排列，每行显示连接类型图标、名称和副标题。
/// 点击行触发连接，点击"查看全部"跳转到首页完整列表。
pub fn recent_connections_view(
    ui: &mut egui::Ui,
    sidebar: &mut Sidebar,
    connections: &[SavedConnection],
    connect_clicked: &mut Option<String>,
    more_clicked: &mut bool,
) {
    let mut recent: Vec<&SavedConnection> = connections.iter().collect();
    recent.sort_by(|a, b| {
        b.last_connected
            .as_deref()
            .unwrap_or("")
            .cmp(&a.last_connected.as_deref().unwrap_or(""))
            .then_with(|| a.name.cmp(&b.name))
    });

    let show_count = recent.len().min(MAX_RECENT_CONNECTIONS);
    let recent = &recent[..show_count];

    // ── Header bar ──────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.style_mut().spacing.button_padding = egui::vec2(4.0, 1.0);
        ui.style_mut().spacing.item_spacing.x = 4.0;

        if sidebar.show_content_hamburger() && sidebar.hamburger(ui).clicked() {
            sidebar.hamburger_click();
        }

        ui.label(
            egui::RichText::new(rust_i18n::t!("recent_connections"))
                .size(12.0)
                .strong()
                .color(ui.visuals().text_color()),
        );
    });

    ui.add_space(4.0);

    if recent.is_empty() {
        empty_state::paint_empty_state(
            ui,
            EmptyStateConfig {
                icon: "\u{1F4CB}",
                title: &rust_i18n::t!("home_no_connections"),
                subtitle: Some(&rust_i18n::t!("open_terminal_hint")),
                ..Default::default()
            },
        );
        return;
    }

    // ── Recent list ──────────────────────────────────────────────────────
    let row_step = RECENT_ROW_HEIGHT + RECENT_ROW_GAP;
    let desired_list_height = recent.len() as f32 * row_step;
    let available_list_height = (ui.available_height() - RECENT_FOOTER_HEIGHT).max(RECENT_ROW_HEIGHT);
    let list_height = desired_list_height.min(available_list_height);

    egui::ScrollArea::vertical()
        .id_salt("home_recent_connections")
        .auto_shrink([false, false])
        .max_height(list_height)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = RECENT_ROW_GAP;

            for conn in recent {
                let available_w = ui.available_width();
                let (row_rect, row_resp) = ui.allocate_exact_size(
                    egui::vec2(available_w, RECENT_ROW_HEIGHT),
                    egui::Sense::click(),
                );

                if row_resp.clicked() {
                    *connect_clicked = Some(conn.id.clone());
                }

                if !ui.is_rect_visible(row_rect) {
                    continue;
                }

                let painter = ui.painter_at(row_rect);

                let bg = if row_resp.hovered() {
                    ui.visuals().widgets.hovered.bg_fill
                } else {
                    ui.visuals().extreme_bg_color
                };

                painter.rect_filled(row_rect, egui::CornerRadius::same(4), bg);

                let icon = conn.conn_type.icon();
                let icon_g = ui.fonts_mut(|f| {
                    f.layout(
                        icon.to_string(),
                        egui::FontId::proportional(15.0),
                        ui.visuals().text_color(),
                        f32::INFINITY,
                    )
                });

                painter.galley(
                    egui::pos2(
                        row_rect.left() + 8.0,
                        row_rect.center().y - icon_g.size().y / 2.0,
                    ),
                    icon_g,
                    ui.visuals().text_color(),
                );

                let text_left = row_rect.left() + 32.0;
                let name_w = row_rect.right() - text_left - 8.0;

                let name_g = ui.fonts_mut(|f| {
                    f.layout(
                        conn.name.clone(),
                        egui::FontId::proportional(12.5),
                        ui.visuals().text_color(),
                        name_w,
                    )
                });

                painter.galley(
                    egui::pos2(text_left, row_rect.top() + 3.0),
                    name_g,
                    ui.visuals().text_color(),
                );

                let det_g = ui.fonts_mut(|f| {
                    f.layout(
                        crate::ui::page::home::conn_subtitle(conn),
                        egui::FontId::proportional(10.0),
                        ui.visuals().weak_text_color(),
                        name_w,
                    )
                });

                painter.galley(
                    egui::pos2(text_left, row_rect.top() + 19.0),
                    det_g,
                    ui.visuals().weak_text_color(),
                );
            }
        });

    // ── More button ──────────────────────────────────────────────────────
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        let more_label = format!("{}  →", rust_i18n::t!("view_all"));

        if ui
            .button(
                egui::RichText::new(&more_label)
                    .size(12.0)
                    .color(crate::ui::widget::style::ACCENT),
            )
            .clicked()
        {
            *more_clicked = true;
        }
    });
}