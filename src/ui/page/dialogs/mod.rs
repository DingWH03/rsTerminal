//! 新建/编辑连接对话框。
//!
//! 支持四种连接类型（Local、SSH、Serial、BLE）的创建和编辑。
//! 包含自动扫描串口设备和 BLE 设备的功能。
//! 以居中弹出 Window 显示；窗口内顶部用下拉框选择类型，下方按类型显示配置。

pub mod notices;
pub mod favorite_commands;
pub mod auth_user;
pub mod profile;

pub use notices::{paint_connection_notice, paint_quit_confirm};
pub use favorite_commands::{
    FavoriteCommandDialog, FavoriteCommandOutcome, ManageCommandsAction,
    ManageFavoriteCommandsDialog,
};
pub use auth_user::{
    auth_users_page, manage_auth_users_dialog, AuthUserDialog, ManageAuthUsersAction,
};
pub use profile::{ProfileDialog, ProfileDialogOutcome};

use std::sync::mpsc;

use std::collections::HashMap;

use crate::connection::enumeration::{enumerate_serial_ports, scan_ble_devices_blocking};
use crate::persist::types::{
    default_local_env_vars, default_ssh_env_vars, AuthUser, ConnectionType, SavedConnection,
    TerminalProfile,
};
use crate::ui::uiframe::style;

/// On Android, show the soft keyboard for a focused dialog text field.
///
/// The terminal canvas uses a custom IME bridge with `IMEPurpose::Terminal`;
/// dialogs must reset to `Normal` and explicitly request soft input.
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

fn env_map_to_rows(map: &HashMap<String, String>) -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}

fn env_rows_to_map(rows: &[(String, String)]) -> HashMap<String, String> {
    rows.iter()
        .filter(|(k, _)| !k.trim().is_empty())
        .map(|(k, v)| (k.trim().to_string(), v.clone()))
        .collect()
}

/// Outcome of painting the connection form for one frame.
pub enum ConnectionFormOutcome {
    None,
    Cancelled,
    Saved(SavedConnection),
}

/// 新建/编辑连接表单状态。
pub struct NewConnectionDialog {
    pub open: bool,
    /// Request keyboard focus on the name field once after open.
    request_name_focus: bool,
    /// When set, form edits an existing connection (preserves id on save).
    edit_id: Option<String>,
    pub name: String,
    pub conn_type: ConnectionType,
    // Local
    pub shell: String,
    pub working_dir: String,
    // SSH
    pub ssh_host: String,
    pub ssh_port: String,
    /// Selected Preferences auth user id (required for SSH).
    pub selected_auth_user_id: Option<String>,
    /// Set when user picks "New user…" in the SSH combo.
    pub request_new_auth_user: bool,
    /// Terminal profile id (`None` = app default).
    pub selected_profile_id: Option<String>,
    /// Set when user picks "New profile…" in the profile combo.
    pub request_new_profile: bool,
    /// Per-connection environment variables (editable rows).
    pub env_rows: Vec<(String, String)>,
    // Serial
    pub serial_port: String,
    pub serial_baud: String,
    serial_devices: Vec<(String, String)>,
    // BLE
    pub ble_device: String,
    ble_devices: Vec<String>,
    ble_scanning: bool,
    ble_scan_rx: Option<mpsc::Receiver<Result<Vec<String>, String>>>,
    ble_scan_error: Option<String>,
}

impl Default for NewConnectionDialog {
    fn default() -> Self {
        Self {
            open: false,
            request_name_focus: false,
            edit_id: None,
            name: String::new(),
            conn_type: ConnectionType::Local,
            shell: String::new(),
            working_dir: String::new(),
            ssh_host: String::new(),
            ssh_port: "22".to_string(),
            selected_auth_user_id: None,
            request_new_auth_user: false,
            selected_profile_id: None,
            request_new_profile: false,
            env_rows: env_map_to_rows(&default_local_env_vars()),
            serial_port: String::new(),
            serial_baud: "115200".to_string(),
            serial_devices: Vec::new(),
            ble_device: String::new(),
            ble_devices: Vec::new(),
            ble_scanning: false,
            ble_scan_rx: None,
            ble_scan_error: None,
        }
    }
}

