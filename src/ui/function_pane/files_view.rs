//! Sidebar Files tab — binds to the focused session's [`SessionFilesCache`].
//!
//! Listing I/O is driven by [`crate::ui::page::terminal::files_cache::tick_session_files`]
//! on each session; this module only paints and applies browse / DnD gestures.

use crate::session::WorkspaceSession;
use crate::storage::types::{ConnectionType, SavedConnection};
use crate::ui::page::terminal::files_cache::tick_session_files;
use crate::ui::shell::messages::FunctionAction;
use crate::ui::uiframe::components::empty_state::{paint_empty_state, EmptyStateConfig};
use crate::ui::uiframe::file_list::FileListView;

/// Keep every terminal session's file cache in sync with cwd / SFTP replies.
///
/// Call once per frame (ideally after connection drains) so switching panes is instant.
pub fn tick_all_session_files(
    sessions: &mut [WorkspaceSession],
    connections: &[SavedConnection],
) {
    for session in sessions {
        if let Some(term) = session.terminal_mut() {
            tick_session_files(term, connections);
        }
    }
}

pub fn render(
    ui: &mut egui::Ui,
    sessions: &mut [WorkspaceSession],
    focused_session_id: Option<&str>,
    connections: &[SavedConnection],
) -> FunctionAction {
    let action = FunctionAction::empty();

    let Some(sid) = focused_session_id else {
        paint_empty(
            ui,
            "📂",
            &rust_i18n::t!("sidebar_files_no_terminal"),
            Some(&rust_i18n::t!("sidebar_files_no_terminal_hint")),
        );
        return action;
    };

    let Some(idx) = sessions.iter().position(|s| s.id() == sid) else {
        paint_empty(
            ui,
            "📂",
            &rust_i18n::t!("sidebar_files_no_terminal"),
            Some(&rust_i18n::t!("sidebar_files_no_terminal_hint")),
        );
        return action;
    };

    let WorkspaceSession::Terminal(term) = &mut sessions[idx] else {
        paint_empty(
            ui,
            "📂",
            &rust_i18n::t!("sidebar_files_no_terminal"),
            Some(&rust_i18n::t!("sidebar_files_no_terminal_hint")),
        );
        return action;
    };

    match term.conn_type {
        ConnectionType::Serial | ConnectionType::Ble => {
            paint_empty(
                ui,
                "🚫",
                &rust_i18n::t!("sidebar_files_unsupported"),
                Some(&rust_i18n::t!("sidebar_files_unsupported_hint")),
            );
            return action;
        }
        ConnectionType::Local | ConnectionType::Ssh => {}
    }

    // Opportunistic tick so the open Files tab stays live even if app tick ordering shifts.
    tick_session_files(term, connections);

    if term.conn_type == ConnectionType::Ssh {
        if let Some(err) = term
            .session_sftp
            .as_ref()
            .and_then(|c| c.connection_error())
        {
            paint_empty(
                ui,
                "⚠",
                &rust_i18n::t!("sidebar_files_sftp_failed"),
                Some(&err),
            );
            return action;
        }
        if term.session_sftp.as_ref().is_some_and(|c| c.is_connecting()) {
            paint_empty(
                ui,
                "⏳",
                &rust_i18n::t!("sidebar_files_sftp_connecting"),
                Some(&rust_i18n::t!("sidebar_files_sftp_connecting_hint")),
            );
            ui.ctx().request_repaint();
            return action;
        }
        if term.session_sftp.is_none() && term.files.error().is_some() {
            paint_empty(
                ui,
                "⚠",
                &rust_i18n::t!("sidebar_files_sftp_failed"),
                term.files.error(),
            );
            return action;
        }
    }

    let Some(cwd_display) = term.files.effective_cwd().map(str::to_string) else {
        let waiting = term.files.is_busy()
            || term.session_sftp.as_ref().is_some_and(|c| c.is_connecting());
        if waiting {
            paint_empty(
                ui,
                "⏳",
                &rust_i18n::t!("sidebar_files_sftp_connecting"),
                Some(&rust_i18n::t!("sidebar_files_sftp_connecting_hint")),
            );
            ui.ctx().request_repaint();
        } else {
            paint_empty(
                ui,
                "⏳",
                &rust_i18n::t!("sidebar_files_waiting_cwd"),
                Some(&rust_i18n::t!("sidebar_files_waiting_cwd_hint")),
            );
        }
        return action;
    };

    let list_action = FileListView::show(
        ui,
        &cwd_display,
        term.files.entries(),
        term.files.error(),
        term.files.is_busy(),
        "sidebar_files_list",
    );

    let conn_type = term.conn_type;
    if list_action.go_up {
        term.files.go_up(conn_type);
    }
    if let Some(idx) = list_action.open_index {
        if let Some(ent) = term.files.entries().get(idx).cloned() {
            if ent.is_dir {
                term.files.enter_dir(conn_type, &ent.name);
            }
        }
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        if !list_action.dropped_paths.is_empty() {
            let sftp = term.session_sftp.clone();
            term.files
                .handle_inbound_drop(conn_type, sftp.as_ref(), &list_action.dropped_paths);
        }
        if !list_action.drag_indices.is_empty() {
            let paths = term
                .files
                .drag_out_paths(conn_type, &list_action.drag_indices);
            let _ = crate::platform::dnd::begin_file_drag_out(&paths);
        }
    }

    if term.files.is_busy() {
        ui.ctx().request_repaint();
    } else {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(400));
    }

    action
}

fn paint_empty(ui: &mut egui::Ui, icon: &str, title: &str, subtitle: Option<&str>) {
    paint_empty_state(
        ui,
        EmptyStateConfig {
            icon,
            title,
            subtitle,
            ..Default::default()
        },
    );
}
