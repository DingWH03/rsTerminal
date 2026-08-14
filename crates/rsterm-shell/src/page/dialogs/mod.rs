//! 新建/编辑连接对话框。
//!
//! 支持四种连接类型（Local、SSH、Serial、BLE）的创建和编辑。
//! 包含自动扫描串口设备和 BLE 设备的功能。
//! 以居中弹出 Window 显示；窗口内顶部用下拉框选择类型，下方按类型显示配置。

pub mod auth_user;
pub mod favorite_commands;
pub mod notices;
pub mod profile;

mod connection_form;
mod local_terminal_settings;

pub use auth_user::{
    AuthUserDialog, ManageAuthUsersAction, auth_users_page, manage_auth_users_dialog,
};
pub use favorite_commands::{
    FavoriteCommandDialog, FavoriteCommandOutcome, ManageCommandsAction,
    ManageFavoriteCommandsDialog,
};
pub use notices::{paint_connection_notice, paint_quit_confirm};
pub use profile::{ProfileDialog, ProfileDialogOutcome};

pub use connection_form::{ConnectionFormOutcome, NewConnectionDialog};
pub use local_terminal_settings::{LocalTerminalSettingsApply, LocalTerminalSettingsDialog};
