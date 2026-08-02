use super::{android_ime_for_text_edit, labeled_row, text_edit};

/// Labeled single-line text field (control fills remaining width).
pub fn labeled_text(
    ui: &mut egui::Ui,
    label: impl Into<String>,
    text: &mut String,
) -> egui::Response {
    let mut resp = None;
    labeled_row(ui, label, |ui| {
        resp = Some(text_edit(ui, text));
    });
    resp.expect("labeled_row always runs closure")
}

/// Labeled password field.
pub fn labeled_password(
    ui: &mut egui::Ui,
    label: impl Into<String>,
    text: &mut String,
) -> egui::Response {
    let mut resp = None;
    labeled_row(ui, label, |ui| {
        let w = ui.available_width();
        let r = ui.add(
            egui::TextEdit::singleline(text)
                .password(true)
                .desired_width(w),
        );
        android_ime_for_text_edit(ui, &r, false);
        resp = Some(r);
    });
    resp.expect("labeled_row always runs closure")
}

/// Labeled multiline text field.
pub fn labeled_multiline(
    ui: &mut egui::Ui,
    label: impl Into<String>,
    text: &mut String,
    rows: usize,
) -> egui::Response {
    let mut resp = None;
    labeled_row(ui, label, |ui| {
        let w = ui.available_width();
        let r = ui.add(
            egui::TextEdit::multiline(text)
                .desired_rows(rows)
                .desired_width(w),
        );
        android_ime_for_text_edit(ui, &r, false);
        resp = Some(r);
    });
    resp.expect("labeled_row always runs closure")
}

/// Labeled combo that fills the remaining row width.
pub fn labeled_combo(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    label: impl Into<String>,
    selected_text: impl Into<egui::WidgetText>,
    add_options: impl FnOnce(&mut egui::Ui),
) {
    labeled_row(ui, label, |ui| {
        let w = ui.available_width();
        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(selected_text)
            .width(w)
            .show_ui(ui, add_options);
    });
}

/// Labeled combo with explicit width (rare; prefer [`labeled_combo`]).
pub fn labeled_combo_width(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    label: impl Into<String>,
    selected_text: impl Into<egui::WidgetText>,
    width: f32,
    add_options: impl FnOnce(&mut egui::Ui),
) {
    labeled_row(ui, label, |ui| {
        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(selected_text)
            .width(width)
            .show_ui(ui, add_options);
    });
}

/// Labeled numeric drag value.
pub fn labeled_number<Num: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    label: impl Into<String>,
    value: &mut Num,
    range: std::ops::RangeInclusive<Num>,
) {
    labeled_row(ui, label, |ui| {
        ui.add(egui::DragValue::new(value).range(range));
    });
}

/// Labeled slider that stretches across the remaining width.
pub fn labeled_slider<Num: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    label: impl Into<String>,
    value: &mut Num,
    range: std::ops::RangeInclusive<Num>,
) {
    labeled_row(ui, label, |ui| {
        let w = ui.available_width().max(80.0);
        ui.add_sized(
            [w, ui.spacing().interact_size.y],
            egui::Slider::new(value, range),
        );
    });
}

/// Labeled checkbox (label on the left column; checkbox text optional).
pub fn labeled_checkbox(
    ui: &mut egui::Ui,
    label: impl Into<String>,
    checked: &mut bool,
    checkbox_text: impl Into<String>,
) -> egui::Response {
    let mut resp = None;
    let checkbox_text = checkbox_text.into();
    labeled_row(ui, label, |ui| {
        resp = Some(ui.checkbox(checked, checkbox_text));
    });
    resp.expect("labeled_row always runs closure")
}

/// Simple checkbox without a left label column (full-width row).
pub fn checkbox_row(
    ui: &mut egui::Ui,
    checked: &mut bool,
    text: impl Into<String>,
) -> egui::Response {
    let resp = ui.checkbox(checked, text.into());
    ui.add_space(super::FIELD_GAP);
    resp
}

/// Horizontal segmented / selectable_value group with a left label.
pub fn segmented_row<T: PartialEq + Clone>(
    ui: &mut egui::Ui,
    label: impl Into<String>,
    value: &mut T,
    options: impl IntoIterator<Item = (T, String)>,
) {
    labeled_row(ui, label, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (opt, opt_label) in options {
                ui.selectable_value(value, opt, opt_label);
            }
        });
    });
}

/// Color edit button on a labeled row. `rgb` is 0..1 channels; caller maps to domain color.
pub fn labeled_color_rgb(ui: &mut egui::Ui, label: impl Into<String>, rgb: &mut [f32; 3]) {
    labeled_row(ui, label, |ui| {
        ui.color_edit_button_rgb(rgb);
    });
}
