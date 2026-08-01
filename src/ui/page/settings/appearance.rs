//! Global UI appearance (not terminal colors).

use crate::i18n::UiTheme;
use crate::ui::page::settings::SettingsPageCtx;
use crate::ui::pane_colors::palette_for_theme;
use crate::ui::uiframe::style;

pub fn page(ui: &mut egui::Ui, ctx: &mut SettingsPageCtx<'_>) {
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(style::CORNER_RADIUS_SM)
        .inner_margin(egui::Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(rust_i18n::t!("settings_tab_appearance"))
                    .size(15.0)
                    .strong(),
            );
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label(rust_i18n::t!("ui_theme"));
                egui::ComboBox::from_id_salt("prefs_ui_theme")
                    .selected_text(ctx.prefs.appearance.ui_theme.label())
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for theme in UiTheme::ALL {
                            if ui
                                .selectable_label(
                                    ctx.prefs.appearance.ui_theme == theme,
                                    theme.label(),
                                )
                                .clicked()
                            {
                                ctx.prefs.appearance.ui_theme = theme;
                            }
                        }
                    });
            });

            ui.add_space(12.0);
            ui.label(rust_i18n::t!("settings_pane_colors"));
            ui.label(
                egui::RichText::new(rust_i18n::t!("settings_pane_colors_desc"))
                    .size(11.0)
                    .weak(),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui
                    .button(rust_i18n::t!("settings_pane_colors_theme"))
                    .clicked()
                {
                    ctx.prefs.appearance.pane_accent_colors =
                        palette_for_theme(ctx.prefs.appearance.ui_theme);
                }
                if ui
                    .button(rust_i18n::t!("settings_pane_colors_reset"))
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
            ui.horizontal(|ui| {
                for c in colors {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
                    if ui.is_rect_visible(rect) {
                        ui.painter().rect_filled(
                            rect,
                            3.0,
                            egui::Color32::from_rgb(c[0], c[1], c[2]),
                        );
                    }
                }
            });
        });
}
