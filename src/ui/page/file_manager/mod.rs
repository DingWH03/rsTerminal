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

use egui::Key;

use crate::fs::FileEntry;
use crate::fs::sftp::SftpClient;
use crate::session::{
    FileActivePane, FileClipboard, FileManagerMode, FileManagerSession, FilePaneState, InfoDialog,
    RemotePane, RenameDialog,
};
use crate::ui::function_pane::FunctionPane;
use crate::ui::page::file_manager::transfer::apply_transfer_done;
use crate::ui::uiframe::style;
use crate::ui::uiframe::tokens;

use self::context_menu::{
    install_context_menu, paint_blank_context_menu, row_context_menu_local, row_context_menu_remote,
};
use self::dialogs::{show_info_dialog, show_rename_dialog};
use self::dnd::{apply_external_drag_out, apply_external_drop};
use self::list::{apply_selection_click, handle_list_keyboard};
use self::ops::{
    paste_into_pane, refresh_if_needed, run_local_ops, run_remote_ops, transfer_to_opposite_pane,
};

/// 文件管理器操作结果。
#[derive(Debug, Default)]
pub struct FileManagerAction {
    /// 是否关闭文件管理器
    pub close: bool,
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
    function_pane: &mut FunctionPane,
    in_split: bool,
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

    {
        use crate::ui::uiframe::components::pane_header::PaneHeader;
        use crate::ui::uiframe::components::toolbar_button::icon_toolbar_danger;
        use crate::ui::uiframe::vector_icons::Icon;

        let show_hamburger = !in_split && function_pane.show_content_hamburger();
        let title = session.title.clone();
        let mut center = |ui: &mut egui::Ui| {
            ui.label(
                egui::RichText::new(&title)
                    .size(tokens::text::COMPACT)
                    .strong()
                    .color(ui.visuals().text_color()),
            );
            if transfer_ui.active {
                ui.add_space(tokens::space::SM);
                ui.add(
                    egui::ProgressBar::new(transfer_ui.progress.clamp(0.0, 1.0))
                        .desired_width(160.0)
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
        };
        let mut trailing = |ui: &mut egui::Ui| {
            // right_to_left: first widget is rightmost.
            if icon_toolbar_danger(ui, ui.id().with("fm_close"), Icon::Close)
                .on_hover_text(rust_i18n::t!("close_pane"))
                .clicked()
            {
                action.close = true;
            }
            if transfer_ui.active
                && ui
                    .add(
                        egui::Button::new(rust_i18n::t!("stop"))
                            .corner_radius(style::CORNER_RADIUS_SM)
                            .min_size(egui::vec2(64.0, tokens::size::TOOLBAR_HEIGHT)),
                    )
                    .clicked()
            {
                session.transfer.request_cancel();
            }
        };
        let header = PaneHeader {
            show_hamburger,
            hamburger_id: Some(ui.id().with("fm_menu")),
            title: None,
            center: Some(&mut center),
            trailing: Some(&mut trailing),
        }
        .show(ui);
        if header.hamburger_clicked {
            function_pane.hamburger_click();
        }
    }

    let block_pane_keyboard = session.rename_dialog.open || session.info_dialog.open;

    if !block_pane_keyboard && !session.transfer.is_active() && ui.input(|i| i.key_pressed(Key::F5))
    {
        transfer_to_opposite_pane(session);
    }

    let available = ui.available_size();
    let pane_w = (available.x - 8.0) / 2.0;
    let pane_h = available.y;

    let pane_size = egui::vec2(pane_w, pane_h);
    ui.horizontal(|ui| {
        ui.set_min_height(pane_h);
        paint_pane_column(ui, pane_size, |ui| match session.mode {
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
                    apply_external_drag_out(
                        session,
                        FileActivePane::LeftLocal,
                        &ops.drag_out_indices,
                    );
                }
            }
        });

        ui.add_space(8.0);

        paint_pane_column(ui, pane_size, |ui| {
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
        });
    });

    show_rename_dialog(ui.ctx(), session);
    show_info_dialog(ui.ctx(), session);

