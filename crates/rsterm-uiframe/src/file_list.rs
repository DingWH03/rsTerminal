//! Reusable file browser control (list / details / icons) with optional DnD.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use egui::{Key, Modifiers};

use crate::style;
use crate::tokens;
use crate::vector_icons::{self, Icon};

/// Minimal row data for file browser rows (implemented by callers / adapters).
pub trait FileRow {
    fn name(&self) -> &str;
    fn is_dir(&self) -> bool;
    fn size(&self) -> u64 {
        0
    }
    fn modified(&self) -> Option<SystemTime> {
        None
    }
}

/// How entries are painted inside a single browser pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileViewMode {
    #[default]
    List,
    Details,
    IconsSmall,
    IconsLarge,
}

/// Host-level single vs dual pane layout (consumed by FM page, not this control).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilePaneLayout {
    Single,
    #[default]
    Dual,
}

/// Details view column widths (logical pixels). Modified uses the remainder.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FileDetailsColumns {
    pub name_w: f32,
    pub size_w: f32,
}

impl FileDetailsColumns {
    pub const NAME_MIN: f32 = 120.0;
    pub const SIZE_MIN: f32 = 64.0;
    pub const MODIFIED_MIN: f32 = 80.0;
    pub const SIZE_DEFAULT: f32 = 88.0;
    /// Visual gap between columns (also hosts the drag handle).
    pub const GAP: f32 = tokens::space::MD;
    /// Pixel width of the drag handle drawn inside [`Self::GAP`].
    pub const SEP_W: f32 = 4.0;

    pub fn defaults_for(avail: f32) -> Self {
        let gaps = Self::GAP * 2.0;
        let content = (avail - gaps).max(Self::NAME_MIN + Self::SIZE_DEFAULT);
        let name_w = (content * 0.50).clamp(
            Self::NAME_MIN,
            (content - Self::SIZE_DEFAULT - Self::MODIFIED_MIN).max(Self::NAME_MIN),
        );
        Self {
            name_w,
            size_w: Self::SIZE_DEFAULT,
        }
    }

    /// Resolve name / size / modified widths for `avail`.
    /// Always fits within `avail` (shrinks when the pane is narrower than preferred mins).
    pub fn resolve(self, avail: f32) -> (f32, f32, f32) {
        let gaps = Self::GAP * 2.0;
        let content = (avail - gaps).max(1.0);

        let mut name_w = self.name_w.max(Self::NAME_MIN);
        let mut size_w = self.size_w.max(Self::SIZE_MIN);
        let mut modified_w = content - name_w - size_w;

        if modified_w < Self::MODIFIED_MIN {
            let need = Self::MODIFIED_MIN - modified_w;
            let shrink_name = (name_w - Self::NAME_MIN).max(0.0).min(need);
            name_w -= shrink_name;
            modified_w += shrink_name;
            if modified_w < Self::MODIFIED_MIN {
                let need2 = Self::MODIFIED_MIN - modified_w;
                let shrink_size = (size_w - Self::SIZE_MIN).max(0.0).min(need2);
                size_w -= shrink_size;
                modified_w += shrink_size;
            }
        }

        // Pane too narrow for preferred mins: scale so columns stay inside avail.
        let total = name_w + size_w + modified_w.max(0.0);
        if total > content + 0.01 {
            let scale = content / total;
            name_w = (name_w * scale).floor();
            size_w = (size_w * scale).floor();
            modified_w = (content - name_w - size_w).max(0.0);
        } else {
            modified_w = (content - name_w - size_w).max(0.0);
        }

        // Final guard: never exceed available width.
        let used = name_w + size_w + modified_w + gaps;
        if used > avail {
            let overflow = used - avail;
            modified_w = (modified_w - overflow).max(0.0);
        }

        (name_w, size_w, modified_w)
    }
}

