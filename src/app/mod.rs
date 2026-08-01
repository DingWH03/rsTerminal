//! Application orchestration: owns state and applies UI actions.

mod auth_users;
mod commands;
mod connect_params;
mod connections;
mod frame;
mod lifecycle;
mod notices;
mod sessions;

use crate::persist::{
    types::{resolve_profile, AuthUser, FavoriteCommand, SavedConnection, TerminalProfile},
    Persist,
};
use crate::prefs::{load_prefs, Prefs};
use crate::session::WorkspaceSession;
use crate::ui::shell::AppShell;
use crate::ui::page::dialogs::{
    AuthUserDialog, FavoriteCommandDialog, LocalTerminalSettingsDialog,
    ManageFavoriteCommandsDialog, NewConnectionDialog, ProfileDialog,
};
use crate::ui::uiframe::keyboard::VirtualKeyboard;

pub struct RsTerminalApp {
    persist: Persist,
    prefs: Prefs,
    profiles: Vec<TerminalProfile>,
    saved_connections: Vec<SavedConnection>,
    favorite_commands: Vec<FavoriteCommand>,
    auth_users: Vec<AuthUser>,
    sessions: Vec<WorkspaceSession>,
    shell: AppShell,
    virtual_keyboard: VirtualKeyboard,
    new_conn_dialog: NewConnectionDialog,
    local_term_dialog: LocalTerminalSettingsDialog,
    favorite_cmd_dialog: FavoriteCommandDialog,
    manage_commands_dialog: ManageFavoriteCommandsDialog,
    auth_user_dialog: AuthUserDialog,
    profile_dialog: ProfileDialog,
    live_font_size: f32,
    connection_notice: Option<String>,
    quit_after_close: bool,
    show_quit_dialog: bool,
    first_frame: bool,
}

impl RsTerminalApp {
    pub fn new(persist: Persist) -> Self {
        let prefs = load_prefs();
        prefs.general.language.apply();
        let profiles = persist.list_profiles();
        let default_profile = resolve_profile(&profiles, None);
        let live_font_size = default_profile.font_size;
        let kbd_mode = default_profile.keyboard_mode;
        let saved = persist.list_connections();
        let favorite_commands = persist.list_commands();
        let auth_users = persist.list_auth_users();
        Self {
            persist,
            shell: AppShell::from_prefs(&prefs),
            prefs,
            profiles,
            saved_connections: saved,
            favorite_commands,
            auth_users,
            sessions: Vec::new(),
            virtual_keyboard: VirtualKeyboard::new(kbd_mode),
            new_conn_dialog: NewConnectionDialog::default(),
            local_term_dialog: LocalTerminalSettingsDialog::default(),
            favorite_cmd_dialog: FavoriteCommandDialog::default(),
            manage_commands_dialog: ManageFavoriteCommandsDialog::default(),
            auth_user_dialog: AuthUserDialog::default(),
            profile_dialog: ProfileDialog::default(),
            live_font_size,
            connection_notice: None,
            quit_after_close: false,
            first_frame: true,
            show_quit_dialog: false,
        }
    }

    pub fn default_terminal_font(&self) -> &str {
        resolve_profile(&self.profiles, None).terminal_font.as_str()
    }

    pub(crate) fn resolve_profile(&self, id: Option<&str>) -> &TerminalProfile {
        resolve_profile(&self.profiles, id)
    }

    pub(crate) fn reload_profiles(&mut self) {
        self.profiles = self.persist.list_profiles();
    }
}

impl Default for RsTerminalApp {
    fn default() -> Self {
        Self::new(Persist::open())
    }
}

impl eframe::App for RsTerminalApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_logic(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frame_ui(ui);
    }
}
