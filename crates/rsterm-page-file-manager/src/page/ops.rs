use std::path::{Path, PathBuf};
use std::sync::Arc;

use rsterm_fs::entry_info;
use rsterm_fs::local;
use rsterm_fs::sftp::join_remote;
use rsterm_session_core::{
    FileActivePane, FileClipboard, FileClipboardMode, FileManagerMode, FileManagerSession,
    FilePaneState, InfoDialog, RemotePane, RenameDialog,
};

use super::PaneOps;
use super::list::{dismiss_multiselect_local, dismiss_multiselect_remote};
use super::transfer::PasteTarget;

fn opposite_pane(active: FileActivePane, mode: FileManagerMode) -> FileActivePane {
    match mode {
        FileManagerMode::SshSftp => match active {
            FileActivePane::Remote => FileActivePane::Right,
            _ => FileActivePane::Remote,
        },
        FileManagerMode::LocalDual => match active {
            FileActivePane::LeftLocal => FileActivePane::Right,
            _ => FileActivePane::LeftLocal,
        },
    }
}

pub(super) fn transfer_to_opposite_pane(session: &mut FileManagerSession) {
    let active = session.active_pane;
    copy_from_pane(session, active);
    let dest = opposite_pane(active, session.mode);
    paste_into_pane(session, dest);
    session.status = Some("Transferred to opposite pane".into());
}

fn copy_from_pane(session: &mut FileManagerSession, pane: FileActivePane) {
    let clip = match pane {
        FileActivePane::Remote => session.remote.as_ref().map(|remote| {
            let paths = selected_remote_paths(remote);
            (paths, true)
        }),
        FileActivePane::LeftLocal => session.left_local.as_ref().map(|left| {
            let paths: Vec<String> = selected_local_paths(left)
                .into_iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            (paths, false)
        }),
        FileActivePane::Right => {
            let paths: Vec<String> = selected_local_paths(&session.right)
                .into_iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            Some((paths, false))
        }
    };
    if let Some((paths, from_remote)) = clip.filter(|(p, _)| !p.is_empty()) {
        session.clipboard = Some(FileClipboard {
            mode: FileClipboardMode::Copy,
            from_remote,
            paths,
        });
    }
}

pub(super) fn paste_into_pane(session: &mut FileManagerSession, pane: FileActivePane) {
    let Some(clip) = session.clipboard.clone() else {
        session.status = Some("Clipboard is empty".into());
        return;
    };
    let was_busy = session.transfer.is_active() || session.transfer.join.is_some();
    let remote_client = session.remote.as_ref().map(|r| Arc::clone(&r.client));
    match pane {
        FileActivePane::Remote => {
            let Some(remote) = session.remote.as_ref() else {
                return;
            };
            session.transfer.start_paste(
                PasteTarget::Remote,
                clip,
                None,
                Some(remote.cwd.clone()),
                remote_client,
            );
        }
        FileActivePane::LeftLocal => {
            let Some(left) = session.left_local.as_ref() else {
                return;
            };
            session.transfer.start_paste(
                PasteTarget::LocalLeft,
                clip,
                Some(left.cwd.clone()),
                None,
                remote_client,
            );
        }
        FileActivePane::Right => {
            session.transfer.start_paste(
                PasteTarget::LocalRight,
                clip,
                Some(session.right.cwd.clone()),
                None,
                remote_client,
            );
        }
    }
    if was_busy {
        session.status = Some(crate::labels::labels().queued);
    }
}

pub(super) fn refresh_if_needed(session: &mut FileManagerSession) {
    if let Some(remote) = session.remote.as_mut()
        && remote.loading
    {
        if let Some(err) = remote.client.connection_error() {
            remote.loading = false;
            remote.error = Some(err);
        } else if remote.client.is_connected() {
            remote.loading = false;
            match remote.client.list_dir(&remote.cwd) {
                Ok(entries) => {
                    remote.apply_listing(entries);
                    remote.error = None;
                }
                Err(e) => remote.error = Some(e),
            }
        }
    }
    if let Some(left) = session.left_local.as_mut() {
        refresh_local_pane(left);
    }
    refresh_local_pane(&mut session.right);
}

