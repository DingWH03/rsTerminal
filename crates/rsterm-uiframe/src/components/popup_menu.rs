//! Unified popup menu shell and row widgets (context, overflow, settings).

use egui::containers::menu::menu_style;
use egui::{Align, Context, FontId, Id, Layout, Response, Ui};

use crate::style;
use crate::tokens;

/// Lower bound for menu width (very short labels).
pub const POPUP_MENU_MIN_WIDTH: f32 = 72.0;

/// Upper bound; longer labels wrap/truncate within this width.
pub const POPUP_MENU_MAX_WIDTH: f32 = 280.0;

const MENU_ROW_HEIGHT: f32 = 24.0;
const CHECK_PREFIX_CHARS: f32 = 12.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PopupMenuOutcome {
    pub open: bool,
}

/// Tracks which anchor popup is open (one at a time).
#[derive(Clone, Debug, Default)]
pub struct PopupMenuState {
    open_base: Option<Id>,
    generation: u32,
}

impl PopupMenuState {
    pub fn is_open(&self, base_id: Id) -> bool {
        self.open_base == Some(base_id)
    }

    pub fn popup_id(&self, base_id: Id) -> Id {
        base_id.with(self.generation)
    }

    pub fn open(&mut self, ctx: &Context, base_id: Id) {
        if let Some(prev) = self.open_base.filter(|&p| p != base_id) {
            egui::Popup::close_id(ctx, prev.with(self.generation));
        }
        self.generation = self.generation.wrapping_add(1);
        self.open_base = Some(base_id);
        egui::Popup::open_id(ctx, self.popup_id(base_id));
    }

    pub fn toggle(&mut self, ctx: &Context, base_id: Id) {
        if self.is_open(base_id) {
            self.close_synced(ctx);
        } else {
            self.open(ctx, base_id);
        }
    }

    pub fn close(&mut self) {
        self.open_base = None;
    }

    pub fn close_synced(&mut self, ctx: &Context) {
        if let Some(base) = self.open_base {
            egui::Popup::close_id(ctx, base.with(self.generation));
        }
        self.open_base = None;
    }
}

/// Measure width to fit menu labels (action rows or `✓` check rows).
pub fn measure_menu_width(ctx: &Context, labels: &[&str], check_rows: bool) -> f32 {
    let font = FontId::proportional(tokens::text::BODY);
    let color = ctx.global_style().visuals.text_color();
    // Row padding + popup frame inner margin (both sides).
    let pad = tokens::space::SM * 2.0 + tokens::space::SM * 2.0;
    let prefix = if check_rows { CHECK_PREFIX_CHARS } else { 0.0 };
    let mut content_w = 0.0f32;
    for label in labels {
        let galley = ctx.fonts_mut(|f| f.layout_no_wrap(label.to_string(), font.clone(), color));
        content_w = content_w.max(galley.size().x);
    }
    (content_w + pad + prefix).clamp(POPUP_MENU_MIN_WIDTH, POPUP_MENU_MAX_WIDTH)
}

/// Inner menu layout: shrink-to-fit width, capped at [`POPUP_MENU_MAX_WIDTH`].
pub fn popup_menu_content(ui: &mut Ui, content: impl FnOnce(&mut Ui)) {
    popup_menu_content_width(ui, None, content);
}

fn popup_menu_content_width(ui: &mut Ui, width: Option<f32>, content: impl FnOnce(&mut Ui)) {
    ui.style_mut().spacing.item_spacing.y = tokens::space::XS;
    ui.set_min_width(0.0);
    if let Some(w) = width.filter(|w| *w > 0.0) {
        ui.set_width(w);
        ui.set_max_width(w);
    } else {
        ui.set_max_width(POPUP_MENU_MAX_WIDTH);
    }
    content(ui);
}

/// Popup body with frame (legacy hosts only).
pub fn popup_body(ui: &mut Ui, content: impl FnOnce(&mut Ui)) {
    egui::Frame::popup(ui.style())
        .corner_radius(style::CORNER_RADIUS_SM)
        .inner_margin(tokens::space::SM)
        .show(ui, |ui| {
            popup_menu_content(ui, content);
        });
}

