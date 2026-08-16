//! Shared overflow (⋮) menu open/dismiss helpers for dense list rows.

use egui::{Id, Response, Ui};

use super::compact_list_row::paint_overflow_dots;

#[derive(Default)]
pub struct OverflowMenuState {
    pub open_id: Option<String>,
}

impl OverflowMenuState {
    pub fn load(ui: &Ui, key: Id) -> Self {
        Self {
            open_id: ui.data(|d| d.get_temp::<Option<String>>(key)).flatten(),
        }
    }

    pub fn store(&self, ui: &mut Ui, key: Id) {
        ui.data_mut(|d| d.insert_temp(key, self.open_id.clone()));
    }

    pub fn is_open(&self, id: &str) -> bool {
        self.open_id.as_deref() == Some(id)
    }

    pub fn close(&mut self) {
        self.open_id = None;
    }

    pub fn open(&mut self, id: String) {
        self.open_id = Some(id);
    }
}

/// Paint ⋮ glyph and manage click / long-press open semantics.
pub fn overflow_trigger(
    ui: &mut Ui,
    dots_resp: &Response,
    row_resp: &Response,
    item_id: &str,
    state: &mut OverflowMenuState,
) {
    paint_overflow_dots(ui, dots_resp.rect, dots_resp.hovered());
    if row_resp.long_touched() || dots_resp.clicked() {
        state.open(item_id.to_string());
    }
}

/// Show popup content when this item's overflow menu is open.
/// Returns whether the popup is still open after this frame.
pub fn show_if_open(
    _ui: &mut Ui,
    dots_resp: &Response,
    dots_id: Id,
    item_id: &str,
    state: &mut OverflowMenuState,
    min_width: f32,
    add_contents: impl FnOnce(&mut Ui),
) -> bool {
    if !state.is_open(item_id) {
        return false;
    }
    let open = egui::Popup::from_response(dots_resp)
        .id(dots_id.with("overflow_popup"))
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_min_width(min_width);
            add_contents(ui);
        })
        .is_some();
    if !open {
        state.close();
    }
    open
}
