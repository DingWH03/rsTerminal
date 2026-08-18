//! Session list in the function pane.

use std::collections::{HashMap, HashSet};

use crate::connection_display::workspace_session_icon;
use crate::session_host::WorkspaceSession;
use crate::uiframe::components::empty_state::{EmptyStateConfig, paint_empty_state};
use crate::uiframe::interactive::{self, AccentTone, RowState};
use crate::uiframe::style;
use crate::uiframe::tokens;
use crate::uiframe::vector_icons::{self, Icon};

pub struct SessionListContext<'a> {
    pub split_enabled: bool,
    pub visible_sessions: &'a HashSet<String>,
    pub session_accents: &'a HashMap<String, egui::Color32>,
}

/// Session row action result.
pub struct SessionRowAction {
    pub select_session: Option<String>,
    pub close_session: Option<String>,
    pub start_session_drag: Option<String>,
    pub duplicate_session: Option<String>,
}

pub fn paint_session_rows(
    ui: &mut egui::Ui,
    sessions: &[WorkspaceSession],
    active_id: Option<&str>,
    ctx: &SessionListContext<'_>,
) -> SessionRowAction {
    let mut action = SessionRowAction {
        select_session: None,
        close_session: None,
        start_session_drag: None,
        duplicate_session: None,
    };

    if sessions.is_empty() {
        paint_empty_state(
            ui,
            EmptyStateConfig::compact(
                Icon::Sessions,
                &crate::i18n_bridge::tr("sidebar_no_sessions"),
                None,
            ),
        );
        return action;
    }

    for session in sessions {
        paint_session_row(ui, session, active_id, ctx, &mut action);
    }

    if sessions.iter().any(|s| {
        let t = format!("{} {}", workspace_session_icon(s), s.tab_label());
        t.chars().count() > 28
    }) {
        ui.ctx().request_repaint();
    }

    action
}

const SESSION_ACTIONS_WIDTH: f32 = 52.0;
const SESSION_ROW_H: f32 = tokens::size::NAV_ROW;

fn paint_session_row(
    ui: &mut egui::Ui,
    session: &WorkspaceSession,
    active_id: Option<&str>,
    ctx: &SessionListContext<'_>,
    action: &mut SessionRowAction,
) {
    let in_background = !ctx.visible_sessions.contains(session.id());
    let active = active_id == Some(session.id());
    let show_dup = session.sidebar_has_new_window();
    let full_text = format!(
        "{} {}",
        workspace_session_icon(session),
        session.tab_label()
    );
    let display_text: String = full_text.chars().take(28).collect();
    let display_text = if full_text.chars().count() > 28 {
        format!("{}…", display_text)
    } else {
        display_text
    };

    let row_w = ui.available_width();
    let actions_w = if show_dup {
        SESSION_ACTIONS_WIDTH
    } else {
        SESSION_ACTIONS_WIDTH * 0.5
    };
    let label_w = (row_w - actions_w - 4.0).max(48.0);
    let row_h = SESSION_ROW_H;

    let sense = if ctx.split_enabled {
        egui::Sense::click_and_drag()
    } else {
        egui::Sense::click()
    };

    let (rect, resp) = ui.allocate_exact_size(egui::vec2(row_w, row_h), sense);

    let dragged = ctx.split_enabled && resp.dragged() && resp.drag_delta().length() > 6.0;
    if resp.drag_started() && ctx.split_enabled {
        action.start_session_drag = Some(session.id().to_string());
    }
    if resp.clicked() && !dragged {
        action.select_session = Some(session.id().to_string());
    }

    if ui.is_rect_visible(rect) {
        let label_rect = egui::Rect::from_min_size(rect.min, egui::vec2(label_w, row_h));
        let label_resp = ui.interact(
            label_rect,
            ui.id().with(("sess_label", session.id())),
            sense,
        );
        if label_resp.clicked() && !dragged {
            action.select_session = Some(session.id().to_string());
        }
        if label_resp.drag_started() && ctx.split_enabled {
            action.start_session_drag = Some(session.id().to_string());
        }
        paint_label_in_rect(
            ui,
            label_rect,
            &display_text,
            active,
            in_background,
            ctx.session_accents.get(session.id()).copied(),
        );
        if in_background {
            label_resp.on_hover_text(crate::i18n_bridge::tr("background_session"));
        }

        let weak_color = ui.visuals().weak_text_color();

        let close_rect = egui::Rect::from_center_size(
            egui::pos2(rect.right() - 14.0, rect.center().y),
            egui::vec2(22.0, 22.0),
        );
        let close_id = ui.id().with(("sess_close", session.id()));
        let close_resp = ui.interact(close_rect, close_id, egui::Sense::click());
        if close_resp.clicked() {
            action.close_session = Some(session.id().to_string());
        }
        close_resp.on_hover_text(crate::i18n_bridge::tr("close_pane"));
        vector_icons::paint(ui, close_rect, Icon::Close, weak_color, 1.3);

        if show_dup {
            let dup_rect = egui::Rect::from_center_size(
                egui::pos2(close_rect.left() - 14.0, rect.center().y),
                egui::vec2(22.0, 22.0),
            );
            let dup_id = ui.id().with(("sess_dup", session.id()));
            let dup_resp = ui.interact(dup_rect, dup_id, egui::Sense::click());
            if dup_resp.clicked() {
                action.duplicate_session = Some(session.id().to_string());
            }
            dup_resp.on_hover_text(crate::i18n_bridge::tr("new_window"));
            vector_icons::paint(ui, dup_rect, Icon::NewWindow, weak_color, 1.2);
        }
    }

    ui.add_space(2.0);
}