fn refresh_local_pane(pane: &mut FilePaneState) {
    if !pane.loading {
        return;
    }
    pane.loading = false;
    match local::list_dir(&pane.cwd) {
        Ok(entries) => {
            pane.apply_listing(entries);
            pane.error = None;
        }
        Err(e) => pane.error = Some(e),
    }
}

pub(super) fn run_local_ops(
    pane: &mut FilePaneState,
    pane_side: FileActivePane,
    clipboard: &mut Option<FileClipboard>,
    status: &mut Option<String>,
    rename_dialog: &mut RenameDialog,
    info_dialog: &mut InfoDialog,
    ops: &mut PaneOps,
) {
    if ops.go_up {
        parent_local(pane);
        pane.loading = true;
    }
    if let Some(i) = ops.open_index.take() {
        open_local_entry(pane, i);
    }
    if let Some(indices) = ops.bulk_copy.take() {
        copy_local_indices(pane, &indices, clipboard, status);
    }
    if let Some(indices) = ops.bulk_cut.take() {
        cut_local_indices(pane, &indices, clipboard, status);
    }
    if let Some(indices) = ops.bulk_delete.take() {
        delete_local_indices(pane, &indices, status);
    }
    if let Some(idx) = ops.rename_index.take()
        && let Some(ent) = pane.entries.get(idx)
    {
        rename_dialog.open_for(pane_side, &ent.name);
    }
    if let Some(idx) = ops.info_index.take()
        && let Some(ent) = pane.entries.get(idx)
    {
        let path = local::join_path(&pane.cwd, &ent.name);
        match entry_info::local_entry_info(&path) {
            Ok(info) => info_dialog.show(info),
            Err(e) => *status = Some(e),
        }
    }
    if ops.dismiss_multiselect {
        dismiss_multiselect_local(pane);
    }
}

pub(super) fn run_remote_ops(
    remote: &mut RemotePane,
    clipboard: &mut Option<FileClipboard>,
    status: &mut Option<String>,
    rename_dialog: &mut RenameDialog,
    info_dialog: &mut InfoDialog,
    ops: &mut PaneOps,
) {
    if ops.go_up {
        parent_remote(remote);
        remote.loading = true;
    }
    if let Some(i) = ops.open_index.take() {
        open_remote_entry(remote, i);
    }
    if let Some(indices) = ops.bulk_copy.take() {
        copy_remote_indices(remote, &indices, clipboard, status);
    }
    if let Some(indices) = ops.bulk_cut.take() {
        cut_remote_indices(remote, &indices, clipboard, status);
    }
    if let Some(indices) = ops.bulk_delete.take() {
        delete_remote_indices(remote, &indices, status);
    }
    if let Some(idx) = ops.rename_index.take()
        && let Some(ent) = remote.entries.get(idx)
    {
        rename_dialog.open_for(FileActivePane::Remote, &ent.name);
    }
    if let Some(idx) = ops.info_index.take()
        && let Some(ent) = remote.entries.get(idx)
    {
        let path = join_remote(&remote.cwd, &ent.name);
        match remote.client.entry_info(&path) {
            Ok(info) => info_dialog.show(info),
            Err(e) => *status = Some(e),
        }
    }
    if ops.dismiss_multiselect {
        dismiss_multiselect_remote(remote);
    }
}

fn open_local_entry(pane: &mut FilePaneState, idx: usize) {
    let Some(ent) = pane.entries.get(idx) else {
        return;
    };
    if ent.is_dir {
        pane.cwd = local::join_path(&pane.cwd, &ent.name);
        pane.loading = true;
        pane.selected.clear();
        pane.focus_index = None;
    }
}