impl NewConnectionDialog {
    /// 打开新建连接表单，并预填本地连接的默认值。
    pub fn open_new(&mut self) {
        *self = Self::default();
        self.open = true;
        self.request_name_focus = true;
        // Pre-fill Local defaults: system shell and home directory.
        self.shell = crate::platform::get().default_shell();
        self.working_dir = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_default();
        self.env_rows = env_map_to_rows(&default_local_env_vars());
    }

    /// 打开编辑连接表单，用已有连接数据填充。
    pub fn open_edit(&mut self, conn: &SavedConnection) {
        *self = Self::default();
        self.open = true;
        self.request_name_focus = true;
        self.edit_id = Some(conn.id.clone());
        self.name = conn.name.clone();
        self.conn_type = conn.conn_type;
        self.shell = conn.shell.clone().unwrap_or_default();
        self.working_dir = conn.working_dir.clone().unwrap_or_default();
        self.ssh_host = conn.ssh_host.clone().unwrap_or_default();
        self.ssh_port = conn.ssh_port.map(|p| p.to_string()).unwrap_or_else(|| "22".into());
        self.selected_auth_user_id = conn.auth_user_id.clone();
        self.selected_profile_id = conn.profile_id.clone();
        self.env_rows = env_map_to_rows(&conn.env_vars);
        self.serial_port = conn.serial_port.clone().unwrap_or_default();
        self.serial_baud = conn
            .serial_baud
            .map(|b| b.to_string())
            .unwrap_or_else(|| "115200".into());
        self.ble_device = conn.ble_device.clone().unwrap_or_default();
    }

    pub fn is_editing(&self) -> bool {
        self.edit_id.is_some()
    }

    pub fn close(&mut self) {
        *self = Self::default();
    }

    /// 获取当前平台支持的连接类型列表。
    fn available_types() -> Vec<ConnectionType> {
        let mut types = Vec::new();
        if crate::platform::get().supports_local_terminal() {
            types.push(ConnectionType::Local);
        }
        if true /* SSH always supported */ {
            types.push(ConnectionType::Ssh);
        }
        if crate::platform::get().supports_serial() {
            types.push(ConnectionType::Serial);
        }
        if crate::platform::get().supports_ble() {
            types.push(ConnectionType::Ble);
        }
        types
    }

    /// 确保当前连接类型在当前平台受支持，否则切换到第一个可用类型。
    fn ensure_conn_type_supported(&mut self) {
        let types = Self::available_types();
        if types.is_empty() {
            return;
        }
        if !types.contains(&self.conn_type) {
            self.conn_type = types[0];
        }
    }

    /// Select an auth user after creating one from the nested dialog.
    pub fn select_auth_user(&mut self, id: String) {
        self.selected_auth_user_id = Some(id);
        self.conn_type = ConnectionType::Ssh;
    }

    /// Select a terminal profile after creating one from the nested dialog.
    pub fn select_profile(&mut self, id: String) {
        self.selected_profile_id = Some(id);
    }

