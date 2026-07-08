//! 可复用 UI 组件模块。
//!
//! 提供应用范围内通用的界面组件：
//! - `clipboard`：系统剪贴板读写
//! - `components`：通用 UI 组件（卡片、图标、筛选标签、空状态、工具栏按钮等）
//! - `dialogs`：新建/编辑连接对话框
//! - `keyboard`：虚拟键盘（特殊键和全键盘模式）
//! - `sidebar`：响应式侧边栏系统
//! - `style`：设计系统（颜色、圆角、卡片、按钮样式）

/// 系统剪贴板读写封装。
pub mod clipboard;
/// 通用 UI 组件集合（卡片、图标、筛选标签、空状态、工具栏按钮等）。
pub mod components;
/// 新建/编辑连接对话框。
pub mod dialogs;
/// 虚拟键盘（特殊键模式 + 全键盘模式）。
pub mod keyboard;
/// 响应式侧边栏（宽屏停靠/窄屏覆盖）。
pub mod sidebar;
/// 设计系统 — 颜色、圆角、卡片、按钮等共享样式常量。
pub mod style;
