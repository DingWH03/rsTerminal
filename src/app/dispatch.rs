//! Dispatch normalized UI actions into application operations.

use super::RsTerminalApp;
use crate::data::prefs::save_prefs;
use crate::ui::actions::UiAction;
use crate::ui::function_pane::pages::FunctionPage;

impl RsTerminalApp {
    pub(crate) fn dispatch_ui_actions(&mut self, actions: Vec<UiAction>, ctx: &egui::Context) {
        for action in actions {
            self.dispatch_ui_action(action, ctx);
        }
    }

    fn dispatch_ui_action(&mut self, action: UiAction, ctx: &egui::Context) {
        match action {
            UiAction::SettingsClosed | UiAction::PersistTerminalSettings => {
                self.shell.layout.ui.settings_dialog_open = false;
                save_prefs(&self.prefs);
                self.live_font_size = self.resolve_profile(None).font_size;
                self.reload_terminal_fonts(ctx);
            }
            UiAction::NewAuthUser => {
                self.dialogs.auth_user.open_new();
                self.release_terminal_keyboard_focus(ctx);
            }
            UiAction::EditAuthUser(id) => {
                if let Some(user) = self.auth_users.iter().find(|u| u.id == id).cloned() {
                    self.dialogs.auth_user.open_edit(&user);
                    self.release_terminal_keyboard_focus(ctx);
                }
            }
            UiAction::DeleteAuthUser(id) => self.delete_auth_user(&id),
            UiAction::NewProfile => {
                self.dialogs.profile.open_new();
                self.release_terminal_keyboard_focus(ctx);
            }
            UiAction::EditProfile(id) => {
                if let Some(profile) = self.profiles.iter().find(|p| p.id == id).cloned() {
                    self.dialogs.profile.open_edit(&profile);
                    self.release_terminal_keyboard_focus(ctx);
                }
            }
            UiAction::DeleteProfile(id) => self.delete_profile(&id),
            UiAction::SetDefaultProfile(id) => self.set_default_profile(&id),
            UiAction::NewConnection => {
                self.dialogs.new_conn.open_new();
                self.release_terminal_keyboard_focus(ctx);
            }
            UiAction::Connect(id) => {
                self.connect_to(&id);
                self.shell.layout.function_page = FunctionPage::Active;
            }
            UiAction::OpenFileManager(id) => self.open_file_manager_for_connection(&id),
            UiAction::EditConnection(id) => {
                if let Some(connection) =
                    self.saved_connections.iter().find(|c| c.id == id).cloned()
                {
                    self.dialogs.new_conn.open_edit(&connection);
                    self.release_terminal_keyboard_focus(ctx);
                }
            }
            UiAction::DeleteConnection(id) => self.delete_connection(&id),
            UiAction::CloseSession(id) => self.close_session(&id),
            UiAction::DuplicateSession(id) => {
                self.duplicate_session(&id);
                if self.shell.function_pane.overlay_visible() {
                    self.shell.function_pane.close_overlay();
                }
            }
            UiAction::NewFavoriteCommand => {
                self.dialogs.favorite_cmd.open_new();
                self.release_terminal_keyboard_focus(ctx);
            }
            UiAction::RunFavoriteCommand(id) => self.run_favorite_command(&id, ctx),
            UiAction::EditFavoriteCommand(id) => {
                if let Some(command) = self.favorite_commands.iter().find(|c| c.id == id).cloned() {
                    self.dialogs.favorite_cmd.open_edit(&command);
                    self.release_terminal_keyboard_focus(ctx);
                }
            }
            UiAction::DeleteFavoriteCommand(id) => self.delete_favorite_command(&id),
            UiAction::ConnectPane {
                pane,
                connection_id,
            } => self.connect_to_pane(&connection_id, pane),
            UiAction::OpenConnectionsForPane(pane) => {
                self.shell.layout.workspace.focused_pane = pane;
                self.shell.layout.ui.connections_dialog_open = true;
            }
            UiAction::ClosePane(pane) => self.close_pane_and_maybe_session(pane),
            UiAction::ReconnectPane {
                pane,
                connection_id,
            } => self.reconnect_ssh_session(pane, &connection_id),
        }
    }
}