    /// 显示居中弹出窗口并处理交互。返回 `Some(SavedConnection)` 表示保存。
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        auth_users: &[AuthUser],
        profiles: &[TerminalProfile],
    ) -> Option<SavedConnection> {
        if !self.open {
            return None;
        }

        self.poll_ble_scan();
        self.ensure_conn_type_supported();
        if self.conn_type == ConnectionType::Serial && self.serial_devices.is_empty() {
            self.refresh_serial_devices();
        }

        let mut outcome = ConnectionFormOutcome::None;
        let title = if self.edit_id.is_some() {
            rust_i18n::t!("dialog_edit_connection")
        } else {
            rust_i18n::t!("dialog_new_connection")
        };

        use crate::ui::uiframe::{DialogFrame, DialogOutcome};
        let frame = DialogFrame::new(title.to_string()).foreground();
        let closed = frame.show(ctx, "new_connection_dialog", |ui| {
            outcome = self.paint_form(ui, ctx, auth_users, profiles);
        }) == DialogOutcome::Closed;

        if closed {
            self.close();
            return None;
        }

        match outcome {
            ConnectionFormOutcome::Saved(conn) => {
                self.close();
                Some(conn)
            }
            ConnectionFormOutcome::Cancelled => {
                self.close();
                None
            }
            ConnectionFormOutcome::None => None,
        }
    }

    /// 绘制窗口内表单（类型下拉 + 名称 + 按类型配置）。
    fn paint_form(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        auth_users: &[AuthUser],
        profiles: &[TerminalProfile],
    ) -> ConnectionFormOutcome {
        let mut outcome = ConnectionFormOutcome::None;
        let editing = self.edit_id.is_some();
        let available_types = Self::available_types();
        let default_profile_name = profiles
            .iter()
            .find(|p| p.is_default)
            .or_else(|| profiles.first())
            .map(|p| p.name.as_str())
            .unwrap_or("Default");

        ui.horizontal(|ui| {
            ui.label(rust_i18n::t!("dialog_type"));
            ui.add_enabled_ui(!editing, |ui| {
                let prev = self.conn_type;
                egui::ComboBox::from_id_salt("add_connection_type")
                    .selected_text(self.conn_type.label())
                    .width(ui.available_width().min(220.0))
                    .show_ui(ui, |ui| {
                        for ct in &available_types {
                            ui.selectable_value(&mut self.conn_type, *ct, ct.label());
                        }
                    });
                if self.conn_type != prev {
                    self.ble_scan_error = None;
                    if self.conn_type == ConnectionType::Serial {
                        self.refresh_serial_devices();
                    }
                    if !editing {
                        self.env_rows = match self.conn_type {
                            ConnectionType::Local => env_map_to_rows(&default_local_env_vars()),
                            ConnectionType::Ssh => env_map_to_rows(&default_ssh_env_vars()),
                            ConnectionType::Serial | ConnectionType::Ble => Vec::new(),
                        };
                    }
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label(rust_i18n::t!("dialog_name"));
            let name_edit = ui.text_edit_singleline(&mut self.name);
            let force_ime = self.request_name_focus;
            if self.request_name_focus {
                name_edit.request_focus();
                self.request_name_focus = false;
            }
            android_ime_for_text_edit(ui, &name_edit, force_ime);
        });

        ui.horizontal(|ui| {
            ui.label(rust_i18n::t!("dialog_profile"));
            let selected_label = self
                .selected_profile_id
                .as_ref()
                .filter(|id| !id.is_empty())
                .and_then(|id| profiles.iter().find(|p| p.id == *id).map(|p| p.name.clone()))
                .unwrap_or_else(|| {
                    format!(
                        "{} ({})",
                        rust_i18n::t!("dialog_profile_default"),
                        default_profile_name
                    )
                });
            egui::ComboBox::from_id_salt("conn_profile_combo")
                .selected_text(selected_label)
                .width(220.0)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(false, rust_i18n::t!("dialog_profile_new"))
                        .clicked()
                    {
                        self.request_new_profile = true;
                    }
                    ui.separator();
                    let default_label = format!(
                        "{} ({})",
                        rust_i18n::t!("dialog_profile_default"),
                        default_profile_name
                    );
                    ui.selectable_value(&mut self.selected_profile_id, None, default_label);
                    for p in profiles {
                        ui.selectable_value(
                            &mut self.selected_profile_id,
                            Some(p.id.clone()),
                            &p.name,
                        );
                    }
                });
        });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(4.0);

        match self.conn_type {
            ConnectionType::Local => {
                ui.horizontal(|ui| {
                    ui.label(rust_i18n::t!("dialog_shell"));
                    dialog_text_edit(ui, &mut self.shell);
                });
                ui.horizontal(|ui| {
                    ui.label(rust_i18n::t!("dialog_working_dir"));
                    dialog_text_edit(ui, &mut self.working_dir);
                });
            }
            ConnectionType::Ssh => {
                ui.horizontal(|ui| {
                    ui.label(rust_i18n::t!("dialog_host"));
                    dialog_text_edit(ui, &mut self.ssh_host);
                });
                ui.horizontal(|ui| {
                    ui.label(rust_i18n::t!("dialog_port"));
                    dialog_text_edit(ui, &mut self.ssh_port);
                });
                ui.horizontal(|ui| {
                    ui.label(rust_i18n::t!("dialog_auth_user"));
                    let selected_label = self
                        .selected_auth_user_id
                        .as_ref()
                        .and_then(|id| auth_users.iter().find(|u| u.id == *id))
                        .map(|u| format!("{} ({})", u.name, u.username))
                        .unwrap_or_else(|| rust_i18n::t!("dialog_auth_user_none").into_owned());
                    egui::ComboBox::from_id_salt("ssh_auth_user_combo")
                        .selected_text(selected_label)
                        .width(220.0)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(false, rust_i18n::t!("dialog_auth_user_new"))
                                .clicked()
                            {
                                self.request_new_auth_user = true;
                            }
                            ui.separator();
                            for u in auth_users {
                                let label = format!("{} ({})", u.name, u.username);
                                if ui
                                    .selectable_value(
                                        &mut self.selected_auth_user_id,
                                        Some(u.id.clone()),
                                        label,
                                    )
                                    .clicked()
                                {
                                    // selection applied via selectable_value
                                }
                            }
                        });
                });
                if auth_users.is_empty() {
                    ui.label(
                        egui::RichText::new(rust_i18n::t!("dialog_auth_user_hint"))
                            .small()
                            .weak(),
                    );
                }
            }
            ConnectionType::Serial => {
                ui.horizontal(|ui| {
                    if ui.button(rust_i18n::t!("dialog_refresh_devices")).clicked() {
                        self.refresh_serial_devices();
                    }
                });
                if self.serial_devices.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(rust_i18n::t!("dialog_device"));
                        dialog_text_edit(ui, &mut self.serial_port);
                    });
                } else {
                    let selected_text = self
                        .serial_devices
                        .iter()
                        .find(|(path, _)| path == &self.serial_port)
                        .map(|(_, label)| label.as_str())
                        .unwrap_or(self.serial_port.as_str())
                        .to_string();
                    egui::ComboBox::from_label(rust_i18n::t!("dialog_device"))
                        .selected_text(selected_text)
                        .show_ui(ui, |ui| {
                            for (path, label) in &self.serial_devices {
                                ui.selectable_value(
                                    &mut self.serial_port,
                                    path.clone(),
                                    label,
                                );
                            }
                        });
                }
                ui.horizontal(|ui| {
                    ui.label(rust_i18n::t!("dialog_baud_rate"));
                    dialog_text_edit(ui, &mut self.serial_baud);
                });
            }
            ConnectionType::Ble => {
                ui.horizontal(|ui| {
                    let scan_label = if self.ble_scanning {
                        rust_i18n::t!("scanning")
                    } else {
                        rust_i18n::t!("dialog_scan_devices")
                    };
                    if ui
                        .add_enabled(!self.ble_scanning, egui::Button::new(scan_label))
                        .clicked()
                    {
                        self.start_ble_scan(ctx);
                    }
                });
                if let Some(err) = &self.ble_scan_error {
                    ui.label(
                        egui::RichText::new(err)
                            .small()
                            .color(style::RED),
                    );
                }
                if self.ble_devices.is_empty() && !self.ble_scanning {
                    ui.label(
                        egui::RichText::new(rust_i18n::t!("dialog_ble_scan_hint")).weak(),
                    );
                    ui.horizontal(|ui| {
                        ui.label(rust_i18n::t!("dialog_device_name"));
                        dialog_text_edit(ui, &mut self.ble_device);
                    });
                } else if !self.ble_devices.is_empty() {
                    let selected = if self.ble_device.is_empty() {
                        self.ble_devices[0].clone()
                    } else {
                        self.ble_device.clone()
                    };
                    egui::ComboBox::from_label(rust_i18n::t!("dialog_device"))
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            for name in &self.ble_devices {
                                ui.selectable_value(
                                    &mut self.ble_device,
                                    name.clone(),
                                    name,
                                );
                            }
                        });
                }
            }
        }

        if matches!(self.conn_type, ConnectionType::Local | ConnectionType::Ssh) {
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(egui::RichText::new(rust_i18n::t!("dialog_env_vars")).strong());
            let mut remove_idx = None;
            for (i, (key, value)) in self.env_rows.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    let key_w = ui.available_width() * 0.35;
                    ui.add(egui::TextEdit::singleline(key).desired_width(key_w));
                    ui.add(
                        egui::TextEdit::singleline(value)
                            .desired_width(ui.available_width() - 36.0),
                    );
                    if ui
                        .add(egui::Button::new("×").min_size(egui::vec2(28.0, 24.0)))
                        .clicked()
                    {
                        remove_idx = Some(i);
                    }
                });
            }
            if let Some(i) = remove_idx {
                self.env_rows.remove(i);
            }
            if ui.button(rust_i18n::t!("dialog_env_add")).clicked() {
                self.env_rows.push((String::new(), String::new()));
            }
        }

        ui.add_space(20.0);

        ui.horizontal(|ui| {
            let cancel_btn = egui::Button::new(
                egui::RichText::new(rust_i18n::t!("cancel"))
                    .size(14.0)
                    .color(ui.visuals().weak_text_color()),
            )
            .fill(ui.visuals().panel_fill)
            .corner_radius(style::CORNER_RADIUS_SM)
            .min_size(egui::vec2(80.0, 34.0));
            if ui.add(cancel_btn).clicked() {
                outcome = ConnectionFormOutcome::Cancelled;
            }

            let can_create = !self.name.trim().is_empty()
                && match self.conn_type {
                    ConnectionType::Ssh => {
                        !self.ssh_host.trim().is_empty() && self.selected_auth_user_id.is_some()
                    }
                    ConnectionType::Serial => !self.serial_port.trim().is_empty(),
                    ConnectionType::Ble => !self.ble_device.trim().is_empty(),
                    ConnectionType::Local => true,
                };

            let save_label = if self.edit_id.is_some() {
                rust_i18n::t!("save")
            } else {
                rust_i18n::t!("create")
            };
            let create_btn = egui::Button::new(
                egui::RichText::new(save_label)
                    .size(14.0)
                    .color(egui::Color32::WHITE),
            )
            .fill(style::ACCENT)
            .corner_radius(style::CORNER_RADIUS_SM)
            .min_size(egui::vec2(100.0, 34.0));
            if ui.add_enabled(can_create, create_btn).clicked() {
                let mut conn = match self.conn_type {
                    ConnectionType::Local => {
                        let shell = if self.shell.is_empty() {
                            None
                        } else {
                            Some(self.shell.as_str())
                        };
                        let mut c = SavedConnection::new_local(&self.name, shell);
                        if !self.working_dir.trim().is_empty() {
                            c.working_dir = Some(self.working_dir.trim().to_string());
                        }
                        c
                    }
                    ConnectionType::Ssh => {
                        let auth_id = self.selected_auth_user_id.clone().unwrap_or_default();
                        let username = auth_users
                            .iter()
                            .find(|u| u.id == auth_id)
                            .map(|u| u.username.as_str())
                            .unwrap_or("root");
                        let mut c = SavedConnection::new_ssh(
                            &self.name,
                            &self.ssh_host,
                            self.ssh_port.parse().unwrap_or(22),
                            username,
                        );
                        c.auth_user_id = Some(auth_id);
                        c.ssh_password = None;
                        c
                    }
                    ConnectionType::Serial => SavedConnection::new_serial(
                        &self.name,
                        &self.serial_port,
                        self.serial_baud.parse().unwrap_or(115200),
                    ),
                    ConnectionType::Ble => {
                        SavedConnection::new_ble(&self.name, &self.ble_device)
                    }
                };
                if let Some(id) = self.edit_id.take() {
                    conn.id = id;
                }
                conn.profile_id = self.selected_profile_id.clone().filter(|id| !id.is_empty());
                conn.env_vars = env_rows_to_map(&self.env_rows);
                outcome = ConnectionFormOutcome::Saved(conn);
            }
        });

        outcome
    }

    fn refresh_serial_devices(&mut self) {
        self.serial_devices = enumerate_serial_ports()
            .into_iter()
            .map(|d| (d.path, d.label))
            .collect();
        if self.serial_port.is_empty() {
            if let Some((path, _)) = self.serial_devices.first() {
                self.serial_port = path.clone();
            }
        }
    }

    fn start_ble_scan(&mut self, ctx: &egui::Context) {
        log::info!("start_ble_scan called");
        if self.ble_scanning {
            log::info!("start_ble_scan: already scanning, skipping");
            return;
        }

        self.ble_scan_error = None;

        #[cfg(target_os = "android")]
        {
            log::info!("start_ble_scan: checking bluetooth permission");
            if !crate::platform::get().has_bluetooth_access() {
                crate::platform::get().request_bluetooth_access();
                self.ble_scan_error = Some(
                    "需要授予附近设备/蓝牙权限后才能扫描。请同意权限弹窗后再点一次扫描。"
                        .to_string(),
                );
                ctx.request_repaint();
                return;
            }
        }

        let (tx, rx) = mpsc::channel();
        self.ble_scan_rx = Some(rx);
        self.ble_scanning = true;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            log::info!("BLE scan thread: starting");
            let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                scan_ble_devices_blocking()
            })) {
                Ok(Ok(devices)) => {
                    log::info!("BLE scan thread: success ({} devices)", devices.len());
                    Ok(devices)
                }
                Ok(Err(e)) => {
                    log::error!("BLE scan thread: error: {e}");
                    Err(e)
                }
                Err(panic) => {
                    let msg = if let Some(s) = panic.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "未知错误".to_string()
                    };
                    log::error!("BLE scan thread: panicked: {msg}");
                    Err(format!("BLE 扫描异常：{msg}"))
                }
            };
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }

    fn poll_ble_scan(&mut self) {
        let Some(rx) = self.ble_scan_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(devices)) => {
                self.ble_scanning = false;
                self.ble_devices = devices;
                self.ble_scan_error = None;
                if self.ble_device.is_empty() {
                    if let Some(first) = self.ble_devices.first() {
                        self.ble_device = first.clone();
                    }
                }
            }
            Ok(Err(e)) => {
                self.ble_scanning = false;
                self.ble_scan_error = Some(format!("BLE 扫描失败：{e}"));
                log::warn!("BLE scan failed: {e}");
            }
            Err(mpsc::TryRecvError::Empty) => {
                self.ble_scan_rx = Some(rx);
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.ble_scanning = false;
                self.ble_scan_error = Some("BLE 扫描线程意外退出，请重试。".to_string());
                log::warn!("BLE scan thread disconnected without sending a result");
            }
        }
    }
}

