//! 文件管理器页面 — 本地和远程 SFTP 文件浏览与管理。
//!
//! 支持双面板布局（本地-本地或本地-远程），
//! 提供文件复制、移动、删除、重命名、信息查看等操作，
//! 以及后台文件传输（上传/下载）支持。

mod context_menu;
mod dialogs;
mod dnd;
mod list;
mod ops;
pub mod transfer;

use std::collections::HashSet;
use std::sync::Arc;
use std::time::SystemTime;

use egui::Key;

use rsterm_data::prefs::{FileManagerPrefs, FileManagerUiState};
use rsterm_fs::FileEntry;
use rsterm_fs::sftp::SftpClient;
use rsterm_session_core::FileSortKey;
use rsterm_session_core::{
    FileActivePane, FileClipboard, FileManagerMode, FileManagerSession, FilePaneState, InfoDialog,
    RemotePane, RenameDialog,
};
use rsterm_uiframe::PaneChrome;
use rsterm_uiframe::file_list::{
    FileBrowserAction, FileBrowserConfig, FileBrowserLabels, FileBrowserState, FileBrowserView,
    FileDetailsColumns, FilePaneLayout, FileRow, FileSortColumn, FileViewMode,
};

use crate::content::{
    DetailsPaneSide, persist_details_columns, persist_dual_split, persist_file_manager_prefs,
};
use rsterm_uiframe::style;
use rsterm_uiframe::tokens;

use crate::labels;
use crate::page::transfer::apply_transfer_done;

use self::context_menu::{
    install_context_menu, paint_blank_context_menu, row_context_menu_local, row_context_menu_remote,
};
use self::dialogs::{show_info_dialog, show_rename_dialog};
use self::dnd::{apply_external_drag_out, apply_external_drop};
use self::ops::{
    go_up_active_pane, paste_into_pane, recompute_active_pane, refresh_if_needed, run_local_ops,
    run_remote_ops, transfer_to_opposite_pane,
};

/// Local adapter so we can implement [`FileRow`] without violating orphan rules.
struct FileEntryRow<'a>(&'a FileEntry);

impl FileRow for FileEntryRow<'_> {
    fn name(&self) -> &str {
        &self.0.name
    }

    fn is_dir(&self) -> bool {
        self.0.is_dir
    }

    fn size(&self) -> u64 {
        self.0.size
    }

    fn modified(&self) -> Option<SystemTime> {
        self.0.modified
    }
}

/// 文件管理器操作结果。
#[derive(Debug, Default)]
pub struct FileManagerAction {
    /// 是否关闭文件管理器
    pub close: bool,
    /// Prefs snapshot to merge into the host app's in-memory prefs.
    pub prefs: Option<rsterm_data::prefs::FileManagerPrefs>,
    /// Silent UI state (column widths) to merge into host prefs.
    pub ui_state: Option<FileManagerUiState>,
}

/// 面板操作集合。
#[derive(Default)]
pub(super) struct PaneOps {
    pub(super) go_up: bool,
    pub(super) open_index: Option<usize>,
    pub(super) paste: bool,
    /// Leave multi-select mode and clear row highlights (bottom bar / Cancel).
    pub(super) dismiss_multiselect: bool,
    /// Copy / cut / delete targets (bottom bar, keyboard, or context menu).
    pub(super) bulk_copy: Option<Vec<usize>>,
    pub(super) bulk_cut: Option<Vec<usize>>,
    pub(super) bulk_delete: Option<Vec<usize>>,
    pub(super) rename_index: Option<usize>,
    pub(super) info_index: Option<usize>,
    /// External files dropped onto this pane (desktop inbound).
    pub(super) dropped_paths: Vec<std::path::PathBuf>,
    /// Local file rows dragged out (desktop outbound).
    pub(super) drag_out_indices: Vec<usize>,
}

/// 底部操作栏高度
const BOTTOM_BAR_H: f32 = tokens::size::BOTTOM_BAR;

