//! Desktop external drag-and-drop helpers (Linux / Windows).
//!
//! Inbound drops are handled via egui `raw.dropped_files`.
//! Outbound file drag uses best-effort OS integration; when unavailable the
//! caller may still copy paths to the clipboard as a fallback.

use std::path::{Path, PathBuf};

/// Whether this build supports external file DnD.
pub fn external_dnd_supported() -> bool {
    cfg!(any(target_os = "linux", target_os = "windows"))
}

/// Begin dragging local files out of the application (best-effort).
///
/// egui/winit do not expose a full outbound file-drag API yet. On supported
/// desktops we copy absolute paths to the system clipboard so the user can
/// paste into a file manager; returns `true` when clipboard was updated.
#[allow(unused_variables)]
pub fn begin_file_drag_out(paths: &[PathBuf]) -> bool {
    if paths.is_empty() || !external_dnd_supported() {
        return false;
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        let text = paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        crate::ui::uiframe::clipboard::write_text(&text)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        false
    }
}

/// Resolve dropped egui files into concrete paths.
pub fn paths_from_dropped(files: &[egui::DroppedFile]) -> Vec<PathBuf> {
    files.iter().filter_map(|f| f.path.clone()).collect()
}

/// Join a dropped file name onto a destination directory.
pub fn dest_path(dir: &Path, source: &Path) -> PathBuf {
    let name = source
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_else(|| std::ffi::OsString::from("dropped"));
    dir.join(name)
}
