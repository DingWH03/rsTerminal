//! Per-frame orchestration: tick → shell.render → dispatch actions → dialogs.

use super::RsTerminalApp;
use crate::prefs::save_prefs;
use crate::session::{ConnectionViewAction, WorkspaceSession};
use crate::ui::function_pane::pages::FunctionPage;
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

        self.prefs.ui_theme().apply(&ctx);
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

        if let Some(idx) = self.focused_session_index() {
            if let WorkspaceSession::Terminal(term) = &mut self.sessions[idx] {
                term.handle.repaint.set_context(ctx.clone());
            }
        }

        // Only modal dialogs block the host (quit / connection failure).
        // Ordinary windows stay interactive with the main UI underneath.
        let suppress_terminal_input =
            self.show_quit_dialog || self.connection_notice.is_some();

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

        crate::session::tick_all_session_files(
            &mut self.sessions,
            &self.saved_connections,
            &self.auth_users,
        );

        if self.shell.sync_focus_change(&mut self.sessions) {
            self.apply_focused_session_terminal_font(&ctx);
        }

        if render.settings_closed {
            self.shell.layout.settings_dialog_open = false;
            save_prefs(&self.prefs);
            self.live_font_size = self.resolve_profile(None).font_size;
            self.reload_terminal_fonts(&ctx);
        }

        // Settings → Users / Profiles nested page actions.
        if render.auth_users_action.new {
            self.auth_user_dialog.open_new();
            self.release_terminal_keyboard_focus(&ctx);
        }
        if let Some(id) = render.auth_users_action.edit_id.clone() {
            if let Some(user) = self.auth_users.iter().find(|u| u.id == id).cloned() {
                self.auth_user_dialog.open_edit(&user);
                self.release_terminal_keyboard_focus(&ctx);
            }
        }
        if let Some(id) = render.auth_users_action.delete_id.clone() {
            self.delete_auth_user(&id);
        }
        if render.request_new_profile {
            self.profile_dialog.open_new();
            self.release_terminal_keyboard_focus(&ctx);
        }
        if let Some(id) = render.request_edit_profile.clone() {
            if let Some(profile) = self.profiles.iter().find(|p| p.id == id).cloned() {
                self.profile_dialog.open_edit(&profile);
                self.release_terminal_keyboard_focus(&ctx);
            }
        }
        if let Some(id) = render.delete_profile_id.clone() {
            self.delete_profile(&id);
        }
        if let Some(id) = render.set_default_profile_id.clone() {
            self.set_default_profile(&id);
        }

        let fa = &render.function_action;
        let wa = &render.workspace_action;

        if fa.new_connection {
            self.new_conn_dialog.open_new();
            self.release_terminal_keyboard_focus(&ctx);
        }
        if fa.new_favorite_command {
            self.favorite_cmd_dialog.open_new();
            self.release_terminal_keyboard_focus(&ctx);
        }
        if let Some(ref id) = fa.connect_connection {
            self.connect_to(id);
            self.shell.layout.function_page = FunctionPage::Active;
        }
        if let Some(ref id) = fa.open_file_mgr {
            self.open_file_manager_for_connection(id);
        }
        if let Some(ref id) = fa.edit_connection {
            if let Some(conn) = self.saved_connections.iter().find(|c| c.id == *id) {
                self.new_conn_dialog.open_edit(conn);
                self.release_terminal_keyboard_focus(&ctx);
            }
        }
        if let Some(ref id) = fa.delete_connection {
            self.delete_connection(id);
        }
        if let Some(ref id) = fa.close_session {
            self.close_session(id);
        }
        if let Some(ref id) = fa.duplicate_session {
            self.duplicate_session(id);
            if self.shell.function_pane.overlay_visible() {
                self.shell.function_pane.close_overlay();
            }
        }
        if let Some(ref id) = fa.run_favorite_command {
            self.run_favorite_command(id, &ctx);
        }
        if let Some(ref id) = fa.edit_favorite_command {
            if let Some(cmd) = self.favorite_commands.iter().find(|c| c.id == *id).cloned() {
                self.favorite_cmd_dialog.open_edit(&cmd);
                self.release_terminal_keyboard_focus(&ctx);
            }
        }
        if let Some(ref id) = fa.delete_favorite_command {
            self.delete_favorite_command(id);
        }

        if let Some(req) = wa.connect_from_empty.clone() {
            self.connect_to_pane(&req.connection_id, req.pane);
        }
        if let Some(pane) = wa.open_connections_from_empty {
            self.shell.layout.workspace.focused_pane = pane;
            self.shell.layout.connections_dialog_open = true;
        }

        if let Some(apply) = self.local_term_dialog.show(&ctx, &self.saved_connections) {
            self.apply_local_terminal_settings(apply);
        }

        if fa.toggle_settings || render.settings_closed {
            save_prefs(&self.prefs);
            self.live_font_size = self.resolve_profile(None).font_size;
            self.reload_terminal_fonts(&ctx);
        }

        if self.new_conn_dialog.request_new_auth_user {
            self.new_conn_dialog.request_new_auth_user = false;
            self.auth_user_dialog.open_new();
            self.release_terminal_keyboard_focus(&ctx);
        }
        if self.new_conn_dialog.request_new_profile {
            self.new_conn_dialog.request_new_profile = false;
            self.profile_dialog.open_new();
            self.release_terminal_keyboard_focus(&ctx);
        }

        if let Some(new_conn) =
            self.new_conn_dialog
                .show(&ctx, &self.auth_users, &self.profiles)
        {
            self.save_connection(new_conn);
        }

        if let Some(user) = self.auth_user_dialog.show(&ctx) {
            let id = user.id.clone();
            self.save_auth_user(user);
            if self.new_conn_dialog.open {
                self.new_conn_dialog.select_auth_user(id);
            }
        }

        match self.profile_dialog.show(&ctx) {
            ProfileDialogOutcome::Saved(profile) => {
                self.apply_saved_profile(profile);
            }
            ProfileDialogOutcome::None => {}
        }

        match self.favorite_cmd_dialog.show(&ctx) {
            FavoriteCommandOutcome::Saved(cmd) => self.save_favorite_command(cmd),
            FavoriteCommandOutcome::None => {}
        }

        // Sync after shell.render so menu "Manage" opens same frame (not cleared).
        if self.shell.layout.commands_manage_dialog_open {
            self.manage_commands_dialog.open = true;
        }
        let manage = self
            .manage_commands_dialog
            .show(&ctx, &self.favorite_commands);
        if !self.manage_commands_dialog.open {
            self.shell.layout.commands_manage_dialog_open = false;
        }
        if manage.new {
            self.favorite_cmd_dialog.open_new();
            self.release_terminal_keyboard_focus(&ctx);
        }
        if let Some(id) = manage.edit_id {
            if let Some(cmd) = self.favorite_commands.iter().find(|c| c.id == id).cloned() {
                self.favorite_cmd_dialog.open_edit(&cmd);
                self.release_terminal_keyboard_focus(&ctx);
            }
        }
        if let Some(id) = manage.delete_id {
            self.delete_favorite_command(&id);
        }

        if let Some(pane) = wa.close_pane_session {
            self.close_pane_and_maybe_session(pane);
        }

        if matches!(wa.terminal, ConnectionViewAction::CloseSession) || wa.file_manager.close {
            let pane = wa
                .terminal_pane
                .unwrap_or(self.shell.layout.workspace.focused_pane);
            self.close_pane_and_maybe_session(pane);
        }
        if let ConnectionViewAction::Reconnect(ref conn_id) = wa.terminal {
            let pane = wa
                .terminal_pane
                .unwrap_or(self.shell.layout.workspace.focused_pane);
            self.reconnect_ssh_session(pane, conn_id);
        }

        self.prefs.chrome.function_pane_width = Some(self.shell.layout.function_width);
        ctx.request_repaint_after(std::time::Duration::from_millis(400));
    }
}
