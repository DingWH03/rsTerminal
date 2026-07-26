//! Sidebar Files tab — single-column listing for the focused terminal.
//!
//! Path / refresh signals:
//! - **OSC 7 / 133** — shell integration (fallback)
//! - **Remote agent** — `SessionMetrics` on SSH sessions (preferred cwd + host stats)
//!
//! Backends:
//! - **Local**: local FS
//! - **SSH**: shared-session SFTP when available, else a dedicated SFTP connect

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Arc;

use crate::fs::local;
use crate::fs::sftp::{join_remote, SftpClient};
use crate::fs::FileEntry;
use crate::session::WorkspaceSession;
use crate::storage::types::{ConnectionType, SavedConnection};
use crate::ui::shell::messages::FunctionAction;
use crate::ui::uiframe::components::empty_state::{paint_empty_state, EmptyStateConfig};
use crate::ui::uiframe::file_list::FileListView;

enum PendingOp {
    Home(mpsc::Receiver<Result<String, String>>),
    List {
        path: String,
        rx: mpsc::Receiver<Result<Vec<FileEntry>, String>>,
    },
}

/// Cached listing state for the sidebar Files tab.
#[derive(Default)]
pub struct SidebarFilesState {
    tracked_session: Option<String>,
    tracked_cwd: Option<String>,
    /// Last consumed [`crate::terminal::screen::SemanticShell::mark_seq`].
    last_semantic_seq: u64,
    entries: Vec<FileEntry>,
    error: Option<String>,
    loading: bool,
    browse_cwd: Option<String>,
    sftp: Option<Arc<SftpClient>>,
    sftp_conn_id: Option<String>,
    pending: Option<PendingOp>,
}

impl SidebarFilesState {
    fn effective_cwd(&self) -> Option<&str> {
        self.browse_cwd
            .as_deref()
            .or(self.tracked_cwd.as_deref())
    }

    fn on_session_cwd(&mut self, session_id: &str, cwd: &str) {
        let changed = self.tracked_session.as_deref() != Some(session_id)
            || self.tracked_cwd.as_deref() != Some(cwd);
        if changed {
            self.tracked_session = Some(session_id.to_string());
            self.tracked_cwd = Some(cwd.to_string());
            self.browse_cwd = None;
            self.entries.clear();
            self.error = None;
            self.loading = true;
            self.pending = None;
        }
    }

    /// Apply OSC 7 cwd + OSC 133 prompt marks from the focused terminal.
    fn follow_shell_integration(&mut self, session_id: &str, osc_cwd: Option<&str>, mark_seq: u64) {
        if let Some(cwd) = osc_cwd {
            self.on_session_cwd(session_id, cwd);
        }
        if self.last_semantic_seq != mark_seq {
            self.last_semantic_seq = mark_seq;
            // While following the shell (not manually browsing), refresh on each mark.
            if self.browse_cwd.is_none() && self.tracked_cwd.is_some() {
                self.loading = true;
                self.pending = None;
                self.error = None;
            }
        }
    }
}