fn open_remote_entry(remote: &mut RemotePane, idx: usize) {
    let Some(ent) = remote.entries.get(idx) else {
        return;
    };
    if ent.is_dir {
        remote.cwd = join_remote(&remote.cwd, &ent.name);
        remote.loading = true;
        remote.selected.clear();
        remote.focus_index = None;
    }
}

pub(super) fn parent_local(pane: &mut FilePaneState) {
    if let Some(parent) = pane.cwd.parent() {
        pane.cwd = parent.to_path_buf();
        pane.selected.clear();
        pane.focus_index = None;
    }
}

pub(super) fn parent_remote(remote: &mut RemotePane) {
    let p = Path::new(&remote.cwd);
    if let Some(parent) = p.parent() {
        remote.cwd = if parent.as_os_str().is_empty() {
            "/".to_string()
        } else {
            parent.to_string_lossy().into_owned()
        };
        remote.selected.clear();
        remote.focus_index = None;
    }
}

/// Navigate active pane one level up and reload.
pub(super) fn go_up_active_pane(session: &mut FileManagerSession) {
    match session.active_pane {
        FileActivePane::Remote => {
            if let Some(remote) = session.remote.as_mut() {
                parent_remote(remote);
                remote.loading = true;
            }
        }
        FileActivePane::LeftLocal => {
            if let Some(left) = session.left_local.as_mut() {
                parent_local(left);
                left.loading = true;
            }
        }
        FileActivePane::Right => {
            parent_local(&mut session.right);
            session.right.loading = true;
        }
    }
}

/// Recompute listing for the focused pane after filter/sort changes.
pub(super) fn recompute_active_pane(session: &mut FileManagerSession) {
    match session.active_pane {
        FileActivePane::Remote => {
            if let Some(remote) = session.remote.as_mut() {
                remote.recompute();
            }
        }
        FileActivePane::LeftLocal => {
            if let Some(left) = session.left_local.as_mut() {
                left.recompute();
            }
        }
        FileActivePane::Right => session.right.recompute(),
    }
}

fn selected_local_paths(pane: &FilePaneState) -> Vec<PathBuf> {
    pane.selected
        .iter()
        .filter_map(|&i| pane.entries.get(i))
        .map(|e| local::join_path(&pane.cwd, &e.name))
        .collect()
}

fn selected_remote_paths(remote: &RemotePane) -> Vec<String> {
    remote
        .selected
        .iter()
        .filter_map(|&i| remote.entries.get(i))
        .map(|e| join_remote(&remote.cwd, &e.name))
        .collect()
}

