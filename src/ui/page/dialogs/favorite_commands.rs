//! Favorite command create/edit dialog and manage list dialog.

use crate::data::persist::types::FavoriteCommand;
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

fn dialog_text_edit(ui: &mut egui::Ui, text: &mut String) -> egui::Response {
    let resp = ui.text_edit_singleline(text);
    android_ime_for_text_edit(ui, &resp, false);
    resp
}

/// Outcome of the favorite-command editor for one frame.
pub enum FavoriteCommandOutcome {
    None,
    Saved(FavoriteCommand),
}

/// Create / edit a favorite command.
#[derive(Default)]
pub struct FavoriteCommandDialog {
    pub open: bool,
    edit_id: Option<String>,
    name: String,
    command: String,
    auto_execute: bool,
    request_name_focus: bool,
    sort_order: i64,
}

impl FavoriteCommandDialog {
    pub fn open_new(&mut self) {
        *self = Self {
            open: true,
            edit_id: None,
            name: String::new(),
            command: String::new(),
            auto_execute: false,
            request_name_focus: true,
            sort_order: 0,
        };
    }

    pub fn open_edit(&mut self, cmd: &FavoriteCommand) {
        *self = Self {
            open: true,
            edit_id: Some(cmd.id.clone()),
            name: cmd.name.clone(),
            command: cmd.command.clone(),
            auto_execute: cmd.auto_execute,
            request_name_focus: true,
            sort_order: cmd.sort_order,
        };
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> FavoriteCommandOutcome {
        if !self.open {
            return FavoriteCommandOutcome::None;
        }

        let mut close_requested = false;
        let mut saved = None;
        let title = if self.edit_id.is_some() {
            rust_i18n::t!("cmd_dialog_edit_title")
        } else {
            rust_i18n::t!("cmd_dialog_new_title")
        };

        use crate::ui::uiframe::{DialogFrame, DialogOutcome};
        let frame = DialogFrame::new(title.to_string()).foreground();
        let closed = frame.show(ctx, "favorite_command_dialog", |ui| {
                ui.add_space(4.0);

                ui.label(rust_i18n::t!("cmd_dialog_name"));
                let name_resp = dialog_text_edit(ui, &mut self.name);
                if self.request_name_focus {
                    name_resp.request_focus();
                    self.request_name_focus = false;
                }

                ui.add_space(8.0);
                ui.label(rust_i18n::t!("cmd_dialog_command"));
                let cmd_edit = egui::TextEdit::multiline(&mut self.command)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY);
                let cmd_resp = ui.add(cmd_edit);
                android_ime_for_text_edit(ui, &cmd_resp, false);

                ui.add_space(8.0);
                ui.checkbox(
                    &mut self.auto_execute,
                    rust_i18n::t!("cmd_dialog_auto_execute"),
                );
                ui.label(
                    egui::RichText::new(rust_i18n::t!("cmd_dialog_auto_execute_hint"))
                        .size(11.0)
                        .color(ui.visuals().weak_text_color()),
                );

                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    let cancel = egui::Button::new(rust_i18n::t!("cancel"))
                        .fill(ui.visuals().panel_fill)
                        .corner_radius(style::CORNER_RADIUS_SM)
                        .min_size(egui::vec2(90.0, 34.0));
                    if ui.add(cancel).clicked() {
                        close_requested = true;
                    }

                    let can_save = !self.name.trim().is_empty() && !self.command.trim().is_empty();
                    let save_btn = egui::Button::new(
                        egui::RichText::new(rust_i18n::t!("save"))
                            .color(egui::Color32::WHITE),
                    )
                    .fill(style::ACCENT)
                    .corner_radius(style::CORNER_RADIUS_SM)
                    .min_size(egui::vec2(90.0, 34.0));
                    if ui.add_enabled(can_save, save_btn).clicked() {
                        let cmd = if let Some(id) = &self.edit_id {
                            FavoriteCommand {
                                id: id.clone(),
                                name: self.name.trim().to_string(),
                                command: self.command.clone(),
                                auto_execute: self.auto_execute,
                                sort_order: self.sort_order,
                            }
                        } else {
                            FavoriteCommand::new(
                                self.name.trim(),
                                &self.command,
                                self.auto_execute,
                            )
                        };
                        saved = Some(cmd);
                        close_requested = true;
                    }
                });
        }) == DialogOutcome::Closed;

        if closed || close_requested {
            self.open = false;
        }

        match saved {
            Some(cmd) => FavoriteCommandOutcome::Saved(cmd),
            None => FavoriteCommandOutcome::None,
        }
    }
}

/// Actions from the manage-commands dialog.
#[derive(Default)]
pub struct ManageCommandsAction {
    pub new: bool,
    pub edit_id: Option<String>,
    pub delete_id: Option<String>,
}

/// List / manage all favorite commands in a centered dialog.
#[derive(Default)]
pub struct ManageFavoriteCommandsDialog {
    pub open: bool,
}

impl ManageFavoriteCommandsDialog {
    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        commands: &[FavoriteCommand],
    ) -> ManageCommandsAction {
        let mut action = ManageCommandsAction::default();
        if !self.open {
            return action;
        }

        use crate::ui::uiframe::{DialogFrame, DialogOutcome};
        let frame = DialogFrame::new(rust_i18n::t!("cmd_manage_title").to_string());
        if frame.show(ctx, "manage_favorite_commands", |ui| {
                ui.horizontal(|ui| {
                    let new_btn = egui::Button::new(
                        egui::RichText::new(rust_i18n::t!("cmd_manage_new"))
                            .color(egui::Color32::WHITE),
                    )
                    .fill(style::ACCENT)
                    .corner_radius(style::CORNER_RADIUS_SM);
                    if ui.add(new_btn).clicked() {
                        action.new = true;
                    }
                });
                ui.add_space(8.0);
                ui.separator();

                if commands.is_empty() {
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new(rust_i18n::t!("cmd_empty"))
                            .color(ui.visuals().weak_text_color()),
                    );
                } else {
                    for cmd in commands {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(&cmd.name).strong().size(13.0),
                                );
                                let preview = if cmd.command.len() > 60 {
                                    format!("{}…", &cmd.command[..60])
                                } else {
                                    cmd.command.clone()
                                };
                                ui.label(
                                    egui::RichText::new(preview)
                                        .size(11.0)
                                        .color(ui.visuals().weak_text_color()),
                                );
                                if cmd.auto_execute {
                                    ui.label(
                                        egui::RichText::new(rust_i18n::t!("cmd_badge_auto"))
                                            .size(10.0)
                                            .color(style::ACCENT),
                                    );
                                }
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button(rust_i18n::t!("delete")).clicked() {
                                        action.delete_id = Some(cmd.id.clone());
                                    }
                                    if ui.small_button(rust_i18n::t!("edit")).clicked() {
                                        action.edit_id = Some(cmd.id.clone());
                                    }
                                },
                            );
                        });
                        ui.separator();
                    }
                }
        }) == DialogOutcome::Closed
        {
            self.open = false;
        }
        action
    }
}
