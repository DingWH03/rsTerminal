//! Single-column file / folder list used by sidebar Files and dual-pane FM.

use crate::fs::FileEntry;
use crate::ui::uiframe::style;

#[derive(Debug, Default)]
pub struct FileListAction {
    pub open_index: Option<usize>,
    pub go_up: bool,
    /// Indices of rows that started an outbound file drag (desktop).
    pub drag_indices: Vec<usize>,
    /// External files dropped onto the list (desktop inbound).
    pub dropped_paths: Vec<std::path::PathBuf>,
}

/// Paint a path header + scrollable single-column entry list.
pub struct FileListView;

impl FileListView {
    pub fn show(
        ui: &mut egui::Ui,
        cwd: &str,
        entries: &[FileEntry],
        error: Option<&str>,
        loading: bool,
        id_salt: &str,
    ) -> FileListAction {
        let mut action = FileListAction::default();

        // Path + up button
        ui.horizontal(|ui| {
            let up = ui
                .add(
                    egui::Button::new("⬆")
                        .frame(false)
                        .corner_radius(style::CORNER_RADIUS_XS),
                )
                .on_hover_text("..");
            if up.clicked() {
                action.go_up = true;
            }
            ui.label(egui::RichText::new(cwd).small().weak());
        });
        ui.add_space(2.0);
        ui.separator();
        ui.add_space(2.0);

        if let Some(err) = error {
            ui.colored_label(style::RED, err);
        }
        if loading {
            ui.label(egui::RichText::new("…").weak());
        }

        let list_resp = egui::ScrollArea::vertical()
            .id_salt(id_salt)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                for (idx, ent) in entries.iter().enumerate() {
                    let label = entry_label(ent);
                    let resp = ui.add(
                        egui::Button::new(egui::RichText::new(label).size(13.0))
                            .frame(false)
                            .corner_radius(style::CORNER_RADIUS_XS)
                            .min_size(egui::vec2(ui.available_width(), 26.0)),
                    );

                    if resp.double_clicked() || (resp.clicked() && ent.is_dir) {
                        if ent.is_dir {
                            action.open_index = Some(idx);
                        }
                    } else if resp.clicked() && !ent.is_dir {
                        // single click on file — no-op for sidebar (preview only)
                    }

                    // Outbound drag: start when the row is dragged.
                    if resp.dragged() && !ent.is_dir {
                        if !action.drag_indices.contains(&idx) {
                            action.drag_indices.push(idx);
                        }
                    }
                }
            });

        // Inbound external drops over the scroll area (desktop only).
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let rect = list_resp.inner_rect;
            let hovering = ui.rect_contains_pointer(rect);
            if hovering {
                let hovered = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
                if hovered {
                    ui.painter().rect_stroke(
                        rect,
                        style::CORNER_RADIUS_XS,
                        egui::Stroke::new(1.5, style::ACCENT),
                        egui::StrokeKind::Inside,
                    );
                }
                let dropped: Vec<_> = ui.ctx().input(|i| {
                    i.raw
                        .dropped_files
                        .iter()
                        .filter_map(|f| f.path.clone())
                        .collect()
                });
                if hovering && !dropped.is_empty() {
                    action.dropped_paths = dropped;
                }
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = list_resp;
        }

        action
    }
}

fn entry_label(ent: &FileEntry) -> String {
    let icon = if ent.is_dir { "📁" } else { "📄" };
    format!("{icon} {}", ent.name)
}
