use crate::connection::ConnectionState;
use crate::session::{ActiveSession, ConnectionViewAction};
use crate::ui::uiframe::style;
use crate::ui::uiframe::tokens;

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
        let scrim = overlay_scrim(ui, 180);
        egui::Frame::new().fill(scrim).show(ui, |ui| {
            ui.set_min_size(area_size);
            ui.vertical_centered(|ui| {
                ui.add_space(area_size.y * 0.35);
                ui.label(
                    egui::RichText::new(rust_i18n::t!("connecting"))
                        .size(tokens::text::EMPHASIS)
                        .color(ui.visuals().weak_text_color()),
                );
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
        rust_i18n::t!("disconnected").into_owned()
    } else {
        rust_i18n::t!("connection_failed").into_owned()
    };
    let can_reconnect = session.core.saved_conn_id.is_some();
    let mut reconnect = false;
    let mut close = false;

    let scrim = overlay_scrim(ui, 220);
    egui::Frame::new().fill(scrim).show(ui, |ui| {
        ui.set_min_size(area_size);
        ui.vertical_centered(|ui| {
            ui.add_space(area_size.y * 0.25);
            ui.label(
                egui::RichText::new(title)
                    .size(tokens::text::HEADING)
                    .strong()
                    .color(style::RED),
            );
            ui.add_space(tokens::space::LG);
            ui.label(
                egui::RichText::new(message)
                    .size(tokens::text::EMPHASIS)
                    .color(ui.visuals().text_color()),
            );
            ui.add_space(tokens::space::XL);
            if can_reconnect {
                let reconnect_label = rust_i18n::t!("reconnect");
                let btn = style::primary_button(&reconnect_label)
                    .min_size(egui::vec2(120.0, tokens::size::BUTTON));
                if ui.add(btn).clicked() {
                    reconnect = true;
                }
                ui.add_space(tokens::space::LG);
            }
            let close_btn = egui::Button::new(rust_i18n::t!("close"))
                .corner_radius(style::CORNER_RADIUS_SM)
                .min_size(egui::vec2(100.0, tokens::size::BUTTON));
            if ui.add(close_btn).clicked() {
                close = true;
            }
        });
    });

    if reconnect && let Some(id) = session.core.saved_conn_id.as_ref() {
        return ConnectionViewAction::Reconnect(id.clone());
    }
    if close {
        return ConnectionViewAction::CloseSession;
    }
    ConnectionViewAction::None
}

fn overlay_scrim(ui: &egui::Ui, alpha: u8) -> egui::Color32 {
    if ui.visuals().dark_mode {
        egui::Color32::from_rgba_unmultiplied(13, 13, 15, alpha)
    } else {
        egui::Color32::from_rgba_unmultiplied(246, 247, 249, alpha)
    }
}
