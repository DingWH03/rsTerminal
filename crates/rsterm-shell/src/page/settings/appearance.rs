//! Global UI appearance (not terminal colors).

use rsterm_config::UiTheme;

use crate::host_hooks;
use crate::page::settings::SettingsPageCtx;
use crate::pane_colors::palette_for_theme;
use crate::uiframe::form;

pub fn page(ui: &mut egui::Ui, ctx: &mut SettingsPageCtx<'_>) {
    form::section_card(ui, |ui| {
        form::labeled_combo(
            ui,
            "prefs_ui_theme",
            crate::i18n_bridge::tr("ui_theme"),
            host_hooks::ui_theme_label(ctx.prefs.appearance.ui_theme),
            |ui| {
                for theme in UiTheme::ALL {
                    if ui
                        .selectable_label(
                            ctx.prefs.appearance.ui_theme == theme,
                            host_hooks::ui_theme_label(theme),
                        )
                        .clicked()
                    {
                        ctx.prefs.appearance.ui_theme = theme;
                        // Apply immediately so the settings window itself updates.
                        host_hooks::apply_ui_theme(theme, ui.ctx());
                    }
                }
            },
        );

        form::section_header(ui, crate::i18n_bridge::tr("settings_pane_colors"));
        ui.label(
            egui::RichText::new(crate::i18n_bridge::tr("settings_pane_colors_desc"))
                .size(11.0)
                .weak(),
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui
                .button(crate::i18n_bridge::tr("settings_pane_colors_theme"))
                .clicked()
            {
                ctx.prefs.appearance.pane_accent_colors =
                    palette_for_theme(ctx.prefs.appearance.ui_theme);
            }
            if ui
                .button(crate::i18n_bridge::tr("settings_pane_colors_reset"))
                .clicked()
            {
                ctx.prefs.appearance.pane_accent_colors.clear();
            }
        });

        let colors = if ctx.prefs.appearance.pane_accent_colors.is_empty() {
            palette_for_theme(ctx.prefs.appearance.ui_theme)
        } else {
            ctx.prefs.appearance.pane_accent_colors.clone()
        };
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            for c in colors {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
                if ui.is_rect_visible(rect) {
                    ui.painter()
                        .rect_filled(rect, 3.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
                }
            }
        });
    });
}
