//! Unified hover / long-press detail floating panel.

use egui::{Align2, Context, FontId, Id, Order, Pos2, Rect, Response, Vec2};

use crate::style;
use crate::tokens;

/// Rich detail shown in the floating panel.
#[derive(Clone, Debug, Default)]
pub struct HoverDetail {
    pub title: String,
    pub lines: Vec<String>,
}

/// Types that can supply hover / long-press detail content.
pub trait HoverDetailSource {
    fn hover_detail(&self) -> Option<HoverDetail>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoverInstallMode {
    /// Desktop: show after pointer rests ~300ms; hide when pointer leaves.
    PointerHover,
    /// Touch: show after 1s hold; stays until dismissed.
    TouchHoldPersistent,
}

#[derive(Clone, Debug)]
struct ActivePanel {
    detail: HoverDetail,
    anchor: Pos2,
    persistent: bool,
}

/// Per-host state for the shared hover panel (one panel at a time).
#[derive(Clone, Debug, Default)]
pub struct HoverPanelState {
    active: Option<ActivePanel>,
    hover_target: Option<Id>,
    hover_elapsed: f32,
    close_label: String,
}

const POINTER_HOVER_DELAY: f32 = 0.3;
const PANEL_MAX_W: f32 = 320.0;
const PANEL_PAD: f32 = 10.0;
const PANEL_MARGIN: f32 = 8.0;

impl HoverPanelState {
    pub fn set_close_label(&mut self, label: impl Into<String>) {
        self.close_label = label.into();
    }

    pub fn dismiss(&mut self) {
        self.active = None;
        self.hover_target = None;
        self.hover_elapsed = 0.0;
    }

    /// Returns true if a persistent panel was dismissed (for Android back).
    pub fn handle_back(&mut self) -> bool {
        if self.active.as_ref().is_some_and(|p| p.persistent) {
            self.dismiss();
            true
        } else {
            false
        }
    }

    pub fn show_persistent(&mut self, anchor: Pos2, detail: HoverDetail) {
        self.active = Some(ActivePanel {
            detail,
            anchor,
            persistent: true,
        });
        self.hover_target = None;
        self.hover_elapsed = 0.0;
    }

    pub fn is_persistent_open(&self) -> bool {
        self.active.as_ref().is_some_and(|p| p.persistent)
    }

    fn set_pointer_hover(&mut self, target: Id, anchor: Pos2, detail: HoverDetail) {
        self.active = Some(ActivePanel {
            detail,
            anchor,
            persistent: false,
        });
        self.hover_target = Some(target);
    }

