//! Per-session sidebar file listing cache.
//!
//! Each [`crate::session::terminal::ActiveSession`] owns a cache. List fetches are driven by
//! cwd / prompt-mark changes (and SFTP replies), not by the Files tab painting.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;

use crate::data::persist::types::{ConnectionType, SavedConnection};
use crate::fs::FileEntry;
use crate::fs::local;
use crate::fs::sftp::{SftpClient, join_remote};
use crate::session::terminal::ActiveSession;
use crate::session::workspace::WorkspaceSession;

enum PendingOp {
    Home(mpsc::Receiver<Result<String, String>>),
    List {
        path: String,
        rx: mpsc::Receiver<Result<Vec<FileEntry>, String>>,
    },
}

/// Reactive file listing state for one terminal session.
#[derive(Default)]
pub struct SessionFilesCache {
    /// Shell / agent cwd being followed when [`Self::browse_cwd`] is `None`.
    tracked_cwd: Option<String>,
    /// Manual browse override (enter dir / go up). Cleared when shell cwd changes.
    browse_cwd: Option<String>,
    last_semantic_seq: u64,
    entries: Vec<FileEntry>,
    error: Option<String>,
    loading: bool,
    pending: Option<PendingOp>,
    /// Bumped when listing contents change — UI can detect updates.
    pub generation: u64,
}

impl SessionFilesCache {
    pub fn effective_cwd(&self) -> Option<&str> {
        self.browse_cwd.as_deref().or(self.tracked_cwd.as_deref())
    }

    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn is_busy(&self) -> bool {
        self.loading || self.pending.is_some()
    }

    pub fn is_waiting_cwd(&self) -> bool {
        self.effective_cwd().is_none()
    }

    /// Drop in-flight work (e.g. after reconnect replaces the SFTP client).
    pub fn invalidate_pending(&mut self) {
        self.pending = None;
        self.loading = true;
        self.error = None;
    }

    fn request_reload(&mut self) {
        self.loading = true;
        self.pending = None;
        self.error = None;
        // Keep stale `entries` until the new list arrives so focus switches stay instant.
    }

    fn set_shell_cwd(&mut self, cwd: &str) {
        if self.tracked_cwd.as_deref() != Some(cwd) {
            self.tracked_cwd = Some(cwd.to_string());
            self.browse_cwd = None;
            self.request_reload();
        }
    }

    fn note_semantic_mark(&mut self, mark_seq: u64) {
        if self.last_semantic_seq == mark_seq {
            return;
        }
        self.last_semantic_seq = mark_seq;
        if self.browse_cwd.is_none() && self.tracked_cwd.is_some() {
            self.request_reload();
        }
    }

