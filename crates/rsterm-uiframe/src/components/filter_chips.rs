//! 筛选标签组件 — 用于按类型过滤列表。
//!
//! 紧凑水平居中的 tag 条，只占用一行高度，不会吞掉下方列表空间。

use crate::interactive;
use crate::style;
use crate::tokens;

/// 筛选标签的配置项。
pub struct FilterChipItem<T> {
    /// 标签显示文本
    pub label: String,
    /// 对应的筛选值（`None` 表示"全部"）
    pub value: Option<T>,
}

const FONT_SIZE: f32 = tokens::text::COMPACT;
const CHIP_PAD_X: f32 = 5.0;
const CHIP_PAD_Y: f32 = 1.0;
const CHIP_H: f32 = FONT_SIZE + CHIP_PAD_Y * 2.0 + 2.0;
const CHIP_GAP: f32 = 3.0;

/// 渲染筛选标签栏（单行、水平居中，无多余上下留白）。
///
/// 使用 `id_salt` 区分不同位置的筛选状态（首页 vs 侧边栏）。
/// 返回当前选中的筛选值。
pub fn paint_filter_chips<T: PartialEq + Clone + Send + Sync + 'static>(
    ui: &mut egui::Ui,
    id_salt: &str,
    chips: &[FilterChipItem<T>],
) -> Option<T> {
    let id = egui::Id::new(id_salt);
    let current: Option<T> = ui.data(|d| d.get_temp(id)).unwrap_or(None);

    let font = egui::FontId::proportional(FONT_SIZE);
    let widths: Vec<f32> = chips
        .iter()
        .map(|chip| {
            let galley = ui.fonts_mut(|f| {
                f.layout_no_wrap(chip.label.clone(), font.clone(), egui::Color32::WHITE)
            });
            (galley.size().x + CHIP_PAD_X * 2.0).max(24.0)
        })
        .collect();

    let total_w = widths.iter().sum::<f32>() + CHIP_GAP * chips.len().saturating_sub(1) as f32;
    let avail_w = ui.available_width();
    let (row_rect, _) = ui.allocate_exact_size(egui::vec2(avail_w, CHIP_H), egui::Sense::hover());

    let start_x = row_rect.left() + ((avail_w - total_w) * 0.5).max(0.0);
    let y = row_rect.top();

    let mut next = current.clone();
    let mut x = start_x;
    for (i, chip) in chips.iter().enumerate() {
        let w = widths[i];
        let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, CHIP_H));
        let resp = ui.interact(rect, id.with(("chip", i)), egui::Sense::click());
        let selected = current.as_ref() == chip.value.as_ref();

        if ui.is_rect_visible(rect) {
            let chrome = interactive::row_chrome(ui, interactive::state(selected, resp.hovered()));
            if chrome.fill != egui::Color32::TRANSPARENT {
                ui.painter()
                    .rect_filled(rect, style::CORNER_RADIUS_XS, chrome.fill);
            }
            if selected {
                ui.painter().rect_stroke(
                    rect.shrink(0.5),
                    style::CORNER_RADIUS_XS,
                    chrome.stroke,
                    egui::StrokeKind::Inside,
                );
            }

            let color = if selected {
                ui.visuals().strong_text_color()
            } else {
                ui.visuals().weak_text_color()
            };
            let galley =
                ui.fonts_mut(|f| f.layout_no_wrap(chip.label.clone(), font.clone(), color));
            let text_pos = egui::pos2(
                rect.center().x - galley.size().x * 0.5,
                rect.center().y - galley.size().y * 0.5,
            );
            ui.painter().galley(text_pos, galley, color);
        }

        if resp.clicked() {
            next = chip.value.clone();
        }
        x += w + CHIP_GAP;
    }

    if next != current {
        ui.data_mut(|d| d.insert_temp(id, next.clone()));
    }
    next
}
