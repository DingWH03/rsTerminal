//! 最近连接视图 — 在侧边栏中显示最近使用的连接列表。

use crate::data::persist::types::SavedConnection;
use crate::ui::connection_display::connection_type_icon;
use crate::ui::function_pane::FunctionPane;
use crate::ui::uiframe::components::empty_state::{self, EmptyStateConfig};
use crate::ui::uiframe::components::toolbar_button::{
    icon_toolbar_button, icon_toolbar_danger,
};
use crate::ui::uiframe::vector_icons::Icon;

/// 最近连接最大显示数量
const MAX_RECENT_CONNECTIONS: usize = 20;
/// 每行高度
const RECENT_ROW_HEIGHT: f32 = 34.0;
/// 行间距
const RECENT_ROW_GAP: f32 = 2.0;
/// 底部"查看全部"按钮区域高度
const RECENT_FOOTER_HEIGHT: f32 = 30.0;

/// 工作区窗格顶栏操作。
pub struct SplitPaneChrome<'a> {
    pub hide_pane: Option<&'a mut bool>,
    pub close_pane: Option<&'a mut bool>,
}

/// 渲染最近连接列表视图。
pub fn recent_connections_view(
    ui: &mut egui::Ui,
    function_pane: &mut FunctionPane,
    connections: &[SavedConnection],
    connect_clicked: &mut Option<String>,
    more_clicked: &mut bool,
    in_split: bool,
    split_chrome: Option<SplitPaneChrome<'_>>,
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
        ui.style_mut().spacing.button_padding = egui::vec2(2.0, 1.0);
        ui.style_mut().spacing.item_spacing.x = 2.0;

        let show_hamburger = !in_split && function_pane.show_content_hamburger();
        if show_hamburger {
            if icon_toolbar_button(ui, ui.id().with("recent_menu"), Icon::Hamburger).clicked() {
                function_pane.hamburger_click();
            }
        }

        ui.label(
            egui::RichText::new(rust_i18n::t!("recent_connections"))
                .size(12.0)
                .strong()
                .color(ui.visuals().text_color()),
        );

        if let Some(chrome) = split_chrome {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.style_mut().spacing.item_spacing.x = 2.0;
                if let Some(close) = chrome.close_pane {
                    if icon_toolbar_danger(ui, ui.id().with("recent_close"), Icon::Close)
                        .on_hover_text(rust_i18n::t!("close_pane"))
                        .clicked()
                    {
                        *close = true;
                    }
                }
                if let Some(hide) = chrome.hide_pane {
                    if icon_toolbar_button(ui, ui.id().with("recent_hide"), Icon::Minimize)
                        .on_hover_text(rust_i18n::t!("minimize_pane"))
                        .clicked()
                    {
                        *hide = true;
                    }
                }
            });
        }
    });

    ui.add_space(2.0);

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

                let icon = connection_type_icon(conn.conn_type);
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

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        let more_label = format!("{}  →", rust_i18n::t!("view_all"));

        if ui
            .button(
                egui::RichText::new(&more_label)
                    .size(12.0)
                    .color(crate::ui::uiframe::style::ACCENT),
            )
            .clicked()
        {
            *more_clicked = true;
        }
    });
}
