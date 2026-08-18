use std::sync::Arc;

use rsterm_fs::local;
use rsterm_fs::sftp::join_remote;
use rsterm_session_core::{FileActivePane, FileManagerSession};

pub(super) fn apply_external_drop(
    session: &mut FileManagerSession,
    pane: FileActivePane,
    paths: &[std::path::PathBuf],
) {
    if paths.is_empty() {
        return;
    }
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        match pane {
            FileActivePane::Right => {
                let cwd = session.right.cwd.clone();
                for src in paths {
                    let dest = rsterm_platform::dnd::dest_path(&cwd, src);
                    if src.is_dir() {
                        let _ = copy_dir_recursive_fm(src, &dest);
                    } else if let Err(e) = std::fs::copy(src, &dest) {
                        session.status = Some(e.to_string());
                    }
                }
                session.right.loading = true;
            }
            FileActivePane::LeftLocal => {
                if let Some(left) = session.left_local.as_mut() {
                    let cwd = left.cwd.clone();
                    for src in paths {
                        let dest = rsterm_platform::dnd::dest_path(&cwd, src);
                        if src.is_dir() {
                            let _ = copy_dir_recursive_fm(src, &dest);
                        } else if let Err(e) = std::fs::copy(src, &dest) {
                            session.status = Some(e.to_string());
                        }
                    }
                    left.loading = true;
                }
            }
            FileActivePane::Remote => {
                if let Some(remote) = session.remote.as_ref() {
                    let cwd = remote.cwd.clone();
                    let client = Arc::clone(&remote.client);
                    for src in paths {
                        let name = src
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "dropped".into());
                        let remote_path = join_remote(&cwd, &name);
                        if let Err(e) = client.upload(src, &remote_path) {
                            session.status = Some(e);
                        }
                    }
                    if let Some(remote) = session.remote.as_mut() {
                        remote.loading = true;
                    }
                }
            }
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (session, pane, paths);
    }
}

pub(super) fn apply_external_drag_out(
    session: &FileManagerSession,
    pane: FileActivePane,
    indices: &[usize],
) {
    if indices.is_empty() {
        return;
    }
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        let paths: Vec<std::path::PathBuf> = match pane {
            FileActivePane::Right => indices
                .iter()
                .filter_map(|&i| session.right.entries.get(i))
                .map(|e| local::join_path(&session.right.cwd, &e.name))
                .collect(),
            FileActivePane::LeftLocal => session
                .left_local
                .as_ref()
                .map(|left| {
                    indices
                        .iter()
                        .filter_map(|&i| left.entries.get(i))
                        .map(|e| local::join_path(&left.cwd, &e.name))
                        .collect()
                })
                .unwrap_or_default(),
            FileActivePane::Remote => Vec::new(),
        };
        let _ = rsterm_platform::dnd::begin_file_drag_out(&paths);
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = (session, pane, indices);
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn copy_dir_recursive_fm(src: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let to = dest.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive_fm(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