    action
}

/// 固定大小的列容器，确保左面板不会重叠右面板并窃取点击事件。
fn paint_pane_column<R>(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let rect = egui::Rect::from_min_size(ui.cursor().min, size);
    let _ = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), body)
        .inner
}

// toolbar_button 已迁移到 crate::ui::uiframe::components::toolbar_button

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
    has_clipboard: bool,
    block_keyboard: bool,
    is_active: bool,
) -> (bool, PaneOps) {
    let mut ops = PaneOps::default();
    let pane_focus_id = ui.id().with((scroll_id, "focus"));
    let mut list_clicked = false;

    ui.vertical(|ui| {
        if paint_pane_toolbar(
            ui,
            &remote.cwd,
            &mut remote.select_mode,
            &mut remote.selected,
        ) {
            ops.go_up = true;
        }
        if let Some(err) = &remote.error {
            ui.colored_label(egui::Color32::LIGHT_RED, err);
        }
        let show_bottom = remote.select_mode || has_clipboard;
        let bottom_h = if show_bottom { BOTTOM_BAR_H } else { 0.0 };
        let list_h = (ui.available_height() - bottom_h).max(32.0);
        list_clicked = paint_file_list_area(
            ui,
            pane_focus_id,
            scroll_id,
            list_h,
            remote.select_mode,
            has_clipboard,
            &mut ops,
            |ui, ops| {
                paint_remote_list(
                    ui,
                    pane_focus_id,
                    scroll_id,
                    remote,
                    anchor,
                    clipboard,
                    status,
                    block_keyboard,
                    is_active,
                    list_h,
                    ops,
                )
            },
        );
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
    has_clipboard: bool,
    block_keyboard: bool,
    is_active: bool,
) -> (bool, PaneOps) {
    let mut ops = PaneOps::default();
    let pane_focus_id = ui.id().with((scroll_id, "focus"));
    let cwd = pane.cwd.display().to_string();
    let mut list_clicked = false;

    ui.vertical(|ui| {
        if paint_pane_toolbar(ui, &cwd, &mut pane.select_mode, &mut pane.selected) {
            ops.go_up = true;
        }
        if let Some(err) = &pane.error {
            ui.colored_label(egui::Color32::LIGHT_RED, err);
        }
        let show_bottom = pane.select_mode || has_clipboard;
        let bottom_h = if show_bottom { BOTTOM_BAR_H } else { 0.0 };
        let list_h = (ui.available_height() - bottom_h).max(32.0);
        list_clicked = paint_file_list_area(
            ui,
            pane_focus_id,
            scroll_id,
            list_h,
            pane.select_mode,
            has_clipboard,
            &mut ops,
            |ui, ops| {
                paint_local_list(
                    ui,
                    pane_focus_id,
                    scroll_id,
                    pane,
                    anchor,
                    clipboard,
                    status,
                    block_keyboard,
                    is_active,
                    list_h,
                    ops,
                )
            },
        );
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
    ui.separator();
    ui.horizontal(|ui| {
        ui.set_min_height(BOTTOM_BAR_H);
        ui.style_mut().spacing.button_padding = egui::vec2(tokens::space::MD, tokens::space::SM);
        if select_mode {
            ui.add_enabled_ui(has_selection, |ui| {
                if ui
                    .add(
                        egui::Button::new(rust_i18n::t!("copy"))
                            .min_size(egui::vec2(0.0, tokens::size::BUTTON)),
                    )
                    .clicked()
                {
                    ops.bulk_copy = Some(selected.iter().copied().collect());
                    ops.dismiss_multiselect = true;
                }
                if ui
                    .add(
                        egui::Button::new(rust_i18n::t!("cut"))
                            .min_size(egui::vec2(0.0, tokens::size::BUTTON)),
                    )
                    .clicked()
                {
                    ops.bulk_cut = Some(selected.iter().copied().collect());
                    ops.dismiss_multiselect = true;
                }
                if ui
                    .add(
                        egui::Button::new(rust_i18n::t!("delete"))
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
                    egui::Button::new(rust_i18n::t!("cancel"))
                        .min_size(egui::vec2(0.0, tokens::size::BUTTON)),
                )
                .clicked()
            {
                ops.dismiss_multiselect = true;
            }
        } else if has_clipboard {
            if ui
                .add(
                    egui::Button::new(rust_i18n::t!("paste"))
                        .min_size(egui::vec2(0.0, tokens::size::BUTTON)),
                )
                .clicked()
            {
                ops.paste = true;
            }
            if ui
                .add(
                    egui::Button::new(rust_i18n::t!("cancel"))
                        .min_size(egui::vec2(0.0, tokens::size::BUTTON)),
                )
                .clicked()
            {
                *clipboard = None;
            }
        }
    });
}

/// List viewport: rows inside; blank right-click only in normal mode.
fn paint_file_list_area(
    ui: &mut egui::Ui,
    pane_focus_id: egui::Id,
    _scroll_id: &str,
    list_h: f32,
    select_mode: bool,
    has_clipboard: bool,
    ops: &mut PaneOps,
    paint_list: impl FnOnce(&mut egui::Ui, &mut PaneOps) -> bool,
) -> bool {
    let list_size = egui::vec2(ui.available_width(), list_h);
    let (list_rect, list_bg) = ui.allocate_exact_size(list_size, egui::Sense::click());
    if !select_mode {
        install_context_menu(ui, &list_bg, |ui| {
            paint_blank_context_menu(ui, has_clipboard, ops);
        });
    }

    let mut interacted = ui
        .scope_builder(egui::UiBuilder::new().max_rect(list_rect), |ui| {
            paint_list(ui, ops)
        })
        .inner;

    if list_bg.clicked_by(egui::PointerButton::Primary) {
        interacted = true;
        ui.memory_mut(|m| m.request_focus(pane_focus_id));
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        let hovering = ui.rect_contains_pointer(list_rect);
        if hovering {
            let hovered = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
            if hovered {
                ui.painter().rect_stroke(
                    list_rect,
                    style::CORNER_RADIUS_XS,
                    egui::Stroke::new(1.5_f32, style::ACCENT),
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
            if !dropped.is_empty() {
                ops.dropped_paths = dropped;
            }
        }
    }

    interacted
}

fn paint_remote_list(
    ui: &mut egui::Ui,
    pane_focus_id: egui::Id,
    scroll_id: &str,
    remote: &mut RemotePane,
    anchor: &mut Option<usize>,
    _clipboard: &mut Option<FileClipboard>,
    _status: &mut Option<String>,
    block_keyboard: bool,
    is_active: bool,
    list_max_height: f32,
    ops: &mut PaneOps,
) -> bool {
    let entries = remote.entries.clone();
    let mut interacted = false;

    let _scroll = egui::ScrollArea::vertical()
        .id_salt(scroll_id)
        .max_height(list_max_height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if remote.loading {
                ui.label(
                    egui::RichText::new(rust_i18n::t!("loading"))
                        .size(tokens::text::SMALL)
                        .weak(),
                );
                return;
            }
            if entries.is_empty() {
                ui.label(
                    egui::RichText::new(rust_i18n::t!("empty_folder"))
                        .size(tokens::text::SMALL)
                        .weak(),
                );
                return;
            }
            for (i, ent) in entries.iter().enumerate() {
                let focused = ui.memory(|m| m.has_focus(pane_focus_id)) || is_active;
                let is_sel = remote.selected.contains(&i);
                let is_focus = remote.focus_index == Some(i);
                let label = entry_label(ent, is_focus && focused);
                let resp = ui.selectable_label(is_sel, label);
                install_context_menu(ui, &resp, |ui| {
                    row_context_menu_remote(ui, remote, i, ent, ops);
                });
                if resp.double_clicked() && ent.is_dir {
                    ops.open_index = Some(i);
                    continue;
                }
                if resp.clicked_by(egui::PointerButton::Primary) {
                    interacted = true;
                    ui.memory_mut(|m| m.request_focus(pane_focus_id));
                    let mods = resp.ctx.input(|inp| inp.modifiers);
                    apply_selection_click(
                        &mut remote.selected,
                        &mut remote.focus_index,
                        anchor,
                        remote.select_mode,
                        i,
                        mods,
                    );
                }
            }
        });

    let focused = ui.memory(|m| m.has_focus(pane_focus_id)) || is_active;
    if focused && !block_keyboard {
        handle_list_keyboard(
            ui,
            &remote.entries,
            &mut remote.selected,
            &mut remote.focus_index,
            remote.select_mode,
            anchor,
            ops,
        );
    }

    interacted
}

fn paint_local_list(
    ui: &mut egui::Ui,
    pane_focus_id: egui::Id,
    scroll_id: &str,
    pane: &mut FilePaneState,
    anchor: &mut Option<usize>,
    _clipboard: &mut Option<FileClipboard>,
    _status: &mut Option<String>,
    block_keyboard: bool,
    is_active: bool,
    list_max_height: f32,
    ops: &mut PaneOps,
) -> bool {
    let entries = pane.entries.clone();
    let mut interacted = false;

    let _scroll = egui::ScrollArea::vertical()
        .id_salt(scroll_id)
        .max_height(list_max_height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if pane.loading {
                ui.label(
                    egui::RichText::new(rust_i18n::t!("loading"))
                        .size(tokens::text::SMALL)
                        .weak(),
                );
                return;
            }
            if entries.is_empty() {
                ui.label(
                    egui::RichText::new(rust_i18n::t!("empty_folder"))
                        .size(tokens::text::SMALL)
                        .weak(),
                );
                return;
            }
            for (i, ent) in entries.iter().enumerate() {
                let focused = ui.memory(|m| m.has_focus(pane_focus_id)) || is_active;
                let is_sel = pane.selected.contains(&i);
                let is_focus = pane.focus_index == Some(i);
                let label = entry_label(ent, is_focus && focused);
                let resp = ui.selectable_label(is_sel, label);
                install_context_menu(ui, &resp, |ui| {
                    row_context_menu_local(ui, pane, i, ent, ops);
                });
                #[cfg(any(target_os = "linux", target_os = "windows"))]
                if resp.dragged() && !ent.is_dir && !ops.drag_out_indices.contains(&i) {
                    ops.drag_out_indices.push(i);
                }
                if resp.double_clicked() && ent.is_dir {
                    ops.open_index = Some(i);
                    continue;
                }
                if resp.clicked_by(egui::PointerButton::Primary) {
                    interacted = true;
                    ui.memory_mut(|m| m.request_focus(pane_focus_id));
                    let mods = resp.ctx.input(|inp| inp.modifiers);
                    apply_selection_click(
                        &mut pane.selected,
                        &mut pane.focus_index,
                        anchor,
                        pane.select_mode,
                        i,
                        mods,
                    );
                }
            }
        });

    let focused = ui.memory(|m| m.has_focus(pane_focus_id)) || is_active;
    if focused && !block_keyboard {
        handle_list_keyboard(
            ui,
            &pane.entries,
            &mut pane.selected,
            &mut pane.focus_index,
            pane.select_mode,
            anchor,
            ops,
        );
    }

    interacted
}

fn paint_pane_toolbar(
    ui: &mut egui::Ui,
    cwd: &str,
    select_mode: &mut bool,
    selected: &mut HashSet<usize>,
) -> bool {
    let mut go_up = false;
    ui.horizontal(|ui| {
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
            .on_hover_text(rust_i18n::t!("parent_folder"))
            .clicked()
        {
            go_up = true;
        }
        ui.label(egui::RichText::new(cwd).size(tokens::text::SMALL).weak());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .checkbox(select_mode, rust_i18n::t!("multi_select"))
                .changed()
                && !*select_mode
            {
                selected.clear();
            }
        });
    });
    go_up
}

fn entry_label(ent: &FileEntry, focused: bool) -> String {
    let marker = if ent.is_dir { "▸" } else { " " };
    if focused {
        format!("● {marker} {}", ent.name)
    } else {
        format!("  {marker} {}", ent.name)
    }
}
