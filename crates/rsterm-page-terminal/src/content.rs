//! `WorkspaceContent` adapter for terminal sessions.

use std::any::Any;

use rsterm_data::persist::types::resolve_profile;
use rsterm_session_core::{ActiveSession, ConnectionViewAction};
use rsterm_uiframe::PaneChrome;
use rsterm_workspace::{ContentAction, ContentUiCtx, WorkspaceContent};

use crate::host_extras::TerminalHostExtras;
use crate::page::connection_view;

/// Orphan-rule newtype owning an [`ActiveSession`].
pub struct ActiveSessionContent {
    pub inner: ActiveSession,
    /// Set during `ui` when the view requests reconnect; host reads after `ui`.
    pub pending_reconnect: Option<String>,
}

/// Wrap a terminal session as workspace content.
pub fn wrap_terminal(s: ActiveSession) -> Box<dyn WorkspaceContent> {
    Box::new(ActiveSessionContent {
        inner: s,
        pending_reconnect: None,
    })
}

impl WorkspaceContent for ActiveSessionContent {
    fn id(&self) -> &str {
        &self.inner.core.id
    }

    fn tab_label(&self) -> String {
        ActiveSession::tab_label(&self.inner)
    }

    fn sidebar_has_new_window(&self) -> bool {
        ActiveSession::sidebar_has_new_window(&self.inner)
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut ContentUiCtx<'_>) -> ContentAction {
        let Some(extras) = ctx.extras.downcast_mut::<TerminalHostExtras>() else {
            return ContentAction::None;
        };

        let profile_id = self.inner.view.profile_id.clone();
        let (theme, cursor_style, cell_width_scale) = {
            let profile = resolve_profile(extras.profiles(), Some(profile_id.as_str()));
            (
                profile.theme.clone(),
                profile.cursor_style,
                profile.cell_width_scale,
            )
        };

        let mut hamburger_clicked = false;
        let view_action = {
            let (_, virtual_keyboard) = extras.split_mut();
            let mut on_hamburger = || {
                hamburger_clicked = true;
            };
            let mut chrome = PaneChrome {
                show_hamburger: ctx.show_hamburger,
                on_hamburger: &mut on_hamburger,
            };
            connection_view(
                ui,
                Some(&mut self.inner),
                virtual_keyboard,
                &theme,
                cursor_style,
                cell_width_scale,
                &mut chrome,
                ctx.pane_id,
                ctx.is_focused,
                ctx.pane_focus_click,
                ctx.in_split,
                ctx.suppress_terminal_input,
            )
        };
        if hamburger_clicked {
            *ctx.hamburger_pending = true;
        }

        match view_action {
            ConnectionViewAction::None => ContentAction::None,
            ConnectionViewAction::CloseSession => ContentAction::Close,
            ConnectionViewAction::MinimizePane => ContentAction::MinimizePane,
            ConnectionViewAction::Reconnect(id) => {
                self.pending_reconnect = Some(id);
                ContentAction::None
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
