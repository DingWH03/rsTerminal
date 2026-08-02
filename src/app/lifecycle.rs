//! Back navigation, quit/exit, close request, focus release, fonts/profile.

use super::RsTerminalApp;
use crate::data::persist::types::TerminalProfile;
use crate::data::persist::types::resolve_profile;
use crate::data::prefs::save_prefs;
use crate::fonts;
use crate::session::WorkspaceSession;
use crate::ui::function_pane::pages::FunctionPage;
use crate::ui::page::dialogs::LocalTerminalSettingsDialog;

impl RsTerminalApp {
    pub(crate) fn reload_terminal_fonts(&mut self, ctx: &egui::Context) {
        let font = self
            .focused_terminal_profile_font()
            .unwrap_or_else(|| resolve_profile(&self.profiles, None).terminal_font.clone());
        fonts::apply_terminal_fonts(ctx, &font);
        let font_gen = fonts::font_generation();
        for session in &mut self.sessions {
            if let WorkspaceSession::Terminal(term) = session {
                term.clear_all_galley_caches();
                term.view.font_generation = font_gen;
            }
        }
    }

    fn focused_terminal_profile_font(&self) -> Option<String> {
        let sid = self
            .shell
            .layout
            .workspace
            .panes
            .get(&self.shell.layout.workspace.focused_pane)?
            .session_id
            .as_deref()?;
        self.sessions.iter().find_map(|s| match s {
            WorkspaceSession::Terminal(t) if t.core.id == sid => Some(
                self.resolve_profile(Some(t.view.profile_id.as_str()))
                    .terminal_font
                    .clone(),
            ),
            _ => None,
        })
    }

    pub(crate) fn apply_focused_session_terminal_font(&mut self, ctx: &egui::Context) {
        let Some(font) = self.focused_terminal_profile_font() else {
            return;
        };
        fonts::apply_terminal_fonts(ctx, &font);
        let font_gen = fonts::font_generation();
        for session in &mut self.sessions {
            if let WorkspaceSession::Terminal(term) = session {
                term.clear_all_galley_caches();
                term.view.font_generation = font_gen;
            }
        }
    }

    /// Apply a profile created/edited in [`ProfileDialog`].
    pub(crate) fn apply_saved_profile(&mut self, profile: TerminalProfile) {
        let id = profile.id.clone();
        let _ = self.persist.upsert_profile(&profile);
        self.reload_profiles();
        if self.dialogs.new_conn.open {
            self.dialogs.new_conn.select_profile(id);
        }
    }

    pub(crate) fn delete_profile(&mut self, id: &str) {
        match self.persist.delete_profile(id) {
            Ok(()) => self.reload_profiles(),
            Err(crate::data::persist::PersistError::ProfileInUse { count }) => {
                self.connection_notice =
                    Some(rust_i18n::t!("err_profile_in_use", count = count).into_owned());
            }
            Err(e) => {
                self.connection_notice = Some(e.to_string());
            }
        }
    }

    pub(crate) fn set_default_profile(&mut self, id: &str) {
        if let Err(e) = self.persist.set_default_profile(id) {
            self.connection_notice = Some(e);
            return;
        }
        self.reload_profiles();
    }

    pub(crate) fn save_profile_tweaks(&mut self) {
        let focused_id = self
            .shell
            .layout
            .workspace
            .panes
            .get(&self.shell.layout.workspace.focused_pane)
            .and_then(|p| p.session_id.clone());
        let (profile_id, font_size) = focused_id
            .and_then(|id| {
                self.sessions.iter().find_map(|s| match s {
                    WorkspaceSession::Terminal(t) if t.core.id == id => {
                        Some((t.view.profile_id.clone(), t.view.live_font_size))
                    }
                    _ => None,
                })
            })
            .unwrap_or_else(|| {
                (
                    resolve_profile(&self.profiles, None).id.clone(),
                    self.live_font_size,
                )
            });
        if let Some(profile) = self.profiles.iter_mut().find(|p| p.id == profile_id) {
            profile.font_size = font_size;
            profile.keyboard_mode = self.virtual_keyboard.mode;
            let p = profile.clone();
            let _ = self.persist.upsert_profile(&p);
        }
    }

    pub(crate) fn release_terminal_keyboard_focus(&mut self, ctx: &egui::Context) {
        for session in &mut self.sessions {
            if let Some(term) = session.terminal_mut() {
                term.view.want_terminal_focus = false;
                term.view.terminal_had_focus = false;
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
        if self.dialogs.new_conn.open {
            self.dialogs.new_conn.close();
            return true;
        }
        if self.dialogs.auth_user.open {
            self.dialogs.auth_user.close();
            return true;
        }
        if self.dialogs.favorite_cmd.open {
            self.dialogs.favorite_cmd.close();
            return true;
        }
        if self.dialogs.manage_commands.open {
            self.dialogs.manage_commands.open = false;
            self.shell.layout.ui.commands_manage_dialog_open = false;
            return true;
        }
        if self.dialogs.profile.open {
            self.dialogs.profile.close();
            return true;
        }
        if self
            .shell
            .layout
            .ui
            .settings_standalone_tab
            .take()
            .is_some()
        {
            save_prefs(&self.prefs);
            self.reload_terminal_fonts(ctx);
            return true;
        }
        if self.dialogs.local_term.open {
            self.dialogs.local_term = LocalTerminalSettingsDialog::default();
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
        if self.shell.layout.ui.settings_dialog_open {
            self.shell.layout.ui.settings_dialog_open = false;
            save_prefs(&self.prefs);
            self.live_font_size = resolve_profile(&self.profiles, None).font_size;
            self.reload_terminal_fonts(ctx);
            return true;
        }
        if self.shell.layout.ui.help_dialog_open {
            self.shell.layout.ui.help_dialog_open = false;
            return true;
        }
        if self.shell.layout.ui.connections_dialog_open {
            self.shell.layout.ui.connections_dialog_open = false;
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
        self.prefs.chrome.function_pane_width = Some(self.shell.layout.function_width);
        save_prefs(&self.prefs);

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
