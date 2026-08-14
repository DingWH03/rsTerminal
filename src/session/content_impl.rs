//! `WorkspaceContent` adapters for terminal and file-manager sessions.

use std::any::Any;

use crate::data::persist::types::resolve_profile;
use crate::session::ConnectionViewAction;
use crate::session::file_manager::FileManagerSession;
use crate::session::terminal::ActiveSession;
use crate::ui::page::file_manager::file_manager_view;
use crate::ui::page::terminal::connection_view;
use crate::ui::workspace_pane::PaneRenderExtras;
use rsterm_workspace::{ContentAction, ContentUiCtx, WorkspaceContent};

impl WorkspaceContent for ActiveSession {
    fn id(&self) -> &str {
        &self.core.id
    }

    fn tab_label(&self) -> String {
        ActiveSession::tab_label(self)
    }

    fn sidebar_has_new_window(&self) -> bool {
        ActiveSession::sidebar_has_new_window(self)
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut ContentUiCtx<'_>) -> ContentAction {
        let Some(extras) = ctx.extras.downcast_mut::<PaneRenderExtras>() else {
            return ContentAction::None;
        };

        let profile_id = self.view.profile_id.clone();
        let (profiles, virtual_keyboard, function_pane, pane_focus_click) = extras.split_mut();
        let (theme, cursor_style, cell_width_scale) = {
            let profile = resolve_profile(profiles, Some(profile_id.as_str()));
            (
                profile.theme.clone(),
                profile.cursor_style,
                profile.cell_width_scale,
            )
        };

        let view_action = connection_view(
            ui,
            Some(self),
            virtual_keyboard,
            &theme,
            cursor_style,
            cell_width_scale,
            function_pane,
            ctx.pane_id,
            ctx.is_focused,
            pane_focus_click,
            ctx.in_split,
            ctx.suppress_terminal_input,
        );

        match view_action {
            ConnectionViewAction::None => ContentAction::None,
            ConnectionViewAction::CloseSession => ContentAction::Close,
            ConnectionViewAction::MinimizePane => ContentAction::MinimizePane,
            ConnectionViewAction::Reconnect(id) => ContentAction::Reconnect(id),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl WorkspaceContent for FileManagerSession {
    fn id(&self) -> &str {
        &self.id
    }

    fn tab_label(&self) -> String {
        FileManagerSession::tab_label(self)
    }

    fn sidebar_has_new_window(&self) -> bool {
        true
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut ContentUiCtx<'_>) -> ContentAction {
        let Some(extras) = ctx.extras.downcast_mut::<PaneRenderExtras>() else {
            return ContentAction::None;
        };

        let (_, _, function_pane, _) = extras.split_mut();
        let fm_action = file_manager_view(ui, self, function_pane, ctx.in_split);
        if fm_action.close {
            ContentAction::Close
        } else {
            ContentAction::None
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
