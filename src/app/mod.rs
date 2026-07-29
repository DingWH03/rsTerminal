//! Application orchestration: owns state and applies UI actions.

mod connections;
mod frame;
mod lifecycle;
mod notices;
mod sessions;

use crate::settings::AppSettings;
use crate::storage;
use crate::storage::types::SavedConnection;
use crate::session::WorkspaceSession;
use crate::ui::shell::AppShell;
use crate::ui::page::dialogs::{LocalTerminalSettingsDialog, NewConnectionDialog};
use crate::ui::uiframe::keyboard::VirtualKeyboard;

pub struct RsTerminalApp {
    settings: AppSettings,
    saved_connections: Vec<SavedConnection>,
    sessions: Vec<WorkspaceSession>,
    shell: AppShell,
    virtual_keyboard: VirtualKeyboard,
    new_conn_dialog: NewConnectionDialog,
    local_term_dialog: LocalTerminalSettingsDialog,
    live_font_size: f32,
    connection_notice: Option<String>,
    quit_after_close: bool,
    show_quit_dialog: bool,
    first_frame: bool,
}

impl Default for RsTerminalApp {
    fn default() -> Self {
        let settings = crate::settings::load_settings();
        settings.language.apply();
        let live_font_size = settings.font_size();
        let kbd_mode = settings.default_profile().keyboard_mode;
        let saved = storage::load_connections();
        Self {
            shell: AppShell::from_settings(&settings),
            settings,
            saved_connections: saved,
            sessions: Vec::new(),
            virtual_keyboard: VirtualKeyboard::new(kbd_mode),
            new_conn_dialog: NewConnectionDialog::default(),
            local_term_dialog: LocalTerminalSettingsDialog::default(),
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
