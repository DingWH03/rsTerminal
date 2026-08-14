use egui::Key;

use rsterm_fs::local;
use rsterm_fs::sftp::join_remote;
use rsterm_session_core::{FileActivePane, FileManagerSession, InfoLine};
use rsterm_uiframe::style;
use rsterm_uiframe::tokens;

use crate::labels;

pub(super) fn show_info_dialog(ctx: &egui::Context, session: &mut FileManagerSession) {
    if !session.info_dialog.open {
        return;
    }

    use rsterm_uiframe::{DialogFrame, DialogOutcome};

    let mut close = false;
    let labels = labels::labels();
    let frame = DialogFrame::new(labels.file_info.clone()).size(420.0, 360.0);
    if frame.show(ctx, "file_info_dialog", |ui| {
        egui::Grid::new("file_info_grid")
            .num_columns(2)
            .spacing([tokens::space::XL, tokens::space::MD])
            .show(ui, |ui| {
                for InfoLine(key, value) in &session.info_dialog.lines {
                    ui.label(egui::RichText::new(key).strong());
                    ui.label(value);
                    ui.end_row();
                }
            });
        ui.add_space(tokens::space::XL);
        let close_btn = egui::Button::new(&labels.close)
            .corner_radius(style::CORNER_RADIUS_SM)
            .min_size(egui::vec2(80.0, tokens::size::BUTTON));
        if ui.add(close_btn).clicked() {
            close = true;
        }
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            close = true;
        }
    }) == DialogOutcome::Closed
    {
        close = true;
    }

    if close {
        session.info_dialog.open = false;
    }
}

pub(super) fn show_rename_dialog(ctx: &egui::Context, session: &mut FileManagerSession) {
    if !session.rename_dialog.open {
        return;
    }

    use rsterm_uiframe::{DialogFrame, DialogOutcome};

    let mut close = false;
    let mut confirm = false;
    let labels = labels::labels();

    let frame = DialogFrame::alert(labels.rename.clone()).size(360.0, 220.0);
    if frame.show(ctx, "file_rename_dialog", |ui| {
        ui.label(format!(
            "{} {}",
            labels.original_name,
            session.rename_dialog.old_name()
        ));
        ui.add_space(tokens::space::MD);
        ui.label(&labels.new_name);
        let name_edit = ui.text_edit_singleline(&mut session.rename_dialog.new_name);
        name_edit.request_focus();
        ui.add_space(tokens::space::XL);
        ui.horizontal(|ui| {
            let cancel_btn = egui::Button::new(&labels.cancel)
                .corner_radius(style::CORNER_RADIUS_SM)
                .min_size(egui::vec2(80.0, tokens::size::BUTTON));
            if ui.add(cancel_btn).clicked() {
                close = true;
            }
            let confirm_btn = style::primary_button(&labels.confirm)
                .min_size(egui::vec2(90.0, tokens::size::BUTTON));
            if ui.add(confirm_btn).clicked() {
                confirm = true;
            }
        });
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            close = true;
        }
        if ui.input(|i| i.key_pressed(Key::Enter)) {
            confirm = true;
        }
    }) == DialogOutcome::Closed
    {
        close = true;
    }

    if close {
        session.rename_dialog.open = false;
        return;
    }

    if confirm {
        let pane = session.rename_dialog.pane;
        let old_name = session.rename_dialog.old_name().to_string();
        let new_name = session.rename_dialog.new_name.trim().to_string();
        match apply_rename(session, pane, &old_name, &new_name) {
            Ok(()) => {
                session.status = Some(format!("Renamed \"{old_name}\" → \"{new_name}\""));
                session.rename_dialog.open = false;
            }
            Err(e) => session.status = Some(e),
        }
    }
}

fn apply_rename(
    session: &mut FileManagerSession,
    pane: FileActivePane,
    old_name: &str,
    new_name: &str,
) -> Result<(), String> {
    match pane {
        FileActivePane::Remote => {
            let remote = session.remote.as_mut().ok_or("No remote pane")?;
            let from = join_remote(&remote.cwd, old_name);
            let to = join_remote(&remote.cwd, new_name);
            remote.client.rename(&from, &to)?;
            remote.loading = true;
        }
        FileActivePane::LeftLocal => {
            let pane = session.left_local.as_mut().ok_or("No left pane")?;
            local::rename_entry(&pane.cwd, old_name, new_name)?;
            pane.loading = true;
        }
        FileActivePane::Right => {
            local::rename_entry(&session.right.cwd, old_name, new_name)?;
            session.right.loading = true;
        }
    }
    Ok(())
}