pub fn render(
    ui: &mut egui::Ui,
    state: &mut SidebarFilesState,
    sessions: &[WorkspaceSession],
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

    let Some(WorkspaceSession::Terminal(term)) = sessions.iter().find(|s| s.id() == sid) else {
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

    let conn_type = term.conn_type;
    let saved_conn_id = term.saved_conn_id.clone();
    let repaint = term.handle.repaint.clone();
    let osc_cwd = term.terminal.screen.cwd.clone();
    let semantic_seq = term.terminal.screen.semantic.mark_seq;
    let metrics = term.metrics.clone();
    let session_sftp = term.session_sftp.clone();

    match conn_type {
        ConnectionType::Local => {
            // Prefer OSC 7; else /proc/<shell>/cwd; else $HOME.
            if state.tracked_session.as_deref() != Some(sid) {
                state.last_semantic_seq = 0;
            }
            let proc_cwd = local_shell_cwd(term.handle.shell_pid);
            let cwd = osc_cwd
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .or(proc_cwd);
            if let Some(ref cwd) = cwd {
                state.follow_shell_integration(sid, Some(cwd), semantic_seq);
            } else if state.tracked_session.as_deref() != Some(sid)
                || state.tracked_cwd.is_none()
            {
                let home = crate::fs::home_dir().to_string_lossy().into_owned();
                state.on_session_cwd(sid, &home);
                state.follow_shell_integration(sid, None, semantic_seq);
            } else {
                state.follow_shell_integration(sid, None, semantic_seq);
            }
            // Poll /proc cwd each frame so `cd` is reflected without OSC.
            if state.browse_cwd.is_none() {
                if let Some(cwd) = local_shell_cwd(term.handle.shell_pid) {
                    state.on_session_cwd(sid, &cwd);
                }
            }
            if state.loading {
                if let Some(cwd) = state.effective_cwd().map(str::to_string) {
                    match local::list_dir(Path::new(&cwd)) {
                        Ok(entries) => {
                            state.entries = entries;
                            state.error = None;
                        }
                        Err(e) => {
                            state.entries.clear();
                            state.error = Some(e);
                        }
                    }
                    state.loading = false;
                }
            }
            // Keep listing fresh while following the shell.
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(500));
        }
        ConnectionType::Ssh => {
            // Agent `/proc` cwd (preferred) + OSC 7; list via shared-session SFTP.
            let session_changed = state.tracked_session.as_deref() != Some(sid);
            if session_changed {
                state.tracked_session = Some(sid.to_string());
                state.tracked_cwd = None;
                state.browse_cwd = None;
                state.last_semantic_seq = 0;
                state.pending = None;
                state.entries.clear();
                state.error = None;
                state.loading = true;
            }
            if let Some(ref cwd) = osc_cwd {
                if !cwd.is_empty() {
                    metrics.note_osc_cwd(Some(cwd));
                }
            }
            let merged_cwd = metrics.effective_cwd(osc_cwd.as_deref());
            if let Some(ref cwd) = merged_cwd {
                state.follow_shell_integration(sid, Some(cwd), semantic_seq);
            } else {
                state.follow_shell_integration(sid, None, semantic_seq);
            }
            poll_pending(state);
            if !ensure_sftp(
                ui,
                state,
                saved_conn_id.as_deref(),
                connections,
                &repaint,
                session_sftp.as_ref(),
            ) {
                return action;
            }
            // Until shell/agent cwd arrives, fall back to remote home so SFTP is usable.
            if state.tracked_cwd.is_none() && state.pending.is_none() {
                if let Some(client) = state.sftp.clone() {
                    match client.begin_home_dir() {
                        Ok(rx) => {
                            state.pending = Some(PendingOp::Home(rx));
                            state.loading = true;
                        }
                        Err(e) => state.error = Some(e),
                    }
                }
            } else if state.loading && state.pending.is_none() {
                if let (Some(cwd), Some(client)) =
                    (state.effective_cwd().map(str::to_string), state.sftp.clone())
                {
                    match client.begin_list_dir(&cwd) {
                        Ok(rx) => {
                            state.pending = Some(PendingOp::List { path: cwd, rx });
                        }
                        Err(e) => {
                            state.error = Some(e);
                            state.loading = false;
                        }
                    }
                }
            }
            poll_pending(state);
            if state.pending.is_some() {
                ui.ctx().request_repaint();
            } else {
                // Agent emits cwd about once/sec — keep Files tab live.
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(400));
            }
        }
        _ => {}
    }

    let Some(cwd_display) = state.effective_cwd().map(str::to_string) else {
        let waiting = state.pending.is_some() || state.sftp.as_ref().is_some_and(|c| c.is_connecting());
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
        &state.entries,
        state.error.as_deref(),
        state.loading || state.pending.is_some(),
        "sidebar_files_list",
    );

    if list_action.go_up {
        go_up(state, conn_type);
        state.loading = true;
        state.pending = None;
        state.entries.clear();
    }
    if let Some(idx) = list_action.open_index {
        if let Some(ent) = state.entries.get(idx).cloned() {
            if ent.is_dir {
                enter_dir(state, conn_type, &ent.name);
                state.loading = true;
                state.pending = None;
                state.entries.clear();
            }
        }
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        if !list_action.dropped_paths.is_empty() {
            handle_inbound_drop(state, conn_type, &list_action.dropped_paths);
            state.loading = true;
            state.pending = None;
        }
        if !list_action.drag_indices.is_empty() {
            let paths = drag_out_paths(state, conn_type, &list_action.drag_indices);
            let _ = crate::platform::dnd::begin_file_drag_out(&paths);
        }
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

fn poll_pending(state: &mut SidebarFilesState) {
    let Some(pending) = state.pending.take() else {
        return;
    };
    match pending {
        PendingOp::Home(rx) => match rx.try_recv() {
            Ok(Ok(home)) => {
                // Agent/OSC may have already filled cwd while home was in flight.
                if state.tracked_cwd.is_none() {
                    let sid = state.tracked_session.clone().unwrap_or_default();
                    state.on_session_cwd(&sid, &home);
                } else {
                    state.loading = true;
                }
            }
            Ok(Err(e)) => {
                log::warn!("SFTP home failed: {e}");
                if state.tracked_cwd.is_none() {
                    let sid = state.tracked_session.clone().unwrap_or_default();
                    state.on_session_cwd(&sid, "/");
                } else {
                    state.loading = true;
                }
            }
            Err(mpsc::TryRecvError::Empty) => {
                state.pending = Some(PendingOp::Home(rx));
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                state.error = Some("SFTP home request disconnected".into());
                state.loading = false;
            }
        },
        PendingOp::List { path, rx } => match rx.try_recv() {
            Ok(Ok(entries)) => {
                if state.effective_cwd() == Some(path.as_str()) {
                    state.entries = entries;
                    state.error = None;
                }
                state.loading = false;
            }
            Ok(Err(e)) => {
                state.entries.clear();
                state.error = Some(e);
                state.loading = false;
            }
            Err(mpsc::TryRecvError::Empty) => {
                state.pending = Some(PendingOp::List { path, rx });
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                state.error = Some("SFTP list request disconnected".into());
                state.loading = false;
            }
        },
    }
}

/// Returns false if an empty/error state was already painted.
fn ensure_sftp(
    ui: &mut egui::Ui,
    state: &mut SidebarFilesState,
    saved_conn_id: Option<&str>,
    connections: &[SavedConnection],
    repaint: &crate::connection::RepaintNotifier,
    shared_sftp: Option<&Arc<SftpClient>>,
) -> bool {
    let Some(conn_id) = saved_conn_id else {
        paint_empty(
            ui,
            "⚠",
            &rust_i18n::t!("sidebar_files_sftp_failed"),
            Some(&rust_i18n::t!("sidebar_files_no_saved_conn")),
        );
        return false;
    };

    if state.sftp_conn_id.as_deref() != Some(conn_id) {
        state.sftp = None;
        state.sftp_conn_id = None;
        state.pending = None;
        state.tracked_cwd = None;
    }

    if state.sftp.is_none() {
        if let Some(shared) = shared_sftp {
            state.sftp = Some(shared.clone());
            state.sftp_conn_id = Some(conn_id.to_string());
        } else {
            let Some(conn) = connections.iter().find(|c| c.id == conn_id) else {
                paint_empty(
                    ui,
                    "⚠",
                    &rust_i18n::t!("sidebar_files_sftp_failed"),
                    Some(&rust_i18n::t!("sidebar_files_no_saved_conn")),
                );
                return false;
            };
            match SftpClient::connect_with_repaint(conn, Some(repaint.clone())) {
                Ok(client) => {
                    state.sftp = Some(Arc::new(client));
                    state.sftp_conn_id = Some(conn_id.to_string());
                }
                Err(e) => {
                    paint_empty(
                        ui,
                        "⚠",
                        &rust_i18n::t!("sidebar_files_sftp_failed"),
                        Some(&e),
                    );
                    return false;
                }
            }
        }
    }

    let client = state.sftp.as_ref().unwrap();
    if let Some(err) = client.connection_error() {
        paint_empty(
            ui,
            "⚠",
            &rust_i18n::t!("sidebar_files_sftp_failed"),
            Some(&err),
        );
        return false;
    }
    if client.is_connecting() {
        paint_empty(
            ui,
            "⏳",
            &rust_i18n::t!("sidebar_files_sftp_connecting"),
            Some(&rust_i18n::t!("sidebar_files_sftp_connecting_hint")),
        );
        ui.ctx().request_repaint();
        return false;
    }
    true
}

fn enter_dir(state: &mut SidebarFilesState, conn_type: ConnectionType, name: &str) {
    let Some(cur) = state.effective_cwd().map(str::to_string) else {
        return;
    };
    let next = match conn_type {
        ConnectionType::Local => local::join_path(Path::new(&cur), name)
            .to_string_lossy()
            .into_owned(),
        ConnectionType::Ssh => join_remote(&cur, name),
        _ => return,
    };
    state.browse_cwd = Some(next);
}

fn go_up(state: &mut SidebarFilesState, conn_type: ConnectionType) {
    let Some(cur) = state.effective_cwd().map(str::to_string) else {
        return;
    };
    let parent = match conn_type {
        ConnectionType::Local => Path::new(&cur)
            .parent()
            .map(|p| p.to_string_lossy().into_owned()),
        ConnectionType::Ssh => {
            let p = Path::new(&cur);
            p.parent().map(|parent| {
                if parent.as_os_str().is_empty() {
                    "/".to_string()
                } else {
                    parent.to_string_lossy().into_owned()
                }
            })
        }
        _ => None,
    };
    if let Some(p) = parent {
        state.browse_cwd = Some(p);
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn handle_inbound_drop(state: &mut SidebarFilesState, conn_type: ConnectionType, paths: &[PathBuf]) {
    let Some(cwd) = state.effective_cwd().map(str::to_string) else {
        return;
    };
    match conn_type {
        ConnectionType::Local => {
            for src in paths {
                let dest = crate::platform::dnd::dest_path(Path::new(&cwd), src);
                if src.is_dir() {
                    let _ = copy_dir_recursive(src, &dest);
                } else if let Err(e) = std::fs::copy(src, &dest) {
                    state.error = Some(e.to_string());
                }
            }
        }
        ConnectionType::Ssh => {
            let Some(client) = state.sftp.clone() else {
                return;
            };
            for src in paths {
                let name = src
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "dropped".into());
                let remote = join_remote(&cwd, &name);
                if let Err(e) = client.upload(src, &remote) {
                    state.error = Some(e);
                }
            }
        }
        _ => {}
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let to = dest.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn drag_out_paths(
    state: &SidebarFilesState,
    conn_type: ConnectionType,
    indices: &[usize],
) -> Vec<PathBuf> {
    let Some(cwd) = state.effective_cwd() else {
        return Vec::new();
    };
    match conn_type {
        ConnectionType::Local => indices
            .iter()
            .filter_map(|&i| state.entries.get(i))
            .map(|e| local::join_path(Path::new(cwd), &e.name))
            .collect(),
        ConnectionType::Ssh => indices
            .iter()
            .filter_map(|&i| state.entries.get(i))
            .map(|e| PathBuf::from(join_remote(cwd, &e.name)))
            .collect(),
        _ => Vec::new(),
    }
}

/// Read the local PTY shell's cwd via `/proc` (Linux) when OSC 7 is unavailable.
fn local_shell_cwd(pid: Option<u32>) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let pid = pid?;
        let path = std::fs::read_link(format!("/proc/{pid}/cwd")).ok()?;
        let s = path.to_string_lossy().into_owned();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        None
    }
}
