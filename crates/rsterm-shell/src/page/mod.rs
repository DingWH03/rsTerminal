//! 页面模块 — 应用的全屏页面与业务对话框。
//!
//! - `home`：首页，显示已保存的连接列表和快捷操作。
//! - `settings`：设置页面。
//! - `dialogs`：新建/编辑连接、本地终端设置等业务对话框。
//!
//! Terminal / file-manager pane pages live in their page crates.

pub mod dialogs;
/// 首页模块 — 连接管理、最近连接、收藏夹。
pub mod home;
/// 设置页面模块 — 应用配置、主题、键盘、行为等。
pub mod settings;

/// Terminal simulation page — re-export from `rsterm-page-terminal`.
pub mod terminal {
    pub use rsterm_page_terminal::connection_view;
    pub use rsterm_page_terminal::page::*;
}

/// File manager — re-export from `rsterm-page-file-manager`.
pub mod file_manager {
    pub use rsterm_page_file_manager::page::*;
    pub use rsterm_page_file_manager::{FileManagerAction, file_manager_view};
}
