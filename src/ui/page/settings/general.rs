//! General settings — language only.

use crate::i18n::Language;
use crate::ui::page::settings::SettingsPageCtx;
use crate::ui::uiframe::form;

pub fn page(ui: &mut egui::Ui, ctx: &mut SettingsPageCtx<'_>) {
    form::section_card(ui, |ui| {
        form::labeled_combo(
            ui,
            "prefs_language",
            rust_i18n::t!("language"),
            ctx.prefs.general.language.label(),
            |ui| {
                for lang in Language::ALL {
                    if ui
                        .selectable_label(ctx.prefs.general.language == lang, lang.label())
                        .clicked()
                    {
                        ctx.prefs.general.language = lang;
                        lang.apply();
                    }
                }
            },
        );
    });
}
