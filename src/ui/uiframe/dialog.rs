//! App popup window — one reusable chrome for settings, forms, alerts, and modals.
//!
//! On desktop (when viewports are not embedded), each dialog is a **real OS window**
//! via [`egui::Context::show_viewport_immediate`]. On Android / unsupported backends
//! egui falls back to an embedded [`egui::Window`] inside the main viewport.
//!
//! Configuration:
//! - default size, resizable, scrollable body
//! - [`DialogFrame::blocks_host`]: dim the main window and swallow its input (quit, errors)

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

/// Configuration for an independent popup window (native OS window when available).
pub struct DialogFrame {
    pub title: String,
    pub open: bool,
    pub default_width: f32,
    pub default_height: f32,
    pub resizable: bool,
    /// When true, dim the main window and block interaction with it.
    pub blocks_host: bool,
    /// Wrap the body in a vertical scroll area.
    pub scroll: bool,
    /// Show the OS / title-bar close affordance.
    pub closable: bool,
}

impl DialogFrame {
    /// Standard app window: 520×480, resizable, scrollable, non-blocking.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            open: true,
            default_width: DEFAULT_WIDTH,
            default_height: DEFAULT_HEIGHT,
            resizable: true,
            blocks_host: false,
            scroll: true,
            closable: true,
        }
    }

    /// Compact alert / confirm: smaller, fixed size, no body scroll.
    pub fn alert(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            open: true,
            default_width: ALERT_WIDTH,
            default_height: ALERT_HEIGHT,
            resizable: false,
            blocks_host: false,
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

    /// Block clicks/keyboard reaching the main UI (quit dialog, fatal notices).
    pub fn blocks_host(mut self, block: bool) -> Self {
        self.blocks_host = block;
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

    /// Kept for call-site compatibility; native OS windows already stack independently.
    pub fn foreground(self) -> Self {
        self
    }

    /// Show as a native child window when viewports are enabled, else embedded.
    ///
    /// Returns [`DialogOutcome::Closed`] when the user closes the window (✕ / WM close).
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
        let viewport_id = egui::ViewportId::from_hash_of(id);
        let scroll_salt = id.with("dialog_body_scroll");

        if self.blocks_host {
            paint_host_blocker(ctx, id.with("host_blocker"));
            mark_host_blocked(ctx);
        }

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

        let scroll = self.scroll;
        let content_w = (self.default_width - 24.0).max(200.0);

        ctx.show_viewport_immediate(viewport_id, builder, |ui, _class| {
            let _ = style::ACCENT;
            let close_requested = ui.ctx().input(|i| i.viewport().close_requested());

            // Child viewports have no default fill — without CentralPanel the OS
            // window clears to black. show_inside paints panel_fill + margins.
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::central_panel(ui.style())
                        .fill(ui.visuals().panel_fill)
                        .inner_margin(egui::Margin::same(12)),
                )
                .show_inside(ui, |ui| {
                    ui.set_min_width(content_w.min(ui.available_width()));
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
                });

            if close_requested {
                DialogOutcome::Closed
            } else {
                DialogOutcome::None
            }
        })
    }
}

fn paint_host_blocker(ctx: &egui::Context, id: egui::Id) {
    // Dim only the calling (usually root) viewport.
    let screen = ctx.content_rect();
    egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .sense(egui::Sense::click_and_drag())
        .show(ctx, |ui| {
            let (rect, _response) =
                ui.allocate_exact_size(screen.size(), egui::Sense::click_and_drag());
            ui.painter().rect_filled(
                rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 110),
            );
        });
}

const HOST_BLOCKED_KEY: &str = "rsterm_dialog_host_blocked";

fn mark_host_blocked(ctx: &egui::Context) {
    ctx.memory_mut(|mem| {
        mem.data
            .insert_temp(egui::Id::new(HOST_BLOCKED_KEY), true);
    });
}

/// True if any modal [`DialogFrame`] with [`DialogFrame::blocks_host`] painted this frame.
pub fn host_blocked_this_frame(ctx: &egui::Context) -> bool {
    ctx.memory(|mem| {
        mem.data
            .get_temp::<bool>(egui::Id::new(HOST_BLOCKED_KEY))
            .unwrap_or(false)
    })
}