fn local_paths_for_indices(pane: &FilePaneState, indices: &[usize]) -> Vec<String> {
    indices
        .iter()
        .filter_map(|&i| pane.entries.get(i))
        .map(|e| {
            local::join_path(&pane.cwd, &e.name)
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

fn remote_paths_for_indices(remote: &RemotePane, indices: &[usize]) -> Vec<String> {
    indices
        .iter()
        .filter_map(|&i| remote.entries.get(i))
        .map(|e| join_remote(&remote.cwd, &e.name))
        .collect()
}

fn cut_local_indices(
    pane: &FilePaneState,
    indices: &[usize],
    clipboard: &mut Option<FileClipboard>,
    status: &mut Option<String>,
) {
    let paths = local_paths_for_indices(pane, indices);
    if paths.is_empty() {
        *status = Some("No items".into());
        return;
    }
    let n = paths.len();
    *clipboard = Some(FileClipboard {
        mode: FileClipboardMode::Cut,
        from_remote: false,
        paths,
    });
    *status = Some(format!("Cut {n} item(s)"));
}

fn copy_local_indices(
    pane: &FilePaneState,
    indices: &[usize],
    clipboard: &mut Option<FileClipboard>,
    status: &mut Option<String>,
) {
    let paths = local_paths_for_indices(pane, indices);
    if paths.is_empty() {
        *status = Some("No items".into());
        return;
    }
    let n = paths.len();
    *clipboard = Some(FileClipboard {
        mode: FileClipboardMode::Copy,
        from_remote: false,
        paths,
    });
    *status = Some(format!("Copied {n} item(s)"));
}

fn cut_remote_indices(
    remote: &RemotePane,
    indices: &[usize],
    clipboard: &mut Option<FileClipboard>,
    status: &mut Option<String>,
) {
    let paths = remote_paths_for_indices(remote, indices);
    if paths.is_empty() {
        *status = Some("No items".into());
        return;
    }
    let n = paths.len();
    *clipboard = Some(FileClipboard {
        mode: FileClipboardMode::Cut,
        from_remote: true,
        paths,
    });
    *status = Some(format!("Cut {n} item(s)"));
}

fn copy_remote_indices(
    remote: &RemotePane,
    indices: &[usize],
    clipboard: &mut Option<FileClipboard>,
    status: &mut Option<String>,
) {
    let paths = remote_paths_for_indices(remote, indices);
    if paths.is_empty() {
        *status = Some("No items".into());
        return;
    }
    let n = paths.len();
    *clipboard = Some(FileClipboard {
        mode: FileClipboardMode::Copy,
        from_remote: true,
        paths,
    });
    *status = Some(format!("Copied {n} item(s)"));
}

fn delete_local_indices(pane: &mut FilePaneState, indices: &[usize], status: &mut Option<String>) {
    let paths: Vec<PathBuf> = indices
        .iter()
        .filter_map(|&i| pane.entries.get(i))
        .map(|e| local::join_path(&pane.cwd, &e.name))
        .collect();
    if paths.is_empty() {
        *status = Some("No items".into());
        return;
    }
    let mut errors = Vec::new();
    for p in &paths {
        if let Err(e) = local::remove_path(p) {
            errors.push(e);
        }
    }
    if errors.is_empty() {
        *status = Some(format!("Deleted {} item(s)", paths.len()));
        pane.loading = true;
    } else {
        *status = Some(errors.join("; "));
    }
}

fn delete_remote_indices(remote: &mut RemotePane, indices: &[usize], status: &mut Option<String>) {
    let paths = remote_paths_for_indices(remote, indices);
    if paths.is_empty() {
        *status = Some("No items".into());
        return;
    }
    let mut errors = Vec::new();
    for path in &paths {
        let is_dir = remote
            .entries
            .iter()
            .filter(|e| e.is_dir)
            .any(|e| join_remote(&remote.cwd, &e.name) == *path);
        let err = if is_dir {
            remote.client.remove(path, true)
        } else {
            remote.client.remove(path, false)
        };
        if let Err(e) = err {
            errors.push(e);
        }
    }
    if errors.is_empty() {
        *status = Some(format!("Deleted {} item(s)", paths.len()));
        remote.loading = true;
    } else {
        *status = Some(errors.join("; "));
    }
}

const RECURSIVE_SEARCH_MAX: usize = 5000;

/// Start or restart recursive name search for the active pane.
pub(super) fn kick_recursive_search(session: &mut FileManagerSession) {
    if let Some(prev) = session.recursive_search.as_ref() {
        prev.request_cancel();
    }
    session.recursive_search = None;

    let pane = session.active_pane;
    let filter = match pane {
        FileActivePane::Remote => session.remote.as_ref().map(|r| r.listing_filter()),
        FileActivePane::LeftLocal => session.left_local.as_ref().map(|p| p.listing_filter()),
        FileActivePane::Right => Some(session.right.listing_filter()),
    };
    let Some(filter) = filter else {
        return;
    };
    if filter.query.trim().is_empty() || !match_recursive_enabled(session, pane) {
        // Fall back to in-memory recompute.
        recompute_active_pane(session);
        return;
    }

    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};
    use std::thread;

    use rsterm_session_core::{RecursiveSearchState, sort_entries};

    let cancel = Arc::new(AtomicBool::new(false));
    let results: Arc<Mutex<Option<rsterm_session_core::RecursiveSearchResult>>> =
        Arc::new(Mutex::new(None));
    let cancel_t = Arc::clone(&cancel);
    let results_t = Arc::clone(&results);

    let join = match pane {
        FileActivePane::Remote => {
            let Some(remote) = session.remote.as_ref() else {
                return;
            };
            let client = Arc::clone(&remote.client);
            let root = remote.cwd.clone();
            let sort_key = remote.sort_key;
            let sort_asc = remote.sort_asc;
            thread::spawn(move || {
                let out = walk_remote_recursive(
                    &client,
                    &root,
                    &root,
                    &filter,
                    &cancel_t,
                    RECURSIVE_SEARCH_MAX,
                );
                let out = out.map(|mut v| {
                    sort_entries(&mut v, sort_key, sort_asc);
                    v
                });
                if let Ok(mut g) = results_t.lock() {
                    *g = Some(out);
                }
            })
        }
        FileActivePane::LeftLocal | FileActivePane::Right => {
            let (root, sort_key, sort_asc) = match pane {
                FileActivePane::LeftLocal => {
                    let p = session.left_local.as_ref().unwrap();
                    (p.cwd.clone(), p.sort_key, p.sort_asc)
                }
                _ => {
                    let p = &session.right;
                    (p.cwd.clone(), p.sort_key, p.sort_asc)
                }
            };
            thread::spawn(move || {
                let out =
                    walk_local_recursive(&root, &root, &filter, &cancel_t, RECURSIVE_SEARCH_MAX);
                let out = out.map(|mut v| {
                    sort_entries(&mut v, sort_key, sort_asc);
                    v
                });
                if let Ok(mut g) = results_t.lock() {
                    *g = Some(out);
                }
            })
        }
    };

    session.recursive_search = Some(RecursiveSearchState {
        pane,
        cancel,
        results,
        join: Some(join),
    });
    session.status = Some("Searching…".into());
}

fn match_recursive_enabled(session: &FileManagerSession, pane: FileActivePane) -> bool {
    match pane {
        FileActivePane::Remote => session
            .remote
            .as_ref()
            .map(|r| r.filter_recursive)
            .unwrap_or(false),
        FileActivePane::LeftLocal => session
            .left_local
            .as_ref()
            .map(|p| p.filter_recursive)
            .unwrap_or(false),
        FileActivePane::Right => session.right.filter_recursive,
    }
}

/// Poll recursive search completion and apply results.
pub(super) fn poll_recursive_search(session: &mut FileManagerSession) {
    let done = session
        .recursive_search
        .as_mut()
        .and_then(|s| s.take_if_done().map(|r| (s.pane, r)));
    let Some((pane, result)) = done else {
        return;
    };
    session.recursive_search = None;
    match result {
        Ok(entries) => {
            session.status = Some(format!("Found {} item(s)", entries.len()));
            match pane {
                FileActivePane::Remote => {
                    if let Some(remote) = session.remote.as_mut() {
                        remote.entries = entries;
                        remote.selected.clear();
                        remote.focus_index = None;
                    }
                }
                FileActivePane::LeftLocal => {
                    if let Some(left) = session.left_local.as_mut() {
                        left.entries = entries;
                        left.selected.clear();
                        left.focus_index = None;
                    }
                }
                FileActivePane::Right => {
                    session.right.entries = entries;
                    session.right.selected.clear();
                    session.right.focus_index = None;
                }
            }
        }
        Err(e) => {
            session.status = Some(e);
            recompute_active_pane(session);
        }
    }
}

pub(super) fn cancel_recursive_search(session: &mut FileManagerSession) {
    if let Some(s) = session.recursive_search.as_ref() {
        s.request_cancel();
    }
}

fn walk_local_recursive(
    root: &Path,
    dir: &Path,
    filter: &rsterm_session_core::ListingFilter,
    cancel: &std::sync::atomic::AtomicBool,
    budget: usize,
) -> Result<Vec<rsterm_fs::FileEntry>, String> {
    use rsterm_session_core::name_matches;
    use std::sync::atomic::Ordering;

    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            return Err("Search cancelled".into());
        }
        if out.len() >= budget {
            break;
        }
        let entries = local::list_dir(&current)?;
        for e in entries {
            if cancel.load(Ordering::Relaxed) {
                return Err("Search cancelled".into());
            }
            if !filter.show_hidden && e.name.starts_with('.') {
                continue;
            }
            let full = current.join(&e.name);
            let rel = full
                .strip_prefix(root)
                .unwrap_or(full.as_path())
                .to_string_lossy()
                .replace('\\', "/");
            if name_matches(&e.name, filter) {
                out.push(rsterm_fs::FileEntry {
                    name: rel.clone(),
                    is_dir: e.is_dir,
                    size: e.size,
                    modified: e.modified,
                });
                if out.len() >= budget {
                    break;
                }
            }
            if e.is_dir {
                stack.push(full);
            }
        }
    }
    Ok(out)
}