/// Configuration for one [`FileBrowserView`] instance (not persisted).
#[derive(Debug, Clone, Copy)]
pub struct FileBrowserConfig {
    pub view_mode: FileViewMode,
    /// When true: primary click toggles; Space toggles focused row.
    pub multi_select: bool,
    pub show_toolbar: bool,
    pub allow_dnd: bool,
    /// Sidebar-style: primary click opens directories. FM uses `false` (double-click).
    pub open_dir_on_single_click: bool,
    /// Details column widths; `None` uses built-in defaults for available width.
    pub details_columns: Option<FileDetailsColumns>,
}

impl Default for FileBrowserConfig {
    fn default() -> Self {
        Self {
            view_mode: FileViewMode::List,
            multi_select: false,
            show_toolbar: true,
            allow_dnd: true,
            open_dir_on_single_click: false,
            details_columns: None,
        }
    }
}

/// Selection / focus state owned by the host and mutated by the control.
#[derive(Debug, Clone, Default)]
pub struct FileBrowserState {
    pub selected: HashSet<usize>,
    pub focus_index: Option<usize>,
    /// Anchor for Shift-range selection.
    pub shift_anchor: Option<usize>,
}

/// Localized strings for [`FileBrowserView`].
pub struct FileBrowserLabels<'a> {
    pub parent_folder: &'a str,
    pub loading: &'a str,
    pub empty_folder: &'a str,
    pub col_name: &'a str,
    pub col_size: &'a str,
    pub col_modified: &'a str,
}

impl<'a> FileBrowserLabels<'a> {
    pub fn basic(parent_folder: &'a str, loading: &'a str, empty_folder: &'a str) -> Self {
        Self {
            parent_folder,
            loading,
            empty_folder,
            col_name: "Name",
            col_size: "Size",
            col_modified: "Modified",
        }
    }
}

/// Sort column clicked in Details headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSortColumn {
    Name,
    Size,
    Modified,
}

/// Actions emitted by [`FileBrowserView::show`].
#[derive(Debug, Default)]
pub struct FileBrowserAction {
    pub go_up: bool,
    pub open_index: Option<usize>,
    pub selection_changed: bool,
    pub focus_changed: bool,
    pub drag_indices: Vec<usize>,
    pub dropped_paths: Vec<PathBuf>,
    pub context_menu_at: Option<usize>,
    pub request_copy: bool,
    pub request_cut: bool,
    pub request_paste: bool,
    pub request_delete: bool,
    /// True if any row received a primary click (host may activate the pane).
    pub list_clicked: bool,
    pub sort_clicked: Option<FileSortColumn>,
    /// Updated Details column widths while resizing (or after a resize).
    pub details_columns: Option<FileDetailsColumns>,
    /// True when a column-resize drag finished this frame (host should persist).
    pub details_columns_committed: bool,
}

/// Per-row context menu installer: `(index, response, current_selection)`.
pub type FileBrowserRowMenu<'a> = dyn FnMut(usize, &egui::Response, &HashSet<usize>) + 'a;

/// Paint a path header + scrollable file browser (list / details / icons).
pub struct FileBrowserView;

