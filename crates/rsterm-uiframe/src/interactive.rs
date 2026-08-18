//! Theme-aware interaction chrome shared by compact widgets.

use egui::{Color32, Stroke, Ui};

use super::tokens;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RowState {
    #[default]
    Default,
    Hovered,
    Selected,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowChrome {
    pub fill: Color32,
    pub stroke: Stroke,
}

/// Standard transparent row chrome for default, hover, and selection states.
pub fn row_chrome(ui: &Ui, state: RowState) -> RowChrome {
    match state {
        RowState::Default => RowChrome {
            fill: Color32::TRANSPARENT,
            stroke: Stroke::NONE,
        },
        RowState::Hovered => RowChrome {
            fill: ui.visuals().widgets.hovered.bg_fill,
            stroke: Stroke::new(
                tokens::stroke::HAIRLINE,
                ui.visuals().widgets.hovered.bg_stroke.color,
            ),
        },
        RowState::Selected => RowChrome {
            fill: ui.visuals().selection.bg_fill,
            stroke: Stroke::new(
                tokens::stroke::HAIRLINE,
                ui.visuals().selection.stroke.color,
            ),
        },
    }
}

/// Card chrome uses the same interaction states, with a surfaced default.
pub fn card_chrome(ui: &Ui, state: RowState) -> RowChrome {
    if state == RowState::Default {
        RowChrome {
            fill: ui.visuals().extreme_bg_color,
            stroke: ui.visuals().widgets.noninteractive.bg_stroke,
        }
    } else {
        let mut chrome = row_chrome(ui, state);
        if state == RowState::Selected {
            chrome.stroke.width = tokens::stroke::EMPHASIS;
        }
        chrome
    }
}

pub fn state(selected: bool, hovered: bool) -> RowState {
    if selected {
        RowState::Selected
    } else if hovered {
        RowState::Hovered
    } else {
        RowState::Default
    }
}

/// Named accent treatments for non-row overlays and subdued pane chrome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccentTone {
    Faint,
    Subtle,
    Dimmed,
    Soft,
    Muted,
    Secondary,
}

pub fn accent_tone(color: Color32, tone: AccentTone) -> Color32 {
    let factor = match tone {
        AccentTone::Faint => 0.12,
        AccentTone::Subtle => 0.28,
        AccentTone::Dimmed => 0.45,
        AccentTone::Soft => 0.5,
        AccentTone::Muted => 0.55,
        AccentTone::Secondary => 0.85,
    };
    color.gamma_multiply(factor)
}
