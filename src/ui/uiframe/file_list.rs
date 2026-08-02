//! Single-column file / folder list used by sidebar Files and dual-pane FM.

use crate::fs::FileEntry;
use crate::ui::uiframe::style;
use crate::ui::uiframe::tokens;

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

        ui.horizontal(|ui| {
            ui.style_mut().spacing.item_spacing.x = tokens::space::SM;
            let up = ui
                .add(
                    egui::Button::new("↑")
                        .frame(false)
                        .corner_radius(style::CORNER_RADIUS_XS)
                        .min_size(egui::vec2(
                            tokens::size::TOOLBAR_WIDTH,
                            tokens::size::TOOLBAR_HEIGHT,
                        )),
                )
                .on_hover_text(rust_i18n::t!("parent_folder"));
            if up.clicked() {
                action.go_up = true;
            }
            ui.label(egui::RichText::new(cwd).size(tokens::text::SMALL).weak());
        });
        ui.add_space(tokens::space::XS);
        ui.add(egui::Separator::default().spacing(tokens::space::XS));

        if let Some(err) = error {
            ui.colored_label(style::RED, err);
        }
        if loading {
            ui.label(
                egui::RichText::new(rust_i18n::t!("loading"))
                    .size(tokens::text::SMALL)
                    .weak(),
            );
        }

        let list_resp = egui::ScrollArea::vertical()
            .id_salt(id_salt)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.style_mut().spacing.scroll.bar_width = 6.0;
                ui.set_min_width(ui.available_width());
                if !loading && entries.is_empty() && error.is_none() {
                    ui.label(
                        egui::RichText::new(rust_i18n::t!("empty_folder"))
                            .size(tokens::text::SMALL)
                            .weak(),
                    );
                    return;
                }
                for (idx, ent) in entries.iter().enumerate() {
                    let label = entry_label(ent);
                    let resp = ui.add(
                        egui::Button::new(egui::RichText::new(label).size(tokens::text::BODY))
                            .frame(false)
                            .corner_radius(style::CORNER_RADIUS_XS)
                            .min_size(egui::vec2(ui.available_width(), tokens::size::NAV_ROW)),
                    );

                    if resp.double_clicked() || (resp.clicked() && ent.is_dir) {
                        if ent.is_dir {
                            action.open_index = Some(idx);
                        }
                    }

                    if resp.dragged() && !ent.is_dir && !action.drag_indices.contains(&idx) {
                        action.drag_indices.push(idx);
                    }
                }
            });

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
                        egui::Stroke::new(tokens::stroke::EMPHASIS, style::ACCENT),
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
    let marker = if ent.is_dir { "▸" } else { " " };
    format!("{marker} {}", ent.name)
}