impl FileBrowserView {
    /// Paint the browser. `row_menu(idx, &response)` installs per-row context menus when provided.
    pub fn show(
        ui: &mut egui::Ui,
        cwd: &str,
        entries: &[impl FileRow],
        error: Option<&str>,
        loading: bool,
        id_salt: &str,
        config: FileBrowserConfig,
        state: &mut FileBrowserState,
        labels: FileBrowserLabels<'_>,
        interactive: bool,
        accept_keyboard: bool,
        mut row_menu: Option<&mut FileBrowserRowMenu<'_>>,
    ) -> FileBrowserAction {
        let mut action = FileBrowserAction::default();
        let n = entries.len();

        if config.show_toolbar {
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
                    .on_hover_text(labels.parent_folder);
                if up.clicked() {
                    action.go_up = true;
                }
                ui.label(egui::RichText::new(cwd).size(tokens::text::SMALL).weak());
            });
            ui.add_space(tokens::space::XS);
            ui.add(egui::Separator::default().spacing(tokens::space::XS));
        }

        if let Some(err) = error {
            ui.colored_label(style::RED, err);
        }
        if loading {
            ui.label(
                egui::RichText::new(labels.loading)
                    .size(tokens::text::SMALL)
                    .weak(),
            );
        }

        if interactive && accept_keyboard && !loading {
            handle_keyboard(ui, entries, state, config.multi_select, &mut action);
        }

        let list_resp = egui::ScrollArea::vertical()
            .id_salt(id_salt)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.style_mut().spacing.scroll.bar_width = 6.0;
                ui.set_min_width(ui.available_width());
                if !loading && n == 0 && error.is_none() {
                    ui.label(
                        egui::RichText::new(labels.empty_folder)
                            .size(tokens::text::SMALL)
                            .weak(),
                    );
                    return;
                }
                if loading {
                    return;
                }
                match config.view_mode {
                    FileViewMode::List => paint_list_rows(
                        ui,
                        entries,
                        state,
                        config,
                        interactive,
                        &mut action,
                        &mut row_menu,
                    ),
                    FileViewMode::Details => paint_details_rows(
                        ui,
                        id_salt,
                        entries,
                        state,
                        config,
                        interactive,
                        &labels,
                        &mut action,
                        &mut row_menu,
                    ),
                    FileViewMode::IconsSmall => paint_icon_grid(
                        ui,
                        id_salt,
                        entries,
                        state,
                        config,
                        interactive,
                        56.0,
                        &mut action,
                        &mut row_menu,
                    ),
                    FileViewMode::IconsLarge => paint_icon_grid(
                        ui,
                        id_salt,
                        entries,
                        state,
                        config,
                        interactive,
                        88.0,
                        &mut action,
                        &mut row_menu,
                    ),
                }
            });

        if config.allow_dnd {
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
        } else {
            let _ = list_resp;
        }

        action
    }
}

fn handle_keyboard(
    ui: &mut egui::Ui,
    entries: &[impl FileRow],
    state: &mut FileBrowserState,
    multi_select: bool,
    action: &mut FileBrowserAction,
) {
    if ui.ctx().egui_wants_keyboard_input() {
        return;
    }
    let len = entries.len();
    if len == 0 {
        return;
    }

    let input = ui.input(|inp| inp.clone());

    if input.key_pressed(Key::A) && input.modifiers.command {
        state.selected.clear();
        for i in 0..len {
            state.selected.insert(i);
        }
        state.focus_index = Some(0);
        state.shift_anchor = Some(0);
        action.selection_changed = true;
        action.focus_changed = true;
        return;
    }
    if input.key_pressed(Key::C) && input.modifiers.command {
        action.request_copy = true;
        return;
    }
    if input.key_pressed(Key::X) && input.modifiers.command {
        action.request_cut = true;
        return;
    }
    if input.key_pressed(Key::V) && input.modifiers.command {
        action.request_paste = true;
        return;
    }
    if input.key_pressed(Key::Delete) {
        action.request_delete = true;
        return;
    }
    if input.key_pressed(Key::Backspace) || input.key_pressed(Key::ArrowLeft) {
        action.go_up = true;
        return;
    }
    if input.key_pressed(Key::Space) && multi_select {
        if let Some(idx) = state.focus_index {
            toggle_index(&mut state.selected, idx);
            state.shift_anchor = Some(idx);
            action.selection_changed = true;
        }
        return;
    }
    if input.key_pressed(Key::ArrowRight) || input.key_pressed(Key::Enter) {
        if let Some(idx) = state.focus_index
            && entries.get(idx).is_some_and(|e| e.is_dir())
        {
            action.open_index = Some(idx);
        }
        return;
    }

    let delta = if input.key_pressed(Key::ArrowDown) {
        1
    } else if input.key_pressed(Key::ArrowUp) {
        -1
    } else {
        return;
    };

    let next = match state.focus_index {
        Some(i) => (i as isize + delta).clamp(0, len as isize - 1) as usize,
        None => {
            if delta > 0 {
                0
            } else {
                len - 1
            }
        }
    };
    state.focus_index = Some(next);
    action.focus_changed = true;

    if input.modifiers.shift {
        let anchor = state.shift_anchor.unwrap_or(next);
        state.shift_anchor.get_or_insert(anchor);
        select_range(&mut state.selected, anchor, next);
        action.selection_changed = true;
    } else if !multi_select {
        state.selected.clear();
        state.selected.insert(next);
        state.shift_anchor = Some(next);
        action.selection_changed = true;
    } else {
        state.shift_anchor = Some(next);
    }
}

