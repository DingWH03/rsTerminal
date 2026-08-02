use crate::ui::page::dialogs::{
    AuthUserDialog, FavoriteCommandDialog, LocalTerminalSettingsDialog,
    ManageFavoriteCommandsDialog, NewConnectionDialog, ProfileDialog,
};

#[derive(Default)]
pub struct AppDialogs {
    pub new_conn: NewConnectionDialog,
    pub local_term: LocalTerminalSettingsDialog,
    pub favorite_cmd: FavoriteCommandDialog,
    pub manage_commands: ManageFavoriteCommandsDialog,
    pub auth_user: AuthUserDialog,
    pub profile: ProfileDialog,
}
