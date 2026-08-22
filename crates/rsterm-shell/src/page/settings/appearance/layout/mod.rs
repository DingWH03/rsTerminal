//! Layout settings under Appearance.

mod file_manager;

use crate::page::settings::SettingsPageCtx;

pub fn page(ui: &mut egui::Ui, ctx: &mut SettingsPageCtx<'_>) {
    file_manager::page(ui, ctx);
}
