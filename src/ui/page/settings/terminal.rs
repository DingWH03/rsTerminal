//! Terminal profiles list (edit via nested ProfileDialog).

use crate::ui::page::settings::SettingsPageCtx;
use crate::ui::uiframe::style;

pub fn page(ui: &mut egui::Ui, ctx: &mut SettingsPageCtx<'_>) {
    ui.horizontal(|ui| {
        let new_btn = egui::Button::new(
            egui::RichText::new(rust_i18n::t!("settings_create_profile"))
                .color(egui::Color32::WHITE),
        )
        .fill(style::ACCENT)
        .corner_radius(style::CORNER_RADIUS_SM);
        if ui.add(new_btn).clicked() {
            ctx.action.request_new_profile = true;
        }
    });
    ui.add_space(8.0);
    ui.separator();

    if ctx.profiles.is_empty() {
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new(rust_i18n::t!("settings_no_profiles"))
                .color(ui.visuals().weak_text_color()),
        );
        return;
    }

    for profile in ctx.profiles {
        egui::Frame::new()
            .fill(if profile.is_default {
                ui.visuals().selection.bg_fill.gamma_multiply(0.25)
            } else {
                egui::Color32::TRANSPARENT
            })
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .corner_radius(style::CORNER_RADIUS_XS)
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        let label = if profile.is_default {
                            format!("● {}", profile.name)
                        } else {
                            profile.name.clone()
                        };
                        ui.label(egui::RichText::new(label).strong().size(13.0));
                        if !profile.description.is_empty() {
                            ui.label(
                                egui::RichText::new(&profile.description)
                                    .size(11.0)
                                    .weak(),
                            );
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !profile.is_default && ctx.profiles.len() > 1 {
                            if ui.small_button(rust_i18n::t!("delete")).clicked() {
                                ctx.action.delete_profile_id = Some(profile.id.clone());
                            }
                        }
                        if ui.small_button(rust_i18n::t!("edit")).clicked() {
                            ctx.action.request_edit_profile = Some(profile.id.clone());
                        }
                        if !profile.is_default {
                            if ui
                                .small_button(rust_i18n::t!("settings_set_default"))
                                .clicked()
                            {
                                ctx.action.set_default_profile_id = Some(profile.id.clone());
                            }
                        }
                    });
                });
            });
        ui.add_space(4.0);
    }
}
