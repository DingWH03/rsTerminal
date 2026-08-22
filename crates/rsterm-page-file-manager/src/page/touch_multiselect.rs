//! Touch-mode multiselect overlay and long-press state machine.

use std::collections::HashSet;
use std::time::Instant;

use egui::{Pos2, Response, Ui};

use rsterm_session_core::{FileActivePane, FileManagerSession};
use rsterm_uiframe::hover_panel::{HoverDetail, HoverPanelState};
use rsterm_uiframe::tokens;
use rsterm_uiframe::vector_icons::{self, Icon};
use rsterm_uiframe::{PopupMenuState, measure_menu_width, menu_action, popup_from_response};

use super::PaneOps;
use crate::labels;

const MULTISELECT_HOLD: f32 = 0.5;
const DETAIL_HOLD: f32 = 1.0;

#[derive(Clone, Debug, Default)]
pub struct TouchMultiselectState {
    pub active: bool,
    hold_row: Option<usize>,
    hold_start: Option<Instant>,
    hold_fired_multiselect: bool,
    hold_fired_detail: bool,
}

impl TouchMultiselectState {
    pub fn reset_hold(&mut self) {
        self.hold_row = None;
        self.hold_start = None;
        self.hold_fired_multiselect = false;
        self.hold_fired_detail = false;
    }

    pub fn exit_multiselect(&mut self, session: &mut FileManagerSession) {
        self.active = false;
        self.reset_hold();
        clear_selections(session);
        set_select_mode_all(session, false);
    }
}

/// Track pointer down on a file row (touch mode only).
pub fn track_row_press(
    touch: &mut TouchMultiselectState,
    row_idx: usize,
    resp: &Response,
    touch_mode: bool,
) {
    if !touch_mode {
        return;
    }
    if resp.is_pointer_button_down_on() {
        if touch.hold_row != Some(row_idx) {
            touch.hold_row = Some(row_idx);
            touch.hold_start = Some(Instant::now());
            touch.hold_fired_multiselect = false;
            touch.hold_fired_detail = false;
        }
    } else if touch.hold_row == Some(row_idx) {
        touch.reset_hold();
    }
}

/// Advance hold timers; returns hold events when thresholds cross.
pub fn poll_row_hold(
    touch: &mut TouchMultiselectState,
    touch_mode: bool,
) -> Option<TouchHoldEvent> {
    if !touch_mode {
        return None;
    }
    let row = touch.hold_row?;
    let start = touch.hold_start?;
    let held = start.elapsed().as_secs_f32();
    if held >= DETAIL_HOLD && !touch.hold_fired_detail {
        touch.hold_fired_detail = true;
        return Some(TouchHoldEvent::ShowDetail { row });
    }
    if held >= MULTISELECT_HOLD && !touch.hold_fired_multiselect {
        touch.hold_fired_multiselect = true;
        touch.active = true;
        return Some(TouchHoldEvent::EnterMultiselect { row });
    }
    None
}

#[derive(Debug)]
pub enum TouchHoldEvent {
    EnterMultiselect { row: usize },
    ShowDetail { row: usize },
}

pub fn paint_touch_multiselect_bar(
    ui: &mut egui::Ui,
    session: &mut FileManagerSession,
    touch: &mut TouchMultiselectState,
    ops: &mut PaneOps,
    menu_state: &mut PopupMenuState,
) -> bool {
    if !touch.active {
        return false;
    }
    let labels = labels::labels();
    let (selected, total) = selection_counts(session);
    let mut dismiss = false;

    egui::Frame::NONE
        .fill(ui.visuals().panel_fill)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().spacing.item_spacing.x = tokens::space::MD;
                ui.label(format!("{selected} / {total}"));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(&labels.cancel).clicked() {
                        dismiss = true;
                    }
                    let menu_btn = ui.button(format!("{} ▾", labels.touch_menu));
                    let menu_id = menu_btn.id.with("touch_ops");
                    if menu_btn.clicked() {
                        menu_state.toggle(&menu_btn.ctx, menu_id);
                    }
                    let width_labels: Vec<&str> = if selected <= 1 {
                        vec![
                            labels.open.as_str(),
                            labels.file_info.as_str(),
                            labels.rename.as_str(),
                            labels.copy.as_str(),
                            labels.cut.as_str(),
                            labels.delete.as_str(),
                        ]
                    } else {
                        vec![
                            labels.copy.as_str(),
                            labels.cut.as_str(),
                            labels.delete.as_str(),
                        ]
                    };
                    let menu_width = Some(measure_menu_width(&menu_btn.ctx, &width_labels, false));
                    popup_from_response(&menu_btn, menu_id, menu_state, menu_width, |ui| {
                        paint_touch_ops_menu(ui, session, selected, ops, &mut dismiss);
                    });
                });

                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let indices: Vec<usize> = active_selected(session).into_iter().collect();
                    if !indices.is_empty() {
                        if icon_btn(ui, Icon::Sessions, &labels.copy).clicked() {
                            ops.bulk_copy = Some(indices.clone());
                        }
                        if icon_btn(ui, Icon::Sessions, &labels.cut).clicked() {
                            ops.bulk_cut = Some(indices);
                        }
                    }
                });
            });
        });

    dismiss
}

