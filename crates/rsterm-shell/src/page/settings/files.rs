//! Files settings — default FM view mode and pane layout.

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
        form::labeled_combo(
            ui,
            "prefs_fm_layout",
            crate::i18n_bridge::tr("fm_pref_layout"),
            layout_label(ctx.prefs.file_manager.pane_layout),
            |ui| {
                for layout in [PrefsFilePaneLayout::Dual, PrefsFilePaneLayout::Single] {
                    if ui
                        .selectable_label(
                            ctx.prefs.file_manager.pane_layout == layout,
                            layout_label(layout),
                        )
                        .clicked()
                    {
                        ctx.prefs.file_manager.pane_layout = layout;
                    }
                }
            },
        );
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

fn layout_label(layout: PrefsFilePaneLayout) -> String {
    match layout {
        PrefsFilePaneLayout::Single => crate::i18n_bridge::tr("fm_layout_single"),
        PrefsFilePaneLayout::Dual => crate::i18n_bridge::tr("fm_layout_dual"),
    }
}
