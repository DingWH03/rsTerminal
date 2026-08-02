//! Settings — four nested pages: General / Appearance / Terminal / Users.
//!
//! Each tab body can embed in the full Settings dialog or open via
//! [`settings_page_dialog`].

mod appearance;
mod general;
mod terminal;
mod users;

use crate::data::persist::types::{AuthUser, TerminalProfile};
use crate::data::prefs::Prefs;
use crate::ui::page::dialogs::ManageAuthUsersAction;
use crate::ui::uiframe::form;

/// Settings tab identifiers (also openable as standalone dialogs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SettingsTab {
    #[default]
    General,
    Appearance,
    Terminal,
    Users,
}

impl SettingsTab {
    pub const ALL: [Self; 4] = [
        Self::General,
        Self::Appearance,
        Self::Terminal,
        Self::Users,
    ];

    pub fn label(self) -> String {
        match self {
            Self::General => rust_i18n::t!("settings_tab_general").into_owned(),
            Self::Appearance => rust_i18n::t!("settings_tab_appearance").into_owned(),
            Self::Terminal => rust_i18n::t!("settings_tab_terminal").into_owned(),
            Self::Users => rust_i18n::t!("settings_tab_users").into_owned(),
        }
    }
}

/// Nested actions from settings pages.
#[derive(Debug, Default)]
pub struct SettingsUiAction {
    pub auth_users: ManageAuthUsersAction,
    pub request_new_profile: bool,
    pub request_edit_profile: Option<String>,
    pub delete_profile_id: Option<String>,
    pub set_default_profile_id: Option<String>,
}

/// Context shared by settings pages.
pub struct SettingsPageCtx<'a> {
    pub prefs: &'a mut Prefs,
    pub profiles: &'a [TerminalProfile],
    pub auth_users: &'a [AuthUser],
    pub action: &'a mut SettingsUiAction,
}

/// Full Settings dialog. Returns `(closed, actions)`.
pub fn settings_dialog(
    ctx: &egui::Context,
    prefs: &mut Prefs,
    profiles: &[TerminalProfile],
    auth_users: &[AuthUser],
) -> (bool, SettingsUiAction) {
    use crate::ui::uiframe::{DialogFrame, DialogOutcome};

    let mut action = SettingsUiAction::default();
    let frame = DialogFrame::new(rust_i18n::t!("settings").to_string());
    let outcome = frame.show(ctx, "settings_dialog", |ui| {
        let mut page_ctx = SettingsPageCtx {
            prefs,
            profiles,
            auth_users,
            action: &mut action,
        };
        settings_body(ui, &mut page_ctx, None);
    });
    (matches!(outcome, DialogOutcome::Closed), action)
}

/// Single-tab standalone dialog.
pub fn settings_page_dialog(
    ctx: &egui::Context,
    tab: SettingsTab,
    prefs: &mut Prefs,
    profiles: &[TerminalProfile],
    auth_users: &[AuthUser],
) -> (bool, SettingsUiAction) {
    use crate::ui::uiframe::{DialogFrame, DialogOutcome};

    let mut action = SettingsUiAction::default();
    let frame = DialogFrame::new(tab.label());
    let id = format!("settings_page_dialog_{tab:?}");
    let outcome = frame.show(ctx, id, |ui| {
        let mut page_ctx = SettingsPageCtx {
            prefs,
            profiles,
            auth_users,
            action: &mut action,
        };
        settings_body(ui, &mut page_ctx, Some(tab));
    });
    (matches!(outcome, DialogOutcome::Closed), action)
}

fn settings_body(ui: &mut egui::Ui, ctx: &mut SettingsPageCtx<'_>, forced: Option<SettingsTab>) {
    // Stable id outside scroll-child noise.
    let tab_id = egui::Id::new("settings_tab_v5");
    let mut active = forced.unwrap_or_else(|| {
        ui.ctx()
            .memory_mut(|m| *m.data.get_temp_mut_or_default::<SettingsTab>(tab_id))
    });

    if forced.is_none() {
        active = form::text_tab_bar(ui, &SettingsTab::ALL, active, SettingsTab::label);
        ui.ctx().memory_mut(|m| {
            *m.data.get_temp_mut_or_default::<SettingsTab>(tab_id) = active;
        });
    }

    match active {
        SettingsTab::General => general::page(ui, ctx),
        SettingsTab::Appearance => appearance::page(ui, ctx),
        SettingsTab::Terminal => terminal::page(ui, ctx),
        SettingsTab::Users => users::page(ui, ctx),
    }
}
