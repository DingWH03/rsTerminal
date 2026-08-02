//! Dense list rows for sidebar and empty-pane resource lists.

use egui::{Color32, FontId, Id, Response, Sense, Ui, Vec2};

use crate::ui::uiframe::interactive;
use crate::ui::uiframe::style;
use crate::ui::uiframe::tokens;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListRowDensity {
    /// Session / nav rows — 28px.
    Compact,
    /// Connections / commands / recent — 32px.
    Standard,
}

impl ListRowDensity {
    pub fn height(self) -> f32 {
        match self {
            Self::Compact => tokens::size::NAV_ROW,
            Self::Standard => tokens::size::RESOURCE_ROW,
        }
    }

    pub fn title_size(self) -> f32 {
        match self {
            Self::Compact => tokens::text::BODY,
            Self::Standard => tokens::text::BODY,
        }
    }

    pub fn subtitle_size(self) -> f32 {
        tokens::text::CAPTION
    }
}

pub struct CompactListRow<'a> {
    pub id: Id,
    pub density: ListRowDensity,
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub leading: Option<&'a str>,
    pub selected: bool,
    pub accent_stripe: Option<Color32>,
    pub sense: Sense,
    pub trailing_width: f32,
    pub menu_open: bool,
}

#[derive(Default)]
pub struct CompactListRowOutcome {
    pub response: Option<Response>,
    pub trailing_rect: Option<egui::Rect>,
    pub trailing_response: Option<Response>,
}

impl<'a> CompactListRow<'a> {
    pub fn show(self, ui: &mut Ui) -> CompactListRowOutcome {
        let row_h = self.density.height();
        let row_w = ui.available_width();
        let row_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(row_w, row_h));
        let response = ui.allocate_rect(row_rect, self.sense);

        let trailing_rect = if self.trailing_width > 0.0 {
            Some(egui::Rect::from_min_size(
                egui::pos2(row_rect.right() - self.trailing_width, row_rect.top()),
                Vec2::new(self.trailing_width, row_h),
            ))
        } else {
            None
        };

        let trailing_response =
            trailing_rect.map(|rect| ui.interact(rect, self.id.with("trailing"), Sense::click()));

        if ui.is_rect_visible(row_rect) {
            let hovered = response.hovered()
                || trailing_response.as_ref().is_some_and(|r| r.hovered())
                || self.menu_open;
            let chrome = interactive::row_chrome(ui, interactive::state(self.selected, hovered));
            let painter = ui.painter_at(row_rect);
            if chrome.fill != Color32::TRANSPARENT {
                painter.rect_filled(row_rect, style::CORNER_RADIUS_XS, chrome.fill);
            }

            if let Some(accent) = self.accent_stripe {
                let stripe =
                    egui::Rect::from_min_size(row_rect.min, Vec2::new(3.0, row_rect.height()));
                painter.rect_filled(stripe, 1.0, accent);
            }

            let mut text_left = row_rect.left() + tokens::space::MD;
            let text_right = trailing_rect
                .map(|r| r.left() - tokens::space::SM)
                .unwrap_or(row_rect.right() - tokens::space::SM);
            let text_w = (text_right - text_left).max(24.0);

            if let Some(leading) = self.leading {
                let icon_g = ui.fonts_mut(|f| {
                    f.layout(
                        leading.to_string(),
                        FontId::proportional(tokens::text::EMPHASIS),
                        ui.visuals().text_color(),
                        f32::INFINITY,
                    )
                });
                painter.galley(
                    egui::pos2(text_left, row_rect.center().y - icon_g.size().y * 0.5),
                    icon_g,
                    ui.visuals().text_color(),
                );
                text_left += 22.0;
            }

            let title_w = (text_right - text_left).max(24.0).min(text_w);
            let title_top = if self.subtitle.is_some() {
                row_rect.top() + 2.0
            } else {
                row_rect.center().y - self.density.title_size() * 0.55
            };
            let title_g = ui.fonts_mut(|f| {
                f.layout(
                    self.title.to_string(),
                    FontId::proportional(self.density.title_size()),
                    ui.visuals().text_color(),
                    title_w,
                )
            });
            painter.galley(
                egui::pos2(text_left, title_top),
                title_g,
                ui.visuals().text_color(),
            );

            if let Some(subtitle) = self.subtitle {
                let sub_g = ui.fonts_mut(|f| {
                    f.layout(
                        subtitle.to_string(),
                        FontId::proportional(self.density.subtitle_size()),
                        ui.visuals().weak_text_color(),
                        title_w,
                    )
                });
                painter.galley(
                    egui::pos2(text_left, row_rect.top() + 17.0),
                    sub_g,
                    ui.visuals().weak_text_color(),
                );
            }
        }

        ui.add_space(tokens::space::XS);
        CompactListRowOutcome {
            response: Some(response),
            trailing_rect,
            trailing_response,
        }
    }
}

pub fn paint_overflow_dots(ui: &Ui, rect: egui::Rect, hovered: bool) {
    let color = if hovered {
        ui.visuals().text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    let galley = ui.fonts_mut(|f| {
        f.layout(
            "\u{22EE}".to_string(),
            FontId::proportional(16.0),
            color,
            f32::INFINITY,
        )
    });
    ui.painter().galley(
        egui::pos2(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        color,
    );
}