fn icon_btn(ui: &mut Ui, icon: Icon, tip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(tokens::size::TOOLBAR_WIDTH, tokens::size::TOOLBAR_HEIGHT),
        egui::Sense::click(),
    );
    vector_icons::paint(
        ui,
        rect,
        icon,
        ui.visuals().text_color(),
        tokens::stroke::EMPHASIS,
    );
    resp.on_hover_text(tip)
}

fn paint_touch_ops_menu(
    ui: &mut Ui,
    session: &FileManagerSession,
    selected_count: usize,
    ops: &mut PaneOps,
    dismiss: &mut bool,
) {
    let labels = labels::labels();
    let indices: Vec<usize> = active_selected(session).into_iter().collect();
    if selected_count <= 1 {
        if menu_action(ui, &labels.open) {
            if let Some(&idx) = indices.first() {
                ops.open_index = Some(idx);
            }
            *dismiss = true;
        }
        if menu_action(ui, &labels.file_info)
            && let Some(&idx) = indices.first()
        {
            ops.info_index = Some(idx);
        }
        if menu_action(ui, &labels.rename) {
            if let Some(&idx) = indices.first() {
                ops.rename_index = Some(idx);
            }
            *dismiss = true;
        }
    }
    if menu_action(ui, &labels.copy) && !indices.is_empty() {
        ops.bulk_copy = Some(indices.clone());
        *dismiss = true;
    }
    if menu_action(ui, &labels.cut) && !indices.is_empty() {
        ops.bulk_cut = Some(indices.clone());
        *dismiss = true;
    }
    if menu_action(ui, &labels.delete) && !indices.is_empty() {
        ops.bulk_delete = Some(indices);
        *dismiss = true;
    }
}

fn selection_counts(session: &FileManagerSession) -> (usize, usize) {
    match session.active_pane {
        FileActivePane::Remote => session
            .remote
            .as_ref()
            .map(|r| (r.selected.len(), r.entries.len()))
            .unwrap_or((0, 0)),
        FileActivePane::LeftLocal => session
            .left_local
            .as_ref()
            .map(|p| (p.selected.len(), p.entries.len()))
            .unwrap_or((0, 0)),
        FileActivePane::Right => (session.right.selected.len(), session.right.entries.len()),
    }
}

fn active_selected(session: &FileManagerSession) -> HashSet<usize> {
    match session.active_pane {
        FileActivePane::Remote => session
            .remote
            .as_ref()
            .map(|r| r.selected.clone())
            .unwrap_or_default(),
        FileActivePane::LeftLocal => session
            .left_local
            .as_ref()
            .map(|p| p.selected.clone())
            .unwrap_or_default(),
        FileActivePane::Right => session.right.selected.clone(),
    }
}

fn clear_selections(session: &mut FileManagerSession) {
    if let Some(r) = session.remote.as_mut() {
        r.selected.clear();
    }
    if let Some(l) = session.left_local.as_mut() {
        l.selected.clear();
    }
    session.right.selected.clear();
}

fn set_select_mode_all(session: &mut FileManagerSession, on: bool) {
    if let Some(r) = session.remote.as_mut() {
        r.select_mode = on;
    }
    if let Some(l) = session.left_local.as_mut() {
        l.select_mode = on;
    }
    session.right.select_mode = on;
}

pub fn enter_multiselect_on_row(session: &mut FileManagerSession, row: usize) {
    set_select_mode_all(session, true);
    clear_selections(session);
    match session.active_pane {
        FileActivePane::Remote => {
            if let Some(r) = session.remote.as_mut() {
                r.selected.insert(row);
                r.focus_index = Some(row);
            }
        }
        FileActivePane::LeftLocal => {
            if let Some(p) = session.left_local.as_mut() {
                p.selected.insert(row);
                p.focus_index = Some(row);
            }
        }
        FileActivePane::Right => {
            session.right.selected.insert(row);
            session.right.focus_index = Some(row);
        }
    }
}

pub fn show_row_detail_panel(
    hover: &mut HoverPanelState,
    anchor: Pos2,
    detail: HoverDetail,
    touch: &mut TouchMultiselectState,
    session: &mut FileManagerSession,
) {
    hover.show_persistent(anchor, detail);
    if touch.active {
        touch.exit_multiselect(session);
    }
}
