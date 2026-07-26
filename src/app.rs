use crate::connection::{ble, serial, ssh};
#[cfg(not(target_os = "android"))]
use crate::connection::local;
use crate::fonts;
use crate::settings::{save_settings, AppSettings};
use crate::storage;
use crate::storage::types::{ConnectionType, SavedConnection};
use crate::terminal::{DEFAULT_GRID_COLS, DEFAULT_GRID_ROWS};
use crate::terminal::Terminal;
use crate::session::{FileManagerMode, FileManagerSession, WorkspaceSession};
use crate::ui::function_pane::pages::FunctionPage;
use crate::ui::page::terminal::{
    drain_connection, ActiveSession, ConnectionViewAction,
};
use crate::ui::shell::coordinator::ShellCoordinator;
use crate::ui::shell::AppShell;
use crate::ui::uiframe::dialogs::{LocalTerminalSettingsDialog, NewConnectionDialog};
use crate::ui::uiframe::keyboard::VirtualKeyboard;
use crate::ui::uiframe::style;
use log::info;

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

fn show_quit_confirm_dialog(
    ctx: &egui::Context,
    open: &mut bool,
    session_count: usize,
) -> bool {
    if !*open {
        return false;
    }
    let mut confirmed = false;
    egui::Window::new(rust_i18n::t!("quit_with_sessions_title"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_max_width(400.0);
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(rust_i18n::t!("quit_with_sessions_body", count = session_count))
                    .size(14.0)
                    .color(ui.visuals().text_color()),
            );
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                let cancel_btn = egui::Button::new(
                    egui::RichText::new(rust_i18n::t!("cancel"))
                        .size(14.0)
                        .color(ui.visuals().weak_text_color()),
                )
                .fill(ui.visuals().panel_fill)
                .corner_radius(style::CORNER_RADIUS_SM)
                .min_size(egui::vec2(90.0, 34.0));
                if ui.add(cancel_btn).clicked() {
                    *open = false;
                }

                let confirm_btn = egui::Button::new(
                    egui::RichText::new(rust_i18n::t!("quit_with_sessions_confirm"))
                        .size(14.0)
                        .strong()
                        .color(egui::Color32::WHITE),
                )
                .fill(style::RED)
                .corner_radius(style::CORNER_RADIUS_SM)
                .min_size(egui::vec2(100.0, 34.0));
                if ui.add(confirm_btn).clicked() {
                    confirmed = true;
                    *open = false;
                }
            });
        });
    confirmed
}

fn show_connection_notice(ctx: &egui::Context, notice: &mut Option<String>) {
    let Some(msg) = notice.clone() else {
        return;
    };
    let mut dismiss = false;
    egui::Window::new(rust_i18n::t!("connection_failed"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_max_width(420.0);
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(&msg)
                    .size(14.0)
                    .color(ui.visuals().text_color()),
            );
            ui.add_space(16.0);
            let ok_btn = egui::Button::new(
                egui::RichText::new(rust_i18n::t!("ok"))
                    .size(14.0)
                    .color(egui::Color32::WHITE),
            )
            .fill(style::ACCENT)
            .corner_radius(style::CORNER_RADIUS_SM)
            .min_size(egui::vec2(80.0, 34.0));
            if ui.add(ok_btn).clicked() {
                dismiss = true;
            }
        });
    if dismiss {
        *notice = None;
    }
}

impl RsTerminalApp {
    fn reload_terminal_fonts(&mut self, ctx: &egui::Context) {
        fonts::apply_terminal_fonts(ctx, &self.settings.default_profile().terminal_font);
        let font_gen = fonts::font_generation();
        for session in &mut self.sessions {
            if let WorkspaceSession::Terminal(term) = session {
                term.clear_all_galley_caches();
                term.font_generation = font_gen;
            }
        }
    }

    fn push_session(&mut self, session: WorkspaceSession) {
        let id = session.id().to_string();
        self.sessions.push(session);
        ShellCoordinator::assign_new_session(&mut self.shell.layout, id);
    }

    fn open_file_manager_ssh(&mut self, conn_id: &str) {
        let config = match self.saved_connections.iter().find(|c| c.id == conn_id) {
            Some(c) => c.clone(),
            None => return,
        };
        match FileManagerSession::open_ssh(&config) {
            Ok(fm) => self.push_session(WorkspaceSession::FileManager(fm)),
            Err(e) => info!("SFTP failed: {e}"),
        }
    }

