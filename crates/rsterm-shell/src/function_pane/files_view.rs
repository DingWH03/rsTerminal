//! Sidebar Files tab — binds to the focused session's [`rsterm_session_core::SessionFilesCache`].
//!
//! Listing I/O is driven by [`rsterm_session_core::tick_session_files`]; this module only paints
//! and applies browse / DnD gestures.

use crate::session_host::WorkspaceSession;
use crate::shell::messages::FunctionAction;
use crate::uiframe::components::empty_state::{EmptyStateConfig, paint_empty_state};
use crate::uiframe::file_list::{FileListLabels, FileListView, FileRow};
use crate::uiframe::vector_icons::Icon;
use rsterm_data::persist::types::{AuthUser, ConnectionType, SavedConnection};
use rsterm_fs::FileEntry;
use rsterm_session_core::tick_session_files;

/// Local adapter so we can implement [`FileRow`] without violating orphan rules.
struct FileEntryRow<'a>(&'a FileEntry);

impl FileRow for FileEntryRow<'_> {
    fn name(&self) -> &str {
        &self.0.name
    }

    fn is_dir(&self) -> bool {
        self.0.is_dir
    }
}

pub fn render(
    ui: &mut egui::Ui,
    sessions: &mut [WorkspaceSession],
    focused_session_id: Option<&str>,
    connections: &[SavedConnection],
    auth_users: &[AuthUser],
) -> FunctionAction {
    let action = FunctionAction::empty();

    let Some(sid) = focused_session_id else {
        paint_empty(
            ui,
            Icon::Folder,
            &crate::i18n_bridge::tr("sidebar_files_no_terminal"),
            Some(&crate::i18n_bridge::tr("sidebar_files_no_terminal_hint")),
        );
        return action;
    };

    let Some(idx) = sessions.iter().position(|s| s.id() == sid) else {
        paint_empty(
            ui,
            Icon::Folder,
            &crate::i18n_bridge::tr("sidebar_files_no_terminal"),
            Some(&crate::i18n_bridge::tr("sidebar_files_no_terminal_hint")),
        );
        return action;
    };

    let Some(term) = sessions[idx].as_terminal_mut() else {
        paint_empty(
            ui,
            Icon::Folder,
            &crate::i18n_bridge::tr("sidebar_files_no_terminal"),
            Some(&crate::i18n_bridge::tr("sidebar_files_no_terminal_hint")),
        );
        return action;
    };

    match term.core.conn_type {
        ConnectionType::Serial | ConnectionType::Ble => {
            paint_empty(
                ui,
                Icon::Close,
                &crate::i18n_bridge::tr("sidebar_files_unsupported"),
                Some(&crate::i18n_bridge::tr("sidebar_files_unsupported_hint")),
            );
            return action;
        }
        ConnectionType::Local | ConnectionType::Ssh => {}
    }

    tick_session_files(term, connections, auth_users);

    if term.core.conn_type == ConnectionType::Ssh {
        if let Some(err) = term
            .core
            .session_sftp
            .as_ref()
            .and_then(|c| c.connection_error())
        {
            paint_empty(
                ui,
                Icon::Close,
                &crate::i18n_bridge::tr("sidebar_files_sftp_failed"),
                Some(&err),
            );
            return action;
        }
        if term
            .core
            .session_sftp
            .as_ref()
            .is_some_and(|c| c.is_connecting())
        {
            paint_empty(
                ui,
                Icon::Folder,
                &crate::i18n_bridge::tr("sidebar_files_sftp_connecting"),
                Some(&crate::i18n_bridge::tr(
                    "sidebar_files_sftp_connecting_hint",
                )),
            );
            ui.ctx().request_repaint();
            return action;
        }
        if term.core.session_sftp.is_none() && term.core.files.error().is_some() {
            paint_empty(
                ui,
                Icon::Close,
                &crate::i18n_bridge::tr("sidebar_files_sftp_failed"),
                term.core.files.error(),
            );
            return action;
        }
    }

    let Some(cwd_display) = term.core.files.effective_cwd().map(str::to_string) else {
        let waiting = term.core.files.is_busy()
            || term
                .core
                .session_sftp
                .as_ref()
                .is_some_and(|c| c.is_connecting());
        if waiting {
            paint_empty(
                ui,
                Icon::Folder,
                &crate::i18n_bridge::tr("sidebar_files_sftp_connecting"),
                Some(&crate::i18n_bridge::tr(
                    "sidebar_files_sftp_connecting_hint",
                )),
            );
            ui.ctx().request_repaint();
        } else {
            paint_empty(
                ui,
                Icon::Folder,
                &crate::i18n_bridge::tr("sidebar_files_waiting_cwd"),
                Some(&crate::i18n_bridge::tr("sidebar_files_waiting_cwd_hint")),
            );
        }
        return action;
    };

    let list_action = {
        let entries = term.core.files.entries();
        let rows: Vec<FileEntryRow<'_>> = entries.iter().map(FileEntryRow).collect();
        FileListView::show(
            ui,
            &cwd_display,
            &rows,
            term.core.files.error(),
            term.core.files.is_busy(),
            "sidebar_files_list",
            FileListLabels {
                parent_folder: &crate::i18n_bridge::tr("parent_folder"),
                loading: &crate::i18n_bridge::tr("loading"),
                empty_folder: &crate::i18n_bridge::tr("empty_folder"),
            },
        )
    };

    let conn_type = term.core.conn_type;
    if list_action.go_up {
        term.core.files.go_up(conn_type);
    }
    if let Some(idx) = list_action.open_index
        && let Some(ent) = term.core.files.entries().get(idx).cloned()
        && ent.is_dir
    {
        term.core.files.enter_dir(conn_type, &ent.name);
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        if !list_action.dropped_paths.is_empty() {
            let sftp = term.core.session_sftp.clone();
            term.core.files.handle_inbound_drop(
                conn_type,
                sftp.as_ref(),
                &list_action.dropped_paths,
            );
        }
        if !list_action.drag_indices.is_empty() {
            let paths = term
                .core
                .files
                .drag_out_paths(conn_type, &list_action.drag_indices);
            let _ = rsterm_platform::dnd::begin_file_drag_out(&paths);
        }
    }

    if term.core.files.is_busy() {
        ui.ctx().request_repaint();
    } else {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(400));
    }

    action
}

fn paint_empty(ui: &mut egui::Ui, icon: Icon, title: &str, subtitle: Option<&str>) {
    paint_empty_state(ui, EmptyStateConfig::compact(icon, title, subtitle));
}
