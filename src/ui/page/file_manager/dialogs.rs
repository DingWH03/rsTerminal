use egui::Key;

use crate::fs::local;
use crate::fs::sftp::join_remote;
use crate::session::{FileActivePane, FileManagerSession};

pub(super) fn show_info_dialog(ctx: &egui::Context, session: &mut FileManagerSession) {
    if !session.info_dialog.open {
        return;
    }

    use crate::ui::uiframe::{DialogFrame, DialogOutcome};

    let mut close = false;
    let frame = DialogFrame::new("Info").size(420.0, 360.0);
    if frame.show(ctx, "file_info_dialog", |ui| {
        egui::Grid::new("file_info_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                for crate::session::InfoLine(key, value) in &session.info_dialog.lines {
                    ui.label(egui::RichText::new(key).strong());
                    ui.label(value);
                    ui.end_row();
                }
            });
        ui.add_space(12.0);
        if ui.button(rust_i18n::t!("close")).clicked() {
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

    use crate::ui::uiframe::{DialogFrame, DialogOutcome};

    let mut close = false;
    let mut confirm = false;

    let frame = DialogFrame::alert("Rename").size(360.0, 220.0);
    if frame.show(ctx, "file_rename_dialog", |ui| {
        ui.label(format!("Original: {}", session.rename_dialog.old_name()));
        ui.add_space(6.0);
        ui.label("New name:");
        let name_edit = ui.text_edit_singleline(&mut session.rename_dialog.new_name);
        name_edit.request_focus();
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button(rust_i18n::t!("cancel")).clicked() {
                close = true;
            }
            if ui.button("Confirm").clicked() {
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
