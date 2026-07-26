//! 空状态组件 — 在列表为空时显示提示信息。
//!
//! 提供一个可配置的空状态视图，包含图标、主标题和副标题。
//! 用于首页无连接、侧边栏无会话、文件管理器空目录等场景。

use crate::ui::uiframe::vector_icons::{self, Icon};

/// 空状态视图的配置。
pub struct EmptyStateConfig<'a> {
    /// 大图标（Emoji 或 Unicode 字符）；若设置了 [`Self::vector_icon`] 则忽略。
    pub icon: &'a str,
    /// 可选矢量图标（优先于 emoji）。
    pub vector_icon: Option<Icon>,
    /// 矢量图标边长。
    pub vector_icon_size: f32,
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
            vector_icon: None,
            vector_icon_size: 40.0,
            title: "",
            subtitle: None,
            icon_size: 36.0,
            title_size: 15.0,
            subtitle_size: 12.0,
        }
    }
}

/// 渲染空状态视图（在可用区域内上下左右居中）。
pub fn paint_empty_state(ui: &mut egui::Ui, config: EmptyStateConfig) {
    let avail = ui.available_size_before_wrap();
    let (rect, _) = ui.allocate_exact_size(avail, egui::Sense::hover());

    let icon_block = if config.vector_icon.is_some() {
        config.vector_icon_size * 1.56
    } else {
        config.icon_size + 4.0
    };
    let title_block = config.title_size + 4.0;
    let sub_block = if config.subtitle.is_some() {
        config.subtitle_size + 8.0
    } else {
        0.0
    };
    let content_h = icon_block + 10.0 + title_block + sub_block;
    let top = (rect.center().y - content_h * 0.5).max(rect.top());
    let content_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left(), top),
        egui::vec2(rect.width(), content_h.min(rect.height())),
    );

    ui.scope_builder(egui::UiBuilder::new().max_rect(content_rect), |ui| {
        ui.vertical_centered(|ui| {
            if let Some(icon) = config.vector_icon {
                let (icon_rect, _) = ui.allocate_exact_size(
                    egui::vec2(config.vector_icon_size, config.vector_icon_size),
                    egui::Sense::hover(),
                );
                let pad = config.vector_icon_size * 0.28;
                let badge = icon_rect.expand(pad);
                let badge_fill = if ui.visuals().dark_mode {
                    egui::Color32::from_white_alpha(12)
                } else {
                    egui::Color32::from_black_alpha(10)
                };
                ui.painter()
                    .circle_filled(badge.center(), badge.width() * 0.5, badge_fill);
                let stroke = (config.vector_icon_size / 22.0).clamp(1.4, 2.2);
                vector_icons::paint(
                    ui,
                    icon_rect,
                    icon,
                    ui.visuals().weak_text_color(),
                    stroke,
                );
            } else {
                ui.label(egui::RichText::new(config.icon).size(config.icon_size));
            }
            ui.add_space(10.0);
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
    });
}
