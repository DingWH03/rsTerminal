//! 文件管理器页面 — 本地和远程 SFTP 文件浏览与管理。
//!
//! 支持双面板布局（本地-本地或本地-远程），
//! 提供文件复制、移动、删除、重命名、信息查看等操作，
//! 以及后台文件传输（上传/下载）支持。

mod context_menu;
mod dialogs;
mod dnd;
mod fm_settings_menu;
mod list;
mod ops;
mod path_autocomplete;
mod path_bar;
pub(crate) mod touch_multiselect;
pub mod transfer;
pub(crate) use touch_multiselect::TouchMultiselectState;

use std::collections::HashSet;
use std::sync::Arc;
use std::time::SystemTime;

use egui::Key;

use rsterm_data::prefs::{FileManagerPrefs, FileManagerUiState, InputInteractionMode, load_prefs};
use rsterm_fs::FileEntry;
use rsterm_fs::sftp::SftpClient;
use rsterm_session_core::FileSortKey;
use rsterm_session_core::{
    FileActivePane, FileClipboard, FileManagerMode, FileManagerSession, FilePaneState, InfoDialog,
    RemotePane, RenameDialog,
};
use rsterm_uiframe::PopupMenuState;
use rsterm_uiframe::file_list::{
    FileBrowserAction, FileBrowserConfig, FileBrowserLabels, FileBrowserState, FileBrowserView,
    FileDetailsColumns, FilePaneLayout, FileRow, FileSortColumn, FileViewMode,
};
use rsterm_uiframe::hover_panel::{
    HoverDetail, HoverInstallMode, HoverPanelState, file_entry_detail, install_hover_detail,
    paint_hover_panel,
};

use rsterm_uiframe::PaneChrome;
use rsterm_uiframe::style;
use rsterm_uiframe::tokens;

use crate::labels;
use crate::page::transfer::apply_transfer_done;

use self::context_menu::{
    blank_context_menu_width, install_context_menu, paint_blank_context_menu,
    row_context_menu_local, row_context_menu_remote, row_context_menu_width,
};
use self::dialogs::{show_info_dialog, show_rename_dialog};
use self::dnd::{apply_external_drag_out, apply_external_drop};
use self::fm_settings_menu::paint_fm_settings_menu;
use self::ops::{
    cancel_recursive_search, go_up_active_pane, kick_recursive_search, paste_into_pane,
    poll_recursive_search, recompute_active_pane, refresh_if_needed, run_local_ops, run_remote_ops,
    submit_path_active_pane, transfer_to_opposite_pane,
};
use self::path_autocomplete::{cancel_path_autocomplete, poll_path_autocomplete};
use self::touch_multiselect::{
    TouchHoldEvent, enter_multiselect_on_row, paint_touch_multiselect_bar, poll_row_hold,
    show_row_detail_panel, track_row_press,
};
use crate::content::{DetailsPaneSide, persist_details_columns, persist_dual_split};

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
    /// Open full settings at Appearance > Layout > File Manager.
    pub open_settings: bool,
    /// Prefs snapshot to merge into the host app's in-memory prefs.
    pub prefs: Option<rsterm_data::prefs::FileManagerPrefs>,
    /// Silent UI state (column widths) to merge into host prefs.
    pub ui_state: Option<FileManagerUiState>,
}

/// Per-pane row hover / touch-hold wiring.
struct FmInteractParams<'a> {
    touch_mode: bool,
    hover: &'a mut HoverPanelState,
    touch: &'a mut TouchMultiselectState,
    labels: &'a labels::FileManagerLabels,
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

