//! Per-frame orchestration: tick → shell.render → dispatch actions → dialogs.

use super::RsTerminalApp;
use crate::ui::page::dialogs::{FavoriteCommandOutcome, ProfileDialogOutcome};

impl RsTerminalApp {
    pub(crate) fn frame_logic(&mut self, ctx: &egui::Context) {
        if self.first_frame {
            self.first_frame = false;
            if ctx.input(|i| i.viewport().close_requested()) {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
            ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::BrowserBack);
            });
            return;
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            self.handle_close_request(ctx);
        }
    }

    pub(crate) fn frame_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();

        #[cfg(target_os = "android")]
        {
            use crate::platform::android_back;

            android_back::consume_back_pressed(|| {
                if self.handle_back_navigation(&ctx) {
                    true
                } else {
                    self.request_app_exit(&ctx);
                    true
                }
            });

            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::BrowserBack)) {
                if !self.handle_back_navigation(&ctx) {
                    self.request_app_exit(&ctx);
                }
            }
        }

        crate::i18n::apply_ui_theme(self.prefs.ui_theme(), &ctx);
        self.shell.sync_width(ctx.content_rect().width());

        // F11 toggles OS fullscreen (consume so the terminal does not receive it).
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F11)) {
            let currently = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!currently));
        }

        self.paint_notices(&ctx);

        let top_inset: f32 = {
            #[cfg(target_os = "android")]
            {
                crate::platform::get().top_inset_points(&ctx)
            }
            #[cfg(not(target_os = "android"))]
            {
                0.0
            }
        };

        self.drain_inactive_sessions();
        crate::session::tick_all_session_files(
            &mut self.sessions,
            &self.saved_connections,
            &self.auth_users,
        );

        if let Some(idx) = self.focused_session_index()
            && let Some(term) = self.sessions[idx].as_terminal_mut()
        {
            let ctx = ctx.clone();
            term.core
                .handle
                .repaint
                .set_wake(move || ctx.request_repaint());
        }

        // Only modal dialogs block the host (quit / connection failure).
        // Ordinary windows stay interactive with the main UI underneath.
        let suppress_terminal_input = self.show_quit_dialog || self.connection_notice.is_some();

        let render = self.shell.render(
            ui,
            top_inset,
            &mut self.sessions,
            &mut self.prefs,
            &self.profiles,
            &self.saved_connections,
            &self.favorite_commands,
            &self.auth_users,
            &mut self.virtual_keyboard,
            &mut self.live_font_size,
            suppress_terminal_input,
        );

        if self.shell.sync_focus_change(&mut self.sessions) {
            self.apply_focused_session_terminal_font(&ctx);
        }

        self.dispatch_ui_actions(render.actions, &ctx);

        if let Some(apply) = self.dialogs.local_term.show(&ctx, &self.saved_connections) {
            self.apply_local_terminal_settings(apply);
        }

        if self.dialogs.new_conn.request_new_auth_user {
            self.dialogs.new_conn.request_new_auth_user = false;
            self.dialogs.auth_user.open_new();
            self.release_terminal_keyboard_focus(&ctx);
        }
        if self.dialogs.new_conn.request_new_profile {
            self.dialogs.new_conn.request_new_profile = false;
            self.dialogs.profile.open_new();
            self.release_terminal_keyboard_focus(&ctx);
        }

        if let Some(new_conn) = self
            .dialogs
            .new_conn
            .show(&ctx, &self.auth_users, &self.profiles)
        {
            self.save_connection(new_conn);
        }

        if let Some(user) = self.dialogs.auth_user.show(&ctx) {
            let id = user.id.clone();
            self.save_auth_user(user);
            if self.dialogs.new_conn.open {
                self.dialogs.new_conn.select_auth_user(id);
            }
        }

        match self.dialogs.profile.show(&ctx) {
            ProfileDialogOutcome::Saved(profile) => {
                self.apply_saved_profile(profile);
            }
            ProfileDialogOutcome::None => {}
        }

        match self.dialogs.favorite_cmd.show(&ctx) {
            FavoriteCommandOutcome::Saved(cmd) => self.save_favorite_command(cmd),
            FavoriteCommandOutcome::None => {}
        }

        // Sync after shell.render so menu "Manage" opens same frame (not cleared).
        if self.shell.layout.ui.commands_manage_dialog_open {
            self.dialogs.manage_commands.open = true;
        }
        let manage = self
            .dialogs
            .manage_commands
            .show(&ctx, &self.favorite_commands);
        if !self.dialogs.manage_commands.open {
            self.shell.layout.ui.commands_manage_dialog_open = false;
        }
        if manage.new {
            self.dialogs.favorite_cmd.open_new();
            self.release_terminal_keyboard_focus(&ctx);
        }
        if let Some(id) = manage.edit_id
            && let Some(cmd) = self.favorite_commands.iter().find(|c| c.id == id).cloned()
        {
            self.dialogs.favorite_cmd.open_edit(&cmd);
            self.release_terminal_keyboard_focus(&ctx);
        }
        if let Some(id) = manage.delete_id {
            self.delete_favorite_command(&id);
        }

        self.prefs.chrome.function_pane_width = Some(self.shell.layout.function_width);
        ctx.request_repaint_after(std::time::Duration::from_millis(400));
    }
}
