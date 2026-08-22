//! Settings — nested pages: General / Appearance / Terminal / Users.

mod appearance;
mod general;
mod terminal;
mod users;

use crate::page::dialogs::ManageAuthUsersAction;
use crate::uiframe::form;
use rsterm_data::persist::types::{AuthUser, TerminalProfile};
use rsterm_data::prefs::Prefs;

/// Deep-link target inside the settings dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SettingsPath {
    #[default]
    Root,
    AppearanceLayoutFileManager,
}

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
    pub const ALL: [Self; 4] = [Self::General, Self::Appearance, Self::Terminal, Self::Users];

    pub fn label(self) -> String {
        match self {
            Self::General => crate::i18n_bridge::tr("settings_tab_general"),
            Self::Appearance => crate::i18n_bridge::tr("settings_tab_appearance"),
            Self::Terminal => crate::i18n_bridge::tr("settings_tab_terminal"),
            Self::Users => crate::i18n_bridge::tr("settings_tab_users"),
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
    initial_path: Option<SettingsPath>,
) -> (bool, SettingsUiAction) {
    use crate::uiframe::{DialogFrame, DialogOutcome};

    if let Some(path) = initial_path {
        apply_settings_path(ctx, path);
    }

    let mut action = SettingsUiAction::default();
    let frame = DialogFrame::new(crate::i18n_bridge::tr("settings"));
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
    use crate::uiframe::{DialogFrame, DialogOutcome};

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

pub fn apply_settings_path(ctx: &egui::Context, path: SettingsPath) {
    let tab_id = egui::Id::new("settings_tab_v5");
    match path {
        SettingsPath::Root => {}
        SettingsPath::AppearanceLayoutFileManager => {
            ctx.memory_mut(|m| {
                *m.data.get_temp_mut_or_default::<SettingsTab>(tab_id) = SettingsTab::Appearance;
            });
        }
    }
}

fn settings_body(ui: &mut egui::Ui, ctx: &mut SettingsPageCtx<'_>, forced: Option<SettingsTab>) {
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
