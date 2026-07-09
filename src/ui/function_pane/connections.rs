//! Connections management page in the function pane.

use crate::storage::types::{ConnectionType, SavedConnection};
use crate::ui::shell::messages::FunctionAction;
use crate::ui::widget::components::filter_chips::{self, CONNECTION_TYPE_FILTERS};
use crate::ui::widget::style;
use crate::ui::widget::vector_icons::{self, Icon};

pub fn render(ui: &mut egui::Ui, connections: &[SavedConnection]) -> FunctionAction {
    let mut action = FunctionAction::empty();

    let top_h = 32.0;
    let top_w = ui.available_width();
    let top_rect = ui
        .allocate_exact_size(egui::vec2(top_w, top_h), egui::Sense::hover())
        .0;

    let back_rect = egui::Rect::from_min_size(top_rect.min, egui::vec2(top_w * 0.5, top_h));
    let back_resp = ui.interact(back_rect, ui.id().with("conn_back"), egui::Sense::click());
    if back_resp.clicked() {
        action.go_back = true;
    }

    let new_rect = egui::Rect::from_min_size(
        egui::pos2(top_rect.center().x, top_rect.top()),
        egui::vec2(top_w * 0.5, top_h),
    );
    let new_resp = ui.interact(new_rect, ui.id().with("conn_new"), egui::Sense::click());
    if new_resp.clicked() {
        action.new_connection = true;
    }

    if ui.is_rect_visible(top_rect) {
        let painter = ui.painter();

        if back_resp.hovered() {
            painter.rect_filled(
                back_rect,
                style::CORNER_RADIUS_XS,
                ui.visuals().widgets.hovered.bg_fill,
            );
        }
        let back_icon = egui::Rect::from_center_size(
            egui::pos2(back_rect.left() + 16.0, back_rect.center().y),
            egui::vec2(16.0, 16.0),
        );
        vector_icons::paint(
            ui,
            back_icon,
            Icon::Back,
            ui.visuals().text_color(),
            1.4,
        );
        let back_g = ui.fonts_mut(|f| {
            f.layout(
                rust_i18n::t!("back").to_string(),
                egui::FontId::proportional(14.0),
                ui.visuals().text_color(),
                top_w * 0.4,
            )
        });
        painter.galley(
            egui::pos2(back_rect.left() + 28.0, back_rect.center().y - back_g.size().y / 2.0),
            back_g,
            ui.visuals().text_color(),
        );

        if new_resp.hovered() {
            painter.rect_filled(
                new_rect,
                style::CORNER_RADIUS_XS,
                ui.visuals().widgets.hovered.bg_fill,
            );
        }
        let new_g = ui.fonts_mut(|f| {
            f.layout(
                rust_i18n::t!("new_connection").to_string(),
                egui::FontId::proportional(14.0),
                style::ACCENT,
                top_w * 0.4,
            )
        });
        let plus_rect = egui::Rect::from_center_size(
            egui::pos2(new_rect.right() - 8.0 - new_g.size().x - 12.0, new_rect.center().y),
            egui::vec2(14.0, 14.0),
        );
        vector_icons::paint(ui, plus_rect, Icon::Plus, style::ACCENT, 1.4);
        painter.galley(
            egui::pos2(
                new_rect.right() - 8.0 - new_g.size().x,
                new_rect.center().y - new_g.size().y / 2.0,
            ),
            new_g,
            style::ACCENT,
        );
    }
    ui.add_space(4.0);

    let filter: Option<ConnectionType> =
        filter_chips::paint_filter_chips(ui, "function_conn_filter", CONNECTION_TYPE_FILTERS);
    ui.add_space(4.0);

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

    ui.separator();

    if sorted.is_empty() {
        ui.add_space(16.0);
        ui.label(
            egui::RichText::new(rust_i18n::t!("home_no_connections"))
                .size(13.0)
                .color(ui.visuals().weak_text_color()),
        );
    } else {
        ui.style_mut().spacing.scroll.bar_width = 6.0;
        ui.style_mut().spacing.scroll.bar_outer_margin = 0.0;
        let menu_id_key = egui::Id::new("conn_menu_id");
        let menu_state: Option<String> = ui.data(|d| d.get_temp(menu_id_key)).unwrap_or(None);

        if menu_state.is_some()
            && ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary))
        {
            ui.data_mut(|d| d.insert_temp(menu_id_key, None::<String>));
        }

        egui::ScrollArea::vertical()
            .id_salt("function_conn_list_scroll")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for conn in &sorted {
                    paint_connection_row(ui, conn, &menu_id_key, &menu_state, &mut action);
                }
            });
    }

    action
}

