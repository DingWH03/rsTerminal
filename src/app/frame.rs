//! Per-frame orchestration: tick → shell.render → dispatch actions → dialogs.

use super::RsTerminalApp;
use crate::settings::save_settings;
use crate::session::{ConnectionViewAction, WorkspaceSession};
use crate::ui::function_pane::pages::FunctionPage;

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

        self.settings.ui_theme.apply(&ctx);
        self.shell.sync_width(ctx.content_rect().width());

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
        );

        if let Some(idx) = self.focused_session_index() {
            if let WorkspaceSession::Terminal(term) = &mut self.sessions[idx] {
                term.handle.repaint.set_context(ctx.clone());
            }
        }

        let suppress_terminal_input = self.new_conn_dialog.open
            || self.local_term_dialog.open
            || self.show_quit_dialog
            || self.shell.layout.settings_dialog_open
            || self.shell.layout.help_dialog_open
            || self.shell.layout.connections_dialog_open;

        let render = self.shell.render(
            ui,
            top_inset,
            &mut self.sessions,
            &mut self.settings,
            &self.saved_connections,
            &mut self.virtual_keyboard,
            &mut self.live_font_size,
            suppress_terminal_input,
        );

        crate::session::tick_all_session_files(
            &mut self.sessions,
            &self.saved_connections,
        );

        self.shell.sync_focus_change(&mut self.sessions);

        if render.settings_closed {
            self.shell.layout.settings_dialog_open = false;
            save_settings(&self.settings);
            self.live_font_size = self.settings.font_size();
            self.reload_terminal_fonts(&ctx);
        }

        let fa = &render.function_action;
        let wa = &render.workspace_action;

        if fa.new_connection {
            self.new_conn_dialog.open_new();
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
            save_settings(&self.settings);
            self.live_font_size = self.settings.font_size();
            self.reload_terminal_fonts(&ctx);
        }

        if let Some(new_conn) = self.new_conn_dialog.show(&ctx) {
            self.save_connection(new_conn);
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

        self.settings.function_pane_width = Some(self.shell.layout.function_width);
        ctx.request_repaint_after(std::time::Duration::from_millis(400));
    }
}
