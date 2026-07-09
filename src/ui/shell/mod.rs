//! Application shell — left function pane + right workspace pane.

pub mod animations;
pub mod coordinator;
pub mod layout_preview;
pub mod layout_state;
pub mod messages;

use crate::session::WorkspaceSession;
use crate::settings::AppSettings;
use crate::storage::types::SavedConnection;
use crate::ui::function_pane::{self, split_enabled, drag_split_enabled, FunctionPane};
use crate::ui::page::settings::settings_page;
use crate::ui::widget::keyboard::VirtualKeyboard;
use crate::ui::workspace_pane::{self, drag_drop::ActiveDrag, WorkspacePaneContext};

use animations::ShellAnimations;

use coordinator::ShellCoordinator;
use layout_state::{FUNCTION_MAX_WIDTH, FUNCTION_MIN_WIDTH, ShellLayout};
use messages::{FunctionAction, WorkspaceAction};

/// Top-level UI shell owning layout state.
pub struct AppShell {
    pub layout: ShellLayout,
    pub function_pane: FunctionPane,
    last_focused_pane: Option<crate::ui::shell::layout_state::PaneId>,
    active_drag: Option<ActiveDrag>,
    animations: ShellAnimations,
    last_drop_zone: Option<crate::ui::shell::layout_state::DropZone>,
}

impl Default for AppShell {
    fn default() -> Self {
        Self {
            layout: ShellLayout::default(),
            function_pane: FunctionPane::new(),
            last_focused_pane: None,
            active_drag: None,
            animations: ShellAnimations::new(),
            last_drop_zone: None,
        }
    }
}

impl AppShell {
    pub fn from_settings(settings: &AppSettings) -> Self {
        Self {
            layout: ShellLayout::from_settings(settings.function_pane_width),
            function_pane: FunctionPane::new(),
            last_focused_pane: None,
            active_drag: None,
            animations: ShellAnimations::new(),
            last_drop_zone: None,
        }
    }

    pub fn focused_session_id(&self) -> Option<&str> {
        self.layout.workspace.focused_session_id()
    }

    pub fn sync_focus_change(&mut self, sessions: &mut [WorkspaceSession]) {
        let current = self.layout.workspace.focused_pane;
        if self.last_focused_pane == Some(current) {
            return;
        }
        self.last_focused_pane = Some(current);
        let focused_sid = self.focused_session_id().map(str::to_string);
        for session in sessions {
            let sid = session.id().to_string();
            if let WorkspaceSession::Terminal(t) = session {
                let active = focused_sid.as_deref() == Some(sid.as_str());
                t.want_terminal_focus = active;
                if !active {
                    t.terminal_had_focus = false;
                }
            }
        }
    }

    pub fn sync_width(&mut self, width: f32) {
        self.function_pane.sync_width(width);
    }

    pub fn sync_split_gate(&mut self, session_count: usize) {
        let split_on = split_enabled(&self.function_pane, session_count);
        let drag_on = drag_split_enabled(&self.function_pane, session_count);
        if !split_on && self.layout.workspace.pane_count() > 1 {
            self.layout.workspace.collapse_to_focused();
            self.last_focused_pane = Some(self.layout.workspace.focused_pane);
        }
        if !drag_on {
            self.active_drag = None;
            self.last_drop_zone = None;
        }
    }

    fn session_label(sessions: &[WorkspaceSession], id: &str) -> String {
        sessions
            .iter()
            .find(|s| s.id() == id)
            .map(|s| format!("{} {}", s.icon(), s.tab_label()))
            .unwrap_or_else(|| id.to_string())
    }

