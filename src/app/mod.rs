//! Application orchestration: owns state and applies UI actions.

mod commands;
mod connections;
mod frame;
mod lifecycle;
mod notices;
mod sessions;

use crate::persist::{Persist, types::{FavoriteCommand, SavedConnection}};
use crate::settings::AppSettings;
use crate::session::WorkspaceSession;
use crate::ui::shell::AppShell;
use crate::ui::page::dialogs::{
    FavoriteCommandDialog, LocalTerminalSettingsDialog, ManageFavoriteCommandsDialog,
    NewConnectionDialog,
};
use crate::ui::uiframe::keyboard::VirtualKeyboard;

pub struct RsTerminalApp {
    persist: Persist,
    settings: AppSettings,
    saved_connections: Vec<SavedConnection>,
    favorite_commands: Vec<FavoriteCommand>,
    sessions: Vec<WorkspaceSession>,
    shell: AppShell,
    virtual_keyboard: VirtualKeyboard,
    new_conn_dialog: NewConnectionDialog,
    local_term_dialog: LocalTerminalSettingsDialog,
    favorite_cmd_dialog: FavoriteCommandDialog,
    manage_commands_dialog: ManageFavoriteCommandsDialog,
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
        Self {
            persist,
            shell: AppShell::from_settings(&settings),
            settings,
            saved_connections: saved,
            favorite_commands,
            sessions: Vec::new(),
            virtual_keyboard: VirtualKeyboard::new(kbd_mode),
            new_conn_dialog: NewConnectionDialog::default(),
            local_term_dialog: LocalTerminalSettingsDialog::default(),
            favorite_cmd_dialog: FavoriteCommandDialog::default(),
            manage_commands_dialog: ManageFavoriteCommandsDialog::default(),
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
