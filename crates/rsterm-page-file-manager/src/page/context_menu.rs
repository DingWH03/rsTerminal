use std::collections::HashSet;

use rsterm_fs::FileEntry;
use rsterm_session_core::{FilePaneState, RemotePane};
use rsterm_uiframe::{install_context_popup, measure_menu_width, menu_action};

use crate::labels;

use super::PaneOps;

pub(super) fn row_context_menu_width(ctx: &egui::Context) -> f32 {
    let labels = labels::labels();
    let refs = [
        labels.open.as_str(),
        labels.copy.as_str(),
        labels.cut.as_str(),
        labels.delete.as_str(),
        labels.rename.as_str(),
        labels.file_info.as_str(),
    ];
    measure_menu_width(ctx, &refs, false)
}

pub(super) fn blank_context_menu_width(ctx: &egui::Context, has_clipboard: bool) -> f32 {
    let labels = labels::labels();
    let refs = if has_clipboard {
        vec![labels.paste.as_str()]
    } else {
        vec![labels.clipboard_empty.as_str()]
    };
    measure_menu_width(ctx, &refs, false)
}

pub(super) fn install_context_menu(
    resp: &egui::Response,
    enable_desktop_context: bool,
    width_hint: Option<f32>,
    mut build: impl FnMut(&mut egui::Ui),
) {
    install_context_popup(resp, enable_desktop_context, None, width_hint, move |ui| {
        build(ui)
    });
}

pub(super) fn paint_blank_context_menu(ui: &mut egui::Ui, has_clipboard: bool, ops: &mut PaneOps) {
    let labels = labels::labels();
    if has_clipboard {
        if menu_action(ui, &labels.paste) {
            ops.paste = true;
        }
    } else {
        ui.label(egui::RichText::new(&labels.clipboard_empty).weak());
    }
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
    let labels = labels::labels();
    let in_multiselect = pane.select_mode;
    if ent.is_dir && menu_action(ui, &labels.open) {
        ops.open_index = Some(idx);
        if in_multiselect {
            ops.dismiss_multiselect = true;
        }
        return;
    }
    if menu_action(ui, &labels.copy) {
        ops.bulk_copy = Some(indices_for_context_action(&pane.selected, idx));
        if in_multiselect {
            ops.dismiss_multiselect = true;
        }
    }
    if menu_action(ui, &labels.cut) {
        ops.bulk_cut = Some(indices_for_context_action(&pane.selected, idx));
        if in_multiselect {
            ops.dismiss_multiselect = true;
        }
    }
    if menu_action(ui, &labels.delete) {
        ops.bulk_delete = Some(indices_for_context_action(&pane.selected, idx));
        if in_multiselect {
            ops.dismiss_multiselect = true;
        }
    }
    if menu_action(ui, &labels.rename) {
        ops.rename_index = Some(idx);
    }
    if menu_action(ui, &labels.file_info) {
        ops.info_index = Some(idx);
    }
}

pub(super) fn row_context_menu_remote(
    ui: &mut egui::Ui,
    remote: &RemotePane,
    idx: usize,
    ent: &FileEntry,
    ops: &mut PaneOps,
) {
    let labels = labels::labels();
    let in_multiselect = remote.select_mode;
    if ent.is_dir && menu_action(ui, &labels.open) {
        ops.open_index = Some(idx);
        if in_multiselect {
            ops.dismiss_multiselect = true;
        }
        return;
    }
    if menu_action(ui, &labels.copy) {
        ops.bulk_copy = Some(indices_for_context_action(&remote.selected, idx));
        if in_multiselect {
            ops.dismiss_multiselect = true;
        }
    }
    if menu_action(ui, &labels.cut) {
        ops.bulk_cut = Some(indices_for_context_action(&remote.selected, idx));
        if in_multiselect {
            ops.dismiss_multiselect = true;
        }
    }
    if menu_action(ui, &labels.delete) {
        ops.bulk_delete = Some(indices_for_context_action(&remote.selected, idx));
        if in_multiselect {
            ops.dismiss_multiselect = true;
        }
    }
    if menu_action(ui, &labels.rename) {
        ops.rename_index = Some(idx);
    }
    if menu_action(ui, &labels.file_info) {
        ops.info_index = Some(idx);
    }
}
