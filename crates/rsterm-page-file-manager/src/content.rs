//! `WorkspaceContent` adapter for file-manager sessions.

use std::any::Any;

use rsterm_session_core::FileManagerSession;
use rsterm_uiframe::PaneChrome;
use rsterm_workspace::{ContentAction, ContentUiCtx, WorkspaceContent};

use crate::page::file_manager_view;

/// Orphan-rule newtype owning a [`FileManagerSession`].
pub struct FileManagerContent {
    pub inner: FileManagerSession,
}

/// Wrap a file-manager session as workspace content.
pub fn wrap_file_manager(s: FileManagerSession) -> Box<dyn WorkspaceContent> {
    Box::new(FileManagerContent { inner: s })
}

impl WorkspaceContent for FileManagerContent {
    fn id(&self) -> &str {
        &self.inner.id
    }

    fn tab_label(&self) -> String {
        FileManagerSession::tab_label(&self.inner)
    }

    fn sidebar_has_new_window(&self) -> bool {
        true
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut ContentUiCtx<'_>) -> ContentAction {
        let mut hamburger_clicked = false;
        let fm_action = {
            let mut on_hamburger = || {
                hamburger_clicked = true;
            };
            let mut chrome = PaneChrome {
                show_hamburger: ctx.show_hamburger,
                on_hamburger: &mut on_hamburger,
            };
            file_manager_view(ui, &mut self.inner, &mut chrome)
        };
        if hamburger_clicked {
            *ctx.hamburger_pending = true;
        }
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
