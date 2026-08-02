//! Imperative form helpers for settings pages and business dialogs.
//!
//! Compose these primitives from `page/*`; keep business DTO logic out of this module.

mod fields;
mod list;

pub use fields::*;
pub use list::*;

use super::{interactive, style, tokens};

/// Fixed label column width (left side of labeled rows).
pub const LABEL_WIDTH: f32 = 108.0;
/// Vertical gap between consecutive fields.
pub const FIELD_GAP: f32 = tokens::space::MD;
/// Gap before/after section separators.
pub const SECTION_GAP: f32 = tokens::space::LG;
/// Space above dialog Cancel/Save row.
pub const FOOTER_GAP: f32 = tokens::space::XL;
/// Dialog action button height.
pub const BTN_H: f32 = tokens::size::BUTTON;
/// Cancel button min width.
pub const BTN_W_CANCEL: f32 = 88.0;
/// Primary/save button min width.
pub const BTN_W_PRIMARY: f32 = 96.0;

/// Kept for call sites that still pass an explicit combo width.
pub const COMBO_WIDTH: f32 = 220.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FooterAction {
    #[default]
    None,
    Cancel,
    Save,
}

/// Theme-aware section card (uses panel/extreme bg — never hard-coded dark SURFACE).
pub fn section_frame_themed(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(style::CORNER_RADIUS_SM)
        .inner_margin(egui::Margin::symmetric(12, 10))
}

/// Card section with optional title. Prefer no title when the parent tab already names the page.
pub fn section(
    ui: &mut egui::Ui,
    title: impl Into<String>,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let title = title.into();
    section_frame_themed(ui).show(ui, |ui| {
        if !title.is_empty() {
            ui.label(egui::RichText::new(title).size(14.0).strong());
            ui.add_space(8.0);
        }
        ui.set_min_width(ui.available_width());
        add_contents(ui);
    });
}

/// Untitled themed card — full-width settings group.
pub fn section_card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    section(ui, "", add_contents);
}

/// In-dialog / in-card subsection header without an outer card.
pub fn section_header(ui: &mut egui::Ui, title: impl Into<String>) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(title.into()).size(12.5).strong());
    ui.add_space(4.0);
}

/// Left fixed-width label + right content that **fills remaining width**.
pub fn labeled_row(
    ui: &mut egui::Ui,
    label: impl Into<String>,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let label = label.into();
    let full_w = ui.available_width();
    ui.allocate_ui_with_layout(
        egui::vec2(full_w, 0.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(full_w);
            ui.add_sized(
                [LABEL_WIDTH, ui.spacing().interact_size.y],
                egui::Label::new(egui::RichText::new(label).size(13.0)).truncate(),
            );
            let remaining = ui.available_width().max(40.0);
            ui.allocate_ui_with_layout(
                egui::vec2(remaining, 0.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_min_width(remaining);
                    ui.spacing_mut().item_spacing.x = 6.0;
                    add_contents(ui);
                },
            );
        },
    );
    ui.add_space(FIELD_GAP);
}

/// Compact equal-width text tab strip that fills the available width.
///
/// Returns the selected tab after handling clicks.
pub fn text_tab_bar<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    tabs: &[T],
    active: T,
    label_fn: impl Fn(T) -> String,
) -> T {
    let mut selected = active;
    let n = tabs.len().max(1) as f32;
    let gap = 4.0;
    let total_w = ui.available_width();
    let tab_w = ((total_w - gap * (n - 1.0)) / n).max(48.0);
    let tab_h = 28.0;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for &tab in tabs {
            let is_sel = selected == tab;
            let state = if is_sel {
                interactive::RowState::Selected
            } else {
                interactive::RowState::Default
            };
            let chrome = interactive::row_chrome(ui, state);
            let fill = if is_sel {
                chrome.fill
            } else {
                ui.visuals().widgets.inactive.bg_fill
            };
            let text_color = if is_sel {
                ui.visuals().strong_text_color()
            } else {
                ui.visuals().weak_text_color()
            };
            let btn = egui::Button::new(
                egui::RichText::new(label_fn(tab))
                    .size(12.5)
                    .color(text_color)
                    .strong(),
            )
            .fill(fill)
            .stroke(if is_sel {
                chrome.stroke
            } else {
                ui.visuals().widgets.noninteractive.bg_stroke
            })
            .corner_radius(style::CORNER_RADIUS_SM)
            .min_size(egui::vec2(tab_w, tab_h));
            if ui.add(btn).clicked() {
                selected = tab;
            }
        }
    });
    ui.add_space(6.0);
    selected
}

/// Cancel + primary Save/Create footer.
pub fn dialog_footer(
    ui: &mut egui::Ui,
    cancel: impl Into<String>,
    save: impl Into<String>,
    can_save: bool,
) -> FooterAction {
    let cancel = cancel.into();
    let save = save.into();
    ui.add_space(FOOTER_GAP);
    let mut action = FooterAction::None;
    ui.horizontal(|ui| {
        let cancel_btn = egui::Button::new(egui::RichText::new(cancel).size(13.5))
            .min_size(egui::vec2(BTN_W_CANCEL, BTN_H))
            .corner_radius(style::CORNER_RADIUS_SM);
        if ui.add(cancel_btn).clicked() {
            action = FooterAction::Cancel;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let save_btn = style::primary_button(&save).min_size(egui::vec2(BTN_W_PRIMARY, BTN_H));
            if ui.add_enabled(can_save, save_btn).clicked() {
                action = FooterAction::Save;
            }
        });
    });
    action
}

/// Prepare Android soft keyboard for a focused text field.
pub fn android_ime_for_text_edit(ui: &egui::Ui, resp: &egui::Response, force: bool) {
    #[cfg(target_os = "android")]
    {
        if force || resp.gained_focus() || resp.clicked() {
            crate::platform::android_ime::prepare_text_field_ime(ui.ctx(), resp.rect);
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (ui, resp, force);
    }
}

/// Single-line text edit with Android IME hook — fills available width.
pub fn text_edit(ui: &mut egui::Ui, text: &mut String) -> egui::Response {
    let w = ui.available_width();
    let resp = ui.add(egui::TextEdit::singleline(text).desired_width(w));
    android_ime_for_text_edit(ui, &resp, false);
    resp
}
