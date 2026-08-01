//! General settings — language only.

use crate::i18n::Language;
use crate::ui::page::settings::SettingsPageCtx;
use crate::ui::uiframe::style;

pub fn page(ui: &mut egui::Ui, ctx: &mut SettingsPageCtx<'_>) {
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(style::CORNER_RADIUS_SM)
        .inner_margin(egui::Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(rust_i18n::t!("settings_tab_general"))
                    .size(15.0)
                    .strong(),
            );
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(rust_i18n::t!("language"));
                egui::ComboBox::from_id_salt("prefs_language")
                    .selected_text(ctx.prefs.language.label())
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for lang in Language::ALL {
                            if ui
                                .selectable_label(ctx.prefs.language == lang, lang.label())
                                .clicked()
                            {
                                ctx.prefs.language = lang;
                                lang.apply();
                            }
                        }
                    });
            });
        });
}