fn paint_list_rows(
    ui: &mut egui::Ui,
    entries: &[impl FileRow],
    state: &mut FileBrowserState,
    config: FileBrowserConfig,
    interactive: bool,
    action: &mut FileBrowserAction,
    row_menu: &mut Option<&mut FileBrowserRowMenu<'_>>,
) {
    for (idx, ent) in entries.iter().enumerate() {
        let selected = state.selected.contains(&idx);
        let focused = state.focus_index == Some(idx);
        let label = list_label(ent, focused && interactive);
        let resp = ui.add(
            egui::Button::new(egui::RichText::new(label).size(tokens::text::BODY))
                .selected(selected && interactive)
                .frame(selected && interactive)
                .corner_radius(style::CORNER_RADIUS_XS)
                .min_size(egui::vec2(ui.available_width(), tokens::size::NAV_ROW)),
        );
        handle_row_interact(
            ui,
            &resp,
            idx,
            ent,
            state,
            config,
            interactive,
            action,
            row_menu,
        );
    }
}

fn paint_details_rows(
    ui: &mut egui::Ui,
    id_salt: &str,
    entries: &[impl FileRow],
    state: &mut FileBrowserState,
    config: FileBrowserConfig,
    interactive: bool,
    labels: &FileBrowserLabels<'_>,
    action: &mut FileBrowserAction,
    row_menu: &mut Option<&mut FileBrowserRowMenu<'_>>,
) {
    let avail = ui.available_width().max(1.0);
    let mut cols = config
        .details_columns
        .unwrap_or_else(|| FileDetailsColumns::defaults_for(avail));
    let (mut name_w, mut size_w, modified_w) = cols.resolve(avail);
    let header_h = tokens::size::TOOLBAR_HEIGHT;
    let gap = FileDetailsColumns::GAP;
    let pad = tokens::space::SM;

    // Absolute header layout (matches row columns; avoids horizontal item_spacing overflow).
    let (header_rect, _) =
        ui.allocate_exact_size(egui::vec2(avail, header_h), egui::Sense::hover());
    let mut x = header_rect.left();
    let name_rect = egui::Rect::from_min_size(
        egui::pos2(x, header_rect.top()),
        egui::vec2(name_w, header_h),
    );
    x += name_w;
    let sep1_rect = egui::Rect::from_center_size(
        egui::pos2(x + gap * 0.5, header_rect.center().y),
        egui::vec2(FileDetailsColumns::SEP_W, header_h),
    );
    x += gap;
    let size_rect = egui::Rect::from_min_size(
        egui::pos2(x, header_rect.top()),
        egui::vec2(size_w, header_h),
    );
    x += size_w;
    let sep2_rect = egui::Rect::from_center_size(
        egui::pos2(x + gap * 0.5, header_rect.center().y),
        egui::vec2(FileDetailsColumns::SEP_W, header_h),
    );
    x += gap;
    let modified_w = modified_w.min((header_rect.right() - x).max(0.0));
    let modified_rect = egui::Rect::from_min_size(
        egui::pos2(x, header_rect.top()),
        egui::vec2(modified_w, header_h),
    );

    // Salt all header interactables so dual panes never share egui Ids.
    let header_id = ui.id().with(id_salt).with("details_header");
    ui.scope_builder(egui::UiBuilder::new().id_salt(id_salt), |ui| {
        header_cell_at(
            ui,
            name_rect,
            labels.col_name,
            FileSortColumn::Name,
            action,
            header_id.with("name"),
        );
        header_cell_at(
            ui,
            size_rect,
            labels.col_size,
            FileSortColumn::Size,
            action,
            header_id.with("size"),
        );
        header_cell_at(
            ui,
            modified_rect,
            labels.col_modified,
            FileSortColumn::Modified,
            action,
            header_id.with("modified"),
        );

        if interactive {
            let sep1 = column_resize_sep_at(ui, header_id.with("sep_ns"), sep1_rect);
            let sep2 = column_resize_sep_at(ui, header_id.with("sep_sm"), sep2_rect);
            let mut changed = false;
            if sep1.dragged() {
                let dx = sep1.drag_delta().x;
                let max_name = (avail
                    - gap * 2.0
                    - FileDetailsColumns::SIZE_MIN
                    - FileDetailsColumns::MODIFIED_MIN)
                    .max(FileDetailsColumns::NAME_MIN);
                name_w = (name_w + dx).clamp(FileDetailsColumns::NAME_MIN, max_name);
                changed = true;
            }
            if sep2.dragged() {
                let dx = sep2.drag_delta().x;
                let max_size = (avail - gap * 2.0 - name_w - FileDetailsColumns::MODIFIED_MIN)
                    .max(FileDetailsColumns::SIZE_MIN);
                size_w = (size_w + dx).clamp(FileDetailsColumns::SIZE_MIN, max_size);
                changed = true;
            }
            if changed {
                let r = FileDetailsColumns { name_w, size_w }.resolve(avail);
                cols = FileDetailsColumns {
                    name_w: r.0,
                    size_w: r.1,
                };
                action.details_columns = Some(cols);
            }
            if sep1.drag_stopped() || sep2.drag_stopped() {
                action.details_columns = Some(cols);
                action.details_columns_committed = true;
            }
        }
    });

    ui.add(egui::Separator::default().spacing(tokens::space::XS));

    if let Some(c) = action.details_columns {
        cols = c;
    }
    let (name_w, size_w, modified_w) = cols.resolve(avail);

    for (idx, ent) in entries.iter().enumerate() {
        let selected = state.selected.contains(&idx);
        let focused = state.focus_index == Some(idx);
        let (rect, resp) = ui.allocate_exact_size(
            egui::vec2(avail, tokens::size::NAV_ROW),
            egui::Sense::click_and_drag(),
        );
        if selected && interactive {
            ui.painter().rect_filled(
                rect,
                style::CORNER_RADIUS_XS,
                ui.visuals().selection.bg_fill,
            );
        } else if focused && interactive {
            ui.painter().rect_stroke(
                rect,
                style::CORNER_RADIUS_XS,
                egui::Stroke::new(tokens::stroke::HAIRLINE, style::ACCENT),
                egui::StrokeKind::Inside,
            );
        }

        let mut x = rect.left();
        let text_color = if selected && interactive {
            ui.visuals().strong_text_color()
        } else {
            ui.visuals().text_color()
        };
        paint_text_in(
            ui,
            egui::pos2(x + pad, rect.center().y),
            &list_label(ent, false),
            (name_w - pad * 2.0).max(1.0),
            text_color,
        );
        x += name_w + gap;
        let size_str = if ent.is_dir() {
            String::new()
        } else {
            format_bytes(ent.size())
        };
        paint_text_in(
            ui,
            egui::pos2(x + pad, rect.center().y),
            &size_str,
            (size_w - pad * 2.0).max(1.0),
            ui.visuals().weak_text_color(),
        );
        x += size_w + gap;
        let mod_str = format_modified(ent.modified());
        paint_text_in(
            ui,
            egui::pos2(x + pad, rect.center().y),
            &mod_str,
            (modified_w - pad * 2.0).max(1.0),
            ui.visuals().weak_text_color(),
        );

        handle_row_interact(
            ui,
            &resp,
            idx,
            ent,
            state,
            config,
            interactive,
            action,
            row_menu,
        );
    }
}

