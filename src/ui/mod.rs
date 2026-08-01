//! UI 层 — 整个应用的界面模块。
//!
//! ## 分层
//!
//! - [`uiframe`]：可复用图形框架（样式、图标、对话框外壳、声明式菜单栏、Tab、列表等）
//! - [`shell`]：应用外壳（布局、顶栏菜单逻辑、分屏协调）
//! - [`function_pane`] / [`workspace_pane`]：左右业务区域
//! - [`page`]：全屏页面与业务对话框
//!
//! 业务逻辑不要写进 `uiframe`；框架只提供可组合的 chrome 与控件。

pub mod function_pane;
pub mod page;
pub mod pane_colors;
pub mod shell;
pub mod theme_color;
pub mod uiframe;
pub mod workspace_pane;