/// 文件管理器主视图渲染入口。
///
/// 处理刷新、传输轮询、标题栏、双面板布局、键盘快捷键（F5 传输）等。
pub fn file_manager_view(
    ui: &mut egui::Ui,
    session: &mut FileManagerSession,
    pane_layout: &mut FilePaneLayout,
    view_mode: &mut FileViewMode,
    details_columns_left: &mut Option<FileDetailsColumns>,
    details_columns_right: &mut Option<FileDetailsColumns>,
    dual_split: &mut f32,
    pending_prefs: &mut Option<FileManagerPrefs>,
    pending_ui_state: &mut Option<FileManagerUiState>,
    chrome: &mut PaneChrome<'_>,
) -> FileManagerAction {
    refresh_if_needed(session);
    if let Some(done) = session.transfer.poll(ui.ctx()) {
        apply_transfer_done(session, done);
    }
    // Keep sidebar path labels animating (marquee) while a file manager session is open.
    ui.ctx().request_repaint();

    let mut action = FileManagerAction::default();
    let has_clipboard = session.clipboard.is_some();
    let transfer_ui = session.transfer.read_ui();
    let labels = labels::labels();

    let (go_up, listing_changed) = paint_fm_top_bar(
        ui,
        session,
        chrome,
        view_mode,
        pane_layout,
        pending_prefs,
        &transfer_ui,
        &labels,
        &mut action,
    );
    if go_up {
        go_up_active_pane(session);
    }
    if listing_changed {
        recompute_active_pane(session);
    }

    let block_pane_keyboard = session.rename_dialog.open || session.info_dialog.open;

    if !block_pane_keyboard && ui.input(|i| i.key_pressed(Key::F5)) {
        transfer_to_opposite_pane(session);
    }

    let available = ui.available_size();
    let pane_h = available.y;

    paint_transfer_queue_panel(ui, session, &labels);

    match *pane_layout {
        FilePaneLayout::Dual => {
            paint_dual_panes(
                ui,
                session,
                *view_mode,
                details_columns_left,
                details_columns_right,
                dual_split,
                pending_ui_state,
                has_clipboard,
                block_pane_keyboard,
                available,
                pane_h,
            );
        }
        FilePaneLayout::Single => {
            let pane_size = egui::vec2(available.x, pane_h);
            paint_pane_column(ui, pane_size, |ui| match session.active_pane {
                FileActivePane::Remote | FileActivePane::LeftLocal => {
                    paint_left_host(
                        ui,
                        session,
                        *view_mode,
                        details_columns_left,
                        pending_ui_state,
                        has_clipboard,
                        block_pane_keyboard,
                    );
                }
                FileActivePane::Right => {
                    paint_right_host(
                        ui,
                        session,
                        *view_mode,
                        details_columns_right,
                        pending_ui_state,
                        has_clipboard,
                        block_pane_keyboard,
                    );
                }
            });
        }
    }

    show_rename_dialog(ui.ctx(), session);
    show_info_dialog(ui.ctx(), session);

    action
}

fn paint_dual_panes(
    ui: &mut egui::Ui,
    session: &mut FileManagerSession,
    view_mode: FileViewMode,
    details_columns_left: &mut Option<FileDetailsColumns>,
    details_columns_right: &mut Option<FileDetailsColumns>,
    dual_split: &mut f32,
    pending_ui_state: &mut Option<FileManagerUiState>,
    has_clipboard: bool,
    block_pane_keyboard: bool,
    available: egui::Vec2,
    pane_h: f32,
) {
    use rsterm_workspace::layout::MIN_PANE_WIDTH;
    use rsterm_workspace::split_handle::SPLITTER_SIZE;

    let total_w = available.x.max(SPLITTER_SIZE + 2.0);
    let content_w = (total_w - SPLITTER_SIZE).max(1.0);
    let min_frac = (MIN_PANE_WIDTH / content_w).clamp(0.15, 0.45);
    let max_frac = (1.0 - min_frac).max(min_frac);
    *dual_split = dual_split.clamp(min_frac, max_frac);

    // Keep left + splitter + right == total_w (avoid float overflow of the right edge).
    let left_w = (content_w * *dual_split).floor().max(1.0);
    let right_w = (content_w - left_w).max(1.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.set_min_height(pane_h);
        ui.set_max_width(total_w);

        paint_pane_column(ui, egui::vec2(left_w, pane_h), |ui| {
            paint_left_host(
                ui,
                session,
                view_mode,
                details_columns_left,
                pending_ui_state,
                has_clipboard,
                block_pane_keyboard,
            );
        });

        let (sep_rect, sep_resp) =
            ui.allocate_exact_size(egui::vec2(SPLITTER_SIZE, pane_h), egui::Sense::drag());
        if sep_resp.hovered() || sep_resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            ui.painter().rect_filled(
                sep_rect,
                0.0,
                ui.visuals().widgets.hovered.bg_fill.gamma_multiply(0.35),
            );
        }
        // Always-visible divider line down the splitter center.
        let line_stroke = if sep_resp.hovered() || sep_resp.dragged() {
            egui::Stroke::new(
                tokens::stroke::EMPHASIS,
                ui.visuals().widgets.hovered.bg_stroke.color,
            )
        } else {
            ui.visuals().widgets.noninteractive.bg_stroke
        };
        let x = sep_rect.center().x;
        ui.painter().line_segment(
            [
                egui::pos2(x, sep_rect.top()),
                egui::pos2(x, sep_rect.bottom()),
            ],
            line_stroke,
        );
        if sep_resp.dragged() {
            let new_left = (left_w + sep_resp.drag_delta().x)
                .clamp(content_w * min_frac, content_w * max_frac);
            *dual_split = (new_left / content_w).clamp(min_frac, max_frac);
        }
        if sep_resp.drag_stopped() {
            *pending_ui_state = Some(persist_dual_split(*dual_split));
        }

        paint_pane_column(ui, egui::vec2(right_w, pane_h), |ui| {
            paint_right_host(
                ui,
                session,
                view_mode,
                details_columns_right,
                pending_ui_state,
                has_clipboard,
                block_pane_keyboard,
            );
        });
    });
}