fn header_cell_at(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    text: &str,
    column: FileSortColumn,
    action: &mut FileBrowserAction,
    id: egui::Id,
) {
    let resp = ui.interact(rect, id, egui::Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(
            rect,
            style::CORNER_RADIUS_XS,
            ui.visuals().widgets.hovered.bg_fill,
        );
    }
    let color = ui.visuals().weak_text_color();
    let galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(
            text.to_string(),
            egui::FontId::proportional(tokens::text::CAPTION),
            color,
        )
    });
    let pos = egui::pos2(
        rect.left() + tokens::space::SM,
        rect.center().y - galley.size().y * 0.5,
    );
    ui.painter().galley(pos, galley, color);
    if resp.clicked() {
        action.sort_clicked = Some(column);
    }
}

fn column_resize_sep_at(ui: &mut egui::Ui, id: egui::Id, rect: egui::Rect) -> egui::Response {
    let resp = ui.interact(rect, id, egui::Sense::drag());
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        ui.painter()
            .rect_filled(rect, 0.0, ui.visuals().widgets.hovered.bg_fill);
    }
    resp
}

fn paint_icon_grid(
    ui: &mut egui::Ui,
    id_salt: &str,
    entries: &[impl FileRow],
    state: &mut FileBrowserState,
    config: FileBrowserConfig,
    interactive: bool,
    cell: f32,
    action: &mut FileBrowserAction,
    row_menu: &mut Option<&mut FileBrowserRowMenu<'_>>,
) {
    let avail = ui.available_width().max(cell);
    let cols = ((avail / (cell + tokens::space::SM)).floor() as usize).max(1);
    let icon_size = (cell * 0.45).clamp(16.0, 40.0);

    egui::Grid::new(ui.id().with(id_salt).with("icons"))
        .num_columns(cols)
        .spacing([tokens::space::SM, tokens::space::SM])
        .show(ui, |ui| {
            for (idx, ent) in entries.iter().enumerate() {
                let selected = state.selected.contains(&idx);
                let (rect, resp) = ui.allocate_exact_size(
                    egui::vec2(cell, cell + 18.0),
                    egui::Sense::click_and_drag(),
                );

                if selected && interactive {
                    ui.painter().rect_filled(
                        rect,
                        style::CORNER_RADIUS_SM,
                        ui.visuals().selection.bg_fill,
                    );
                }

                let icon_rect = egui::Rect::from_center_size(
                    egui::pos2(
                        rect.center().x,
                        rect.top() + icon_size * 0.5 + tokens::space::MD,
                    ),
                    egui::vec2(icon_size, icon_size),
                );
                let icon = if ent.is_dir() {
                    Icon::Folder
                } else {
                    Icon::Sessions
                };
                let color = if selected && interactive {
                    ui.visuals().strong_text_color()
                } else {
                    ui.visuals().weak_text_color()
                };
                vector_icons::paint(ui, icon_rect, icon, color, tokens::stroke::EMPHASIS);

                let name = truncate_name(ent.name(), 14);
                let galley = ui.fonts_mut(|f| {
                    f.layout_no_wrap(
                        name,
                        egui::FontId::proportional(tokens::text::SMALL),
                        ui.visuals().text_color(),
                    )
                });
                let text_pos = egui::pos2(
                    rect.center().x - galley.size().x * 0.5,
                    rect.bottom() - galley.size().y - tokens::space::XS,
                );
                ui.painter()
                    .galley(text_pos, galley, ui.visuals().text_color());

                handle_row_interact(
                    ui,
                    &resp,
                    idx,
                    ent,
                    state,
                    config,
                    interactive,
                    action,
                    row_menu,
                );

                if (idx + 1) % cols == 0 {
                    ui.end_row();
                }
            }
        });
}

