//! 本地终端运行时设置对话框。

use rsterm_data::persist::types::{ConnectionType, SavedConnection};
use crate::uiframe::form::{self, FooterAction};

/// Runtime settings for an active local terminal (shell, cwd, saved profile).
#[derive(Default)]
pub struct LocalTerminalSettingsDialog {
    pub open: bool,
    session_id: Option<String>,
    connection_id: Option<String>,
    pub shell: String,
    pub working_dir: String,
}

#[derive(Clone)]
pub struct LocalTerminalSettingsApply {
    /// When set, reconnect this workspace session after saving.
    pub session_id: Option<String>,
    pub config: SavedConnection,
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
        } else if let Some(c) = connections
            .iter()
            .find(|c| c.conn_type == ConnectionType::Local)
        {
            self.fill_fields(Some(&c.id), None, None, connections);
        } else {
            self.connection_id = None;
            self.shell = rsterm_platform::get().default_shell();
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
        self.connection_id = saved_conn_id.map(|s| s.to_string());
        if let Some(id) = saved_conn_id
            && let Some(c) = connections.iter().find(|c| c.id == id)
        {
            self.shell = c.shell.clone().unwrap_or_default();
            self.working_dir = c.working_dir.clone().unwrap_or_default();
            return;
        }
        self.shell = shell
            .map(|s| s.to_string())
            .unwrap_or_else(|| rsterm_platform::get().default_shell());
        self.working_dir = working_dir.unwrap_or_default().to_string();
    }

    fn load_connection(&mut self, id: &str, connections: &[SavedConnection]) {
        let Some(c) = connections
            .iter()
            .find(|c| c.id == id && c.conn_type == ConnectionType::Local)
        else {
            return;
        };
        self.connection_id = Some(id.to_string());
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

        use crate::uiframe::{DialogFrame, DialogOutcome};
        let frame = DialogFrame::new(crate::i18n_bridge::tr("dialog_local_terminal_settings"));
        let closed = frame.show(ctx, "local_terminal_settings", |ui| {
            let local_profiles: Vec<&SavedConnection> = connections
                .iter()
                .filter(|c| c.conn_type == ConnectionType::Local)
                .collect();

            let custom_label = crate::i18n_bridge::tr("dialog_custom_profile");
            let selected_label = self
                .connection_id
                .as_ref()
                .and_then(|id| local_profiles.iter().find(|c| c.id == *id))
                .map(|c| c.name.as_str())
                .unwrap_or(&custom_label)
                .to_string();
            form::labeled_combo(
                ui,
                "local_term_profile",
                crate::i18n_bridge::tr("dialog_saved_profile"),
                selected_label,
                |ui| {
                    if ui
                        .selectable_label(self.connection_id.is_none(), "(custom)")
                        .clicked()
                    {
                        self.connection_id = None;
                    }
                    for c in &local_profiles {
                        if ui
                            .selectable_label(
                                self.connection_id.as_deref() == Some(c.id.as_str()),
                                &c.name,
                            )
                            .clicked()
                        {
                            self.load_connection(&c.id, connections);
                        }
                    }
                },
            );

            form::labeled_text(ui, crate::i18n_bridge::tr("dialog_shell"), &mut self.shell);
            form::labeled_text(
                ui,
                crate::i18n_bridge::tr("dialog_working_dir"),
                &mut self.working_dir,
            );
            let hint = if self.session_id.is_some() {
                crate::i18n_bridge::tr("dialog_reconnect_hint")
            } else {
                crate::i18n_bridge::tr("dialog_next_time_hint")
            };
            ui.label(egui::RichText::new(hint).small().weak());

            let apply_label = if self.session_id.is_some() {
                crate::i18n_bridge::tr("dialog_apply_reconnect")
            } else {
                crate::i18n_bridge::tr("dialog_apply")
            };
            match form::dialog_footer(ui, crate::i18n_bridge::tr("cancel"), apply_label, true) {
                FooterAction::Cancel => close = true,
                FooterAction::Save => {
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

                    let mut config = if let Some(id) = &self.connection_id {
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
                    result = Some(LocalTerminalSettingsApply { session_id, config });
                    close = true;
                }
                FooterAction::None => {}
            }
        }) == DialogOutcome::Closed;

        if closed || close {
            self.open = false;
            *self = Self::default();
        }

        result
    }
}
