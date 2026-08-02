//! 新建/编辑连接表单。

use std::sync::mpsc;

use std::collections::HashMap;

use crate::connection::enumeration::{enumerate_serial_ports, scan_ble_devices_blocking};
use crate::data::persist::types::{
    AuthUser, ConnectionType, SavedConnection, TerminalProfile, default_local_env_vars,
    default_ssh_env_vars,
};
use crate::ui::connection_display::connection_type_label;
use crate::ui::uiframe::form::{self, FooterAction};
use crate::ui::uiframe::style;

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
        self.ssh_port = conn
            .ssh_port
            .map(|p| p.to_string())
            .unwrap_or_else(|| "22".into());
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
        if true
        /* SSH always supported */
        {
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

        form::labeled_row(ui, rust_i18n::t!("dialog_type"), |ui| {
            ui.add_enabled_ui(!editing, |ui| {
                let prev = self.conn_type;
                egui::ComboBox::from_id_salt("add_connection_type")
                    .selected_text(connection_type_label(self.conn_type))
                    .width(form::COMBO_WIDTH)
                    .show_ui(ui, |ui| {
                        for ct in &available_types {
                            ui.selectable_value(
                                &mut self.conn_type,
                                *ct,
                                connection_type_label(*ct),
                            );
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

        let name_resp = form::labeled_text(ui, rust_i18n::t!("dialog_name"), &mut self.name);
        if self.request_name_focus {
            name_resp.request_focus();
            form::android_ime_for_text_edit(ui, &name_resp, true);
            self.request_name_focus = false;
        }

        let selected_profile_label = self
            .selected_profile_id
            .as_ref()
            .filter(|id| !id.is_empty())
            .and_then(|id| {
                profiles
                    .iter()
                    .find(|p| p.id == *id)
                    .map(|p| p.name.clone())
            })
            .unwrap_or_else(|| {
                format!(
                    "{} ({})",
                    rust_i18n::t!("dialog_profile_default"),
                    default_profile_name
                )
            });
        form::labeled_combo(
            ui,
            "conn_profile_combo",
            rust_i18n::t!("dialog_profile"),
            selected_profile_label,
            |ui| {
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
                    ui.selectable_value(&mut self.selected_profile_id, Some(p.id.clone()), &p.name);
                }
            },
        );

        ui.add_space(form::SECTION_GAP);
        ui.separator();
        ui.add_space(4.0);

        match self.conn_type {
            ConnectionType::Local => {
                form::labeled_text(ui, rust_i18n::t!("dialog_shell"), &mut self.shell);
                form::labeled_text(
                    ui,
                    rust_i18n::t!("dialog_working_dir"),
                    &mut self.working_dir,
                );
            }
            ConnectionType::Ssh => {
                form::labeled_text(ui, rust_i18n::t!("dialog_host"), &mut self.ssh_host);
                form::labeled_text(ui, rust_i18n::t!("dialog_port"), &mut self.ssh_port);
                let selected_auth_label = self
                    .selected_auth_user_id
                    .as_ref()
                    .and_then(|id| auth_users.iter().find(|u| u.id == *id))
                    .map(|u| format!("{} ({})", u.name, u.username))
                    .unwrap_or_else(|| rust_i18n::t!("dialog_auth_user_none").into_owned());
                form::labeled_combo(
                    ui,
                    "ssh_auth_user_combo",
                    rust_i18n::t!("dialog_auth_user"),
                    selected_auth_label,
                    |ui| {
                        if ui
                            .selectable_label(false, rust_i18n::t!("dialog_auth_user_new"))
                            .clicked()
                        {
                            self.request_new_auth_user = true;
                        }
                        ui.separator();
                        for u in auth_users {
                            let label = format!("{} ({})", u.name, u.username);
                            ui.selectable_value(
                                &mut self.selected_auth_user_id,
                                Some(u.id.clone()),
                                label,
                            );
                        }
                    },
                );
                if auth_users.is_empty() {
                    ui.label(
                        egui::RichText::new(rust_i18n::t!("dialog_auth_user_hint"))
                            .small()
                            .weak(),
                    );
                }
            }
            ConnectionType::Serial => {
                if ui.button(rust_i18n::t!("dialog_refresh_devices")).clicked() {
                    self.refresh_serial_devices();
                }
                ui.add_space(form::FIELD_GAP);
                if self.serial_devices.is_empty() {
                    form::labeled_text(ui, rust_i18n::t!("dialog_device"), &mut self.serial_port);
                } else {
                    let selected_text = self
                        .serial_devices
                        .iter()
                        .find(|(path, _)| path == &self.serial_port)
                        .map(|(_, label)| label.as_str())
                        .unwrap_or(self.serial_port.as_str())
                        .to_string();
                    form::labeled_combo(
                        ui,
                        "serial_device_combo",
                        rust_i18n::t!("dialog_device"),
                        selected_text,
                        |ui| {
                            for (path, label) in &self.serial_devices {
                                ui.selectable_value(&mut self.serial_port, path.clone(), label);
                            }
                        },
                    );
                }
                form::labeled_text(ui, rust_i18n::t!("dialog_baud_rate"), &mut self.serial_baud);
            }
            ConnectionType::Ble => {
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
                ui.add_space(form::FIELD_GAP);
                if let Some(err) = &self.ble_scan_error {
                    ui.label(egui::RichText::new(err).small().color(style::RED));
                }
                if self.ble_devices.is_empty() && !self.ble_scanning {
                    ui.label(egui::RichText::new(rust_i18n::t!("dialog_ble_scan_hint")).weak());
                    form::labeled_text(
                        ui,
                        rust_i18n::t!("dialog_device_name"),
                        &mut self.ble_device,
                    );
                } else if !self.ble_devices.is_empty() {
                    let selected = if self.ble_device.is_empty() {
                        self.ble_devices[0].clone()
                    } else {
                        self.ble_device.clone()
                    };
                    form::labeled_combo(
                        ui,
                        "ble_device_combo",
                        rust_i18n::t!("dialog_device"),
                        selected,
                        |ui| {
                            for name in &self.ble_devices {
                                ui.selectable_value(&mut self.ble_device, name.clone(), name);
                            }
                        },
                    );
                }
            }
        }

        if matches!(self.conn_type, ConnectionType::Local | ConnectionType::Ssh) {
            ui.add_space(form::SECTION_GAP);
            ui.separator();
            form::section_header(ui, rust_i18n::t!("dialog_env_vars"));
            let mut remove_idx = None;
            for (i, (key, value)) in self.env_rows.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    let key_w = ui.available_width() * 0.35;
                    let key_resp = ui.add(egui::TextEdit::singleline(key).desired_width(key_w));
                    form::android_ime_for_text_edit(ui, &key_resp, false);
                    let val_resp = ui.add(
                        egui::TextEdit::singleline(value)
                            .desired_width(ui.available_width() - 36.0),
                    );
                    form::android_ime_for_text_edit(ui, &val_resp, false);
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
        match form::dialog_footer(ui, rust_i18n::t!("cancel"), save_label, can_create) {
            FooterAction::Cancel => outcome = ConnectionFormOutcome::Cancelled,
            FooterAction::Save => {
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
                    ConnectionType::Ble => SavedConnection::new_ble(&self.name, &self.ble_device),
                };
                if let Some(id) = self.edit_id.take() {
                    conn.id = id;
                }
                conn.profile_id = self.selected_profile_id.clone().filter(|id| !id.is_empty());
                conn.env_vars = env_rows_to_map(&self.env_rows);
                outcome = ConnectionFormOutcome::Saved(conn);
            }
            FooterAction::None => {}
        }

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