fn paint_left_host(
    ui: &mut egui::Ui,
    session: &mut FileManagerSession,
    view_mode: FileViewMode,
    details_columns: &mut Option<FileDetailsColumns>,
    pending_ui_state: &mut Option<FileManagerUiState>,
    has_clipboard: bool,
    block_pane_keyboard: bool,
) {
    match session.mode {
        FileManagerMode::SshSftp => {
            if let Some(remote) = session.remote.as_mut() {
                let (clicked, ops) = paint_remote_pane(
                    ui,
                    remote,
                    &mut session.remote_anchor,
                    &mut session.clipboard,
                    &mut session.status,
                    &mut session.rename_dialog,
                    &mut session.info_dialog,
                    "fm_scroll_remote",
                    view_mode,
                    details_columns,
                    DetailsPaneSide::Left,
                    pending_ui_state,
                    has_clipboard,
                    block_pane_keyboard,
                    session.active_pane == FileActivePane::Remote,
                );
                if clicked {
                    session.active_pane = FileActivePane::Remote;
                }
                if ops.paste {
                    paste_into_pane(session, FileActivePane::Remote);
                }
                apply_external_drop(session, FileActivePane::Remote, &ops.dropped_paths);
            }
        }
        FileManagerMode::LocalDual => {
            if let Some(left) = session.left_local.as_mut() {
                let (clicked, ops) = paint_local_pane(
                    ui,
                    left,
                    FileActivePane::LeftLocal,
                    &mut session.local_anchor,
                    &mut session.clipboard,
                    &mut session.status,
                    &mut session.rename_dialog,
                    &mut session.info_dialog,
                    None,
                    "fm_scroll_left",
                    view_mode,
                    details_columns,
                    DetailsPaneSide::Left,
                    pending_ui_state,
                    has_clipboard,
                    block_pane_keyboard,
                    session.active_pane == FileActivePane::LeftLocal,
                );
                if clicked {
                    session.active_pane = FileActivePane::LeftLocal;
                }
                if ops.paste {
                    paste_into_pane(session, FileActivePane::LeftLocal);
                }
                apply_external_drop(session, FileActivePane::LeftLocal, &ops.dropped_paths);
                apply_external_drag_out(session, FileActivePane::LeftLocal, &ops.drag_out_indices);
            }
        }
    }
}

fn paint_right_host(
    ui: &mut egui::Ui,
    session: &mut FileManagerSession,
    view_mode: FileViewMode,
    details_columns: &mut Option<FileDetailsColumns>,
    pending_ui_state: &mut Option<FileManagerUiState>,
    has_clipboard: bool,
    block_pane_keyboard: bool,
) {
    let remote_client = session.remote.as_ref().map(|r| &r.client);
    let (clicked, ops) = paint_local_pane(
        ui,
        &mut session.right,
        FileActivePane::Right,
        &mut session.right_anchor,
        &mut session.clipboard,
        &mut session.status,
        &mut session.rename_dialog,
        &mut session.info_dialog,
        remote_client,
        "fm_scroll_right",
        view_mode,
        details_columns,
        DetailsPaneSide::Right,
        pending_ui_state,
        has_clipboard,
        block_pane_keyboard,
        session.active_pane == FileActivePane::Right,
    );
    if clicked {
        session.active_pane = FileActivePane::Right;
    }
    if ops.paste {
        paste_into_pane(session, FileActivePane::Right);
    }
    apply_external_drop(session, FileActivePane::Right, &ops.dropped_paths);
    apply_external_drag_out(session, FileActivePane::Right, &ops.drag_out_indices);
}

/// 固定大小的列容器，确保左面板不会重叠右面板并窃取点击事件。
fn paint_pane_column<R>(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let rect = egui::Rect::from_min_size(ui.cursor().min, size);
    let _ = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        // Clip children so Details / icons never paint across the pane divider.
        ui.set_clip_rect(rect.intersect(ui.clip_rect()));
        body(ui)
    })
    .inner
}

