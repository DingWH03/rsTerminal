//! Auth user (SSH identity) create/edit dialog.

use crate::data::persist::types::{AuthMethod, AuthUser};
use crate::ui::uiframe::form::{self, FooterAction};

/// Create / edit an SSH auth user.
#[derive(Default)]
pub struct AuthUserDialog {
    pub open: bool,
    edit_id: Option<String>,
    name: String,
    username: String,
    auth_method: AuthMethod,
    password: String,
    private_key: String,
    key_passphrase: String,
    request_name_focus: bool,
    /// When set by parent after save-from-new, connection form can select this id.
    pub last_saved_id: Option<String>,
}

impl AuthUserDialog {
    pub fn open_new(&mut self) {
        *self = Self {
            open: true,
            request_name_focus: true,
            ..Default::default()
        };
    }

    pub fn open_edit(&mut self, user: &AuthUser) {
        *self = Self {
            open: true,
            edit_id: Some(user.id.clone()),
            name: user.name.clone(),
            username: user.username.clone(),
            auth_method: user.auth_method,
            password: user.password.clone().unwrap_or_default(),
            private_key: user.private_key.clone().unwrap_or_default(),
            key_passphrase: user.key_passphrase.clone().unwrap_or_default(),
            request_name_focus: true,
            last_saved_id: None,
        };
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<AuthUser> {
        if !self.open {
            return None;
        }

        let mut close_requested = false;
        let mut saved = None;
        let title = if self.edit_id.is_some() {
            rust_i18n::t!("auth_user_edit_title")
        } else {
            rust_i18n::t!("auth_user_new_title")
        };

        use crate::ui::uiframe::{DialogFrame, DialogOutcome};
        let frame = DialogFrame::new(title.to_string()).foreground();
        let closed = frame.show(ctx, "auth_user_dialog", |ui| {
            ui.add_space(4.0);

            let name_resp = form::labeled_text(ui, rust_i18n::t!("auth_user_name"), &mut self.name);
            if self.request_name_focus {
                name_resp.request_focus();
                self.request_name_focus = false;
            }
            form::labeled_text(ui, rust_i18n::t!("auth_user_username"), &mut self.username);

            form::labeled_row(ui, rust_i18n::t!("auth_user_method"), |ui| {
                ui.radio_value(
                    &mut self.auth_method,
                    AuthMethod::Password,
                    rust_i18n::t!("auth_user_method_password"),
                );
                ui.radio_value(
                    &mut self.auth_method,
                    AuthMethod::PrivateKey,
                    rust_i18n::t!("auth_user_method_key"),
                );
            });

            match self.auth_method {
                AuthMethod::Password => {
                    form::labeled_password(
                        ui,
                        rust_i18n::t!("auth_user_password"),
                        &mut self.password,
                    );
                }
                AuthMethod::PrivateKey => {
                    form::labeled_row(ui, rust_i18n::t!("auth_user_private_key"), |ui| {
                        #[cfg(not(target_os = "android"))]
                        {
                            if ui
                                .button(rust_i18n::t!("auth_user_pick_key_file"))
                                .clicked()
                            {
                                if let Some(path) = rfd::FileDialog::new()
                                    .set_title(rust_i18n::t!("auth_user_pick_key_file"))
                                    .pick_file()
                                {
                                    if let Ok(data) = std::fs::read_to_string(&path) {
                                        self.private_key = data;
                                    }
                                }
                            }
                        }
                    });
                    let key_edit = egui::TextEdit::multiline(&mut self.private_key)
                        .desired_rows(6)
                        .desired_width(f32::INFINITY)
                        .hint_text(rust_i18n::t!("auth_user_key_hint"));
                    let resp = ui.add(key_edit);
                    form::android_ime_for_text_edit(ui, &resp, false);
                    ui.add_space(form::FIELD_GAP);
                    form::labeled_password(
                        ui,
                        rust_i18n::t!("auth_user_key_passphrase"),
                        &mut self.key_passphrase,
                    );
                }
            }

            let can_save = !self.name.trim().is_empty()
                && !self.username.trim().is_empty()
                && match self.auth_method {
                    AuthMethod::Password => !self.password.is_empty(),
                    AuthMethod::PrivateKey => !self.private_key.trim().is_empty(),
                };
            match form::dialog_footer(
                ui,
                rust_i18n::t!("cancel"),
                rust_i18n::t!("save"),
                can_save,
            ) {
                FooterAction::Cancel => close_requested = true,
                FooterAction::Save => {
                    let id = self
                        .edit_id
                        .clone()
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                    let user = match self.auth_method {
                        AuthMethod::Password => AuthUser {
                            id,
                            name: self.name.trim().to_string(),
                            username: self.username.trim().to_string(),
                            auth_method: AuthMethod::Password,
                            password: Some(self.password.clone()),
                            private_key: None,
                            key_passphrase: None,
                        },
                        AuthMethod::PrivateKey => AuthUser {
                            id,
                            name: self.name.trim().to_string(),
                            username: self.username.trim().to_string(),
                            auth_method: AuthMethod::PrivateKey,
                            password: None,
                            private_key: Some(self.private_key.clone()),
                            key_passphrase: if self.key_passphrase.is_empty() {
                                None
                            } else {
                                Some(self.key_passphrase.clone())
                            },
                        },
                    };
                    saved = Some(user);
                    close_requested = true;
                }
                FooterAction::None => {}
            }
        }) == DialogOutcome::Closed;

        if closed || close_requested {
            self.open = false;
        }
        if let Some(ref u) = saved {
            self.last_saved_id = Some(u.id.clone());
        }
        saved
    }
}

/// Actions from the Users settings page / standalone dialog.
#[derive(Debug, Default, Clone)]
pub struct ManageAuthUsersAction {
    pub new: bool,
    pub edit_id: Option<String>,
    pub delete_id: Option<String>,
}

/// Embeddable Users list (Settings tab or standalone page body).
pub fn auth_users_page(ui: &mut egui::Ui, auth_users: &[AuthUser], action: &mut ManageAuthUsersAction) {
    form::section_card(ui, |ui| {
        if form::manage_list_toolbar(ui, rust_i18n::t!("auth_users_manage_new")) {
            action.new = true;
        }

        if auth_users.is_empty() {
            ui.add_space(form::SECTION_GAP);
            ui.label(
                egui::RichText::new(rust_i18n::t!("settings_users_empty"))
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        }

        for user in auth_users {
            let method = match user.auth_method {
                AuthMethod::Password => rust_i18n::t!("auth_user_method_password"),
                AuthMethod::PrivateKey => rust_i18n::t!("auth_user_method_key"),
            };
            form::manage_list_item_frame(ui, false, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(&user.name).strong().size(13.0));
                        ui.label(
                            egui::RichText::new(format!("{} · {}", user.username, method))
                                .size(11.0)
                                .color(ui.visuals().weak_text_color()),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(rust_i18n::t!("delete")).clicked() {
                            action.delete_id = Some(user.id.clone());
                        }
                        if ui.small_button(rust_i18n::t!("edit")).clicked() {
                            action.edit_id = Some(user.id.clone());
                        }
                    });
                });
            });
            ui.add_space(4.0);
        }
    });
}

/// Preferences → Users standalone window. Returns `true` when closed.
pub fn manage_auth_users_dialog(
    ctx: &egui::Context,
    auth_users: &[AuthUser],
    action: &mut ManageAuthUsersAction,
) -> bool {
    use crate::ui::uiframe::{DialogFrame, DialogOutcome};

    let frame = DialogFrame::new(rust_i18n::t!("auth_users_manage_title").to_string());

    let outcome = frame.show(ctx, "manage_auth_users_dialog", |ui| {
        auth_users_page(ui, auth_users, action);
    });

    matches!(outcome, DialogOutcome::Closed)
}