/// Runtime settings for an active local terminal (shell, cwd, saved profile).
pub struct LocalTerminalSettingsDialog {
    pub open: bool,
    session_id: Option<String>,
    profile_id: Option<String>,
    pub shell: String,
    pub working_dir: String,
}

#[derive(Clone)]
pub struct LocalTerminalSettingsApply {
    /// When set, reconnect this workspace session after saving.
    pub session_id: Option<String>,
    pub config: SavedConnection,
}

impl Default for LocalTerminalSettingsDialog {
    fn default() -> Self {
        Self {
            open: false,
            session_id: None,
            profile_id: None,
            shell: String::new(),
            working_dir: String::new(),
        }
    }
}

impl LocalTerminalSettingsDialog {
    pub fn open_for(
        &mut self,
        session_id: &str,
        saved_conn_id: Option<&str>,
        shell: Option<&str>,
        working_dir: Option<&str>,
        connections: &[SavedConnection],
    ) {
        self.open = true;
        self.session_id = Some(session_id.to_string());
        self.fill_fields(saved_conn_id, shell, working_dir, connections);
    }

    /// Home screen: edit default local terminal without an active session.
    pub fn open_for_home(
        &mut self,
        connections: &[SavedConnection],
        default_local_id: Option<&str>,
    ) {
        self.open = true;
        self.session_id = None;
        if let Some(id) = default_local_id {
            self.fill_fields(Some(id), None, None, connections);
        } else if let Some(c) = connections.iter().find(|c| c.conn_type == ConnectionType::Local) {
            self.fill_fields(Some(&c.id), None, None, connections);
        } else {
            self.profile_id = None;
            self.shell = crate::platform::get().default_shell();
            self.working_dir.clear();
        }
    }