/// 渲染远程 SFTP 面板：工具栏、文件列表、底部操作栏。
fn paint_remote_pane(
    ui: &mut egui::Ui,
    remote: &mut RemotePane,
    anchor: &mut Option<usize>,
    clipboard: &mut Option<FileClipboard>,
    status: &mut Option<String>,
    rename_dialog: &mut RenameDialog,
    info_dialog: &mut InfoDialog,
    scroll_id: &str,
    view_mode: FileViewMode,
    details_columns: &mut Option<FileDetailsColumns>,
    details_side: DetailsPaneSide,
    pending_ui_state: &mut Option<FileManagerUiState>,
    has_clipboard: bool,
    block_keyboard: bool,
    is_active: bool,
) -> (bool, PaneOps) {
    let mut ops = PaneOps::default();
    let mut list_clicked = false;

    ui.vertical(|ui| {
        if let Some(err) = &remote.error {
            ui.colored_label(egui::Color32::LIGHT_RED, err);
        }
        let show_bottom = remote.select_mode || has_clipboard;
        let bottom_h = if show_bottom { BOTTOM_BAR_H } else { 0.0 };
        let list_h = (ui.available_height() - bottom_h).max(32.0);

        let entries = remote.entries.clone();
        let rows: Vec<FileEntryRow<'_>> = entries.iter().map(FileEntryRow).collect();
        let mut state = FileBrowserState {
            selected: remote.selected.clone(),
            focus_index: remote.focus_index,
            shift_anchor: *anchor,
        };
        let sort_key = remote.sort_key;
        let sort_asc = remote.sort_asc;

        list_clicked = paint_browser_host(
            ui,
            list_h,
            remote.select_mode,
            has_clipboard,
            &mut ops,
            |ui, ops| {
                let labels = labels::labels();
                let parent = labels.parent_folder.clone();
                let loading = labels.loading.clone();
                let empty = labels.empty_folder.clone();
                let col_name =
                    sort_header_label(&labels.col_name, FileSortKey::Name, sort_key, sort_asc);
                let col_size =
                    sort_header_label(&labels.col_size, FileSortKey::Size, sort_key, sort_asc);
                let col_modified = sort_header_label(
                    &labels.col_modified,
                    FileSortKey::Modified,
                    sort_key,
                    sort_asc,
                );
                let fb_labels = FileBrowserLabels {
                    parent_folder: &parent,
                    loading: &loading,
                    empty_folder: &empty,
                    col_name: &col_name,
                    col_size: &col_size,
                    col_modified: &col_modified,
                };
                let accept_kb = is_active && !block_keyboard;
                let select_mode = remote.select_mode;
                let loading_flag = remote.loading;
                let action = {
                    let mut row_menu =
                        |idx: usize, resp: &egui::Response, selected: &HashSet<usize>| {
                            remote.selected = selected.clone();
                            if let Some(ent) = entries.get(idx) {
                                install_context_menu(resp, |ui| {
                                    row_context_menu_remote(ui, remote, idx, ent, ops);
                                });
                            }
                        };
                    FileBrowserView::show(
                        ui,
                        "",
                        &rows,
                        None,
                        loading_flag,
                        scroll_id,
                        FileBrowserConfig {
                            view_mode,
                            multi_select: select_mode,
                            show_toolbar: false,
                            allow_dnd: true,
                            open_dir_on_single_click: false,
                            details_columns: *details_columns,
                        },
                        &mut state,
                        fb_labels,
                        true,
                        accept_kb,
                        Some(&mut row_menu),
                    )
                };
                if let Some(col) = action.sort_clicked {
                    apply_sort_click(&mut remote.sort_key, &mut remote.sort_asc, col);
                    remote.recompute();
                    state.selected.clear();
                    state.focus_index = None;
                    state.shift_anchor = None;
                }
                apply_details_columns_action(
                    &action,
                    details_columns,
                    details_side,
                    pending_ui_state,
                );
                merge_browser_action(&action, &state, select_mode, ops);
                action.list_clicked
            },
        );

        remote.selected = state.selected;
        remote.focus_index = state.focus_index;
        *anchor = state.shift_anchor;

        paint_bottom_action_bar(
            ui,
            remote.select_mode,
            !remote.selected.is_empty(),
            has_clipboard,
            &mut remote.selected,
            clipboard,
            &mut ops,
        );
        run_remote_ops(
            remote,
            clipboard,
            status,
            rename_dialog,
            info_dialog,
            &mut ops,
        );
    });

    (list_clicked, ops)
}