fn paint_label_in_rect(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    text: &str,
    active: bool,
    in_background: bool,
    pane_accent: Option<egui::Color32>,
) {
    let font_id = egui::FontId::proportional(13.0);
    let text_color = if let Some(accent) = pane_accent {
        if in_background {
            interactive::accent_tone(accent, AccentTone::Dimmed)
        } else if active {
            accent
        } else {
            interactive::accent_tone(accent, AccentTone::Secondary)
        }
    } else if active {
        ui.visuals().selection.stroke.color
    } else if in_background {
        ui.visuals().weak_text_color()
    } else {
        ui.visuals().text_color()
    };
    let corner = style::CORNER_RADIUS_XS;

    let painter = ui.painter_at(rect);

    if let Some(accent) = pane_accent.filter(|_| !in_background) {
        let stripe = egui::Rect::from_min_size(rect.min, egui::vec2(3.0, rect.height()));
        painter.rect_filled(stripe, 1.0, accent);
    }

    let hovered = if active {
        false
    } else {
        rect.contains(ui.input(|i| i.pointer.interact_pos().unwrap_or(egui::Pos2::ZERO)))
    };
    let state = if active {
        RowState::Selected
    } else if hovered {
        RowState::Hovered
    } else {
        RowState::Default
    };
    let chrome = interactive::row_chrome(ui, state);
    if chrome.fill != egui::Color32::TRANSPARENT {
        painter.rect_filled(rect, corner, chrome.fill);
    }

    let clip = rect.shrink2(egui::vec2(4.0, 0.0));
    let full =
        ui.fonts_mut(|f| f.layout(text.to_owned(), font_id.clone(), text_color, f32::INFINITY));

    if full.size().x <= clip.width() {
        painter.galley(
            egui::pos2(clip.left(), clip.center().y - full.size().y * 0.5),
            full,
            text_color,
        );
    } else {
        let ellipsis = "…";
        let ellipsis_w = ui.fonts_mut(|f| {
            f.layout(
                ellipsis.to_owned(),
                font_id.clone(),
                text_color,
                f32::INFINITY,
            )
            .size()
            .x
        });
        let budget = (clip.width() - ellipsis_w).max(8.0);
        let chars: Vec<char> = text.chars().collect();
        let mut end = chars.len();
        while end > 0 {
            let s: String = chars[..end].iter().collect();
            let g = ui.fonts_mut(|f| f.layout(s, font_id.clone(), text_color, f32::INFINITY));
            if g.size().x <= budget {
                painter.galley(
                    egui::pos2(clip.left(), clip.center().y - g.size().y * 0.5),
                    g,
                    text_color,
                );
                painter.galley(
                    egui::pos2(clip.left() + budget, clip.center().y - full.size().y * 0.5),
                    ui.fonts_mut(|f| f.layout(ellipsis.into(), font_id, text_color, f32::INFINITY)),
                    text_color,
                );
                break;
            }
            end -= 1;
        }
    }
}
