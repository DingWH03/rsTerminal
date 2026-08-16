//! Connections management page in the function pane.

use crate::filter_chips_conn::connection_type_filters;
use crate::shell::messages::FunctionAction;
use crate::uiframe::components::compact_list_row::{CompactListRow, ListRowDensity};
use crate::uiframe::components::empty_state::{EmptyStateConfig, paint_empty_state};
use crate::uiframe::components::filter_chips;
use crate::uiframe::components::overflow_menu::{self, OverflowMenuState};
use crate::uiframe::vector_icons::Icon;
use rsterm_data::persist::types::{ConnectionType, SavedConnection};

/// Paint saved-connection list (filter chips + rows).
/// Used by the sidebar Connections tab and the "Open Connection" dialog.
/// New connection is created from the top menu (Connection → New), not here.
pub fn render(ui: &mut egui::Ui, connections: &[SavedConnection]) -> FunctionAction {
    render_with_id(ui, connections, "function_conn")
}

/// Same as [`render`] with a distinct egui id salt (sidebar vs dialog).
pub fn render_with_id(
    ui: &mut egui::Ui,
    connections: &[SavedConnection],
    id_salt: &str,
) -> FunctionAction {
    let mut action = FunctionAction::empty();

    if connections.is_empty() {
        paint_empty_state(
            ui,
            EmptyStateConfig::compact(
                Icon::Connections,
                &crate::i18n_bridge::tr("home_no_connections"),
                None,
            ),
        );
        return action;
    }

    let filter: Option<ConnectionType> = filter_chips::paint_filter_chips(
        ui,
        &format!("{id_salt}_filter"),
        &connection_type_filters(),
    );

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

    // Hairline under tags — avoid egui Separator's default vertical margins.
    let y = ui.cursor().top();
    let full = ui.max_rect();
    ui.painter().hline(
        full.x_range(),
        y,
        egui::Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color),
    );
    ui.add_space(1.0);

    if sorted.is_empty() {
        paint_empty_state(
            ui,
            EmptyStateConfig::compact(
                Icon::Connections,
                &crate::i18n_bridge::tr("home_no_connections"),
                None,
            ),
        );
        return action;
    }

    ui.style_mut().spacing.scroll.bar_width = 6.0;
    ui.style_mut().spacing.scroll.bar_outer_margin = 0.0;
    let menu_id_key = egui::Id::new(format!("{id_salt}_menu_id"));
    let mut menu_state = OverflowMenuState::load(ui, menu_id_key);

    egui::ScrollArea::vertical()
        .id_salt(format!("{id_salt}_list_scroll"))
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for conn in &sorted {
                paint_connection_row(ui, conn, &mut menu_state, &mut action);
            }
        });

    // Dismiss overflow after a menu action (do not close on the click that selects the item).
    if action.connect_connection.is_some()
        || action.open_file_mgr.is_some()
        || action.edit_connection.is_some()
        || action.delete_connection.is_some()
    {
        menu_state.close();
    }

    menu_state.store(ui, menu_id_key);
    action
}

fn paint_connection_row(
    ui: &mut egui::Ui,
    conn: &SavedConnection,
    menu_state: &mut OverflowMenuState,
    action: &mut FunctionAction,
) {
    let subtitle = conn_subtitle(conn);
    let outcome = CompactListRow {
        id: ui.id().with(("conn_row", &conn.id)),
        density: ListRowDensity::Standard,
        title: &conn.name,
        subtitle: Some(&subtitle),
        leading: None,
        selected: false,
        accent_stripe: None,
        sense: egui::Sense::click(),
        trailing_width: 24.0,
        menu_open: menu_state.is_open(&conn.id),
    }
    .show(ui);

    let Some(row_resp) = outcome.response else {
        return;
    };
    let Some(dots_resp) = outcome.trailing_response else {
        return;
    };
    let dots_id = ui.id().with(("dots", &conn.id));

    if row_resp.clicked() && !dots_resp.clicked() && !row_resp.long_touched() {
        menu_state.close();
        action.connect_connection = Some(conn.id.clone());
    }

    let show_file = matches!(conn.conn_type, ConnectionType::Local | ConnectionType::Ssh);
    row_resp.context_menu(|ui| {
        menu_state.close();
        paint_conn_menu(ui, conn, show_file, action);
    });
    overflow_menu::overflow_trigger(ui, &dots_resp, &row_resp, &conn.id, menu_state);
    overflow_menu::show_if_open(ui, &dots_resp, dots_id, &conn.id, menu_state, 130.0, |ui| {
        paint_conn_menu(ui, conn, show_file, action)
    });
}

fn paint_conn_menu(
    ui: &mut egui::Ui,
    conn: &SavedConnection,
    show_file: bool,
    action: &mut FunctionAction,
) {
    ui.set_min_width(130.0);
    if ui.button(crate::i18n_bridge::tr("connect")).clicked() {
        action.connect_connection = Some(conn.id.clone());
        ui.close();
    }
    if show_file
        && ui
            .button(crate::i18n_bridge::tr("home_file_manager"))
            .clicked()
    {
        action.open_file_mgr = Some(conn.id.clone());
        ui.close();
    }
    if ui.button(crate::i18n_bridge::tr("edit")).clicked() {
        action.edit_connection = Some(conn.id.clone());
        ui.close();
    }
    if ui.button(crate::i18n_bridge::tr("delete")).clicked() {
        action.delete_connection = Some(conn.id.clone());
        ui.close();
    }
}

fn conn_subtitle(conn: &SavedConnection) -> String {
    match conn.conn_type {
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
    }
}
