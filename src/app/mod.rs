//! Application orchestration: owns state and applies UI actions.

mod auth_users;
mod commands;
mod connections;
mod frame;
mod lifecycle;
mod notices;
mod sessions;

use crate::persist::{
    types::{AuthUser, FavoriteCommand, SavedConnection},
    Persist,
};
use crate::settings::AppSettings;
use crate::session::WorkspaceSession;
use crate::ui::shell::AppShell;
use crate::ui::page::dialogs::{
    AuthUserDialog, FavoriteCommandDialog, LocalTerminalSettingsDialog,
    ManageFavoriteCommandsDialog, NewConnectionDialog,
};
use crate::ui::uiframe::keyboard::VirtualKeyboard;

pub struct RsTerminalApp {
    persist: Persist,
    settings: AppSettings,
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
    live_font_size: f32,
    connection_notice: Option<String>,
    quit_after_close: bool,
    show_quit_dialog: bool,
    first_frame: bool,
}

impl Default for RsTerminalApp {
    fn default() -> Self {
        let persist = Persist::open();
        let settings = crate::settings::load_settings();
        settings.language.apply();
        let live_font_size = settings.font_size();
        let kbd_mode = settings.default_profile().keyboard_mode;
        let saved = persist.list_connections();
        let favorite_commands = persist.list_commands();
        let auth_users = persist.list_auth_users();
        Self {
            persist,
            shell: AppShell::from_settings(&settings),
            settings,
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
            live_font_size,
            connection_notice: None,
            quit_after_close: false,
            first_frame: true,
            show_quit_dialog: false,
        }
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