fn handle_row_interact(
    ui: &egui::Ui,
    resp: &egui::Response,
    idx: usize,
    ent: &impl FileRow,
    state: &mut FileBrowserState,
    config: FileBrowserConfig,
    interactive: bool,
    action: &mut FileBrowserAction,
    row_menu: &mut Option<&mut FileBrowserRowMenu<'_>>,
) {
    if !interactive {
        if let Some(menu) = row_menu.as_mut() {
            menu(idx, resp, &state.selected);
        }
        return;
    }

    if resp.secondary_clicked() {
        action.context_menu_at = Some(idx);
        action.list_clicked = true;
        if !state.selected.contains(&idx) {
            state.selected.clear();
            state.selected.insert(idx);
            action.selection_changed = true;
        }
        state.focus_index = Some(idx);
        action.focus_changed = true;
    }

    if resp.double_clicked() && ent.is_dir() {
        action.open_index = Some(idx);
        action.list_clicked = true;
        if let Some(menu) = row_menu.as_mut() {
            menu(idx, resp, &state.selected);
        }
        return;
    }

    if resp.clicked() {
        action.list_clicked = true;
        if config.open_dir_on_single_click && ent.is_dir() {
            action.open_index = Some(idx);
            if let Some(menu) = row_menu.as_mut() {
                menu(idx, resp, &state.selected);
            }
            return;
        }
        let modifiers = ui.input(|i| i.modifiers);
        apply_click_selection(state, idx, config.multi_select, modifiers, action);
    }

    if config.allow_dnd && resp.dragged() && !ent.is_dir() && !action.drag_indices.contains(&idx) {
        action.drag_indices.push(idx);
    }

    // Install after selection updates so context menus see current state.
    if let Some(menu) = row_menu.as_mut() {
        menu(idx, resp, &state.selected);
    }
}

