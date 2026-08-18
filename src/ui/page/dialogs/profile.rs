//! Terminal profile create/edit dialog (standalone nested page).

use crate::config::{BellStyle, CursorStyle, KeyboardMode, TerminalTheme, TerminalType};
use crate::data::persist::types::TerminalProfile;
use crate::fonts;
use crate::ui::uiframe::form::{self, FooterAction};

#[allow(clippy::large_enum_variant)] // Saved carries a full profile draft; rare short-lived dialog outcome.
pub enum ProfileDialogOutcome {
    None,
    Saved(TerminalProfile),
}

#[derive(Default)]
pub struct ProfileDialog {
    pub open: bool,
    draft: TerminalProfile,
    is_new: bool,
    request_name_focus: bool,
    pub last_saved_id: Option<String>,
}

impl ProfileDialog {
    pub fn open_new(&mut self) {
        let mut draft = TerminalProfile {
            is_default: false,
            ..Default::default()
        };
        draft.name.clear();
        *self = Self {
            open: true,
            draft,
            is_new: true,
            request_name_focus: true,
            last_saved_id: None,
        };
    }

    pub fn open_edit(&mut self, profile: &TerminalProfile) {
        *self = Self {
            open: true,
            draft: profile.clone(),
            is_new: false,
            request_name_focus: true,
            last_saved_id: None,
        };
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> ProfileDialogOutcome {
        if !self.open {
            return ProfileDialogOutcome::None;
        }

        let mut close_requested = false;
        let mut saved = None;
        let title = if self.is_new {
            rust_i18n::t!("profile_dialog_new_title")
        } else {
            rust_i18n::t!("profile_dialog_edit_title")
        };

        use crate::ui::uiframe::{DialogFrame, DialogOutcome};
        let frame = DialogFrame::new(title.to_string()).foreground();
        let closed = frame.show(ctx, "profile_dialog", |ui| {
            ui.add_space(4.0);
            let name_resp = form::labeled_text(
                ui,
                rust_i18n::t!("settings_profile_name"),
                &mut self.draft.name,
            );
            if self.request_name_focus {
                name_resp.request_focus();
                self.request_name_focus = false;
            }
            form::labeled_text(
                ui,
                rust_i18n::t!("settings_profile_description"),
                &mut self.draft.description,
            );

            form::section_header(ui, rust_i18n::t!("settings_tab_appearance"));
            form::labeled_slider(
                ui,
                rust_i18n::t!("settings_font_size"),
                &mut self.draft.font_size,
                8.0..=32.0,
            );
            form::labeled_slider(
                ui,
                rust_i18n::t!("settings_line_spacing"),
                &mut self.draft.line_spacing,
                0.8..=2.0,
            );
            form::labeled_slider(
                ui,
                rust_i18n::t!("settings_cell_width"),
                &mut self.draft.cell_width_scale,
                0.7..=1.5,
            );
            form::labeled_row(ui, rust_i18n::t!("settings_terminal_font"), |ui| {
                match fonts::monospace_catalog_status() {
                    fonts::MonospaceCatalogStatus::Loading => {
                        ui.label(rust_i18n::t!("settings_terminal_font_loading"));
                        ui.ctx()
                            .request_repaint_after(std::time::Duration::from_millis(150));
                    }
                    fonts::MonospaceCatalogStatus::Ready(entries) => {
                        let label = if self.draft.terminal_font.is_empty() {
                            rust_i18n::t!("settings_terminal_font_auto").into_owned()
                        } else {
                            std::path::Path::new(&self.draft.terminal_font)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(self.draft.terminal_font.as_str())
                                .to_owned()
                        };
                        egui::ComboBox::from_id_salt("profile_dialog_font")
                            .selected_text(label)
                            .width(form::COMBO_WIDTH)
                            .show_ui(ui, |ui| {
                                for entry in entries.iter() {
                                    if ui
                                        .selectable_label(
                                            self.draft.terminal_font == entry.path,
                                            &entry.label,
                                        )
                                        .clicked()
                                    {
                                        self.draft.terminal_font = entry.path.clone();
                                        fonts::apply_terminal_fonts(ui.ctx(), &entry.path);
                                    }
                                }
                            });
                    }
                }
            });
            form::segmented_row(
                ui,
                rust_i18n::t!("settings_cursor_style"),
                &mut self.draft.cursor_style,
                CursorStyle::ALL.iter().map(|s| (*s, s.label().to_string())),
            );
            form::segmented_row(
                ui,
                rust_i18n::t!("settings_default_keyboard"),
                &mut self.draft.keyboard_mode,
                [
                    (
                        KeyboardMode::Full,
                        rust_i18n::t!("settings_keyboard_full").into_owned(),
                    ),
                    (
                        KeyboardMode::Special,
                        rust_i18n::t!("settings_keyboard_special").into_owned(),
                    ),
                ],
            );

            form::section_header(ui, rust_i18n::t!("settings_tab_theme"));
            form::labeled_combo(
                ui,
                "profile_dialog_theme_preset",
                rust_i18n::t!("settings_theme_preset"),
                "—",
                |ui| {
                    for (name, preset_fn) in TerminalTheme::presets() {
                        if ui.selectable_label(false, name).clicked() {
                            self.draft.theme = preset_fn();
                        }
                    }
                },
            );
            ui.horizontal(|ui| {
                color_edit_btn(ui, &rust_i18n::t!("theme_bg"), &mut self.draft.theme.bg);
                color_edit_btn(ui, &rust_i18n::t!("theme_fg"), &mut self.draft.theme.fg);
                color_edit_btn(
                    ui,
                    &rust_i18n::t!("theme_cursor"),
                    &mut self.draft.theme.cursor,
                );
            });
            ui.add_space(form::FIELD_GAP);

            form::section_header(ui, rust_i18n::t!("settings_tab_behavior"));
            form::labeled_combo(
                ui,
                "profile_dialog_term_type",
                rust_i18n::t!("settings_terminal_type"),
                self.draft.terminal_type.label(),
                |ui| {
                    for t in TerminalType::ALL {
                        ui.selectable_value(&mut self.draft.terminal_type, t, t.label());
                    }
                },
            );
            form::segmented_row(
                ui,
                rust_i18n::t!("settings_bell"),
                &mut self.draft.bell,
                BellStyle::ALL.iter().map(|b| (*b, b.label().to_string())),
            );
            form::checkbox_row(
                ui,
                &mut self.draft.bold_is_bright,
                rust_i18n::t!("settings_bold_is_bright"),
            );
            form::checkbox_row(
                ui,
                &mut self.draft.enable_bracketed_paste,
                rust_i18n::t!("settings_bracketed_paste"),
            );
            form::checkbox_row(
                ui,
                &mut self.draft.enable_sgr_mouse,
                rust_i18n::t!("settings_sgr_mouse"),
            );
            form::checkbox_row(
                ui,
                &mut self.draft.auto_wrap,
                rust_i18n::t!("settings_auto_wrap"),
            );
            form::labeled_slider(
                ui,
                rust_i18n::t!("settings_scrollback_lines"),
                &mut self.draft.scrollback_lines,
                100..=100_000,
            );
            form::labeled_text(
                ui,
                rust_i18n::t!("settings_word_separators"),
                &mut self.draft.word_separators,
            );

            let can_save = !self.draft.name.trim().is_empty();
            match form::dialog_footer(ui, rust_i18n::t!("cancel"), rust_i18n::t!("save"), can_save)
            {
                FooterAction::Cancel => close_requested = true,
                FooterAction::Save => {
                    self.draft.name = self.draft.name.trim().to_string();
                    saved = Some(self.draft.clone());
                    close_requested = true;
                }
                FooterAction::None => {}
            }
        }) == DialogOutcome::Closed;

        if closed || close_requested {
            self.open = false;
        }
        if let Some(profile) = saved {
            self.last_saved_id = Some(profile.id.clone());
            ProfileDialogOutcome::Saved(profile)
        } else {
            ProfileDialogOutcome::None
        }
    }
}

fn color_edit_btn(ui: &mut egui::Ui, label: &str, color: &mut crate::config::Rgba) {
    ui.label(label);
    let mut rgb = [
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
    ];
    if ui.color_edit_button_rgb(&mut rgb).changed() {
        *color = crate::config::Rgba::from_rgb(
            (rgb[0] * 255.0) as u8,
            (rgb[1] * 255.0) as u8,
            (rgb[2] * 255.0) as u8,
        );
    }
}
