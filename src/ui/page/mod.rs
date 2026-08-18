//! 页面模块 — 应用的所有全屏页面与业务对话框。
//!
//! - `home`：首页，显示已保存的连接列表和快捷操作。
//! - `terminal`：终端仿真页面。
//! - `settings`：设置页面。
//! - `file_manager`：文件管理器。
//! - `dialogs`：新建/编辑连接、本地终端设置等业务对话框（非框架外壳）。

pub mod dialogs;
/// 文件管理器模块 — 本地和远程 SFTP 文件浏览、复制、移动、传输。
pub mod file_manager;
/// 首页模块 — 连接管理、最近连接、收藏夹。
pub mod home;
/// 设置页面模块 — 应用配置、主题、键盘、行为等。
pub mod settings;
/// 终端仿真页面模块 — 网格渲染、键盘输入、鼠标交互、文本选择。
pub mod terminal;