fn paint_connection_row(
    ui: &mut egui::Ui,
    conn: &SavedConnection,
    menu_id_key: &egui::Id,
    menu_state: &Option<String>,
    action: &mut FunctionAction,
) {
    let row_h = 40.0;
    let row_rect = egui::Rect::from_min_size(
        ui.cursor().min,
        egui::vec2(ui.available_width(), row_h),
    );
    let row_resp = ui.allocate_rect(row_rect, egui::Sense::click());

    let dots_rect = egui::Rect::from_min_size(
        egui::pos2(row_rect.right() - 24.0, row_rect.top()),
        egui::vec2(24.0, row_h),
    );
    let dots_id = ui.id().with(("dots", &conn.id));
    let dots_resp = ui.interact(dots_rect, dots_id, egui::Sense::click());

    if row_resp.clicked() && !dots_resp.clicked() && !row_resp.long_touched() {
        ui.data_mut(|d| d.insert_temp(*menu_id_key, None::<String>));
        action.connect_connection = Some(conn.id.clone());
    }

    let show_file = matches!(conn.conn_type, ConnectionType::Local | ConnectionType::Ssh);
    row_resp.context_menu(|ui| {
        ui.data_mut(|d| d.insert_temp(*menu_id_key, None::<String>));
        paint_conn_menu(ui, conn, show_file, action);
    });
    if row_resp.long_touched() || dots_resp.clicked() {
        ui.data_mut(|d| d.insert_temp(*menu_id_key, Some(conn.id.clone())));
    }

    if ui.is_rect_visible(row_rect) {
        let painter = ui.painter_at(row_rect);
        if row_resp.hovered() || menu_state.as_deref() == Some(conn.id.as_str()) {
            painter.rect_filled(
                row_rect,
                style::CORNER_RADIUS_XS,
                ui.visuals().widgets.hovered.bg_fill,
            );
        }

        let text_left = row_rect.left() + 10.0;
        let name_w = row_rect.right() - text_left - 30.0;
        let name_g = ui.fonts_mut(|f| {
            f.layout(
                conn.name.clone(),
                egui::FontId::proportional(13.0),
                ui.visuals().text_color(),
                name_w,
            )
        });
        painter.galley(
            egui::pos2(text_left, row_rect.top() + 4.0),
            name_g,
            ui.visuals().text_color(),
        );

        let det_g = ui.fonts_mut(|f| {
            f.layout(
                conn_subtitle(conn),
                egui::FontId::proportional(10.0),
                ui.visuals().weak_text_color(),
                name_w,
            )
        });
        painter.galley(
            egui::pos2(text_left, row_rect.top() + 22.0),
            det_g,
            ui.visuals().weak_text_color(),
        );

        let dots_g = ui.fonts_mut(|f| {
            f.layout(
                "\u{22EE}".to_string(),
                egui::FontId::proportional(16.0),
                if dots_resp.hovered() {
                    ui.visuals().text_color()
                } else {
                    ui.visuals().weak_text_color()
                },
                f32::INFINITY,
            )
        });
        painter.galley(
            egui::pos2(
                dots_rect.center().x - dots_g.size().x / 2.0,
                dots_rect.center().y - dots_g.size().y / 2.0,
            ),
            dots_g,
            if dots_resp.hovered() {
                ui.visuals().text_color()
            } else {
                ui.visuals().weak_text_color()
            },
        );
    }

    if menu_state.as_deref() == Some(conn.id.as_str()) {
        egui::Popup::from_response(&dots_resp)
            .id(dots_id.with("ctx"))
            .show(|ui| {
                ui.set_min_width(130.0);
                paint_conn_menu(ui, conn, show_file, action);
            });
    }

    ui.add_space(2.0);
}

fn paint_conn_menu(
    ui: &mut egui::Ui,
    conn: &SavedConnection,
    show_file: bool,
    action: &mut FunctionAction,
) {
    ui.set_min_width(130.0);
    if ui.button(rust_i18n::t!("connect")).clicked() {
        action.connect_connection = Some(conn.id.clone());
        ui.close();
    }
    if show_file {
        if ui.button(rust_i18n::t!("home_file_manager")).clicked() {
            action.open_file_mgr = Some(conn.id.clone());
            ui.close();
        }
    }
    if ui.button(rust_i18n::t!("edit")).clicked() {
        action.edit_connection = Some(conn.id.clone());
        ui.close();
    }
    if ui.button(rust_i18n::t!("delete")).clicked() {
        action.delete_connection = Some(conn.id.clone());
        ui.close();
    }
}

fn conn_subtitle(conn: &SavedConnection) -> String {
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
    detail
}
