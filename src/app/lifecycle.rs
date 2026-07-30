//! Back navigation, quit/exit, close request, focus release, fonts/profile.

use super::RsTerminalApp;
use crate::fonts;
use crate::settings::save_settings;
use crate::session::WorkspaceSession;
use crate::ui::function_pane::pages::FunctionPage;
use crate::ui::page::dialogs::LocalTerminalSettingsDialog;

impl RsTerminalApp {
    pub(crate) fn reload_terminal_fonts(&mut self, ctx: &egui::Context) {
        fonts::apply_terminal_fonts(ctx, &self.settings.default_profile().terminal_font);
        let font_gen = fonts::font_generation();
        for session in &mut self.sessions {
            if let WorkspaceSession::Terminal(term) = session {
                term.clear_all_galley_caches();
                term.font_generation = font_gen;
            }
        }
    }

    pub(crate) fn save_profile_tweaks(&mut self) {
        if let Some(profile) = self
            .settings
            .profiles
            .iter_mut()
            .find(|p| p.name == self.settings.default_profile_name)
        {
            profile.font_size = self.live_font_size;
            profile.keyboard_mode = self.virtual_keyboard.mode;
            save_settings(&self.settings);
        }
    }

    /// Stop the terminal from reclaiming keyboard focus (modals / text fields).
    pub(crate) fn release_terminal_keyboard_focus(&mut self, ctx: &egui::Context) {
        for session in &mut self.sessions {
            if let Some(term) = session.terminal_mut() {
                term.want_terminal_focus = false;
                term.terminal_had_focus = false;
            }
        }
        #[cfg(target_os = "android")]
        {
            self.virtual_keyboard.terminal_ime_enabled = false;
            crate::platform::android_ime::release_terminal_ime_for_text_fields(ctx);
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = ctx;
        }
    }

    pub(crate) fn handle_back_navigation(&mut self, ctx: &egui::Context) -> bool {
        if self.connection_notice.take().is_some() {
            return true;
        }
        if self.show_quit_dialog {
            self.show_quit_dialog = false;
            return true;
        }
        if self.new_conn_dialog.open {
            self.new_conn_dialog.close();
            return true;
        }
        if self.auth_user_dialog.open {
            self.auth_user_dialog.close();
            return true;
        }
        if self.favorite_cmd_dialog.open {
            self.favorite_cmd_dialog.close();
            return true;
        }
        if self.manage_commands_dialog.open {
            self.manage_commands_dialog.open = false;
            self.shell.layout.commands_manage_dialog_open = false;
            return true;
        }
        if self.shell.layout.users_manage_dialog_open {
            self.shell.layout.users_manage_dialog_open = false;
            return true;
        }
        if self.local_term_dialog.open {
            self.local_term_dialog = LocalTerminalSettingsDialog::default();
            return true;
        }
        if self.shell.layout.function_page == FunctionPage::Connections
            || self.shell.layout.function_page == FunctionPage::Commands
        {
            self.shell.layout.function_page = FunctionPage::Active;
            return true;
        }
        if self.shell.function_pane.overlay_visible() {
            self.shell.function_pane.close_overlay();
            return true;
        }
        if self.shell.layout.settings_dialog_open {
            self.shell.layout.settings_dialog_open = false;
            save_settings(&self.settings);
            self.live_font_size = self.settings.font_size();
            self.reload_terminal_fonts(ctx);
            return true;
        }
        if self.shell.layout.help_dialog_open {
            self.shell.layout.help_dialog_open = false;
            return true;
        }
        if self.shell.layout.connections_dialog_open {
            self.shell.layout.connections_dialog_open = false;
            return true;
        }
        if self.has_open_sessions() {
            self.show_quit_dialog = true;
            return true;
        }

        false
    }

    pub(crate) fn request_app_exit(&mut self, ctx: &egui::Context) {
        self.save_profile_tweaks();
        self.settings.function_pane_width = Some(self.shell.layout.function_width);
        save_settings(&self.settings);

        #[cfg(target_os = "android")]
        {
            if crate::platform::android_back::move_task_to_back() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            } else {
                let _ = crate::platform::android_back::finish_activity();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        #[cfg(not(target_os = "android"))]
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    pub(crate) fn handle_close_request(&mut self, ctx: &egui::Context) {
        if self.quit_after_close {
            return;
        }
        if self.handle_back_navigation(ctx) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        } else {
            self.request_app_exit(ctx);
        }
    }
}
