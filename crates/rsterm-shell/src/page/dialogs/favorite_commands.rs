//! Favorite command create/edit dialog and manage list dialog.

use rsterm_data::persist::types::FavoriteCommand;
use crate::uiframe::form::{self, FooterAction};
use crate::uiframe::style;

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
            crate::i18n_bridge::tr("cmd_dialog_edit_title")
        } else {
            crate::i18n_bridge::tr("cmd_dialog_new_title")
        };

        use crate::uiframe::{DialogFrame, DialogOutcome};
        let frame = DialogFrame::new(title.to_string()).foreground();
        let closed = frame.show(ctx, "favorite_command_dialog", |ui| {
            ui.add_space(4.0);

            let name_resp =
                form::labeled_text(ui, crate::i18n_bridge::tr("cmd_dialog_name"), &mut self.name);
            if self.request_name_focus {
                name_resp.request_focus();
                self.request_name_focus = false;
            }

            form::labeled_multiline(
                ui,
                crate::i18n_bridge::tr("cmd_dialog_command"),
                &mut self.command,
                3,
            );

            form::checkbox_row(
                ui,
                &mut self.auto_execute,
                crate::i18n_bridge::tr("cmd_dialog_auto_execute"),
            );
            ui.label(
                egui::RichText::new(crate::i18n_bridge::tr("cmd_dialog_auto_execute_hint"))
                    .size(11.0)
                    .color(ui.visuals().weak_text_color()),
            );

            let can_save = !self.name.trim().is_empty() && !self.command.trim().is_empty();
            match form::dialog_footer(ui, crate::i18n_bridge::tr("cancel"), crate::i18n_bridge::tr("save"), can_save)
            {
                FooterAction::Cancel => close_requested = true,
                FooterAction::Save => {
                    let cmd = if let Some(id) = &self.edit_id {
                        FavoriteCommand {
                            id: id.clone(),
                            name: self.name.trim().to_string(),
                            command: self.command.clone(),
                            auto_execute: self.auto_execute,
                            sort_order: self.sort_order,
                        }
                    } else {
                        FavoriteCommand::new(self.name.trim(), &self.command, self.auto_execute)
                    };
                    saved = Some(cmd);
                    close_requested = true;
                }
                FooterAction::None => {}
            }
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

        use crate::uiframe::{DialogFrame, DialogOutcome};
        let frame = DialogFrame::new(crate::i18n_bridge::tr("cmd_manage_title"));
        if frame.show(ctx, "manage_favorite_commands", |ui| {
            if form::manage_list_toolbar(ui, crate::i18n_bridge::tr("cmd_manage_new")) {
                action.new = true;
            }

            if commands.is_empty() {
                ui.add_space(form::SECTION_GAP);
                ui.label(
                    egui::RichText::new(crate::i18n_bridge::tr("cmd_empty"))
                        .color(ui.visuals().weak_text_color()),
                );
            } else {
                for cmd in commands {
                    form::manage_list_item_frame(ui, false, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(&cmd.name).strong().size(13.0));
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
                                        egui::RichText::new(crate::i18n_bridge::tr("cmd_badge_auto"))
                                            .size(10.0)
                                            .color(style::ACCENT),
                                    );
                                }
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button(crate::i18n_bridge::tr("delete")).clicked() {
                                        action.delete_id = Some(cmd.id.clone());
                                    }
                                    if ui.small_button(crate::i18n_bridge::tr("edit")).clicked() {
                                        action.edit_id = Some(cmd.id.clone());
                                    }
                                },
                            );
                        });
                    });
                    ui.add_space(4.0);
                }
            }
        }) == DialogOutcome::Closed
        {
            self.open = false;
        }
        action
    }
}
