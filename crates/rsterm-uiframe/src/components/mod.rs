//! 可复用的 UI 组件集合。
//!
//! 这些组件是从各页面中提取的通用 UI 模式，
//! 用于消除重复代码并保持界面一致性。

pub mod card;
pub mod compact_list_row;
pub mod empty_state;
pub mod filter_chips;
pub mod icon_widget;
pub mod overflow_menu;
pub mod pane_header;
pub mod popup_menu;
pub mod toolbar_button;

pub use compact_list_row::{CompactListRow, ListRowDensity};
pub use empty_state::{EmptyStateConfig, paint_empty_state};
pub use overflow_menu::OverflowMenuState;
pub use pane_header::{PaneHeader, PaneHeaderOutcome};
pub use popup_menu::{
    POPUP_MENU_MAX_WIDTH, POPUP_MENU_MIN_WIDTH, PopupMenuOutcome, PopupMenuState,
    install_context_popup, measure_menu_width, menu_action, menu_action_enabled, menu_check,
    menu_heading, menu_separator, popup_body, popup_from_response, popup_menu_content,
    show_anchor_popup,
};
