//! Connection-type filter chips (labels via shell i18n bridge).

use rsterm_data::persist::types::ConnectionType;
use crate::uiframe::components::filter_chips::FilterChipItem;

/// 连接类型筛选标签（文案随当前 locale）。
pub fn connection_type_filters() -> Vec<FilterChipItem<ConnectionType>> {
    vec![
        FilterChipItem {
            label: crate::i18n_bridge::tr("filter_all"),
            value: None,
        },
        FilterChipItem {
            label: crate::i18n_bridge::tr("filter_local"),
            value: Some(ConnectionType::Local),
        },
        FilterChipItem {
            label: crate::i18n_bridge::tr("filter_ssh"),
            value: Some(ConnectionType::Ssh),
        },
        FilterChipItem {
            label: crate::i18n_bridge::tr("filter_serial"),
            value: Some(ConnectionType::Serial),
        },
        FilterChipItem {
            label: crate::i18n_bridge::tr("filter_ble"),
            value: Some(ConnectionType::Ble),
        },
    ]
}