    fn open_file_manager_local(&mut self) {
        self.push_session(WorkspaceSession::FileManager(FileManagerSession::open_local()));
    }

    #[cfg(not(target_os = "android"))]
    fn reconnect_local_session(&mut self, session_id: &str, config: &SavedConnection) {
        let Some(idx) = self.sessions.iter().position(|s| s.id() == session_id) else {
            return;
        };
        let WorkspaceSession::Terminal(term) = &mut self.sessions[idx] else {
            return;
        };
        if term.conn_type != ConnectionType::Local {
            return;
        }
        term.handle.close();
        let profile = self.settings.default_profile().clone();
        let rows = term.last_pty_rows.max(1);
        let cols = term.last_pty_cols.max(1);
        match local::connect_local(config, &profile, rows, cols) {
            Ok(handle) => {
                term.handle = handle;
                term.saved_conn_id = Some(config.id.clone());
                term.name = config.name.clone();
                term.user_at_host = crate::platform::get().local_user_at_host();
                term.want_terminal_focus = true;
                term.selection = None;
                term.selection_pointer = None;
            }
            Err(e) => term.disconnect_message = Some(e),
        }
    }

    #[cfg(not(target_os = "android"))]
    fn connect_local(&mut self) {
        let Some(config) = self
            .saved_connections
            .iter()
            .find(|c| c.conn_type == ConnectionType::Local)
            .cloned()
        else {
            self.connection_notice = Some(
                "No saved Local Terminal connection. Add one via the + button.".into(),
            );
            return;
        };
        let profile = self.settings.default_profile().clone();
        match local::connect_local(&config, &profile, 24, 80) {
            Ok(handle) => self.open_session(handle, &config, profile.scrollback_lines),
            Err(e) => self.connection_notice = Some(e),
        }
    }

    fn duplicate_session(&mut self, session_id: &str) {
        enum DupPlan {
            #[cfg(not(target_os = "android"))]
            TerminalLocal,
            TerminalSsh(String),
            FileSsh(String),
            FileLocal,
        }
        let plan = self.sessions.iter().find(|s| s.id() == session_id).and_then(|s| {
            match s {
                WorkspaceSession::Terminal(term) => match term.conn_type {
                    #[cfg(not(target_os = "android"))]
                    ConnectionType::Local => Some(DupPlan::TerminalLocal),
                    #[cfg(target_os = "android")]
                    ConnectionType::Local => None,
                    ConnectionType::Ssh => term.saved_conn_id.clone().map(DupPlan::TerminalSsh),
                    ConnectionType::Serial | ConnectionType::Ble => None,
                },
                WorkspaceSession::FileManager(fm) => match fm.mode {
                    FileManagerMode::SshSftp => fm.saved_conn_id.clone().map(DupPlan::FileSsh),
                    FileManagerMode::LocalDual => Some(DupPlan::FileLocal),
                },
            }
        });
        match plan {
            #[cfg(not(target_os = "android"))]
            Some(DupPlan::TerminalLocal) => self.connect_local(),
            Some(DupPlan::TerminalSsh(id)) => self.connect_to(&id),
            Some(DupPlan::FileSsh(id)) => self.open_file_manager_ssh(&id),
            Some(DupPlan::FileLocal) => self.open_file_manager_local(),
            None => {}
        }
    }

    fn apply_local_terminal_settings(
        &mut self,
        apply: crate::ui::uiframe::dialogs::LocalTerminalSettingsApply,
    ) {
        if self
            .saved_connections
            .iter()
            .any(|c| c.id == apply.config.id)
        {
            if let Some(pos) = self
                .saved_connections
                .iter()
                .position(|c| c.id == apply.config.id)
            {
                self.saved_connections[pos] = apply.config.clone();
            }
            storage::save_connections(&self.saved_connections);
            self.settings.default_local_connection_id = Some(apply.config.id.clone());
            save_settings(&self.settings);
        }
        #[cfg(not(target_os = "android"))]
        if let Some(session_id) = &apply.session_id {
            self.reconnect_local_session(session_id, &apply.config);
        }
    }

    fn connect_to(&mut self, conn_id: &str) {
        self.connect_to_pane(conn_id, self.shell.layout.workspace.focused_pane);
    }