/// Build a shrink-wrapped menu popup anchored to `anchor`.
///
/// `width_hint` seeds the Area sizing pass (use measured label width, or `0.0` to
/// avoid egui's default 600px `default_area_size`).
pub(crate) fn menu_popup<'a>(
    anchor: &Response,
    popup_id: Id,
    width_hint: Option<f32>,
) -> egui::Popup<'a> {
    let sizing_w = width_hint.unwrap_or(0.0).clamp(0.0, POPUP_MENU_MAX_WIDTH);
    egui::Popup::from_response(anchor)
        .id(popup_id)
        .kind(egui::PopupKind::Menu)
        .layout(Layout::top_down(Align::LEFT))
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .open_memory(None)
        .gap(0.0)
        .style(menu_style)
        .width(sizing_w)
}

/// Show popup anchored to `anchor` when `state` marks `popup_id` open.
pub fn popup_from_response(
    anchor: &Response,
    base_popup_id: Id,
    state: &mut PopupMenuState,
    width_hint: Option<f32>,
    build: impl FnOnce(&mut Ui),
) -> PopupMenuOutcome {
    let ctx = &anchor.ctx;
    if !state.is_open(base_popup_id) {
        return PopupMenuOutcome { open: false };
    }

    let popup_id = state.popup_id(base_popup_id);
    if !egui::Popup::is_id_open(ctx, popup_id) {
        egui::Popup::open_id(ctx, popup_id);
    }

    menu_popup(anchor, popup_id, width_hint).show(|ui| {
        popup_menu_content_width(ui, width_hint, build);
    });

    let still_open = egui::Popup::is_id_open(ctx, popup_id);
    if !still_open {
        state.close();
    }
    PopupMenuOutcome { open: still_open }
}

/// Show a shrink-wrapped menu popup anchored to a widget (path autocomplete, etc.).
pub fn show_anchor_popup(
    anchor: &Response,
    popup_id: Id,
    width_hint: Option<f32>,
    build: impl FnOnce(&mut Ui),
) {
    menu_popup(anchor, popup_id, width_hint).show(|ui| {
        popup_menu_content_width(ui, width_hint, build);
    });
}

/// Desktop right-click + touch long-press popup menu on a widget response.
///
/// Uses pointer-fixed positioning (not `Response::context_menu`) so width is not
/// inherited from full-width row anchors.
pub fn install_context_popup(
    resp: &Response,
    enable_desktop_context: bool,
    touch_open: Option<egui::SetOpenCommand>,
    width_hint: Option<f32>,
    mut build: impl FnMut(&mut Ui),
) {
    let menu_id = resp.id.with("ctx_popup");

    let desktop_open = if enable_desktop_context {
        if resp.secondary_clicked() {
            Some(egui::SetOpenCommand::Bool(true))
        } else if resp.clicked() {
            Some(egui::SetOpenCommand::Bool(false))
        } else {
            None
        }
    } else {
        None
    };

    let open = touch_open.or(desktop_open).or_else(|| {
        resp.long_touched()
            .then_some(egui::SetOpenCommand::Bool(true))
    });

    menu_popup(resp, menu_id, width_hint)
        .at_pointer_fixed()
        .open_memory(open)
        .show(|ui| {
            popup_menu_content_width(ui, width_hint, |ui| build(ui));
        });
}

/// Weak section heading inside a popup menu.
pub fn menu_heading(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(tokens::text::CAPTION).weak());
}

pub fn menu_separator(ui: &mut Ui) {
    ui.separator();
}

/// Action row; closes the popup on click.
pub fn menu_action(ui: &mut Ui, label: &str) -> bool {
    menu_action_enabled(ui, label, true)
}

/// Action row with optional disabled state.
pub fn menu_action_enabled(ui: &mut Ui, label: &str, enabled: bool) -> bool {
    let clicked = ui
        .add_enabled(
            enabled,
            egui::Button::new(label)
                .frame(false)
                .min_size(egui::vec2(0.0, MENU_ROW_HEIGHT)),
        )
        .clicked();
    if clicked {
        ui.close();
    }
    clicked
}

/// Checkable menu row (settings-style); does not auto-close.
pub fn menu_check(ui: &mut Ui, label: &str, checked: bool) -> bool {
    let mark = if checked { "✓ " } else { "   " };
    ui.add(
        egui::Button::new(format!("{mark}{label}"))
            .frame(false)
            .min_size(egui::vec2(0.0, MENU_ROW_HEIGHT)),
    )
    .clicked()
}
