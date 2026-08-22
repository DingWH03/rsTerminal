//! File manager top-bar settings popup (view / layout / hidden files).

use crate::labels;
use rsterm_data::prefs::FileManagerPrefs;
use rsterm_session_core::{FileActivePane, FileManagerSession};
use rsterm_uiframe::file_list::{FilePaneLayout, FileViewMode};
use rsterm_uiframe::{
    PopupMenuState, measure_menu_width, menu_action, menu_check, menu_heading, menu_separator,
    popup_from_response,
};

use crate::content::persist_file_manager_prefs_full;

#[derive(Debug, Default)]
pub struct FmSettingsMenuAction {
    pub listing_changed: bool,
    pub open_settings: bool,
}

pub fn paint_fm_settings_menu(
    settings_btn: &egui::Response,
    settings_menu: &mut PopupMenuState,
    session: &mut FileManagerSession,
    view_mode: &mut FileViewMode,
    pane_layout: &mut FilePaneLayout,
    pending_prefs: &mut Option<FileManagerPrefs>,
) -> FmSettingsMenuAction {
    let mut action = FmSettingsMenuAction::default();
    let labels = labels::labels();
    let popup_id = settings_btn.id.with("fm_settings_popup");

    if settings_btn.clicked() {
        settings_menu.toggle(&settings_btn.ctx, popup_id);
    }

    let width_labels: Vec<String> = [
        labels.view_list.as_str(),
        labels.view_details.as_str(),
        labels.view_icons_small.as_str(),
        labels.view_icons_large.as_str(),
        labels.layout_dual.as_str(),
        labels.show_hidden.as_str(),
        labels.settings_open_in_prefs.as_str(),
    ]
    .into_iter()
    .map(|s| format!("✓ {s}"))
    .collect();
    let label_refs: Vec<&str> = width_labels.iter().map(|s| s.as_str()).collect();
    let menu_width = Some(measure_menu_width(&settings_btn.ctx, &label_refs, false));

    let mut close_menu = false;
    popup_from_response(settings_btn, popup_id, settings_menu, menu_width, |ui| {
        menu_heading(ui, &labels.settings_view_group);
        for (mode, text) in [
            (FileViewMode::List, labels.view_list.as_str()),
            (FileViewMode::Details, labels.view_details.as_str()),
            (FileViewMode::IconsSmall, labels.view_icons_small.as_str()),
            (FileViewMode::IconsLarge, labels.view_icons_large.as_str()),
        ] {
            if menu_check(ui, text, *view_mode == mode) {
                *view_mode = mode;
                commit_prefs(session, view_mode, pane_layout, pending_prefs);
            }
        }

        menu_separator(ui);
        menu_heading(ui, &labels.settings_layout_group);
        let dual = matches!(*pane_layout, FilePaneLayout::Dual);
        if menu_check(ui, &labels.layout_dual, dual) {
            *pane_layout = if dual {
                FilePaneLayout::Single
            } else {
                FilePaneLayout::Dual
            };
            commit_prefs(session, view_mode, pane_layout, pending_prefs);
        }

        menu_separator(ui);
        menu_heading(ui, &labels.settings_display_group);
        let hidden = active_show_hidden(session);
        if menu_check(ui, &labels.show_hidden, hidden) {
            set_active_show_hidden(session, !hidden);
            action.listing_changed = true;
            commit_prefs(session, view_mode, pane_layout, pending_prefs);
        }

        menu_separator(ui);
        if menu_action(ui, &labels.settings_open_in_prefs) {
            action.open_settings = true;
            close_menu = true;
        }
    });
    if close_menu {
        settings_menu.close_synced(&settings_btn.ctx);
    }

    action
}

fn active_show_hidden(session: &FileManagerSession) -> bool {
    match session.active_pane {
        FileActivePane::Remote => session
            .remote
            .as_ref()
            .map(|r| r.show_hidden)
            .unwrap_or(false),
        FileActivePane::LeftLocal => session
            .left_local
            .as_ref()
            .map(|p| p.show_hidden)
            .unwrap_or(false),
        FileActivePane::Right => session.right.show_hidden,
    }
}

fn set_active_show_hidden(session: &mut FileManagerSession, v: bool) {
    match session.active_pane {
        FileActivePane::Remote => {
            if let Some(r) = session.remote.as_mut() {
                r.show_hidden = v;
            }
        }
        FileActivePane::LeftLocal => {
            if let Some(p) = session.left_local.as_mut() {
                p.show_hidden = v;
            }
        }
        FileActivePane::Right => session.right.show_hidden = v,
    }
    if let Some(left) = session.left_local.as_mut() {
        left.show_hidden = v;
    }
    if let Some(remote) = session.remote.as_mut() {
        remote.show_hidden = v;
    }
    session.right.show_hidden = v;
}

fn commit_prefs(
    session: &FileManagerSession,
    view_mode: &FileViewMode,
    pane_layout: &FilePaneLayout,
    pending_prefs: &mut Option<FileManagerPrefs>,
) {
    let show_hidden = active_show_hidden(session);
    *pending_prefs = Some(persist_file_manager_prefs_full(
        *view_mode,
        *pane_layout,
        show_hidden,
    ));
}