fn apply_click_selection(
    state: &mut FileBrowserState,
    idx: usize,
    multi_select: bool,
    modifiers: Modifiers,
    action: &mut FileBrowserAction,
) {
    state.focus_index = Some(idx);
    action.focus_changed = true;

    if modifiers.shift {
        let anchor = state.shift_anchor.unwrap_or(idx);
        select_range(&mut state.selected, anchor, idx);
        action.selection_changed = true;
        return;
    }
    if modifiers.command {
        toggle_index(&mut state.selected, idx);
        state.shift_anchor = Some(idx);
        action.selection_changed = true;
        return;
    }
    if multi_select {
        toggle_index(&mut state.selected, idx);
        state.shift_anchor = Some(idx);
        action.selection_changed = true;
        return;
    }
    state.selected.clear();
    state.selected.insert(idx);
    state.shift_anchor = Some(idx);
    action.selection_changed = true;
}

fn toggle_index(selected: &mut HashSet<usize>, idx: usize) {
    if !selected.remove(&idx) {
        selected.insert(idx);
    }
}

fn select_range(selected: &mut HashSet<usize>, a: usize, b: usize) {
    selected.clear();
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    for i in lo..=hi {
        selected.insert(i);
    }
}

fn list_label(ent: &impl FileRow, focused: bool) -> String {
    let marker = if ent.is_dir() { "▸" } else { " " };
    if focused {
        format!("● {marker} {}", ent.name())
    } else {
        format!("  {marker} {}", ent.name())
    }
}

fn paint_text_in(ui: &egui::Ui, pos: egui::Pos2, text: &str, max_w: f32, color: egui::Color32) {
    let galley = ui.fonts_mut(|f| {
        f.layout(
            text.to_string(),
            egui::FontId::proportional(tokens::text::BODY),
            color,
            max_w,
        )
    });
    let y = pos.y - galley.size().y * 0.5;
    ui.painter().galley(egui::pos2(pos.x, y), galley, color);
}

fn format_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} {}", n, UNITS[i])
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

fn format_modified(t: Option<SystemTime>) -> String {
    let Some(t) = t else {
        return String::new();
    };
    let Ok(dur) = t.duration_since(UNIX_EPOCH) else {
        return String::new();
    };
    let secs = dur.as_secs();
    let days = secs / 86_400;
    let tod = secs % 86_400;
    let hh = tod / 3600;
    let mm = (tod % 3600) / 60;
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}")
}

/// Howard Hinnant's civil_from_days (UTC calendar date from days since 1970-01-01).
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

fn truncate_name(name: &str, max_chars: usize) -> String {
    let count = name.chars().count();
    if count <= max_chars {
        name.to_string()
    } else {
        let t: String = name.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{t}…")
    }
}
