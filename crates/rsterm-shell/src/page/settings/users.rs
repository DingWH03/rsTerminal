//! Users settings page — reuses auth users list body.

use crate::page::dialogs::auth_users_page;
use crate::page::settings::SettingsPageCtx;

pub fn page(ui: &mut egui::Ui, ctx: &mut SettingsPageCtx<'_>) {
    auth_users_page(ui, ctx.auth_users, &mut ctx.action.auth_users);
}
