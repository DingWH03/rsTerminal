use crate::{interactive, style, tokens};

/// Accent primary button used as a list-page toolbar action (New profile / New user).
pub fn accent_toolbar_button(ui: &mut egui::Ui, label: impl Into<String>) -> bool {
    let label = label.into();
    let btn = egui::Button::new(egui::RichText::new(label).color(egui::Color32::WHITE))
        .fill(style::ACCENT)
        .corner_radius(style::CORNER_RADIUS_SM)
        .min_size(egui::vec2(0.0, tokens::size::BUTTON));
    ui.add(btn).clicked()
}

/// Theme-aware list item frame; fills available width.
pub fn manage_list_item_frame(
    ui: &mut egui::Ui,
    highlighted: bool,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let state = if highlighted {
        interactive::RowState::Selected
    } else {
        interactive::RowState::Default
    };
    let chrome = interactive::card_chrome(ui, state);
    egui::Frame::new()
        .fill(chrome.fill)
        .stroke(chrome.stroke)
        .corner_radius(style::CORNER_RADIUS_XS)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add_contents(ui);
        });
}

/// Standard toolbar + separator header for manage lists.
pub fn manage_list_toolbar(ui: &mut egui::Ui, new_label: impl Into<String>) -> bool {
    let clicked = ui
        .horizontal(|ui| accent_toolbar_button(ui, new_label))
        .inner;
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(6.0);
    clicked
}
