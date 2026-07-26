//! Centered dialog window chrome — shared by settings, about, and connection dialogs.

use crate::ui::uiframe::style;

/// Outcome of a dialog frame for one paint.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DialogOutcome {
    None,
    Closed,
}

/// Configuration for a centered, non-resizable dialog window.
pub struct DialogFrame {
    pub title: String,
    pub open: bool,
    pub default_width: f32,
    pub default_height: Option<f32>,
    pub resizable: bool,
}

impl DialogFrame {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            open: true,
            default_width: 520.0,
            default_height: Some(480.0),
            resizable: true,
        }
    }

    pub fn width(mut self, w: f32) -> Self {
        self.default_width = w;
        self
    }

    pub fn height(mut self, h: Option<f32>) -> Self {
        self.default_height = h;
        self
    }

    pub fn resizable(mut self, r: bool) -> Self {
        self.resizable = r;
        self
    }

    /// Show a centered window. Returns [`DialogOutcome::Closed`] when the user closes it.
    pub fn show(
        &self,
        ctx: &egui::Context,
        id: impl Into<egui::Id>,
        mut body: impl FnMut(&mut egui::Ui),
    ) -> DialogOutcome {
        if !self.open {
            return DialogOutcome::None;
        }

        let mut open = true;
        let mut win = egui::Window::new(&self.title)
            .id(id.into())
            .open(&mut open)
            .collapsible(false)
            .resizable(self.resizable)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .default_width(self.default_width);

        if let Some(h) = self.default_height {
            win = win.default_height(h);
        }

        win.show(ctx, |ui| {
            ui.set_min_width(self.default_width.min(ui.available_width()));
            // Subtle frame consistency with design tokens.
            let _ = style::ACCENT;
            body(ui);
        });

        if open {
            DialogOutcome::None
        } else {
            DialogOutcome::Closed
        }
    }
}