/// 底部操作栏总高度（含分隔线），与 [`paint_bottom_action_bar`] 实际占用一致。
fn pane_bottom_chrome_h(ui: &egui::Ui, show: bool) -> f32 {
    if !show {
        return 0.0;
    }
    let sep = ui.spacing().item_spacing.y + ui.visuals().widgets.noninteractive.bg_stroke.width;
    BOTTOM_BAR_H + sep
}

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
    search_panel_open: &mut bool,
    settings_menu: &mut PopupMenuState,
    hover_panel: &mut HoverPanelState,
    touch_multiselect: &mut TouchMultiselectState,
    touch_ops_menu: &mut PopupMenuState,
    pending_prefs: &mut Option<FileManagerPrefs>,
    pending_ui_state: &mut Option<FileManagerUiState>,
    chrome: &mut PaneChrome<'_>,
) -> FileManagerAction {
    refresh_if_needed(session);
    if let Some(done) = session.transfer.poll(ui.ctx()) {
        apply_transfer_done(session, done);
    }
    poll_recursive_search(session);
    {
        let pane = session.active_pane;
        let remote = matches!(pane, FileActivePane::Remote);
        let client = session.remote.as_ref().map(|r| Arc::clone(&r.client));
        poll_path_autocomplete(
            &mut session.path_autocomplete,
            pane,
            remote,
            client.as_ref(),
        );
    }

    let prev_pane = ui.memory(|m| {
        m.data
            .get_temp::<FileActivePane>(egui::Id::new("fm_path_ac_pane"))
    });
    if prev_pane.is_some_and(|p| p != session.active_pane) {
        cancel_path_autocomplete(&mut session.path_autocomplete);
    }
    ui.memory_mut(|m| {
        m.data
            .insert_temp(egui::Id::new("fm_path_ac_pane"), session.active_pane);
    });
    if session.path_autocomplete.loading || session.path_autocomplete.debounce_at.is_some() {
        ui.ctx().request_repaint();
    }

    let mut action = FileManagerAction::default();
    let has_clipboard = session.clipboard.is_some();
    let transfer_ui = session.transfer.read_ui();
    let labels = labels::labels();
    let input_mode = load_prefs().general.input_mode;
    let touch_mode = matches!(input_mode, InputInteractionMode::Touch);
    hover_panel.set_close_label(labels.close.clone());

    let mut touch_ops = PaneOps::default();
    if touch_mode && touch_multiselect.active {
        if paint_touch_multiselect_bar(
            ui,
            session,
            touch_multiselect,
            &mut touch_ops,
            touch_ops_menu,
        ) {
            touch_multiselect.exit_multiselect(session);
        }
        apply_touch_ops(session, &mut touch_ops);
    }

    if ui.input(|i| i.key_pressed(Key::Escape)) {
        if hover_panel.handle_back() {
            // consumed
        } else if touch_multiselect.active {
            touch_multiselect.exit_multiselect(session);
        }
    }

    if let Some(event) = poll_row_hold(touch_multiselect, touch_mode) {
        match event {
            TouchHoldEvent::EnterMultiselect { row } => {
                enter_multiselect_on_row(session, row);
            }
            TouchHoldEvent::ShowDetail { row } => {
                if let Some(detail) = row_detail_for_active_pane(session, row, &labels) {
                    let anchor =
                        ui.input(|i| i.pointer.interact_pos().unwrap_or(egui::pos2(0.0, 0.0)));
                    show_row_detail_panel(hover_panel, anchor, detail, touch_multiselect, session);
                }
            }
        }
    }

    let top = paint_fm_top_bar(
        ui,
        session,
        chrome,
        view_mode,
        pane_layout,
        search_panel_open,
        settings_menu,
        pending_prefs,
        &transfer_ui,
        &labels,
        &mut action,
    );
    if top.go_up {
        go_up_active_pane(session);
    }
    if let Some(path) = top.path_submitted {
        submit_path_active_pane(session, &path);
    }

    {
        let pane = session.active_pane;
        let remote = matches!(pane, FileActivePane::Remote);
        let client = session.remote.as_ref().map(|r| Arc::clone(&r.client));
        poll_path_autocomplete(
            &mut session.path_autocomplete,
            pane,
            remote,
            client.as_ref(),
        );
    }

    if top.cancel_recursive_search {
        cancel_recursive_search(session);
    }
    if top.kick_recursive_search {
        kick_recursive_search(session);
    } else if top.listing_changed {
        let recursive = match session.active_pane {
            FileActivePane::Remote => session
                .remote
                .as_ref()
                .map(|r| r.filter_recursive && !r.filter.trim().is_empty())
                .unwrap_or(false),
            FileActivePane::LeftLocal => session
                .left_local
                .as_ref()
                .map(|p| p.filter_recursive && !p.filter.trim().is_empty())
                .unwrap_or(false),
            FileActivePane::Right => {
                session.right.filter_recursive && !session.right.filter.trim().is_empty()
            }
        };
        if recursive {
            kick_recursive_search(session);
        } else {
            recompute_active_pane(session);
        }
    }

    let block_pane_keyboard = session.rename_dialog.open || session.info_dialog.open;
    let mut interact = FmInteractParams {
        touch_mode,
        hover: hover_panel,
        touch: touch_multiselect,
        labels: &labels,
    };

    if !block_pane_keyboard && ui.input(|i| i.key_pressed(Key::F5)) {
        transfer_to_opposite_pane(session);
    }

    let available_w = ui.available_width();

    paint_transfer_queue_panel(ui, session, &labels);

    let pane_h = ui.available_height().max(32.0);
    let available = egui::vec2(available_w, pane_h);

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
                &mut interact,
                available,
                pane_h,
            );
        }
        FilePaneLayout::Single => {
            let pane_size = egui::vec2(available.x, pane_h);
            const ACTIVE_SCROLL: &str = "fm_scroll_active";
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
                        &mut interact,
                        ACTIVE_SCROLL,
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
                        &mut interact,
                        ACTIVE_SCROLL,
                    );
                }
            });
        }
    }

    show_rename_dialog(ui.ctx(), session);
    show_info_dialog(ui.ctx(), session);
    paint_hover_panel(ui.ctx(), hover_panel);

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
    interact: &mut FmInteractParams<'_>,
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

    ui.allocate_ui_with_layout(
        egui::vec2(total_w, pane_h),
        egui::Layout::left_to_right(egui::Align::TOP).with_main_wrap(false),
        |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;

            paint_pane_column(ui, egui::vec2(left_w, pane_h), |ui| {
                paint_left_host(
                    ui,
                    session,
                    view_mode,
                    details_columns_left,
                    pending_ui_state,
                    has_clipboard,
                    block_pane_keyboard,
                    interact,
                    default_left_scroll_id(session),
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
                    interact,
                    "fm_scroll_right",
                );
            });
        },
    );
}

