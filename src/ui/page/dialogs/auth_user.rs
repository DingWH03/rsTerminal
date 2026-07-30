//! Auth user (SSH identity) create/edit dialog.

use crate::persist::types::{AuthMethod, AuthUser};
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

                ui.horizontal(|ui| {
                    ui.label(rust_i18n::t!("auth_user_name"));
                    let resp = dialog_text_edit(ui, &mut self.name);
                    if self.request_name_focus {
                        resp.request_focus();
                        self.request_name_focus = false;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(rust_i18n::t!("auth_user_username"));
                    dialog_text_edit(ui, &mut self.username);
                });

                ui.add_space(6.0);
                ui.label(rust_i18n::t!("auth_user_method"));
                ui.horizontal(|ui| {
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

                ui.add_space(6.0);
                match self.auth_method {
                    AuthMethod::Password => {
                        ui.horizontal(|ui| {
                            ui.label(rust_i18n::t!("auth_user_password"));
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut self.password).password(true),
                            );
                            android_ime_for_text_edit(ui, &resp, false);
                        });
                    }
                    AuthMethod::PrivateKey => {
                        ui.horizontal(|ui| {
                            ui.label(rust_i18n::t!("auth_user_private_key"));
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
                        android_ime_for_text_edit(ui, &resp, false);
                        ui.horizontal(|ui| {
                            ui.label(rust_i18n::t!("auth_user_key_passphrase"));
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut self.key_passphrase).password(true),
                            );
                            android_ime_for_text_edit(ui, &resp, false);
                        });
                    }
                }

                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    let cancel = egui::Button::new(rust_i18n::t!("cancel"))
                        .fill(ui.visuals().panel_fill)
                        .corner_radius(style::CORNER_RADIUS_SM)
                        .min_size(egui::vec2(90.0, 34.0));
                    if ui.add(cancel).clicked() {
                        close_requested = true;
                    }

                    let can_save = !self.name.trim().is_empty()
                        && !self.username.trim().is_empty()
                        && match self.auth_method {
                            AuthMethod::Password => !self.password.is_empty(),
                            AuthMethod::PrivateKey => !self.private_key.trim().is_empty(),
                        };
                    let save_btn = egui::Button::new(
                        egui::RichText::new(rust_i18n::t!("save")).color(egui::Color32::WHITE),
                    )
                    .fill(style::ACCENT)
                    .corner_radius(style::CORNER_RADIUS_SM)
                    .min_size(egui::vec2(90.0, 34.0));
                    if ui.add_enabled(can_save, save_btn).clicked() {
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
                });
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

/// Actions from Preferences → Users manage dialog.
#[derive(Debug, Default, Clone)]
pub struct ManageAuthUsersAction {
    pub new: bool,
    pub edit_id: Option<String>,
    pub delete_id: Option<String>,
}

/// Preferences → Users: centered manage page. Returns `true` when closed.
pub fn manage_auth_users_dialog(
    ctx: &egui::Context,
    auth_users: &[AuthUser],
    action: &mut ManageAuthUsersAction,
) -> bool {
    use crate::ui::uiframe::{DialogFrame, DialogOutcome};

    let frame = DialogFrame::new(rust_i18n::t!("auth_users_manage_title").to_string());

    let outcome = frame.show(ctx, "manage_auth_users_dialog", |ui| {
        ui.horizontal(|ui| {
            let new_btn = egui::Button::new(
                egui::RichText::new(rust_i18n::t!("auth_users_manage_new"))
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

        if auth_users.is_empty() {
            ui.add_space(12.0);
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
            ui.separator();
        }
    });

    matches!(outcome, DialogOutcome::Closed)
}
