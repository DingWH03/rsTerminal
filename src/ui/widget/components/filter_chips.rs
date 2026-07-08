//! 筛选标签组件 — 用于按类型过滤连接列表。
//!
//! 提供统一的筛选标签栏，支持单选筛选和"全部"选项。
//! 当前用于首页和侧边栏的连接列表。

use crate::storage::types::ConnectionType;

/// 筛选标签的配置项。
pub struct FilterChipItem<T> {
    /// 标签显示文本
    pub label: &'static str,
    /// 对应的筛选值（`None` 表示"全部"）
    pub value: Option<T>,
}

/// 连接类型筛选标签的预定义列表。
pub const CONNECTION_TYPE_FILTERS: &[FilterChipItem<ConnectionType>] = &[
    FilterChipItem { label: "All", value: None },
    FilterChipItem { label: "Local", value: Some(ConnectionType::Local) },
    FilterChipItem { label: "SSH", value: Some(ConnectionType::Ssh) },
    FilterChipItem { label: "Serial", value: Some(ConnectionType::Serial) },
    FilterChipItem { label: "BLE", value: Some(ConnectionType::Ble) },
];

/// 渲染筛选标签栏。
///
/// 使用 `id_salt` 区分不同位置的筛选状态（首页 vs 侧边栏）。
/// 返回当前选中的筛选值。
///
/// 注意：`T` 需要 `Clone + Send + Sync + 'static` 以存入 egui 的临时数据存储。
pub fn paint_filter_chips<T: PartialEq + Clone + Send + Sync + 'static>(
    ui: &mut egui::Ui,
    id_salt: &str,
    chips: &[FilterChipItem<T>],
) -> Option<T> {
    let current: Option<T> = ui
        .data(|d| d.get_temp(egui::Id::new(id_salt)))
        .unwrap_or(None);

    ui.horizontal(|ui| {
        ui.style_mut().spacing.item_spacing.x = 4.0;
        for chip in chips {
            let selected = current.as_ref() == chip.value.as_ref();
            if ui.selectable_label(selected, chip.label).clicked() {
                ui.data_mut(|d| d.insert_temp(egui::Id::new(id_salt), chip.value.clone()));
            }
        }
    });

    current
}
