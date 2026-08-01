//! Terminal profile create/edit dialog (standalone nested page).

use crate::config::{BellStyle, CursorStyle, TerminalTheme, TerminalType};
use crate::fonts;
use crate::persist::types::TerminalProfile;
use crate::ui::uiframe::keyboard::KeyboardMode;
use crate::ui::uiframe::style;

fn android_ime_for_text_edit(ui: &egui::Ui, resp: &egui::Response, force: bool) {
    #[cfg(target_os = "android")]
    {
        if force || resp.gained_focus() || resp.clicked() {
            crate::platform::android_ime::prepare_text_field_ime(ui.ctx(), resp.rect);
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (ui, resp, force);
    }
}

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
        let mut draft = TerminalProfile::default();
        draft.is_default = false;
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
            ui.horizontal(|ui| {
                ui.label(rust_i18n::t!("settings_profile_name"));
                let resp = ui.text_edit_singleline(&mut self.draft.name);
                android_ime_for_text_edit(ui, &resp, self.request_name_focus);
                if self.request_name_focus {
                    resp.request_focus();
                    self.request_name_focus = false;
                }
            });
            ui.horizontal(|ui| {
                ui.label(rust_i18n::t!("settings_profile_description"));
                let resp = ui.text_edit_singleline(&mut self.draft.description);
                android_ime_for_text_edit(ui, &resp, false);
            });

            ui.add_space(8.0);
            ui.label(egui::RichText::new(rust_i18n::t!("settings_tab_appearance")).strong());
            ui.horizontal(|ui| {
                ui.label(rust_i18n::t!("settings_font_size"));
                ui.add(egui::Slider::new(&mut self.draft.font_size, 8.0..=32.0));
            });
            ui.horizontal(|ui| {
                ui.label(rust_i18n::t!("settings_line_spacing"));
                ui.add(egui::Slider::new(&mut self.draft.line_spacing, 0.8..=2.0));
            });
            ui.horizontal(|ui| {
                ui.label(rust_i18n::t!("settings_cell_width"));
                ui.add(egui::Slider::new(&mut self.draft.cell_width_scale, 0.7..=1.5));
            });
            ui.horizontal(|ui| {
                ui.label(rust_i18n::t!("settings_terminal_font"));
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
                            .width(220.0)
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
            ui.horizontal_wrapped(|ui| {
                ui.label(rust_i18n::t!("settings_cursor_style"));
                for style_opt in CursorStyle::ALL {
                    ui.selectable_value(
                        &mut self.draft.cursor_style,
                        style_opt,
                        style_opt.label(),
                    );
                }
            });
            ui.horizontal(|ui| {
                ui.label(rust_i18n::t!("settings_default_keyboard"));
                for mode in [KeyboardMode::Full, KeyboardMode::Special] {
                    let label = match mode {
                        KeyboardMode::Full => rust_i18n::t!("settings_keyboard_full"),
                        KeyboardMode::Special => rust_i18n::t!("settings_keyboard_special"),
                    };
                    ui.selectable_value(&mut self.draft.keyboard_mode, mode, label);
                }
            });

            ui.add_space(8.0);
            ui.label(egui::RichText::new(rust_i18n::t!("settings_tab_theme")).strong());
            ui.horizontal(|ui| {
                ui.label(rust_i18n::t!("settings_theme_preset"));
                egui::ComboBox::from_id_salt("profile_dialog_theme_preset")
                    .selected_text("—")
                    .width(180.0)
                    .show_ui(ui, |ui| {
                        for (name, preset_fn) in TerminalTheme::presets() {
                            if ui.selectable_label(false, name).clicked() {
                                self.draft.theme = preset_fn();
                            }
                        }
                    });
            });
            ui.horizontal(|ui| {
                color_edit(ui, &rust_i18n::t!("theme_bg"), &mut self.draft.theme.bg);
                color_edit(ui, &rust_i18n::t!("theme_fg"), &mut self.draft.theme.fg);
                color_edit(ui, &rust_i18n::t!("theme_cursor"), &mut self.draft.theme.cursor);
            });

            ui.add_space(8.0);
            ui.label(egui::RichText::new(rust_i18n::t!("settings_tab_behavior")).strong());
            ui.horizontal(|ui| {
                ui.label(rust_i18n::t!("settings_terminal_type"));
                egui::ComboBox::from_id_salt("profile_dialog_term_type")
                    .selected_text(self.draft.terminal_type.label())
                    .show_ui(ui, |ui| {
                        for t in TerminalType::ALL {
                            ui.selectable_value(&mut self.draft.terminal_type, t, t.label());
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label(rust_i18n::t!("settings_bell"));
                for b in BellStyle::ALL {
                    ui.selectable_value(&mut self.draft.bell, b, b.label());
                }
            });
            ui.checkbox(
                &mut self.draft.bold_is_bright,
                rust_i18n::t!("settings_bold_is_bright"),
            );
            ui.checkbox(
                &mut self.draft.enable_bracketed_paste,
                rust_i18n::t!("settings_bracketed_paste"),
            );
            ui.checkbox(
                &mut self.draft.enable_sgr_mouse,
                rust_i18n::t!("settings_sgr_mouse"),
            );
            ui.checkbox(&mut self.draft.auto_wrap, rust_i18n::t!("settings_auto_wrap"));
            ui.horizontal(|ui| {
                ui.label(rust_i18n::t!("settings_scrollback_lines"));
                ui.add(egui::Slider::new(&mut self.draft.scrollback_lines, 100..=100_000));
            });
            ui.horizontal(|ui| {
                ui.label(rust_i18n::t!("settings_word_separators"));
                let resp = ui.text_edit_singleline(&mut self.draft.word_separators);
                android_ime_for_text_edit(ui, &resp, false);
            });

            ui.add_space(16.0);
            ui.horizontal(|ui| {
                let cancel = egui::Button::new(rust_i18n::t!("cancel"))
                    .fill(ui.visuals().panel_fill)
                    .corner_radius(style::CORNER_RADIUS_SM)
                    .min_size(egui::vec2(90.0, 34.0));
                if ui.add(cancel).clicked() {
                    close_requested = true;
                }
                let can_save = !self.draft.name.trim().is_empty();
                let save_btn = egui::Button::new(
                    egui::RichText::new(rust_i18n::t!("save")).color(egui::Color32::WHITE),
                )
                .fill(style::ACCENT)
                .corner_radius(style::CORNER_RADIUS_SM)
                .min_size(egui::vec2(90.0, 34.0));
                if ui.add_enabled(can_save, save_btn).clicked() {
                    self.draft.name = self.draft.name.trim().to_string();
                    saved = Some(self.draft.clone());
                    close_requested = true;
                }
            });
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

fn color_edit(ui: &mut egui::Ui, label: &str, color: &mut egui::Color32) {
    ui.label(label);
    let mut rgb = [
        color.r() as f32 / 255.0,
        color.g() as f32 / 255.0,
        color.b() as f32 / 255.0,
    ];
    if ui.color_edit_button_rgb(&mut rgb).changed() {
        *color = egui::Color32::from_rgb(
            (rgb[0] * 255.0) as u8,
            (rgb[1] * 255.0) as u8,
            (rgb[2] * 255.0) as u8,
        );
    }
}
