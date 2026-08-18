//! General settings — language only.

use rsterm_config::Language;

use crate::host_hooks;
use crate::page::settings::SettingsPageCtx;
use crate::uiframe::form;

pub fn page(ui: &mut egui::Ui, ctx: &mut SettingsPageCtx<'_>) {
    form::section_card(ui, |ui| {
        form::labeled_combo(
            ui,
            "prefs_language",
            crate::i18n_bridge::tr("language"),
            host_hooks::language_label(ctx.prefs.general.language),
            |ui| {
                for lang in Language::ALL {
                    if ui
                        .selectable_label(
                            ctx.prefs.general.language == lang,
                            host_hooks::language_label(lang),
                        )
                        .clicked()
                    {
                        ctx.prefs.general.language = lang;
                        host_hooks::apply_language(lang);
                    }
                }
            },
        );
    });
}
