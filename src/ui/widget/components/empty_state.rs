//! 空状态组件 — 在列表为空时显示提示信息。
//!
//! 提供一个可配置的空状态视图，包含图标、主标题和副标题。
//! 用于首页无连接、侧边栏无会话、文件管理器空目录等场景。

/// 空状态视图的配置。
pub struct EmptyStateConfig<'a> {
    /// 大图标（Emoji 或 Unicode 字符）
    pub icon: &'a str,
    /// 主标题文本
    pub title: &'a str,
    /// 副标题文本（可选）
    pub subtitle: Option<&'a str>,
    /// 图标大小
    pub icon_size: f32,
    /// 标题字体大小
    pub title_size: f32,
    /// 副标题字体大小
    pub subtitle_size: f32,
}

impl<'a> Default for EmptyStateConfig<'a> {
    fn default() -> Self {
        Self {
            icon: "\u{1F4CB}",
            title: "",
            subtitle: None,
            icon_size: 36.0,
            title_size: 15.0,
            subtitle_size: 12.0,
        }
    }
}

/// 渲染空状态视图。
///
/// 在 UI 中垂直居中显示图标、标题和可选的副标题。
pub fn paint_empty_state(ui: &mut egui::Ui, config: EmptyStateConfig) {
    ui.add_space(32.0);
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new(config.icon).size(config.icon_size));
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(config.title)
                .size(config.title_size)
                .color(ui.visuals().weak_text_color()),
        );
        if let Some(sub) = config.subtitle {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(sub)
                    .size(config.subtitle_size)
                    .color(ui.visuals().weak_text_color()),
            );
        }
    });
    ui.add_space(8.0);
}