/// 渲染本地面板：工具栏、文件列表、底部操作栏。
#[allow(clippy::too_many_arguments)]
fn paint_local_pane(
    ui: &mut egui::Ui,
    pane: &mut FilePaneState,
    pane_side: FileActivePane,
    anchor: &mut Option<usize>,
    clipboard: &mut Option<FileClipboard>,
    status: &mut Option<String>,
    rename_dialog: &mut RenameDialog,
    info_dialog: &mut InfoDialog,
    _remote_client: Option<&Arc<SftpClient>>,
    scroll_id: &str,
    view_mode: FileViewMode,
    details_columns: &mut Option<FileDetailsColumns>,
    details_side: DetailsPaneSide,
    pending_ui_state: &mut Option<FileManagerUiState>,
    has_clipboard: bool,
    block_keyboard: bool,
    is_active: bool,
) -> (bool, PaneOps) {
    let mut ops = PaneOps::default();
    let mut list_clicked = false;

    ui.vertical(|ui| {
        if let Some(err) = &pane.error {
            ui.colored_label(egui::Color32::LIGHT_RED, err);
        }
        let show_bottom = pane.select_mode || has_clipboard;
        let bottom_h = if show_bottom { BOTTOM_BAR_H } else { 0.0 };
        let list_h = (ui.available_height() - bottom_h).max(32.0);

        let entries = pane.entries.clone();
        let rows: Vec<FileEntryRow<'_>> = entries.iter().map(FileEntryRow).collect();
        let mut state = FileBrowserState {
            selected: pane.selected.clone(),
            focus_index: pane.focus_index,
            shift_anchor: *anchor,
        };
        let sort_key = pane.sort_key;
        let sort_asc = pane.sort_asc;

        list_clicked = paint_browser_host(
            ui,
            list_h,
            pane.select_mode,
            has_clipboard,
            &mut ops,
            |ui, ops| {
                let labels = labels::labels();
                let parent = labels.parent_folder.clone();
                let loading = labels.loading.clone();
                let empty = labels.empty_folder.clone();
                let col_name =
                    sort_header_label(&labels.col_name, FileSortKey::Name, sort_key, sort_asc);
                let col_size =
                    sort_header_label(&labels.col_size, FileSortKey::Size, sort_key, sort_asc);
                let col_modified = sort_header_label(
                    &labels.col_modified,
                    FileSortKey::Modified,
                    sort_key,
                    sort_asc,
                );
                let fb_labels = FileBrowserLabels {
                    parent_folder: &parent,
                    loading: &loading,
                    empty_folder: &empty,
                    col_name: &col_name,
                    col_size: &col_size,
                    col_modified: &col_modified,
                };
                let accept_kb = is_active && !block_keyboard;
                let select_mode = pane.select_mode;
                let loading_flag = pane.loading;
                let action = {
                    let mut row_menu =
                        |idx: usize, resp: &egui::Response, selected: &HashSet<usize>| {
                            pane.selected = selected.clone();
                            if let Some(ent) = entries.get(idx) {
                                install_context_menu(resp, |ui| {
                                    row_context_menu_local(ui, pane, idx, ent, ops);
                                });
                            }
                        };
                    FileBrowserView::show(
                        ui,
                        "",
                        &rows,
                        None,
                        loading_flag,
                        scroll_id,
                        FileBrowserConfig {
                            view_mode,
                            multi_select: select_mode,
                            show_toolbar: false,
                            allow_dnd: true,
                            open_dir_on_single_click: false,
                            details_columns: *details_columns,
                        },
                        &mut state,
                        fb_labels,
                        true,
                        accept_kb,
                        Some(&mut row_menu),
                    )
                };
                if let Some(col) = action.sort_clicked {
                    apply_sort_click(&mut pane.sort_key, &mut pane.sort_asc, col);
                    pane.recompute();
                    state.selected.clear();
                    state.focus_index = None;
                    state.shift_anchor = None;
                }
                apply_details_columns_action(
                    &action,
                    details_columns,
                    details_side,
                    pending_ui_state,
                );
                merge_browser_action(&action, &state, select_mode, ops);
                action.list_clicked
            },
        );

        pane.selected = state.selected;
        pane.focus_index = state.focus_index;
        *anchor = state.shift_anchor;

        paint_bottom_action_bar(
            ui,
            pane.select_mode,
            !pane.selected.is_empty(),
            has_clipboard,
            &mut pane.selected,
            clipboard,
            &mut ops,
        );
        run_local_ops(
            pane,
            pane_side,
            clipboard,
            status,
            rename_dialog,
            info_dialog,
            &mut ops,
        );
    });

    (list_clicked, ops)
}

fn apply_details_columns_action(
    action: &FileBrowserAction,
    details_columns: &mut Option<FileDetailsColumns>,
    side: DetailsPaneSide,
    pending_ui_state: &mut Option<FileManagerUiState>,
) {
    if let Some(cols) = action.details_columns {
        *details_columns = Some(cols);
        if action.details_columns_committed {
            *pending_ui_state = Some(persist_details_columns(side, cols));
        }
    }
}

