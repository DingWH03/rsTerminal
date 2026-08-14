//! Application menu bar — construction and action routing.
//!
//! UI chrome lives in [`crate::uiframe::menu_bar`]; this module owns the
//! Connection / Commands / View / Preferences / Help tree and how actions affect the shell.

use crate::function_pane::FunctionPane;
use crate::shell::ShellRenderResult;
use crate::shell::layout_state::ShellLayout;
use crate::uiframe::menu_bar::{MenuBar, MenuBarSpec, MenuEntry, MenuEntryId, MenuGroup};

/// Snapshot of shell state needed to build the View menu.
#[derive(Clone, Copy)]
pub struct AppMenuState {
    /// When false (narrow / portrait), the sidebar toggle is disabled.
    pub sidebar_toggle_enabled: bool,
    /// Whether the docked sidebar is currently open.
    pub sidebar_visible: bool,
    /// Whether the root OS window is borderless-fullscreen.
    pub fullscreen: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AppMenuAction {
    #[default]
    None,
    NewConnection,
    OpenConnections,
    NewFavoriteCommand,
    ManageFavoriteCommands,
    ToggleSidebar,
    ToggleFullscreen,
    OpenSettings,
    ManageAuthUsers,
    OpenHelp,
}

const ID_CONN_NEW: MenuEntryId = 1;
const ID_CONN_OPEN: MenuEntryId = 2;
const ID_VIEW_SIDEBAR: MenuEntryId = 3;
const ID_PREF_SETTINGS: MenuEntryId = 4;
const ID_HELP_ABOUT: MenuEntryId = 5;
const ID_CMD_NEW: MenuEntryId = 6;
const ID_CMD_MANAGE: MenuEntryId = 7;
const ID_PREF_USERS: MenuEntryId = 8;
const ID_VIEW_FULLSCREEN: MenuEntryId = 9;

/// Height of the menu bar chrome.
pub const HEIGHT: f32 = MenuBar::HEIGHT;

/// Paint the app menu bar and map the activated entry to [`AppMenuAction`].
pub fn show(ui: &mut egui::Ui, state: AppMenuState) -> AppMenuAction {
    let t_conn = crate::i18n_bridge::tr("menu_connection");
    let t_conn_new = crate::i18n_bridge::tr("menu_connection_new");
    let t_conn_open = crate::i18n_bridge::tr("menu_connection_open");
    let t_cmds = crate::i18n_bridge::tr("menu_commands");
    let t_cmd_new = crate::i18n_bridge::tr("menu_commands_new");
    let t_cmd_manage = crate::i18n_bridge::tr("menu_commands_manage");
    let t_view = crate::i18n_bridge::tr("menu_view");
    let t_sidebar = crate::i18n_bridge::tr("menu_view_sidebar");
    let t_fullscreen = crate::i18n_bridge::tr("menu_view_fullscreen");
    let t_pref = crate::i18n_bridge::tr("menu_preferences");
    let t_settings = crate::i18n_bridge::tr("menu_settings");
    let t_users = crate::i18n_bridge::tr("menu_preferences_users");
    let t_help = crate::i18n_bridge::tr("menu_help");
    let t_about = crate::i18n_bridge::tr("menu_about");

    let connection_entries = [
        MenuEntry::Button {
            id: ID_CONN_NEW,
            label: t_conn_new.as_ref(),
        },
        MenuEntry::Button {
            id: ID_CONN_OPEN,
            label: t_conn_open.as_ref(),
        },
    ];
    let command_entries = [
        MenuEntry::Button {
            id: ID_CMD_NEW,
            label: t_cmd_new.as_ref(),
        },
        MenuEntry::Button {
            id: ID_CMD_MANAGE,
            label: t_cmd_manage.as_ref(),
        },
    ];
    let view_entries = [
        MenuEntry::Checkbox {
            id: ID_VIEW_SIDEBAR,
            label: t_sidebar.as_ref(),
            checked: state.sidebar_visible,
            enabled: state.sidebar_toggle_enabled,
        },
        MenuEntry::Checkbox {
            id: ID_VIEW_FULLSCREEN,
            label: t_fullscreen.as_ref(),
            checked: state.fullscreen,
            enabled: true,
        },
    ];
    let pref_entries = [
        MenuEntry::Button {
            id: ID_PREF_SETTINGS,
            label: t_settings.as_ref(),
        },
        MenuEntry::Button {
            id: ID_PREF_USERS,
            label: t_users.as_ref(),
        },
    ];
    let help_entries = [MenuEntry::Button {
        id: ID_HELP_ABOUT,
        label: t_about.as_ref(),
    }];

    let groups = [
        MenuGroup {
            title: t_conn.as_ref(),
            entries: &connection_entries,
        },
        MenuGroup {
            title: t_view.as_ref(),
            entries: &view_entries,
        },
        MenuGroup {
            title: t_cmds.as_ref(),
            entries: &command_entries,
        },
        MenuGroup {
            title: t_pref.as_ref(),
            entries: &pref_entries,
        },
        MenuGroup {
            title: t_help.as_ref(),
            entries: &help_entries,
        },
    ];

    match MenuBar::show(ui, MenuBarSpec { groups: &groups }) {
        Some(ID_CONN_NEW) => AppMenuAction::NewConnection,
        Some(ID_CONN_OPEN) => AppMenuAction::OpenConnections,
        Some(ID_CMD_NEW) => AppMenuAction::NewFavoriteCommand,
        Some(ID_CMD_MANAGE) => AppMenuAction::ManageFavoriteCommands,
        Some(ID_VIEW_SIDEBAR) => AppMenuAction::ToggleSidebar,
        Some(ID_VIEW_FULLSCREEN) => AppMenuAction::ToggleFullscreen,
        Some(ID_PREF_SETTINGS) => AppMenuAction::OpenSettings,
        Some(ID_PREF_USERS) => AppMenuAction::ManageAuthUsers,
        Some(ID_HELP_ABOUT) => AppMenuAction::OpenHelp,
        _ => AppMenuAction::None,
    }
}

/// Apply a menu action to shell layout / function pane / render result.
pub fn apply(
    action: AppMenuAction,
    layout: &mut ShellLayout,
    function_pane: &mut FunctionPane,
    result: &mut ShellRenderResult,
) {
    match action {
        AppMenuAction::NewConnection => {
            result.function_action.new_connection = true;
        }
        AppMenuAction::OpenConnections => {
            layout.ui.connections_dialog_open = true;
        }
        AppMenuAction::NewFavoriteCommand => {
            result.function_action.new_favorite_command = true;
        }
        AppMenuAction::ManageFavoriteCommands => {
            layout.ui.commands_manage_dialog_open = true;
        }
        AppMenuAction::ToggleSidebar => {
            function_pane.toggle_docked_sidebar();
        }
        AppMenuAction::ToggleFullscreen => {
            result.toggle_app_fullscreen = true;
        }
        AppMenuAction::OpenSettings => {
            layout.ui.settings_dialog_open = true;
            result.settings_opened = true;
        }
        AppMenuAction::ManageAuthUsers => {
            layout.ui.settings_standalone_tab = Some(crate::page::settings::SettingsTab::Users);
        }
        AppMenuAction::OpenHelp => {
            layout.ui.help_dialog_open = true;
        }
        AppMenuAction::None => {}
    }
}

/// Paint the menu bar and apply the resulting action.
pub fn show_and_apply(
    ui: &mut egui::Ui,
    state: AppMenuState,
    layout: &mut ShellLayout,
    function_pane: &mut FunctionPane,
    result: &mut ShellRenderResult,
) {
    let action = show(ui, state);
    apply(action, layout, function_pane, result);
}
