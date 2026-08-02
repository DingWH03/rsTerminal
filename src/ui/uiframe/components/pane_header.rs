//! Compact pane chrome header shared by terminal, file manager, and empty panes.

use egui::{Id, Ui};

use crate::ui::uiframe::components::toolbar_button::icon_toolbar_button;
use crate::ui::uiframe::tokens;
use crate::ui::uiframe::vector_icons::Icon;

#[derive(Default)]
pub struct PaneHeader<'a> {
    pub show_hamburger: bool,
    pub hamburger_id: Option<Id>,
    pub title: Option<&'a str>,
    pub center: Option<&'a mut dyn FnMut(&mut Ui)>,
    pub trailing: Option<&'a mut dyn FnMut(&mut Ui)>,
}

#[derive(Default)]
pub struct PaneHeaderOutcome {
    pub hamburger_clicked: bool,
}

impl<'a> PaneHeader<'a> {
    pub fn show(self, ui: &mut Ui) -> PaneHeaderOutcome {
        let mut outcome = PaneHeaderOutcome::default();
        ui.horizontal(|ui| {
            ui.style_mut().spacing.button_padding =
                egui::vec2(tokens::space::XS, tokens::space::XS * 0.5);
            ui.style_mut().spacing.item_spacing.x = tokens::space::XS;

            if self.show_hamburger {
                let id = self
                    .hamburger_id
                    .unwrap_or_else(|| ui.id().with("pane_hamburger"));
                if icon_toolbar_button(ui, id, Icon::Hamburger).clicked() {
                    outcome.hamburger_clicked = true;
                }
            }

            if let Some(center) = self.center {
                center(ui);
            } else if let Some(title) = self.title {
                ui.label(
                    egui::RichText::new(title)
                        .size(tokens::text::COMPACT)
                        .strong()
                        .color(ui.visuals().text_color()),
                );
            }

            if let Some(trailing) = self.trailing {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.style_mut().spacing.item_spacing.x = tokens::space::XS;
                    trailing(ui);
                });
            }
        });
        ui.add(egui::Separator::default().spacing(tokens::space::XS));
        outcome
    }
}