    fn fill_fields(
        &mut self,
        saved_conn_id: Option<&str>,
        shell: Option<&str>,
        working_dir: Option<&str>,
        connections: &[SavedConnection],
    ) {
        self.profile_id = saved_conn_id.map(|s| s.to_string());
        if let Some(id) = saved_conn_id {
            if let Some(c) = connections.iter().find(|c| c.id == id) {
                self.shell = c.shell.clone().unwrap_or_default();
                self.working_dir = c.working_dir.clone().unwrap_or_default();
                return;
            }
        }
        self.shell = shell
            .map(|s| s.to_string())
            .unwrap_or_else(|| crate::platform::get().default_shell());
        self.working_dir = working_dir.unwrap_or_default().to_string();
    }

    fn load_profile(&mut self, id: &str, connections: &[SavedConnection]) {
        let Some(c) = connections
            .iter()
            .find(|c| c.id == id && c.conn_type == ConnectionType::Local)
        else {
            return;
        };
        self.profile_id = Some(id.to_string());
        self.shell = c.shell.clone().unwrap_or_default();
        self.working_dir = c.working_dir.clone().unwrap_or_default();
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        connections: &[SavedConnection],
    ) -> Option<LocalTerminalSettingsApply> {
        if !self.open {
            return None;
        }

        let mut result = None;
        let mut close = false;

        use crate::ui::uiframe::{DialogFrame, DialogOutcome};
        let frame = DialogFrame::new(rust_i18n::t!("dialog_local_terminal_settings").to_string());
        let closed = frame.show(ctx, "local_terminal_settings", |ui| {
                let local_profiles: Vec<&SavedConnection> = connections
                    .iter()
                    .filter(|c| c.conn_type == ConnectionType::Local)
                    .collect();

                ui.label(rust_i18n::t!("dialog_saved_profile"));
                let custom_label = rust_i18n::t!("dialog_custom_profile");
                let selected_label = self
                    .profile_id
                    .as_ref()
                    .and_then(|id| local_profiles.iter().find(|c| c.id == *id))
                    .map(|c| c.name.as_str())
                    .unwrap_or(&custom_label);
                egui::ComboBox::from_id_salt("local_term_profile")
                    .selected_text(selected_label)
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(self.profile_id.is_none(), "(custom)").clicked() {
                            self.profile_id = None;
                        }
                        for c in &local_profiles {
                            if ui
                                .selectable_label(
                                    self.profile_id.as_deref() == Some(c.id.as_str()),
                                    &c.name,
                                )
                                .clicked()
                            {
                                self.load_profile(&c.id, connections);
                            }
                        }
                    });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(rust_i18n::t!("dialog_shell"));
                    dialog_text_edit(ui, &mut self.shell);
                });
                ui.horizontal(|ui| {
                    ui.label(rust_i18n::t!("dialog_working_dir"));
                    dialog_text_edit(ui, &mut self.working_dir);
                });
                let hint = if self.session_id.is_some() {
                    rust_i18n::t!("dialog_reconnect_hint")
                } else {
                    rust_i18n::t!("dialog_next_time_hint")
                };
                ui.label(egui::RichText::new(hint).small().weak());

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button(rust_i18n::t!("cancel")).clicked() {
                        close = true;
                    }
                    let apply_label = if self.session_id.is_some() {
                        rust_i18n::t!("dialog_apply_reconnect")
                    } else {
                        rust_i18n::t!("dialog_apply")
                    };
                    if ui.button(apply_label).clicked() {
                        let session_id = self.session_id.clone();
                        let shell = if self.shell.trim().is_empty() {
                            None
                        } else {
                            Some(self.shell.trim().to_string())
                        };
                        let working_dir = if self.working_dir.trim().is_empty() {
                            None
                        } else {
                            Some(self.working_dir.trim().to_string())
                        };

                        let mut config = if let Some(id) = &self.profile_id {
                            connections
                                .iter()
                                .find(|c| c.id == *id)
                                .cloned()
                                .unwrap_or_else(|| {
                                    SavedConnection::new_local("Local Terminal", shell.as_deref())
                                })
                        } else {
                            SavedConnection::new_local("Local Terminal", shell.as_deref())
                        };
                        config.shell = shell;
                        config.working_dir = working_dir;
                        result = Some(LocalTerminalSettingsApply {
                            session_id,
                            config,
                        });
                        close = true;
                    }
                });
        }) == DialogOutcome::Closed;

        if closed || close {
            self.open = false;
            *self = Self::default();
        }

        result
    }
}