fn default_left_scroll_id(session: &FileManagerSession) -> &'static str {
    match session.mode {
        FileManagerMode::SshSftp => "fm_scroll_remote",
        FileManagerMode::LocalDual => "fm_scroll_left",
    }
}

fn paint_left_host(
    ui: &mut egui::Ui,
    session: &mut FileManagerSession,
    view_mode: FileViewMode,
    details_columns: &mut Option<FileDetailsColumns>,
    pending_ui_state: &mut Option<FileManagerUiState>,
    has_clipboard: bool,
    block_pane_keyboard: bool,
    interact: &mut FmInteractParams<'_>,
    scroll_id: &str,
) -> (bool, PaneOps) {
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
                    scroll_id,
                    view_mode,
                    details_columns,
                    DetailsPaneSide::Left,
                    pending_ui_state,
                    has_clipboard,
                    block_pane_keyboard,
                    session.active_pane == FileActivePane::Remote,
                    interact,
                );
                if clicked {
                    session.active_pane = FileActivePane::Remote;
                }
                if ops.paste {
                    paste_into_pane(session, FileActivePane::Remote);
                }
                apply_external_drop(session, FileActivePane::Remote, &ops.dropped_paths);
                return (clicked, ops);
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
                    scroll_id,
                    view_mode,
                    details_columns,
                    DetailsPaneSide::Left,
                    pending_ui_state,
                    has_clipboard,
                    block_pane_keyboard,
                    session.active_pane == FileActivePane::LeftLocal,
                    interact,
                );
                if clicked {
                    session.active_pane = FileActivePane::LeftLocal;
                }
                if ops.paste {
                    paste_into_pane(session, FileActivePane::LeftLocal);
                }
                apply_external_drop(session, FileActivePane::LeftLocal, &ops.dropped_paths);
                apply_external_drag_out(session, FileActivePane::LeftLocal, &ops.drag_out_indices);
                return (clicked, ops);
            }
        }
    }
    (false, PaneOps::default())
}

