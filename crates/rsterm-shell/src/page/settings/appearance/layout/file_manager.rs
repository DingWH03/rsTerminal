//! File manager page under Appearance > Layout.

use rsterm_data::prefs::{PrefsFilePaneLayout, PrefsFileViewMode};

use crate::page::settings::SettingsPageCtx;
use crate::uiframe::form;

pub fn page(ui: &mut egui::Ui, ctx: &mut SettingsPageCtx<'_>) {
    form::section_card(ui, |ui| {
        form::labeled_combo(
            ui,
            "prefs_fm_view",
            crate::i18n_bridge::tr("fm_pref_view"),
            view_label(ctx.prefs.file_manager.view_mode),
            |ui| {
                for mode in [
                    PrefsFileViewMode::List,
                    PrefsFileViewMode::Details,
                    PrefsFileViewMode::IconsSmall,
                    PrefsFileViewMode::IconsLarge,
                ] {
                    if ui
                        .selectable_label(
                            ctx.prefs.file_manager.view_mode == mode,
                            view_label(mode),
                        )
                        .clicked()
                    {
                        ctx.prefs.file_manager.view_mode = mode;
                    }
                }
            },
        );
        ui.horizontal(|ui| {
            ui.label(crate::i18n_bridge::tr("fm_pref_layout"));
            let mut dual = matches!(
                ctx.prefs.file_manager.pane_layout,
                PrefsFilePaneLayout::Dual
            );
            if ui
                .checkbox(&mut dual, crate::i18n_bridge::tr("fm_layout_dual"))
                .changed()
            {
                ctx.prefs.file_manager.pane_layout = if dual {
                    PrefsFilePaneLayout::Dual
                } else {
                    PrefsFilePaneLayout::Single
                };
            }
        });
        ui.horizontal(|ui| {
            let mut hidden = ctx.prefs.file_manager.show_hidden;
            if ui
                .checkbox(&mut hidden, crate::i18n_bridge::tr("fm_show_hidden"))
                .changed()
            {
                ctx.prefs.file_manager.show_hidden = hidden;
            }
        });
    });
}

fn view_label(mode: PrefsFileViewMode) -> String {
    match mode {
        PrefsFileViewMode::List => crate::i18n_bridge::tr("fm_view_list"),
        PrefsFileViewMode::Details => crate::i18n_bridge::tr("fm_view_details"),
        PrefsFileViewMode::IconsSmall => crate::i18n_bridge::tr("fm_view_icons_small"),
        PrefsFileViewMode::IconsLarge => crate::i18n_bridge::tr("fm_view_icons_large"),
    }
}
