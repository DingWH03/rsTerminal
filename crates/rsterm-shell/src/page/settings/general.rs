//! General settings — language and input mode.

use rsterm_config::Language;
use rsterm_data::prefs::InputInteractionMode;

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
        form::labeled_combo(
            ui,
            "prefs_input_mode",
            crate::i18n_bridge::tr("settings_input_mode"),
            input_mode_label(ctx.prefs.general.input_mode),
            |ui| {
                for mode in InputInteractionMode::ALL {
                    if ui
                        .selectable_label(
                            ctx.prefs.general.input_mode == mode,
                            input_mode_label(mode),
                        )
                        .clicked()
                    {
                        ctx.prefs.general.input_mode = mode;
                    }
                }
            },
        );
    });
}

fn input_mode_label(mode: InputInteractionMode) -> String {
    match mode {
        InputInteractionMode::Pointer => crate::i18n_bridge::tr("settings_input_pointer"),
        InputInteractionMode::Touch => crate::i18n_bridge::tr("settings_input_touch"),
    }
}