fn paint_right_host(
    ui: &mut egui::Ui,
    session: &mut FileManagerSession,
    view_mode: FileViewMode,
    details_columns: &mut Option<FileDetailsColumns>,
    pending_ui_state: &mut Option<FileManagerUiState>,
    has_clipboard: bool,
    block_pane_keyboard: bool,
    interact: &mut FmInteractParams<'_>,
    scroll_id: &str,
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
        scroll_id,
        view_mode,
        details_columns,
        DetailsPaneSide::Right,
        pending_ui_state,
        has_clipboard,
        block_pane_keyboard,
        session.active_pane == FileActivePane::Right,
        interact,
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
#[allow(clippy::too_many_arguments)]
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
    interact: &mut FmInteractParams<'_>,
) -> (bool, PaneOps) {
    let mut ops = PaneOps::default();
    let mut list_clicked = false;
    let pointer_mode = !interact.touch_mode;

    ui.vertical(|ui| {
        if let Some(err) = &remote.error {
            ui.colored_label(egui::Color32::LIGHT_RED, err);
        }
        let show_bottom = !interact.touch_mode && (remote.select_mode || has_clipboard);
        let bottom_h = pane_bottom_chrome_h(ui, show_bottom);
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
            scroll_id,
            remote.select_mode,
            has_clipboard,
            pointer_mode,
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
                                install_context_menu(
                                    resp,
                                    pointer_mode,
                                    Some(row_context_menu_width(&resp.ctx)),
                                    |ui| {
                                        row_context_menu_remote(ui, remote, idx, ent, ops);
                                    },
                                );
                            }
                        };
                    let mut row_hook = |idx: usize, resp: &egui::Response, ent: &dyn FileRow| {
                        hook_fm_row(interact, idx, resp, ent);
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
                        Some(&mut row_hook),
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
    interact: &mut FmInteractParams<'_>,
) -> (bool, PaneOps) {
    let mut ops = PaneOps::default();
    let mut list_clicked = false;
    let pointer_mode = !interact.touch_mode;

    ui.vertical(|ui| {
        if let Some(err) = &pane.error {
            ui.colored_label(egui::Color32::LIGHT_RED, err);
        }
        let show_bottom = !interact.touch_mode && (pane.select_mode || has_clipboard);
        let bottom_h = pane_bottom_chrome_h(ui, show_bottom);
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
            scroll_id,
            pane.select_mode,
            has_clipboard,
            pointer_mode,
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
                                install_context_menu(
                                    resp,
                                    pointer_mode,
                                    Some(row_context_menu_width(&resp.ctx)),
                                    |ui| {
                                        row_context_menu_local(ui, pane, idx, ent, ops);
                                    },
                                );
                            }
                        };
                    let mut row_hook = |idx: usize, resp: &egui::Response, ent: &dyn FileRow| {
                        hook_fm_row(interact, idx, resp, ent);
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
                        Some(&mut row_hook),
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
    scroll_id: &str,
    select_mode: bool,
    has_clipboard: bool,
    enable_context_menu: bool,
    ops: &mut PaneOps,
    paint_browser: impl FnOnce(&mut egui::Ui, &mut PaneOps) -> bool,
) -> bool {
    let viewport_id = egui::Id::new(scroll_id).with("viewport");
    ui.push_id(viewport_id, |ui| {
        let list_size = egui::vec2(ui.available_width(), list_h);
        let (list_rect, list_bg) = ui.allocate_exact_size(list_size, egui::Sense::click());
        if !select_mode {
            install_context_menu(
                &list_bg,
                enable_context_menu,
                Some(blank_context_menu_width(ui.ctx(), has_clipboard)),
                |ui| {
                    paint_blank_context_menu(ui, has_clipboard, ops);
                },
            );
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
    })
    .inner
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
    search_panel_open: &mut bool,
    settings_menu: &mut PopupMenuState,
    pending_prefs: &mut Option<FileManagerPrefs>,
    transfer_ui: &rsterm_fs::TransferSnapshot,
    labels: &labels::FileManagerLabels,
    action: &mut FileManagerAction,
) -> path_bar::PathBarAction {
    use rsterm_uiframe::components::toolbar_button::{
        icon_toolbar_button, icon_toolbar_danger, text_toolbar_button,
    };
    use rsterm_uiframe::vector_icons::Icon;

    let mut top = path_bar::PathBarAction::default();

    ui.horizontal(|ui| {
        ui.style_mut().spacing.button_padding =
            egui::vec2(tokens::space::XS, tokens::space::XS * 0.5);
        ui.style_mut().spacing.item_spacing.x = tokens::space::XS;

        if chrome.show_hamburger
            && icon_toolbar_button(ui, egui::Id::new("fm_topbar_menu"), Icon::Hamburger).clicked()
        {
            (chrome.on_hamburger)();
        }

        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.style_mut().spacing.item_spacing.x = tokens::space::XS;
            let path_action = path_bar::paint_active_path_chrome(ui, session);
            top.go_up = path_action.go_up;
            top.listing_changed |= path_action.listing_changed;
            top.path_submitted = path_action.path_submitted;

            if transfer_ui.active {
                ui.add_space(tokens::space::SM);
                ui.add(
                    egui::ProgressBar::new(transfer_ui.progress.clamp(0.0, 1.0))
                        .desired_width(72.0)
                        .show_percentage(),
                );
                ui.label(
                    egui::RichText::new(&transfer_ui.label)
                        .size(tokens::text::CAPTION)
                        .color(ui.visuals().weak_text_color()),
                );
            } else if let Some(msg) = &session.status {
                ui.add_space(tokens::space::SM);
                ui.add(
                    egui::Label::new(egui::RichText::new(msg).size(tokens::text::CAPTION).weak())
                        .truncate(),
                );
            }
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.style_mut().spacing.item_spacing.x = tokens::space::XS;
            if icon_toolbar_danger(ui, egui::Id::new("fm_topbar_close"), Icon::Close)
                .on_hover_text(&labels.close_pane)
                .clicked()
            {
                action.close = true;
            }
            if transfer_ui.active {
                let stop = ui
                    .push_id(egui::Id::new("fm_topbar_stop"), |ui| {
                        ui.add(
                            egui::Button::new(&labels.stop)
                                .corner_radius(style::CORNER_RADIUS_SM)
                                .min_size(egui::vec2(56.0, tokens::size::TOOLBAR_HEIGHT)),
                        )
                    })
                    .inner;
                if stop.clicked() {
                    session.transfer.request_cancel();
                }
            }
            let settings_btn =
                icon_toolbar_button(ui, egui::Id::new("fm_topbar_settings"), Icon::Settings);
            let menu_action = paint_fm_settings_menu(
                &settings_btn,
                settings_menu,
                session,
                view_mode,
                pane_layout,
                pending_prefs,
            );
            top.listing_changed |= menu_action.listing_changed;
            if menu_action.open_settings {
                action.open_settings = true;
            }
            let search_label = if *search_panel_open { "▾" } else { "🔍" };
            if text_toolbar_button(ui, egui::Id::new("fm_topbar_search"), search_label)
                .on_hover_text(&labels.search_toggle)
                .clicked()
            {
                *search_panel_open = !*search_panel_open;
            }
        });
    });
    ui.add(egui::Separator::default().spacing(tokens::space::XS));

    if *search_panel_open {
        let searching = session
            .recursive_search
            .as_ref()
            .map(|s| s.is_running())
            .unwrap_or(false);
        let search = path_bar::paint_search_panel(ui, session, searching);
        top.listing_changed |= search.listing_changed;
        top.kick_recursive_search |= search.kick_recursive_search;
        top.cancel_recursive_search |= search.cancel_recursive_search;
    }

    top
}

fn hook_fm_row(
    interact: &mut FmInteractParams<'_>,
    idx: usize,
    resp: &egui::Response,
    ent: &dyn FileRow,
) {
    if interact.touch_mode {
        track_row_press(interact.touch, idx, resp, true);
        return;
    }
    let size_line = if ent.is_dir() {
        None
    } else {
        Some(format_bytes(ent.size()))
    };
    let mod_line = ent
        .modified()
        .map(|t| format_modified_label(t, interact.labels));
    let detail = file_entry_detail(ent.name(), size_line, mod_line);
    install_hover_detail(resp, detail, HoverInstallMode::PointerHover, interact.hover);
}

fn row_detail_for_active_pane(
    session: &FileManagerSession,
    row: usize,
    labels: &labels::FileManagerLabels,
) -> Option<HoverDetail> {
    let ent = match session.active_pane {
        FileActivePane::Remote => session.remote.as_ref()?.entries.get(row)?,
        FileActivePane::LeftLocal => session.left_local.as_ref()?.entries.get(row)?,
        FileActivePane::Right => session.right.entries.get(row)?,
    };
    let size_line = if ent.is_dir {
        None
    } else {
        Some(format_bytes(ent.size))
    };
    let mod_line = ent.modified.map(|t| format_modified_label(t, labels));
    Some(file_entry_detail(&ent.name, size_line, mod_line))
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

fn format_modified_label(t: SystemTime, labels: &labels::FileManagerLabels) -> String {
    let _ = labels;
    use std::time::UNIX_EPOCH;
    let Ok(dur) = t.duration_since(UNIX_EPOCH) else {
        return String::new();
    };
    let secs = dur.as_secs();
    let days = secs / 86_400;
    let tod = secs % 86_400;
    let hh = tod / 3600;
    let mm = (tod % 3600) / 60;
    format!("{days}d {hh:02}:{mm:02}")
}

fn apply_touch_ops(session: &mut FileManagerSession, ops: &mut PaneOps) {
    let mut status = session.status.take();
    let mut rename = session.rename_dialog.clone();
    let mut info = session.info_dialog.clone();
    match session.active_pane {
        FileActivePane::Remote => {
            if let Some(remote) = session.remote.as_mut() {
                run_remote_ops(
                    remote,
                    &mut session.clipboard,
                    &mut status,
                    &mut rename,
                    &mut info,
                    ops,
                );
            }
        }
        FileActivePane::LeftLocal => {
            if let Some(left) = session.left_local.as_mut() {
                run_local_ops(
                    left,
                    FileActivePane::LeftLocal,
                    &mut session.clipboard,
                    &mut status,
                    &mut rename,
                    &mut info,
                    ops,
                );
            }
        }
        FileActivePane::Right => {
            run_local_ops(
                &mut session.right,
                FileActivePane::Right,
                &mut session.clipboard,
                &mut status,
                &mut rename,
                &mut info,
                ops,
            );
        }
    }
    session.status = status;
    session.rename_dialog = rename;
    session.info_dialog = info;
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
