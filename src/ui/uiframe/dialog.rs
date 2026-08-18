//! App popup window — reusable chrome for settings, forms, and manage dialogs.
//!
//! - **Desktop**: real OS child windows via [`egui::Context::show_viewport_immediate`]
//!   when viewports are not embedded.
//! - **Android / embedded**: classic [`egui::Window`], **centered** on screen.
//!
//! Modal alerts (quit / connection failure) live in `page::dialogs::notices` and
//! always use embedded centered windows so host blocking stays reliable.

use crate::ui::uiframe::style;

/// Default outer size for standard app windows (settings, manage lists, forms).
pub const DEFAULT_WIDTH: f32 = 520.0;
/// Default outer height; body scrolls when taller.
pub const DEFAULT_HEIGHT: f32 = 480.0;

/// Compact default for alerts / confirms.
pub const ALERT_WIDTH: f32 = 400.0;
pub const ALERT_HEIGHT: f32 = 200.0;

/// Outcome of a dialog frame for one paint.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DialogOutcome {
    None,
    Closed,
}

/// Configuration for an independent popup window.
pub struct DialogFrame {
    pub title: String,
    pub open: bool,
    pub default_width: f32,
    pub default_height: f32,
    pub resizable: bool,
    /// Wrap the body in a vertical scroll area.
    pub scroll: bool,
    /// Show the close (✕) affordance.
    pub closable: bool,
}

impl DialogFrame {
    /// Standard app window: 520×480, resizable, scrollable.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            open: true,
            default_width: DEFAULT_WIDTH,
            default_height: DEFAULT_HEIGHT,
            resizable: true,
            scroll: true,
            closable: true,
        }
    }

    /// Compact alert-sized frame (prefer `notices` for true modals).
    pub fn alert(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            open: true,
            default_width: ALERT_WIDTH,
            default_height: ALERT_HEIGHT,
            resizable: false,
            scroll: false,
            closable: true,
        }
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.default_width = width;
        self.default_height = height;
        self
    }

    pub fn width(mut self, w: f32) -> Self {
        self.default_width = w;
        self
    }

    pub fn height(mut self, h: f32) -> Self {
        self.default_height = h;
        self
    }

    pub fn resizable(mut self, r: bool) -> Self {
        self.resizable = r;
        self
    }

    /// Deprecated no-op: quit/connection modals use `notices` instead.
    pub fn blocks_host(self, _block: bool) -> Self {
        self
    }

    pub fn scroll(mut self, scroll: bool) -> Self {
        self.scroll = scroll;
        self
    }

    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Kept for call-site compatibility.
    pub fn foreground(self) -> Self {
        self
    }

    /// Show the dialog. Returns [`DialogOutcome::Closed`] when the user closes it.
    pub fn show(
        &self,
        ctx: &egui::Context,
        id: impl Into<egui::Id>,
        mut body: impl FnMut(&mut egui::Ui),
    ) -> DialogOutcome {
        if !self.open {
            return DialogOutcome::None;
        }

        let id = id.into();
        let scroll_salt = id.with("dialog_body_scroll");
        let scroll = self.scroll;
        let content_w = (self.default_width - 24.0).max(200.0);

        if use_native_viewport(ctx) {
            self.show_native(ctx, id, scroll_salt, content_w, scroll, &mut body)
        } else {
            self.show_embedded_centered(ctx, id, scroll_salt, content_w, scroll, &mut body)
        }
    }

    fn show_native(
        &self,
        ctx: &egui::Context,
        id: egui::Id,
        scroll_salt: egui::Id,
        content_w: f32,
        scroll: bool,
        body: &mut dyn FnMut(&mut egui::Ui),
    ) -> DialogOutcome {
        let viewport_id = egui::ViewportId::from_hash_of(id);
        let builder = egui::ViewportBuilder::default()
            .with_title(self.title.clone())
            .with_inner_size([self.default_width, self.default_height])
            .with_min_inner_size([
                (self.default_width * 0.5).max(240.0),
                (self.default_height * 0.4).max(120.0),
            ])
            .with_resizable(self.resizable)
            .with_close_button(self.closable)
            .with_minimize_button(false)
            .with_maximize_button(self.resizable);

        ctx.show_viewport_immediate(viewport_id, builder, |ui, _class| {
            let _ = style::ACCENT;
            let close_requested = ui.ctx().input(|i| i.viewport().close_requested());

            egui::CentralPanel::default()
                .frame(
                    egui::Frame::central_panel(ui.style())
                        .fill(ui.visuals().panel_fill)
                        .inner_margin(egui::Margin::same(12)),
                )
                .show_inside(ui, |ui| {
                    paint_body(ui, scroll_salt, content_w, scroll, body);
                });

            if close_requested {
                DialogOutcome::Closed
            } else {
                DialogOutcome::None
            }
        })
    }

    fn show_embedded_centered(
        &self,
        ctx: &egui::Context,
        id: egui::Id,
        scroll_salt: egui::Id,
        content_w: f32,
        scroll: bool,
        body: &mut dyn FnMut(&mut egui::Ui),
    ) -> DialogOutcome {
        let mut open = true;
        let mut win = egui::Window::new(&self.title)
            .id(id)
            .collapsible(false)
            .resizable(self.resizable)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .default_size([self.default_width, self.default_height])
            .min_width((self.default_width * 0.5).max(240.0))
            .min_height((self.default_height * 0.4).max(120.0));

        if self.closable {
            win = win.open(&mut open);
        }

        win.show(ctx, |ui| {
            let _ = style::ACCENT;
            ui.set_min_width(content_w.min(ui.available_width()));
            paint_body(ui, scroll_salt, content_w, scroll, body);
        });

        if self.closable && !open {
            DialogOutcome::Closed
        } else {
            DialogOutcome::None
        }
    }
}

fn paint_body(
    ui: &mut egui::Ui,
    scroll_salt: egui::Id,
    content_w: f32,
    scroll: bool,
    body: &mut dyn FnMut(&mut egui::Ui),
) {
    if scroll {
        egui::ScrollArea::vertical()
            .id_salt(scroll_salt)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.set_min_width(content_w.min(ui.available_width()));
                body(ui);
            });
    } else {
        body(ui);
    }
}

/// Desktop with multi-viewport enabled → native OS window; Android always embedded.
fn use_native_viewport(ctx: &egui::Context) -> bool {
    #[cfg(target_os = "android")]
    {
        let _ = ctx;
        false
    }
    #[cfg(not(target_os = "android"))]
    {
        !ctx.embed_viewports()
    }
}

/// Legacy helper — modal blocking is handled by `notices` now.
pub fn host_blocked_this_frame(_ctx: &egui::Context) -> bool {
    false
}
