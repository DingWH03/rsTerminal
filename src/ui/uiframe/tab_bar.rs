//! Compact square icon tab strip for sidebar navigation.

use crate::ui::uiframe::style;
use crate::ui::uiframe::vector_icons::{self, Icon};

#[derive(Clone, Copy)]
pub struct TabBarItem<'a> {
    pub id: usize,
    pub icon: Icon,
    /// Hover tooltip (accessibility / discoverability).
    pub tip: &'a str,
}

/// Fixed-size square icon tabs. Sized so ≥4 tabs fit in the default sidebar width.
pub struct TabBar;

impl TabBar {
    /// Outer square hit target (padding + icon).
    pub const CELL: f32 = 32.0;
    pub const ICON: f32 = 16.0;
    pub const GAP: f32 = 4.0;
    pub const HEIGHT: f32 = Self::CELL;

    /// Minimum width needed for `n` tabs (including gaps).
    pub fn min_width_for(n: usize) -> f32 {
        if n == 0 {
            0.0
        } else {
            n as f32 * Self::CELL + (n.saturating_sub(1) as f32) * Self::GAP
        }
    }

    pub fn show(ui: &mut egui::Ui, selected: &mut usize, items: &[TabBarItem<'_>]) -> bool {
        if items.is_empty() {
            return false;
        }
        if !items.iter().any(|t| t.id == *selected) {
            *selected = items[0].id;
        }

        let mut changed = false;
        let cell = Self::CELL;
        let gap = Self::GAP;
        let needed = Self::min_width_for(items.len());
        let avail = ui.available_width();
        // Center the strip when there is leftover width.
        let start_x = ((avail - needed) * 0.5).max(0.0);

        let (_, row_resp) = ui.allocate_exact_size(
            egui::vec2(avail, cell),
            egui::Sense::hover(),
        );
        let row_left = row_resp.rect.left() + start_x;
        let row_top = row_resp.rect.top();

        for (i, item) in items.iter().enumerate() {
            let x = row_left + i as f32 * (cell + gap);
            let rect = egui::Rect::from_min_size(egui::pos2(x, row_top), egui::vec2(cell, cell));
            let resp = ui
                .interact(rect, ui.id().with(("tab", item.id)), egui::Sense::click())
                .on_hover_text(item.tip);

            if ui.is_rect_visible(rect) {
                let selected_here = *selected == item.id;
                let bg = if selected_here {
                    ui.visuals().selection.bg_fill.gamma_multiply(0.45)
                } else if resp.hovered() {
                    ui.visuals().widgets.hovered.bg_fill
                } else {
                    egui::Color32::TRANSPARENT
                };
                if bg != egui::Color32::TRANSPARENT {
                    ui.painter()
                        .rect_filled(rect, style::CORNER_RADIUS_XS, bg);
                }
                if selected_here {
                    ui.painter().rect_stroke(
                        rect.shrink(0.5),
                        style::CORNER_RADIUS_XS,
                        egui::Stroke::new(1.0, style::ACCENT),
                        egui::StrokeKind::Inside,
                    );
                }

                let color = if selected_here {
                    ui.visuals().selection.stroke.color
                } else {
                    ui.visuals().weak_text_color()
                };
                let icon_rect = egui::Rect::from_center_size(
                    rect.center(),
                    egui::vec2(Self::ICON, Self::ICON),
                );
                vector_icons::paint(ui, icon_rect, item.icon, color, 1.4);
            }

            if resp.clicked() && *selected != item.id {
                *selected = item.id;
                changed = true;
            }
        }

        changed
    }
}