fn merge_browser_action(
    action: &FileBrowserAction,
    state: &FileBrowserState,
    select_mode: bool,
    ops: &mut PaneOps,
) {
    if action.go_up {
        ops.go_up = true;
    }
    if let Some(idx) = action.open_index {
        ops.open_index = Some(idx);
    }
    if !action.dropped_paths.is_empty() {
        ops.dropped_paths = action.dropped_paths.clone();
    }
    if !action.drag_indices.is_empty() {
        ops.drag_out_indices = action.drag_indices.clone();
    }
    if action.request_paste {
        ops.paste = true;
    }
    let selected: Vec<usize> = state.selected.iter().copied().collect();
    if action.request_copy && !selected.is_empty() {
        ops.bulk_copy = Some(selected.clone());
        if select_mode {
            ops.dismiss_multiselect = true;
        }
    }
    if action.request_cut && !selected.is_empty() {
        ops.bulk_cut = Some(selected.clone());
        if select_mode {
            ops.dismiss_multiselect = true;
        }
    }
    if action.request_delete && !selected.is_empty() {
        ops.bulk_delete = Some(selected);
        if select_mode {
            ops.dismiss_multiselect = true;
        }
    }
}

/// List viewport: browser inside; blank right-click only in normal mode.
fn paint_browser_host(
    ui: &mut egui::Ui,
    list_h: f32,
    select_mode: bool,
    has_clipboard: bool,
    ops: &mut PaneOps,
    paint_browser: impl FnOnce(&mut egui::Ui, &mut PaneOps) -> bool,
) -> bool {
    let list_size = egui::vec2(ui.available_width(), list_h);
    let (list_rect, list_bg) = ui.allocate_exact_size(list_size, egui::Sense::click());
    if !select_mode {
        install_context_menu(&list_bg, |ui| {
            paint_blank_context_menu(ui, has_clipboard, ops);
        });
    }

    let mut interacted = ui
        .scope_builder(egui::UiBuilder::new().max_rect(list_rect), |ui| {
            paint_browser(ui, ops)
        })
        .inner;

    if list_bg.clicked_by(egui::PointerButton::Primary) {
        interacted = true;
    }

    interacted
}

/// Multi-select on: Copy / Cut / Delete / Cancel — any click ends multi-select.
/// After clipboard filled: Paste / Cancel (Cancel clears clipboard).
fn paint_bottom_action_bar(
    ui: &mut egui::Ui,
    select_mode: bool,
    has_selection: bool,
    has_clipboard: bool,
    selected: &mut HashSet<usize>,
    clipboard: &mut Option<FileClipboard>,
    ops: &mut PaneOps,
) {
    if !select_mode && !has_clipboard {
        return;
    }
    let labels = labels::labels();
    ui.separator();
    ui.horizontal(|ui| {
        ui.set_min_height(BOTTOM_BAR_H);
        ui.style_mut().spacing.button_padding = egui::vec2(tokens::space::MD, tokens::space::SM);
        if select_mode {
            ui.add_enabled_ui(has_selection, |ui| {
                if ui
                    .add(
                        egui::Button::new(&labels.copy)
                            .min_size(egui::vec2(0.0, tokens::size::BUTTON)),
                    )
                    .clicked()
                {
                    ops.bulk_copy = Some(selected.iter().copied().collect());
                    ops.dismiss_multiselect = true;
                }
                if ui
                    .add(
                        egui::Button::new(&labels.cut)
                            .min_size(egui::vec2(0.0, tokens::size::BUTTON)),
                    )
                    .clicked()
                {
                    ops.bulk_cut = Some(selected.iter().copied().collect());
                    ops.dismiss_multiselect = true;
                }
                if ui
                    .add(
                        egui::Button::new(&labels.delete)
                            .min_size(egui::vec2(0.0, tokens::size::BUTTON)),
                    )
                    .clicked()
                {
                    ops.bulk_delete = Some(selected.iter().copied().collect());
                    ops.dismiss_multiselect = true;
                }
            });
            if ui
                .add(
                    egui::Button::new(&labels.cancel)
                        .min_size(egui::vec2(0.0, tokens::size::BUTTON)),
                )
                .clicked()
            {
                ops.dismiss_multiselect = true;
            }
        } else if has_clipboard {
            if ui
                .add(
                    egui::Button::new(&labels.paste)
                        .min_size(egui::vec2(0.0, tokens::size::BUTTON)),
                )
                .clicked()
            {
                ops.paste = true;
            }
            if ui
                .add(
                    egui::Button::new(&labels.cancel)
                        .min_size(egui::vec2(0.0, tokens::size::BUTTON)),
                )
                .clicked()
            {
                *clipboard = None;
            }
        }
    });
}

