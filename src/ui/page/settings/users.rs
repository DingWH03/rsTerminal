//! Users settings page — reuses auth users list body.

use crate::ui::page::dialogs::auth_users_page;
use crate::ui::page::settings::SettingsPageCtx;

pub fn page(ui: &mut egui::Ui, ctx: &mut SettingsPageCtx<'_>) {
    auth_users_page(ui, ctx.auth_users, &mut ctx.action.auth_users);
}