    pub fn enter_dir(&mut self, conn_type: ConnectionType, name: &str) {
        let Some(cur) = self.effective_cwd().map(str::to_string) else {
            return;
        };
        let next = match conn_type {
            ConnectionType::Local => local::join_path(Path::new(&cur), name)
                .to_string_lossy()
                .into_owned(),
            ConnectionType::Ssh => join_remote(&cur, name),
            _ => return,
        };
        self.browse_cwd = Some(next);
        self.request_reload();
        self.entries.clear();
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn go_up(&mut self, conn_type: ConnectionType) {
        let Some(cur) = self.effective_cwd().map(str::to_string) else {
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
            self.browse_cwd = Some(p);
            self.request_reload();
            self.entries.clear();
            self.generation = self.generation.wrapping_add(1);
        }
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    pub fn handle_inbound_drop(
        &mut self,
        conn_type: ConnectionType,
        sftp: Option<&Arc<SftpClient>>,
        paths: &[PathBuf],
    ) {
        let Some(cwd) = self.effective_cwd().map(str::to_string) else {
            return;
        };
        match conn_type {
            ConnectionType::Local => {
                for src in paths {
                    let dest = crate::platform::dnd::dest_path(Path::new(&cwd), src);
                    if src.is_dir() {
                        let _ = copy_dir_recursive(src, &dest);
                    } else if let Err(e) = std::fs::copy(src, &dest) {
                        self.error = Some(e.to_string());
                    }
                }
            }
            ConnectionType::Ssh => {
                let Some(client) = sftp else {
                    return;
                };
                for src in paths {
                    let name = src
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "dropped".into());
                    let remote = join_remote(&cwd, &name);
                    if let Err(e) = client.upload(src, &remote) {
                        self.error = Some(e);
                    }
                }
            }
            _ => {}
        }
        self.request_reload();
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    pub fn drag_out_paths(&self, conn_type: ConnectionType, indices: &[usize]) -> Vec<PathBuf> {
        let Some(cwd) = self.effective_cwd() else {
            return Vec::new();
        };
        match conn_type {
            ConnectionType::Local => indices
                .iter()
                .filter_map(|&i| self.entries.get(i))
                .map(|e| local::join_path(Path::new(cwd), &e.name))
                .collect(),
            ConnectionType::Ssh => indices
                .iter()
                .filter_map(|&i| self.entries.get(i))
                .map(|e| PathBuf::from(join_remote(cwd, &e.name)))
                .collect(),
            _ => Vec::new(),
        }
    }
}

/// Advance listing state from session signals (cwd, marks, SFTP replies).
///
/// Call from the session drain path so caches stay hot even when the Files tab is hidden.
pub fn tick_session_files(
    session: &mut ActiveSession,
    connections: &[SavedConnection],
    auth_users: &[crate::data::persist::types::AuthUser],
) {
    match session.core.conn_type {
        ConnectionType::Local => tick_local(session),
        ConnectionType::Ssh => tick_ssh(session, connections, auth_users),
        ConnectionType::Serial | ConnectionType::Ble => {}
    }
}

fn tick_local(session: &mut ActiveSession) {
    let cache = &mut session.core.files;
    let osc = session.core.terminal.screen.cwd.clone();
    let mark_seq = session.core.terminal.screen.semantic.mark_seq;
    let proc_cwd = local_shell_cwd(session.core.handle.shell_pid);

    let cwd = osc
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or(proc_cwd.clone());

    if let Some(ref cwd) = cwd {
        cache.set_shell_cwd(cwd);
    } else if cache.tracked_cwd.is_none() {
        let home = crate::fs::home_dir().to_string_lossy().into_owned();
        cache.set_shell_cwd(&home);
    }

    // Prefer live /proc while following the shell (no manual browse).
    if cache.browse_cwd.is_none() {
        if let Some(cwd) = local_shell_cwd(session.core.handle.shell_pid) {
            cache.set_shell_cwd(&cwd);
        }
    }

    cache.note_semantic_mark(mark_seq);

    if cache.loading {
        if let Some(cwd) = cache.effective_cwd().map(str::to_string) {
            match local::list_dir(Path::new(&cwd)) {
                Ok(entries) => {
                    cache.entries = entries;
                    cache.error = None;
                    cache.generation = cache.generation.wrapping_add(1);
                }
                Err(e) => {
                    cache.entries.clear();
                    cache.error = Some(e);
                    cache.generation = cache.generation.wrapping_add(1);
                }
            }
            cache.loading = false;
        }
    }
}

fn tick_ssh(
    session: &mut ActiveSession,
    connections: &[SavedConnection],
    auth_users: &[crate::data::persist::types::AuthUser],
) {
    if let Some(cwd) = session.core.terminal.screen.cwd.as_deref() {
        if !cwd.is_empty() {
            session.core.metrics.note_osc_cwd(Some(cwd));
        }
    }

    let osc = session.core.terminal.screen.cwd.clone();
    let mark_seq = session.core.terminal.screen.semantic.mark_seq;
    let merged = session.core.metrics.effective_cwd(osc.as_deref());

    {
        let cache = &mut session.core.files;
        if let Some(ref cwd) = merged {
            cache.set_shell_cwd(cwd);
        }
        cache.note_semantic_mark(mark_seq);
        poll_pending(cache);
    }

    ensure_session_sftp(session, connections, auth_users);

    let Some(client) = session.core.session_sftp.clone() else {
        return;
    };
    if client.connection_error().is_some() || client.is_connecting() {
        return;
    }

    let cache = &mut session.core.files;
    if cache.tracked_cwd.is_none() && cache.pending.is_none() {
        match client.begin_home_dir() {
            Ok(rx) => {
                cache.pending = Some(PendingOp::Home(rx));
                cache.loading = true;
            }
            Err(e) => cache.error = Some(e),
        }
    } else if cache.loading && cache.pending.is_none() {
        if let Some(cwd) = cache.effective_cwd().map(str::to_string) {
            match client.begin_list_dir(&cwd) {
                Ok(rx) => {
                    cache.pending = Some(PendingOp::List { path: cwd, rx });
                }
                Err(e) => {
                    cache.error = Some(e);
                    cache.loading = false;
                }
            }
        }
    }
    poll_pending(cache);
}

fn ensure_session_sftp(
    session: &mut ActiveSession,
    connections: &[SavedConnection],
    auth_users: &[crate::data::persist::types::AuthUser],
) {
    if session.core.session_sftp.is_some() {
        return;
    }
    let Some(conn_id) = session.core.saved_conn_id.as_deref() else {
        return;
    };
    let Some(conn) = connections.iter().find(|c| c.id == conn_id) else {
        return;
    };
    let auth_user = conn
        .auth_user_id
        .as_ref()
        .and_then(|id| auth_users.iter().find(|u| u.id == *id));
    let auth = crate::app::connect_params::ssh_auth(conn, auth_user);
    let Some(host) = conn.ssh_host.clone() else {
        session.core.files.error = Some("SSH host not configured".into());
        session.core.files.loading = false;
        return;
    };
    let port = conn.ssh_port.unwrap_or(22);
    let repaint = session.core.handle.repaint.clone();
    match SftpClient::connect(host, port, auth, Some(repaint)) {
        Ok(client) => {
            session.core.session_sftp = Some(Arc::new(client));
            session.core.files.invalidate_pending();
        }
        Err(e) => {
            session.core.files.error = Some(e);
            session.core.files.loading = false;
        }
    }
}

fn poll_pending(cache: &mut SessionFilesCache) {
    let Some(pending) = cache.pending.take() else {
        return;
    };
    match pending {
        PendingOp::Home(rx) => match rx.try_recv() {
            Ok(Ok(home)) => {
                if cache.tracked_cwd.is_none() {
                    cache.set_shell_cwd(&home);
                } else {
                    cache.request_reload();
                }
            }
            Ok(Err(e)) => {
                log::warn!("SFTP home failed: {e}");
                if cache.tracked_cwd.is_none() {
                    cache.set_shell_cwd("/");
                } else {
                    cache.request_reload();
                }
            }
            Err(mpsc::TryRecvError::Empty) => {
                cache.pending = Some(PendingOp::Home(rx));
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                cache.error = Some("SFTP home request disconnected".into());
                cache.loading = false;
            }
        },
        PendingOp::List { path, rx } => match rx.try_recv() {
            Ok(Ok(entries)) => {
                if cache.effective_cwd() == Some(path.as_str()) {
                    cache.entries = entries;
                    cache.error = None;
                    cache.generation = cache.generation.wrapping_add(1);
                }
                cache.loading = false;
            }
            Ok(Err(e)) => {
                cache.entries.clear();
                cache.error = Some(e);
                cache.loading = false;
                cache.generation = cache.generation.wrapping_add(1);
            }
            Err(mpsc::TryRecvError::Empty) => {
                cache.pending = Some(PendingOp::List { path, rx });
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                cache.error = Some("SFTP list request disconnected".into());
                cache.loading = false;
            }
        },
    }
}

fn local_shell_cwd(pid: Option<u32>) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let pid = pid?;
        let path = std::fs::read_link(format!("/proc/{pid}/cwd")).ok()?;
        let s = path.to_string_lossy().into_owned();
        if s.is_empty() { None } else { Some(s) }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        None
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

/// Keep every terminal session's file cache in sync with cwd / SFTP replies.
pub fn tick_all_session_files(
    sessions: &mut [WorkspaceSession],
    connections: &[SavedConnection],
    auth_users: &[crate::data::persist::types::AuthUser],
) {
    for session in sessions {
        if let Some(term) = session.terminal_mut() {
            tick_session_files(term, connections, auth_users);
        }
    }
}