fn paint_fm_top_bar(
    ui: &mut egui::Ui,
    session: &mut FileManagerSession,
    chrome: &mut PaneChrome<'_>,
    view_mode: &mut FileViewMode,
    pane_layout: &mut FilePaneLayout,
    pending_prefs: &mut Option<FileManagerPrefs>,
    transfer_ui: &rsterm_fs::TransferSnapshot,
    labels: &labels::FileManagerLabels,
    action: &mut FileManagerAction,
) -> (bool, bool) {
    use rsterm_uiframe::components::toolbar_button::{icon_toolbar_button, icon_toolbar_danger};
    use rsterm_uiframe::vector_icons::Icon;

    let active_id = match session.active_pane {
        FileActivePane::Remote => "fm_active_remote",
        FileActivePane::LeftLocal => "fm_active_left",
        FileActivePane::Right => "fm_active_right",
    };
    let mut go_up = false;
    let mut listing_changed = false;

    ui.horizontal(|ui| {
        ui.style_mut().spacing.button_padding =
            egui::vec2(tokens::space::XS, tokens::space::XS * 0.5);
        ui.style_mut().spacing.item_spacing.x = tokens::space::XS;

        if chrome.show_hamburger
            && icon_toolbar_button(ui, ui.id().with("fm_menu"), Icon::Hamburger).clicked()
        {
            (chrome.on_hamburger)();
        }

        let chrome_result = paint_active_pane_chrome(ui, session, active_id);
        go_up = chrome_result.0;
        listing_changed = chrome_result.1;

        if transfer_ui.active {
            ui.add_space(tokens::space::SM);
            ui.add(
                egui::ProgressBar::new(transfer_ui.progress.clamp(0.0, 1.0))
                    .desired_width(100.0)
                    .show_percentage(),
            );
            ui.label(
                egui::RichText::new(&transfer_ui.label)
                    .size(tokens::text::CAPTION)
                    .color(ui.visuals().weak_text_color()),
            );
        } else if let Some(msg) = &session.status {
            ui.add_space(tokens::space::SM);
            ui.label(egui::RichText::new(msg).size(tokens::text::CAPTION).weak());
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_toolbar_danger(ui, ui.id().with("fm_close"), Icon::Close)
                .on_hover_text(&labels.close_pane)
                .clicked()
            {
                action.close = true;
            }
            if transfer_ui.active
                && ui
                    .add(
                        egui::Button::new(&labels.stop)
                            .corner_radius(style::CORNER_RADIUS_SM)
                            .min_size(egui::vec2(64.0, tokens::size::TOOLBAR_HEIGHT)),
                    )
                    .clicked()
            {
                session.transfer.request_cancel();
            }
            paint_view_layout_controls(ui, view_mode, pane_layout, pending_prefs, labels);
        });
    });

    (go_up, listing_changed)
}

/// Shared top chrome for the focused pane: ↑ / cwd / filter / hidden / multi-select.
fn paint_active_pane_chrome(
    ui: &mut egui::Ui,
    session: &mut FileManagerSession,
    id_salt: &str,
) -> (bool, bool) {
    match session.active_pane {
        FileActivePane::Remote => {
            let Some(remote) = session.remote.as_mut() else {
                return (false, false);
            };
            paint_pane_toolbar(
                ui,
                id_salt,
                &remote.cwd.clone(),
                &mut remote.select_mode,
                &mut remote.selected,
                &mut remote.filter,
                &mut remote.show_hidden,
            )
        }
        FileActivePane::LeftLocal => {
            let Some(left) = session.left_local.as_mut() else {
                return (false, false);
            };
            let cwd = left.cwd.display().to_string();
            paint_pane_toolbar(
                ui,
                id_salt,
                &cwd,
                &mut left.select_mode,
                &mut left.selected,
                &mut left.filter,
                &mut left.show_hidden,
            )
        }
        FileActivePane::Right => {
            let cwd = session.right.cwd.display().to_string();
            paint_pane_toolbar(
                ui,
                id_salt,
                &cwd,
                &mut session.right.select_mode,
                &mut session.right.selected,
                &mut session.right.filter,
                &mut session.right.show_hidden,
            )
        }
    }
}

fn paint_pane_toolbar(
    ui: &mut egui::Ui,
    id_salt: &str,
    cwd: &str,
    select_mode: &mut bool,
    selected: &mut HashSet<usize>,
    filter: &mut String,
    show_hidden: &mut bool,
) -> (bool, bool) {
    let mut go_up = false;
    let mut listing_changed = false;
    let labels = labels::labels();
    ui.style_mut().spacing.item_spacing.x = tokens::space::SM;
    if ui
        .add(
            egui::Button::new("↑")
                .frame(false)
                .corner_radius(style::CORNER_RADIUS_XS)
                .min_size(egui::vec2(
                    tokens::size::TOOLBAR_WIDTH,
                    tokens::size::TOOLBAR_HEIGHT,
                )),
        )
        .on_hover_text(&labels.parent_folder)
        .clicked()
    {
        go_up = true;
    }
    ui.label(egui::RichText::new(cwd).size(tokens::text::SMALL).weak());
    let filter_edit = ui.add(
        egui::TextEdit::singleline(filter)
            .id_salt((id_salt, "fm_filter"))
            .desired_width(120.0)
            .hint_text(&labels.filter_placeholder),
    );
    if filter_edit.changed() {
        listing_changed = true;
    }
    if ui.checkbox(show_hidden, &labels.show_hidden).changed() {
        listing_changed = true;
    }
    if ui.checkbox(select_mode, &labels.multi_select).changed() && !*select_mode {
        selected.clear();
    }
    (go_up, listing_changed)
}

