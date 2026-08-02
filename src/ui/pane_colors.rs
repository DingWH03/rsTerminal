//! Per-pane accent colors — sidebar labels match workspace pane borders.

use std::collections::HashMap;

use crate::data::prefs::Prefs;
use crate::ui::layout::{PaneId, WorkspaceLayout};
use crate::ui::uiframe::style;

pub fn default_palette() -> Vec<[u8; 3]> {
    vec![
        [74, 158, 255],
        [255, 149, 64],
        [88, 214, 141],
        [255, 107, 129],
        [167, 139, 250],
        [255, 214, 102],
    ]
}

pub fn palette_for_theme(ui_theme: crate::i18n::UiTheme) -> Vec<[u8; 3]> {
    match ui_theme {
        crate::i18n::UiTheme::Light => vec![
            [36, 114, 200],
            [220, 120, 20],
            [40, 160, 90],
            [210, 60, 90],
            [120, 90, 200],
            [180, 140, 20],
        ],
        _ => default_palette(),
    }
}

pub fn resolve_palette(prefs: &Prefs) -> Vec<[u8; 3]> {
    if prefs.appearance.pane_accent_colors.is_empty() {
        palette_for_theme(prefs.appearance.ui_theme)
    } else {
        prefs.appearance.pane_accent_colors.clone()
    }
}

pub fn color_at_index(prefs: &Prefs, index: usize) -> egui::Color32 {
    let palette = resolve_palette(prefs);
    if palette.is_empty() {
        return style::ACCENT;
    }
    let [r, g, b] = palette[index % palette.len()];
    egui::Color32::from_rgb(r, g, b)
}

pub fn pane_color(prefs: &Prefs, color_index: usize) -> egui::Color32 {
    color_at_index(prefs, color_index)
}

pub fn next_color_index(workspace: &WorkspaceLayout, palette_len: usize) -> usize {
    if palette_len == 0 {
        return 0;
    }
    workspace.pane_count() % palette_len
}

pub fn session_accent_map(
    workspace: &WorkspaceLayout,
    prefs: &Prefs,
) -> HashMap<String, egui::Color32> {
    let mut map = HashMap::new();
    for state in workspace.panes.values() {
        if let Some(ref sid) = state.session_id {
            map.insert(sid.clone(), pane_color(prefs, state.color_index));
        }
    }
    map
}

pub fn pane_id_color(workspace: &WorkspaceLayout, prefs: &Prefs, pane_id: PaneId) -> egui::Color32 {
    workspace
        .panes
        .get(&pane_id)
        .map(|p| pane_color(prefs, p.color_index))
        .unwrap_or(style::ACCENT)
}
