//! Small modal notices painted by the UI layer (quit confirm, connection errors).
//!
//! These stay as **embedded** centered windows (not native OS viewports) so the
//! dimmer cannot cover the dialog buttons, and the main UI is properly blocked.

use crate::ui::uiframe::style;

/// Paint quit-with-sessions confirmation. Returns `true` if the user confirmed.
///
/// Embedded centered modal: dims and blocks the host; dialog stays clickable.
pub fn paint_quit_confirm(ctx: &egui::Context, open: &mut bool, session_count: usize) -> bool {
    if !*open {
        return false;
    }

    paint_modal_dimmer(ctx, egui::Id::new("quit_confirm_dimmer"));

    let mut confirmed = false;
    let mut open_flag = true;

    egui::Window::new(rust_i18n::t!("quit_with_sessions_title").as_ref())
        .id(egui::Id::new("quit_confirm_dialog"))
        .open(&mut open_flag)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.set_max_width(400.0);
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(rust_i18n::t!(
                    "quit_with_sessions_body",
                    count = session_count
                ))
                .size(14.0)
                .color(ui.visuals().text_color()),
            );
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                let cancel_btn = egui::Button::new(
                    egui::RichText::new(rust_i18n::t!("cancel"))
                        .size(14.0)
                        .color(ui.visuals().weak_text_color()),
                )
                .fill(ui.visuals().panel_fill)
                .corner_radius(style::CORNER_RADIUS_SM)
                .min_size(egui::vec2(90.0, 34.0));
                if ui.add(cancel_btn).clicked() {
                    *open = false;
                }

                let confirm_btn = egui::Button::new(
                    egui::RichText::new(rust_i18n::t!("quit_with_sessions_confirm"))
                        .size(14.0)
                        .strong()
                        .color(egui::Color32::WHITE),
                )
                .fill(style::RED)
                .corner_radius(style::CORNER_RADIUS_SM)
                .min_size(egui::vec2(100.0, 34.0));
                if ui.add(confirm_btn).clicked() {
                    confirmed = true;
                    *open = false;
                }
            });
        });

    if !open_flag {
        *open = false;
    }
    confirmed
}

/// Paint a connection-failure notice; clears `notice` when dismissed.
pub fn paint_connection_notice(ctx: &egui::Context, notice: &mut Option<String>) {
    let Some(msg) = notice.clone() else {
        return;
    };

    paint_modal_dimmer(ctx, egui::Id::new("connection_notice_dimmer"));

    let mut dismiss = false;
    let mut open_flag = true;

    egui::Window::new(rust_i18n::t!("connection_failed").as_ref())
        .id(egui::Id::new("connection_notice_dialog"))
        .open(&mut open_flag)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.set_max_width(420.0);
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(&msg)
                    .size(14.0)
                    .color(ui.visuals().text_color()),
            );
            ui.add_space(16.0);
            let ok_btn = egui::Button::new(
                egui::RichText::new(rust_i18n::t!("ok"))
                    .size(14.0)
                    .color(egui::Color32::WHITE),
            )
            .fill(style::ACCENT)
            .corner_radius(style::CORNER_RADIUS_SM)
            .min_size(egui::vec2(80.0, 34.0));
            if ui.add(ok_btn).clicked() {
                dismiss = true;
            }
        });

    if dismiss || !open_flag {
        *notice = None;
    }
}

/// Dimmer below [`Order::Foreground`] dialogs so buttons stay clickable.
fn paint_modal_dimmer(ctx: &egui::Context, id: egui::Id) {
    let screen = ctx.content_rect();
    egui::Area::new(id)
        .order(egui::Order::Middle)
        .fixed_pos(screen.min)
        .interactable(true)
        .sense(egui::Sense::click_and_drag())
        .show(ctx, |ui| {
            let (rect, _resp) =
                ui.allocate_exact_size(screen.size(), egui::Sense::click_and_drag());
            ui.painter().rect_filled(
                rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 110),
            );
        });
}
