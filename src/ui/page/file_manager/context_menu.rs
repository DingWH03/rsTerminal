use std::collections::HashSet;

use crate::fs::FileEntry;
use crate::session::{FilePaneState, RemotePane};

use super::PaneOps;

const CONTEXT_MENU_MIN_WIDTH: f32 = 140.0;

pub(super) fn install_context_menu(
    _ui: &egui::Ui,
    resp: &egui::Response,
    mut build: impl FnMut(&mut egui::Ui),
) {
    let menu_id = resp.id.with("ctx_popup");
    resp.context_menu(|ui| build(ui));
    if resp.long_touched() {
        egui::Popup::open_id(&resp.ctx, menu_id);
    }
    let long_touch_open = resp
        .long_touched()
        .then_some(egui::SetOpenCommand::Bool(true));
    egui::Popup::from_response(resp)
        .id(menu_id)
        .open_memory(long_touch_open)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_min_width(CONTEXT_MENU_MIN_WIDTH);
            build(ui);
        });
}

pub(super) fn paint_blank_context_menu(ui: &mut egui::Ui, has_clipboard: bool, ops: &mut PaneOps) {
    paint_horizontal_context_menu(ui, |ui| {
        if has_clipboard {
            if ui.button("Paste").clicked() {
                ops.paste = true;
                ui.close();
            }
        } else {
            ui.label(egui::RichText::new("Clipboard empty").weak());
        }
    });
}

fn paint_horizontal_context_menu(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    ui.set_min_width(CONTEXT_MENU_MIN_WIDTH);
    ui.horizontal(|ui| {
        ui.style_mut().spacing.item_spacing.x = 8.0;
        content(ui);
    });
}

fn indices_for_context_action(selected: &HashSet<usize>, right_clicked: usize) -> Vec<usize> {
    if selected.is_empty() {
        vec![right_clicked]
    } else {
        selected.iter().copied().collect()
    }
}

pub(super) fn row_context_menu_local(
    ui: &mut egui::Ui,
    pane: &FilePaneState,
    idx: usize,
    ent: &FileEntry,
    ops: &mut PaneOps,
) {
    let in_multiselect = pane.select_mode;
    paint_horizontal_context_menu(ui, |ui| {
        if ent.is_dir && ui.button("Open").clicked() {
            ops.open_index = Some(idx);
            if in_multiselect {
                ops.dismiss_multiselect = true;
            }
            ui.close();
            return;
        }
        if ui.button("Copy").clicked() {
            ops.bulk_copy = Some(indices_for_context_action(&pane.selected, idx));
            if in_multiselect {
                ops.dismiss_multiselect = true;
            }
            ui.close();
        }
        if ui.button("Cut").clicked() {
            ops.bulk_cut = Some(indices_for_context_action(&pane.selected, idx));
            if in_multiselect {
                ops.dismiss_multiselect = true;
            }
            ui.close();
        }
        if ui.button(rust_i18n::t!("delete")).clicked() {
            ops.bulk_delete = Some(indices_for_context_action(&pane.selected, idx));
            if in_multiselect {
                ops.dismiss_multiselect = true;
            }
            ui.close();
        }
        if ui.button("Rename").clicked() {
            ops.rename_index = Some(idx);
            ui.close();
        }
        if ui.button("Info").clicked() {
            ops.info_index = Some(idx);
            ui.close();
        }
    });
}

pub(super) fn row_context_menu_remote(
    ui: &mut egui::Ui,
    remote: &RemotePane,
    idx: usize,
    ent: &FileEntry,
    ops: &mut PaneOps,
) {
    let in_multiselect = remote.select_mode;
    paint_horizontal_context_menu(ui, |ui| {
        if ent.is_dir && ui.button("Open").clicked() {
            ops.open_index = Some(idx);
            if in_multiselect {
                ops.dismiss_multiselect = true;
            }
            ui.close();
            return;
        }
        if ui.button("Copy").clicked() {
            ops.bulk_copy = Some(indices_for_context_action(&remote.selected, idx));
            if in_multiselect {
                ops.dismiss_multiselect = true;
            }
            ui.close();
        }
        if ui.button("Cut").clicked() {
            ops.bulk_cut = Some(indices_for_context_action(&remote.selected, idx));
            if in_multiselect {
                ops.dismiss_multiselect = true;
            }
            ui.close();
        }
        if ui.button(rust_i18n::t!("delete")).clicked() {
            ops.bulk_delete = Some(indices_for_context_action(&remote.selected, idx));
            if in_multiselect {
                ops.dismiss_multiselect = true;
            }
            ui.close();
        }
        if ui.button("Rename").clicked() {
            ops.rename_index = Some(idx);
            ui.close();
        }
        if ui.button("Info").clicked() {
            ops.info_index = Some(idx);
            ui.close();
        }
    });
}
