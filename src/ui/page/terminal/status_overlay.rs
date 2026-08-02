use crate::connection::ConnectionState;
use crate::session::{ActiveSession, ConnectionViewAction};

/// Draws a blocking connection-state overlay.
///
/// `Some` means the terminal surface must not be rendered for this frame.
pub(super) fn render(
    ui: &mut egui::Ui,
    session: &ActiveSession,
    area_size: egui::Vec2,
) -> Option<ConnectionViewAction> {
    if let Some(msg) = session.core.disconnect_message.as_ref() {
        return Some(render_disconnected(ui, session, area_size, msg));
    }
    if matches!(session.core.handle.state, ConnectionState::Connecting) {
        egui::Frame::new()
            .fill(egui::Color32::from_rgba_unmultiplied(20, 20, 20, 200))
            .show(ui, |ui| {
                ui.set_min_size(area_size);
                ui.vertical_centered(|ui| {
                    ui.add_space(area_size.y * 0.35);
                    ui.label(egui::RichText::new("Connecting…").size(16.0).weak());
                });
            });
        return Some(ConnectionViewAction::None);
    }
    None
}

fn render_disconnected(
    ui: &mut egui::Ui,
    session: &ActiveSession,
    area_size: egui::Vec2,
    message: &str,
) -> ConnectionViewAction {
    let lost = matches!(session.core.handle.state, ConnectionState::Lost(_));
    let title: String = if lost {
        "Disconnected".to_string()
    } else {
        rust_i18n::t!("connection_failed").into_owned()
    };
    let can_reconnect = session.core.saved_conn_id.is_some();
    let mut reconnect = false;
    let mut close = false;

    egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(20, 20, 20, 240))
        .show(ui, |ui| {
            ui.set_min_size(area_size);
            ui.vertical_centered(|ui| {
                ui.add_space(area_size.y * 0.25);
                ui.label(
                    egui::RichText::new(title)
                        .size(18.0)
                        .strong()
                        .color(egui::Color32::from_rgb(255, 120, 120)),
                );
                ui.add_space(8.0);
                ui.label(egui::RichText::new(message).size(14.0));
                ui.add_space(16.0);
                if can_reconnect {
                    if ui.button(rust_i18n::t!("reconnect")).clicked() {
                        reconnect = true;
                    }
                    ui.add_space(8.0);
                }
                if ui.button(rust_i18n::t!("close")).clicked() {
                    close = true;
                }
            });
        });

    if reconnect {
        if let Some(id) = session.core.saved_conn_id.as_ref() {
            return ConnectionViewAction::Reconnect(id.clone());
        }
    }
    if close {
        return ConnectionViewAction::CloseSession;
    }
    ConnectionViewAction::None
}
