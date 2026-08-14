use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::fs::entry_info;
use crate::fs::local;
use crate::fs::sftp::join_remote;
use crate::session::{
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
    if session.transfer.is_active() {
        session.status = Some("Transfer already in progress".into());
        return;
    }
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
                    remote.entries = entries;
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
            pane.entries = entries;
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

fn parent_local(pane: &mut FilePaneState) {
    if let Some(parent) = pane.cwd.parent() {
        pane.cwd = parent.to_path_buf();
        pane.selected.clear();
        pane.focus_index = None;
    }
}

fn parent_remote(remote: &mut RemotePane) {
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