fn paint_view_layout_controls(
    ui: &mut egui::Ui,
    view_mode: &mut FileViewMode,
    pane_layout: &mut FilePaneLayout,
    pending_prefs: &mut Option<FileManagerPrefs>,
    labels: &labels::FileManagerLabels,
) {
    let mut changed = false;
    ui.scope(|ui| {
        ui.style_mut().spacing.interact_size.y = tokens::size::TOOLBAR_HEIGHT;
        ui.style_mut().spacing.button_padding =
            egui::vec2(tokens::space::SM, tokens::space::XS * 0.5);

        let view_text = match *view_mode {
            FileViewMode::List => labels.view_list.as_str(),
            FileViewMode::Details => labels.view_details.as_str(),
            FileViewMode::IconsSmall => labels.view_icons_small.as_str(),
            FileViewMode::IconsLarge => labels.view_icons_large.as_str(),
        };
        egui::ComboBox::from_id_salt("fm_view_mode")
            .selected_text(view_text)
            .width(100.0)
            .show_ui(ui, |ui| {
                for (mode, text) in [
                    (FileViewMode::List, labels.view_list.as_str()),
                    (FileViewMode::Details, labels.view_details.as_str()),
                    (FileViewMode::IconsSmall, labels.view_icons_small.as_str()),
                    (FileViewMode::IconsLarge, labels.view_icons_large.as_str()),
                ] {
                    if ui.selectable_label(*view_mode == mode, text).clicked() {
                        *view_mode = mode;
                        changed = true;
                    }
                }
            });
        let layout_text = match *pane_layout {
            FilePaneLayout::Single => labels.layout_single.as_str(),
            FilePaneLayout::Dual => labels.layout_dual.as_str(),
        };
        egui::ComboBox::from_id_salt("fm_pane_layout")
            .selected_text(layout_text)
            .width(90.0)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(*pane_layout == FilePaneLayout::Dual, &labels.layout_dual)
                    .clicked()
                {
                    *pane_layout = FilePaneLayout::Dual;
                    changed = true;
                }
                if ui
                    .selectable_label(
                        *pane_layout == FilePaneLayout::Single,
                        &labels.layout_single,
                    )
                    .clicked()
                {
                    *pane_layout = FilePaneLayout::Single;
                    changed = true;
                }
            });
    });
    if changed {
        *pending_prefs = Some(persist_file_manager_prefs(*view_mode, *pane_layout));
    }
}

fn paint_transfer_queue_panel(
    ui: &mut egui::Ui,
    session: &mut FileManagerSession,
    labels: &labels::FileManagerLabels,
) {
    let transfer_ui = session.transfer.read_ui();
    let has_queue = !session.transfer.queue.is_empty();
    let has_failed = session.transfer.last_failed.is_some();
    if !transfer_ui.active && !has_queue && !has_failed {
        return;
    }
    egui::CollapsingHeader::new(&labels.transfer_queue)
        .default_open(true)
        .show(ui, |ui| {
            if transfer_ui.active {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::ProgressBar::new(transfer_ui.progress.clamp(0.0, 1.0))
                            .desired_width(200.0)
                            .show_percentage(),
                    );
                    ui.label(egui::RichText::new(&transfer_ui.label).size(tokens::text::CAPTION));
                    if ui.button(&labels.stop).clicked() {
                        session.transfer.request_cancel();
                    }
                });
            }
            let queued: Vec<(u64, String)> = session
                .transfer
                .queue
                .iter()
                .map(|j| (j.id, j.label.clone()))
                .collect();
            for (id, label) in queued {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&label).size(tokens::text::CAPTION));
                    if ui.small_button(&labels.remove).clicked() {
                        session.transfer.remove_queued(id);
                    }
                });
            }
            ui.horizontal(|ui| {
                if has_queue && ui.button(&labels.clear_queue).clicked() {
                    session.transfer.clear_queue();
                }
                if has_failed && ui.button(&labels.retry).clicked() {
                    session.transfer.retry_last_failed();
                }
            });
        });
}

fn sort_header_label(base: &str, key: FileSortKey, current: FileSortKey, asc: bool) -> String {
    if key != current {
        return base.to_string();
    }
    let arrow = if asc { " ▲" } else { " ▼" };
    format!("{base}{arrow}")
}

fn apply_sort_click(sort_key: &mut FileSortKey, sort_asc: &mut bool, col: FileSortColumn) {
    let key = match col {
        FileSortColumn::Name => FileSortKey::Name,
        FileSortColumn::Size => FileSortKey::Size,
        FileSortColumn::Modified => FileSortKey::Modified,
    };
    if *sort_key == key {
        *sort_asc = !*sort_asc;
    } else {
        *sort_key = key;
        *sort_asc = true;
    }
}