    fn connect_to_pane(&mut self, conn_id: &str, pane: crate::ui::shell::layout_state::PaneId) {
        let config = match self.saved_connections.iter().find(|c| c.id == conn_id) {
            Some(c) => c.clone(),
            None => return,
        };
        let profile = self.settings.default_profile().clone();
        let result = match config.conn_type {
            #[cfg(not(target_os = "android"))]
            ConnectionType::Local => local::connect_local(&config, &profile, 24, 80),
            #[cfg(target_os = "android")]
            ConnectionType::Local => Err("Local terminal is not supported on Android".into()),
            ConnectionType::Ssh => ssh::connect_ssh(&config, &self.settings.ssh_env_vars, 24, 80),
            ConnectionType::Serial => serial::connect_serial(&config),
            ConnectionType::Ble => ble::connect_ble(&config),
        };
        match result {
            Ok(handle) => self.open_session_in_pane(handle, &config, profile.scrollback_lines, pane),
            Err(e) => self.connection_notice = Some(e),
        }
    }

    fn open_session(
        &mut self,
        handle: crate::connection::ConnectionHandle,
        config: &SavedConnection,
        scrollback_lines: usize,
    ) {
        self.open_session_in_pane(
            handle,
            config,
            scrollback_lines,
            self.shell.layout.workspace.focused_pane,
        );
    }

    fn open_session_in_pane(
        &mut self,
        handle: crate::connection::ConnectionHandle,
        config: &SavedConnection,
        scrollback_lines: usize,
        pane: crate::ui::shell::layout_state::PaneId,
    ) {
        let profile = self.settings.default_profile();
        let mut terminal = Terminal::new(DEFAULT_GRID_ROWS, DEFAULT_GRID_COLS);
        terminal.set_scrollback_limit(scrollback_lines);
        self.live_font_size = profile.font_size;
        self.virtual_keyboard = VirtualKeyboard::new(profile.keyboard_mode);

        let user_at_host = match config.conn_type {
            ConnectionType::Local => crate::platform::get().local_user_at_host(),
            ConnectionType::Ssh => {
                let user = config.ssh_user.as_deref().unwrap_or("root");
                let host = config.ssh_host.as_deref().unwrap_or("host");
                crate::platform::get().ssh_user_at_host(user, host)
            }
            _ => String::new(),
        };

        let id = uuid::Uuid::new_v4().to_string();
        self.sessions.push(WorkspaceSession::Terminal(ActiveSession {
            id: id.clone(),
            conn_type: config.conn_type.clone(),
            saved_conn_id: Some(config.id.clone()),
            name: config.name.clone(),
            user_at_host,
            handle,
            terminal,
            active_port: 0,
            ports: Vec::new(),
            inactive_port_states: Default::default(),
            port_unread: Default::default(),
            scrollback_lines,
            scroll_offset: 0,
            selection: None,
            selection_pointer: None,
            touch_state: Default::default(),
            want_terminal_focus: true,
            terminal_had_focus: false,
            row_galley_cache: Default::default(),
            layout_font_size: self.live_font_size,
            grid_rows: DEFAULT_GRID_ROWS,
            grid_cols: DEFAULT_GRID_COLS,
            last_pty_rows: DEFAULT_GRID_ROWS as u16,
            last_pty_cols: DEFAULT_GRID_COLS as u16,
            size_label_dims: (DEFAULT_GRID_COLS, DEFAULT_GRID_ROWS),
            size_label_hide_at: None,
            size_label_active: false,
            mouse_motion_last: None,
            font_generation: crate::fonts::font_generation(),
            disconnect_message: None,
        }));
        ShellCoordinator::assign_session_to_pane(&mut self.shell.layout, pane, id);
    }

    fn has_open_sessions(&self) -> bool {
        !self.sessions.is_empty()
    }

    fn close_all_sessions(&mut self) {
        let ids: Vec<String> = self.sessions.iter().map(|s| s.id().to_string()).collect();
        for id in ids {
            self.close_session(&id);
        }
    }

    fn close_pane_and_maybe_session(&mut self, pane: crate::ui::shell::layout_state::PaneId) {
        let sid = self
            .shell
            .layout
            .workspace
            .panes
            .get(&pane)
            .and_then(|p| p.session_id.clone());
        if self.shell.layout.workspace.pane_count() > 1 {
            self.shell.layout.workspace.close_pane(pane);
        }
        if let Some(id) = sid {
            self.close_session(&id);
        }
    }

