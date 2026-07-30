//! Small modal notices painted by the UI layer (quit confirm, connection errors).

use crate::ui::uiframe::style;
use crate::ui::uiframe::{DialogFrame, DialogOutcome};

/// Paint quit-with-sessions confirmation. Returns `true` if the user confirmed.
pub fn paint_quit_confirm(
    ctx: &egui::Context,
    open: &mut bool,
    session_count: usize,
) -> bool {
    if !*open {
        return false;
    }

    let mut confirmed = false;
    let mut dismiss = false;

    let frame = DialogFrame::alert(rust_i18n::t!("quit_with_sessions_title").to_string())
        .blocks_host(true)
        .closable(true);

    if frame.show(ctx, "quit_confirm_dialog", |ui| {
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
                dismiss = true;
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
                dismiss = true;
            }
        });
    }) == DialogOutcome::Closed
    {
        dismiss = true;
    }

    if dismiss {
        *open = false;
    }
    confirmed
}

/// Paint a connection-failure notice; clears `notice` when dismissed.
pub fn paint_connection_notice(ctx: &egui::Context, notice: &mut Option<String>) {
    let Some(msg) = notice.clone() else {
        return;
    };
    let mut dismiss = false;

    let frame = DialogFrame::alert(rust_i18n::t!("connection_failed").to_string())
        .blocks_host(true)
        .closable(true);

    if frame.show(ctx, "connection_notice_dialog", |ui| {
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
    }) == DialogOutcome::Closed
    {
        dismiss = true;
    }

    if dismiss {
        *notice = None;
    }
}