    fn pane_label(sessions: &[WorkspaceSession], layout: &ShellLayout, pane_id: crate::ui::shell::layout_state::PaneId) -> String {
        layout
            .workspace
            .panes
            .get(&pane_id)
            .and_then(|p| p.session_id.as_deref())
            .map(|id| Self::session_label(sessions, id))
            .unwrap_or_else(|| rust_i18n::t!("empty_pane").to_string())
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        top_inset: f32,
        sessions: &mut [WorkspaceSession],
        settings: &mut AppSettings,
        saved_connections: &[SavedConnection],
        virtual_keyboard: &mut VirtualKeyboard,
        live_font_size: &mut f32,
    ) -> ShellRenderResult {
        let mut result = ShellRenderResult::default();
        let session_count = sessions.len();
        self.sync_split_gate(session_count);
        if self.layout.workspace.pane_count() > 1 {
            virtual_keyboard.visible = false;
        }
        let split_on = split_enabled(&self.function_pane, session_count);
        let drag_split_on = drag_split_enabled(&self.function_pane, session_count);

        let dt = ui.ctx().input(|i| i.unstable_dt);
        if self.animations.tick(dt) {
            ui.ctx().request_repaint();
        }

        let is_wide = self.function_pane.wide;
        let show_function = if is_wide {
            self.function_pane.docked_visible()
        } else {
            self.function_pane.overlay_visible()
        };

        if show_function && !(self.layout.settings_overlay && !is_wide) {
            let panel_id = if is_wide {
                "function_pane"
            } else {
                "function_pane_narrow"
            };
            let mut panel = egui::Panel::left(panel_id)
                .resizable(is_wide)
                .default_size(self.layout.function_width);

            if is_wide {
                panel = panel
                    .min_size(FUNCTION_MIN_WIDTH)
                    .max_size(FUNCTION_MAX_WIDTH);
            } else {
                panel = panel
                    .min_size(ui.available_width())
                    .resizable(false);
            }

            let inner = panel.show_inside(ui, |ui| {
                ui.add_space(top_inset);
                result.function_action = function_pane::render(
                    ui,
                    &mut self.function_pane,
                    &self.layout.function_page,
                    sessions,
                    &self.layout.workspace,
                    self.layout.workspace.highlighted_session_id(),
                    self.layout.settings_overlay,
                    saved_connections,
                    settings,
                    self.animations.page_slide.current,
                );
            });

            if is_wide {
                let w = inner.response.rect.width();
                if w > FUNCTION_MIN_WIDTH {
                    self.layout.function_width = w.clamp(FUNCTION_MIN_WIDTH, FUNCTION_MAX_WIDTH);
                }
            }

            if let Some(ref id) = result.function_action.start_session_drag {
                if self.active_drag.is_none() && drag_split_on {
                    self.active_drag = Some(ActiveDrag::Session {
                        session_id: id.clone(),
                        label: Self::session_label(sessions, id),
                    });
                }
            }
        }

        let show_workspace =
            is_wide || !show_function || self.layout.settings_overlay;

        if show_workspace {
            egui::CentralPanel::default().show_inside(ui, |ui| {
                ui.add_space(top_inset);
                if self.layout.settings_overlay {
                    if settings_page(ui, settings) {
                        result.settings_closed = true;
                    }
                } else {
                    let mut session_fade = std::collections::HashMap::new();
                    for (&pane, _) in &self.layout.workspace.panes {
                        session_fade.insert(pane, self.animations.session_fade_value(pane));
                    }

                    let mut last_drop_zone = self.last_drop_zone;
                    let ws_result = {
                        let mut ws_ctx = WorkspacePaneContext {
                            sessions,
                            settings,
                            saved_connections,
                            virtual_keyboard,
                            live_font_size,
                            function_pane: &mut self.function_pane,
                            split_enabled: split_on,
                            active_drag: self.active_drag.clone(),
                            ratio_overrides: std::collections::HashMap::new(),
                            session_fade,
                            split_layout_active: false,
                            last_drop_zone: &mut last_drop_zone,
                        };
                        workspace_pane::render(ui, &mut self.layout.workspace, &mut ws_ctx)
                    };
                    result.workspace_action = ws_result.action;

                    if let Some(pane) = result.workspace_action.start_pane_drag {
                        if self.active_drag.is_none() && drag_split_on {
                            self.active_drag = Some(ActiveDrag::Pane {
                                pane_id: pane,
                                label: Self::pane_label(sessions, &self.layout, pane),
                            });
                        }
                    }

                    if ws_result.drag_ended {
                        if result.workspace_action.drop_applied {
                            self.animations
                                .fade_in_pane(self.layout.workspace.focused_pane);
                        }
                        self.active_drag = None;
                        self.last_drop_zone = None;
                    } else {
                        self.last_drop_zone = last_drop_zone;
                        if self.active_drag.is_some() {
                            ui.ctx().request_repaint();
                        }
                    }
                }
            });
        } else if ui.input(|i| i.pointer.any_released()) {
            if let (Some(drag), Some(zone)) = (&self.active_drag, self.last_drop_zone) {
                let palette_len =
                    crate::ui::pane_colors::resolve_palette(settings).len().max(1);
                if let Some(focused) = crate::ui::workspace_pane::drag_drop::apply_drop(
                    &mut self.layout.workspace,
                    drag,
                    zone,
                    palette_len,
                ) {
                    self.layout.workspace.focused_pane = focused;
                    result.workspace_action.drop_applied = true;
                    result.workspace_action.focus_pane = Some(focused);
                    self.animations.fade_in_pane(focused);
                }
            }
            self.active_drag = None;
            self.last_drop_zone = None;
        }

        let in_overlay = self.function_pane.overlay_visible();
        ShellCoordinator::apply_function(&mut self.layout, &result.function_action, in_overlay);
        ShellCoordinator::apply_workspace(&mut self.layout, &result.workspace_action);

        if result.function_action.select_session.is_some()
            || result.function_action.duplicate_session.is_some()
        {
            self.animations
                .fade_in_pane(self.layout.workspace.focused_pane);
        }

        if result.function_action.toggle_settings {
            self.function_pane.close_overlay();
        }
        if result.function_action.select_session.is_some() {
            if in_overlay {
                self.function_pane.close_overlay();
            }
        }
        if result.function_action.go_back
            || result.function_action.connect_connection.is_some()
            || result.function_action.open_file_mgr.is_some()
        {
            self.function_pane.close_overlay();
        }

        result
    }
}

#[derive(Debug, Default)]
pub struct ShellRenderResult {
    pub function_action: FunctionAction,
    pub workspace_action: WorkspaceAction,
    pub settings_closed: bool,
}