    fn close_session(&mut self, id: &str) {
        if let Some(pos) = self.sessions.iter().position(|s| s.id() == id) {
            if let WorkspaceSession::Terminal(s) = &mut self.sessions[pos] {
                s.handle.close();
            }
            self.sessions.remove(pos);
        }
        ShellCoordinator::on_sessions_closed(&mut self.shell.layout, id);
        if self.sessions.is_empty() {
            self.save_profile_tweaks();
        }
    }

    fn save_profile_tweaks(&mut self) {
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

    fn drain_inactive_sessions(&mut self) {
        let active = self.shell.focused_session_id();
        for session in &mut self.sessions {
            if active == Some(session.id()) {
                continue;
            }
            if let Some(term) = session.terminal_mut() {
                let mut noop = ConnectionViewAction::None;
                drain_connection(term, &mut noop);
            }
        }
    }

    fn focused_session_index(&self) -> Option<usize> {
        self.shell
            .focused_session_id()
            .and_then(|id| self.sessions.iter().position(|s| s.id() == id))
    }

    fn handle_back_navigation(&mut self, ctx: &egui::Context) -> bool {
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
        if self.local_term_dialog.open {
            self.local_term_dialog = LocalTerminalSettingsDialog::default();
            return true;
        }
        if self.shell.layout.function_page == FunctionPage::Connections {
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

    fn request_app_exit(&mut self, ctx: &egui::Context) {
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

    fn handle_close_request(&mut self, ctx: &egui::Context) {
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

impl eframe::App for RsTerminalApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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

        show_connection_notice(&ctx, &mut self.connection_notice);

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

        let session_count = self.sessions.len();
        if show_quit_confirm_dialog(&ctx, &mut self.show_quit_dialog, session_count) {
            self.quit_after_close = true;
            self.close_all_sessions();
            self.request_app_exit(&ctx);
        }

        self.drain_inactive_sessions();

        if let Some(idx) = self.focused_session_index() {
            if let WorkspaceSession::Terminal(term) = &mut self.sessions[idx] {
                term.handle.repaint.set_context(ctx.clone());
            }
        }

        let render = self.shell.render(
            ui,
            top_inset,
            &mut self.sessions,
            &mut self.settings,
            &self.saved_connections,
            &mut self.virtual_keyboard,
            &mut self.live_font_size,
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
        }
        if let Some(ref id) = fa.connect_connection {
            self.connect_to(id);
            self.shell.layout.function_page = FunctionPage::Active;
        }
        if let Some(ref id) = fa.open_file_mgr {
            if let Some(conn) = self.saved_connections.iter().find(|c| c.id == *id) {
                match conn.conn_type {
                    ConnectionType::Local => self.open_file_manager_local(),
                    ConnectionType::Ssh => self.open_file_manager_ssh(id),
                    _ => {}
                }
            }
            self.shell.layout.function_page = FunctionPage::Active;
        }
        if let Some(ref id) = fa.edit_connection {
            if let Some(conn) = self.saved_connections.iter().find(|c| c.id == *id) {
                self.new_conn_dialog.open_edit(conn);
            }
        }
        if let Some(ref id) = fa.delete_connection {
            self.saved_connections.retain(|c| c.id != *id);
            storage::save_connections(&self.saved_connections);
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
            if let Some(pos) = self
                .saved_connections
                .iter()
                .position(|c| c.id == new_conn.id)
            {
                self.saved_connections[pos] = new_conn;
            } else {
                self.saved_connections.push(new_conn);
            }
            storage::save_connections(&self.saved_connections);
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
            if let Some(sid) = self
                .shell
                .layout
                .workspace
                .panes
                .get(&pane)
                .and_then(|p| p.session_id.clone())
            {
                if let Some(idx) = self.sessions.iter().position(|s| s.id() == sid) {
                    if let WorkspaceSession::Terminal(session) = &mut self.sessions[idx] {
                        if matches!(session.conn_type, ConnectionType::Ssh) {
                            if let Some(config) =
                                self.saved_connections.iter().find(|c| c.id == *conn_id)
                            {
                                match ssh::connect_ssh(
                                    config,
                                    &self.settings.ssh_env_vars,
                                    24,
                                    80,
                                ) {
                                    Ok(new_handle) => {
                                        session.handle = new_handle;
                                        session.disconnect_message = None;
                                        session.want_terminal_focus = true;
                                    }
                                    Err(e) => self.connection_notice = Some(e),
                                }
                            }
                        }
                    }
                }
            }
        }

        self.settings.function_pane_width = Some(self.shell.layout.function_width);
        ctx.request_repaint_after(std::time::Duration::from_millis(400));
    }
}
