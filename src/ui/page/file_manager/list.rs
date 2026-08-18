use std::collections::HashSet;

use egui::{Key, Modifiers};

use crate::fs::FileEntry;
use crate::session::{FilePaneState, RemotePane};

use super::PaneOps;

pub(super) fn handle_list_keyboard(
    ui: &egui::Ui,
    entries: &[FileEntry],
    selected: &mut HashSet<usize>,
    focus_index: &mut Option<usize>,
    select_mode: bool,
    anchor: &mut Option<usize>,
    ops: &mut PaneOps,
) {
    if ui.ctx().egui_wants_keyboard_input() {
        return;
    }

    let len = entries.len();
    if len == 0 {
        return;
    }

    let input = ui.input(|inp| inp.clone());

    if input.key_pressed(Key::A) && input.modifiers.ctrl {
        selected.clear();
        for i in 0..len {
            selected.insert(i);
        }
        *focus_index = Some(0);
        *anchor = Some(0);
        return;
    }

    if input.key_pressed(Key::C) && input.modifiers.ctrl {
        let indices: Vec<usize> = selected.iter().copied().collect();
        if !indices.is_empty() {
            ops.bulk_copy = Some(indices);
            if select_mode {
                ops.dismiss_multiselect = true;
            }
        }
        return;
    }
    if input.key_pressed(Key::X) && input.modifiers.ctrl {
        let indices: Vec<usize> = selected.iter().copied().collect();
        if !indices.is_empty() {
            ops.bulk_cut = Some(indices);
            if select_mode {
                ops.dismiss_multiselect = true;
            }
        }
        return;
    }
    if input.key_pressed(Key::V) && input.modifiers.ctrl {
        ops.paste = true;
        return;
    }
    if input.key_pressed(Key::Delete) {
        let indices: Vec<usize> = selected.iter().copied().collect();
        if !indices.is_empty() {
            ops.bulk_delete = Some(indices);
            if select_mode {
                ops.dismiss_multiselect = true;
            }
        }
        return;
    }

    if input.key_pressed(Key::Backspace) || input.key_pressed(Key::ArrowLeft) {
        ops.go_up = true;
        return;
    }

    if input.key_pressed(Key::Space) && select_mode {
        if let Some(idx) = *focus_index {
            toggle_index(selected, idx);
            *anchor = Some(idx);
        }
        return;
    }

    if input.key_pressed(Key::ArrowRight) || input.key_pressed(Key::Enter) {
        if let Some(idx) = *focus_index {
            if entries.get(idx).is_some_and(|e| e.is_dir) {
                ops.open_index = Some(idx);
            }
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

    let next = match *focus_index {
        Some(i) => (i as i32 + delta).clamp(0, len as i32 - 1) as usize,
        None => {
            if delta > 0 {
                0
            } else {
                len - 1
            }
        }
    };

    if input.modifiers.shift {
        let a = anchor.unwrap_or(next);
        selected.clear();
        let lo = a.min(next);
        let hi = a.max(next);
        for i in lo..=hi {
            selected.insert(i);
        }
    } else if !select_mode {
        selected.clear();
        selected.insert(next);
        *anchor = Some(next);
    } else {
        *anchor = Some(next);
    }
    *focus_index = Some(next);
}

pub(super) fn apply_selection_click(
    selected: &mut HashSet<usize>,
    focus_index: &mut Option<usize>,
    anchor: &mut Option<usize>,
    select_mode: bool,
    idx: usize,
    mods: Modifiers,
) {
    *focus_index = Some(idx);

    if mods.shift {
        if let Some(a) = *anchor {
            selected.clear();
            let lo = a.min(idx);
            let hi = a.max(idx);
            for i in lo..=hi {
                selected.insert(i);
            }
        } else {
            selected.clear();
            selected.insert(idx);
            *anchor = Some(idx);
        }
        return;
    }

    if mods.ctrl {
        toggle_index(selected, idx);
        *anchor = Some(idx);
        return;
    }

    if select_mode {
        toggle_index(selected, idx);
        *anchor = Some(idx);
        return;
    }

    selected.clear();
    selected.insert(idx);
    *anchor = Some(idx);
}

fn toggle_index(selected: &mut HashSet<usize>, idx: usize) {
    if selected.contains(&idx) {
        selected.remove(&idx);
    } else {
        selected.insert(idx);
    }
}

pub(super) fn dismiss_multiselect_local(pane: &mut FilePaneState) {
    pane.select_mode = false;
    pane.selected.clear();
}

pub(super) fn dismiss_multiselect_remote(remote: &mut RemotePane) {
    remote.select_mode = false;
    remote.selected.clear();
}
