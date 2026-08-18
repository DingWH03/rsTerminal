//! Terminal profiles list (edit via nested ProfileDialog).

use crate::ui::page::settings::SettingsPageCtx;
use crate::ui::uiframe::form;

pub fn page(ui: &mut egui::Ui, ctx: &mut SettingsPageCtx<'_>) {
    form::section_card(ui, |ui| {
        if form::manage_list_toolbar(ui, rust_i18n::t!("settings_create_profile")) {
            ctx.action.request_new_profile = true;
        }

        if ctx.profiles.is_empty() {
            ui.add_space(form::SECTION_GAP);
            ui.label(
                egui::RichText::new(rust_i18n::t!("settings_no_profiles"))
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        }

        for profile in ctx.profiles {
            form::manage_list_item_frame(ui, profile.is_default, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        let label = if profile.is_default {
                            format!("● {}", profile.name)
                        } else {
                            profile.name.clone()
                        };
                        ui.label(egui::RichText::new(label).strong().size(13.0));
                        if !profile.description.is_empty() {
                            ui.label(egui::RichText::new(&profile.description).size(11.0).weak());
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !profile.is_default
                            && ctx.profiles.len() > 1
                            && ui.small_button(rust_i18n::t!("delete")).clicked()
                        {
                            ctx.action.delete_profile_id = Some(profile.id.clone());
                        }
                        if ui.small_button(rust_i18n::t!("edit")).clicked() {
                            ctx.action.request_edit_profile = Some(profile.id.clone());
                        }
                        if !profile.is_default
                            && ui
                                .small_button(rust_i18n::t!("settings_set_default"))
                                .clicked()
                        {
                            ctx.action.set_default_profile_id = Some(profile.id.clone());
                        }
                    });
                });
            });
            ui.add_space(4.0);
        }
    });
}
