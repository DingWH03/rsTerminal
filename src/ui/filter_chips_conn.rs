//! Connection-type filter chips (app-owned; uses rust_i18n).

use crate::data::persist::types::ConnectionType;
use crate::ui::uiframe::components::filter_chips::FilterChipItem;

/// 连接类型筛选标签（文案随当前 locale）。
pub fn connection_type_filters() -> Vec<FilterChipItem<ConnectionType>> {
    vec![
        FilterChipItem {
            label: rust_i18n::t!("filter_all").into_owned(),
            value: None,
        },
        FilterChipItem {
            label: rust_i18n::t!("filter_local").into_owned(),
            value: Some(ConnectionType::Local),
        },
        FilterChipItem {
            label: rust_i18n::t!("filter_ssh").into_owned(),
            value: Some(ConnectionType::Ssh),
        },
        FilterChipItem {
            label: rust_i18n::t!("filter_serial").into_owned(),
            value: Some(ConnectionType::Serial),
        },
        FilterChipItem {
            label: rust_i18n::t!("filter_ble").into_owned(),
            value: Some(ConnectionType::Ble),
        },
    ]
}