fn walk_remote_recursive(
    client: &rsterm_fs::sftp::SftpClient,
    root: &str,
    dir: &str,
    filter: &rsterm_session_core::ListingFilter,
    cancel: &std::sync::atomic::AtomicBool,
    budget: usize,
) -> Result<Vec<rsterm_fs::FileEntry>, String> {
    use rsterm_session_core::name_matches;
    use std::sync::atomic::Ordering;

    let mut out = Vec::new();
    let mut stack = vec![dir.to_string()];
    while let Some(current) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            return Err("Search cancelled".into());
        }
        if out.len() >= budget {
            break;
        }
        let entries = client.list_dir(&current)?;
        for e in entries {
            if cancel.load(Ordering::Relaxed) {
                return Err("Search cancelled".into());
            }
            if !filter.show_hidden && e.name.starts_with('.') {
                continue;
            }
            let full = join_remote(&current, &e.name);
            let rel = full
                .strip_prefix(root.trim_end_matches('/'))
                .unwrap_or(full.as_str())
                .trim_start_matches('/')
                .to_string();
            if name_matches(&e.name, filter) {
                out.push(rsterm_fs::FileEntry {
                    name: if rel.is_empty() { e.name.clone() } else { rel },
                    is_dir: e.is_dir,
                    size: e.size,
                    modified: e.modified,
                });
                if out.len() >= budget {
                    break;
                }
            }
            if e.is_dir {
                stack.push(full);
            }
        }
    }
    Ok(out)
}

/// Submit a path from the address bar for the active pane.
pub(super) fn submit_path_active_pane(session: &mut FileManagerSession, raw: &str) {
    use super::path_bar::{apply_local_path, apply_remote_path};
    match session.active_pane {
        FileActivePane::Remote => {
            if let Some(remote) = session.remote.as_mut() {
                match apply_remote_path(remote, raw) {
                    Ok(()) => session.status = None,
                    Err(e) => {
                        session.status = Some(e);
                        remote.sync_path_edit_from_cwd();
                    }
                }
            }
        }
        FileActivePane::LeftLocal => {
            if let Some(left) = session.left_local.as_mut() {
                match apply_local_path(left, raw) {
                    Ok(()) => session.status = None,
                    Err(e) => {
                        session.status = Some(e);
                        left.sync_path_edit_from_cwd();
                    }
                }
            }
        }
        FileActivePane::Right => match apply_local_path(&mut session.right, raw) {
            Ok(()) => session.status = None,
            Err(e) => {
                session.status = Some(e);
                session.right.sync_path_edit_from_cwd();
            }
        },
    }
}