    fn tick_pointer_hover(&mut self, target: Id, hovered: bool, dt: f32) {
        if !hovered {
            if self.hover_target == Some(target)
                && !self.active.as_ref().is_some_and(|p| p.persistent)
            {
                self.dismiss();
            }
            return;
        }
        if self.active.as_ref().is_some_and(|p| p.persistent) {
            return;
        }
        if self.hover_target == Some(target) {
            self.hover_elapsed += dt;
        } else {
            self.hover_target = Some(target);
            self.hover_elapsed = 0.0;
        }
    }
}

fn place_panel(screen: Rect, anchor: Pos2, panel_w: f32, panel_h: f32) -> Pos2 {
    let candidates = [
        Align2::LEFT_TOP,
        Align2::RIGHT_TOP,
        Align2::LEFT_BOTTOM,
        Align2::RIGHT_BOTTOM,
    ];
    let mut best_pos = anchor;
    let mut best_score = -1.0f32;
    for align in candidates {
        let r = align.anchor_size(anchor, Vec2::new(panel_w, panel_h));
        let inside = r.intersect(screen);
        let score = inside.width() * inside.height();
        if score > best_score {
            best_score = score;
            best_pos = Pos2::new(
                r.min.x.clamp(
                    screen.left() + PANEL_MARGIN,
                    screen.right() - panel_w - PANEL_MARGIN,
                ),
                r.min.y.clamp(
                    screen.top() + PANEL_MARGIN,
                    screen.bottom() - panel_h - PANEL_MARGIN,
                ),
            );
        }
    }
    best_pos
}

/// Paint the active hover panel (call once per frame from the host view).
pub fn paint_hover_panel(ctx: &Context, state: &mut HoverPanelState) {
    let Some(panel) = state.active.clone() else {
        return;
    };

    let screen = ctx.content_rect();
    let title_font = FontId::proportional(tokens::text::BODY);
    let line_font = FontId::proportional(tokens::text::SMALL);
    let text_color = ctx.global_style().visuals.text_color();
    let weak = ctx.global_style().visuals.weak_text_color();
    let title_galley =
        ctx.fonts_mut(|f| f.layout_no_wrap(panel.detail.title.clone(), title_font, text_color));
    let mut content_h = title_galley.size().y + tokens::space::SM;
    let mut line_galleys = Vec::new();
    for line in &panel.detail.lines {
        let g = ctx.fonts_mut(|f| {
            f.layout(
                line.clone(),
                line_font.clone(),
                weak,
                PANEL_MAX_W - PANEL_PAD * 2.0,
            )
        });
        content_h += g.size().y + tokens::space::XS;
        line_galleys.push(g);
    }
    let close_h = if panel.persistent {
        tokens::size::BUTTON + tokens::space::SM
    } else {
        0.0
    };
    let panel_w = (title_galley.size().x + PANEL_PAD * 2.0).clamp(160.0, PANEL_MAX_W);
    let panel_h = content_h + PANEL_PAD * 2.0 + close_h;
    let fixed_pos = place_panel(screen, panel.anchor, panel_w, panel_h);

    let panel_id = Id::new("rsterm_hover_panel");
    let mut dismissed = false;
    let close_label = state.close_label.clone();
    egui::Area::new(panel_id)
        .order(Order::Tooltip)
        .fixed_pos(fixed_pos)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .corner_radius(style::CORNER_RADIUS_SM)
                .inner_margin(PANEL_PAD)
                .show(ui, |ui| {
                    ui.set_max_width(panel_w - PANEL_PAD * 2.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&panel.detail.title)
                                .strong()
                                .size(tokens::text::BODY),
                        );
                        if panel.persistent {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button("✕").clicked() {
                                        dismissed = true;
                                    }
                                },
                            );
                        }
                    });
                    ui.separator();
                    for g in &line_galleys {
                        let text: String = g.text().into();
                        ui.label(egui::RichText::new(text).size(tokens::text::SMALL).weak());
                    }
                    if panel.persistent {
                        ui.add_space(tokens::space::SM);
                        if ui.button(&close_label).clicked() {
                            dismissed = true;
                        }
                    }
                });
        });
    if dismissed {
        state.dismiss();
    }
}

/// Attach hover detail behaviour to a widget [`Response`].
pub fn install_hover_detail(
    resp: &Response,
    detail: HoverDetail,
    mode: HoverInstallMode,
    state: &mut HoverPanelState,
) {
    let dt = resp.ctx.input(|i| i.stable_dt);
    let anchor = resp
        .hover_pos()
        .or_else(|| resp.interact_pointer_pos())
        .unwrap_or(resp.rect.center());

    match mode {
        HoverInstallMode::PointerHover => {
            state.tick_pointer_hover(resp.id, resp.hovered(), dt);
            if state.hover_target == Some(resp.id) && state.hover_elapsed >= POINTER_HOVER_DELAY {
                state.set_pointer_hover(resp.id, anchor, detail);
            }
        }
        HoverInstallMode::TouchHoldPersistent => {
            let _ = detail;
            let _ = anchor;
        }
    }
}

/// Build a file-entry style detail block.
pub fn file_entry_detail(
    name: &str,
    size_line: Option<String>,
    modified_line: Option<String>,
) -> HoverDetail {
    let mut lines = Vec::new();
    if let Some(s) = size_line {
        lines.push(s);
    }
    if let Some(m) = modified_line {
        lines.push(m);
    }
    HoverDetail {
        title: name.to_string(),
        lines,
    }
}
