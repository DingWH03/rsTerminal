//! Path bar editing, autocomplete, and search panel chrome for the file manager.

use std::path::PathBuf;

use egui::Key;
use rsterm_session_core::{FileActivePane, FileManagerSession, PathAutocompleteState};
use rsterm_uiframe::tokens;

use crate::labels;

use super::path_autocomplete::{
    build_suggestions, cancel_path_autocomplete, parse_path_input, request_path_autocomplete,
};

const PATH_BAR_MIN_WIDTH: f32 = 200.0;
const PATH_BAR_MAX_WIDTH: f32 = 480.0;
const PATH_BAR_WIDTH_FRAC: f32 = 0.55;

/// Result of painting the active-pane path / search chrome.
#[derive(Debug, Default)]
pub struct PathBarAction {
    pub go_up: bool,
    pub listing_changed: bool,
    pub path_submitted: Option<String>,
    pub kick_recursive_search: bool,
    pub cancel_recursive_search: bool,
}

pub fn paint_active_path_chrome(
    ui: &mut egui::Ui,
    session: &mut FileManagerSession,
) -> PathBarAction {
    match session.active_pane {
        FileActivePane::Remote => {
            let Some(remote) = session.remote.as_mut() else {
                return PathBarAction::default();
            };
            let cwd = remote.cwd.clone();
            let show_hidden = remote.show_hidden;
            paint_pane_path_row(
                ui,
                &mut session.path_autocomplete,
                FileActivePane::Remote,
                &mut remote.path_edit,
                &cwd,
                true,
                show_hidden,
            )
        }
        FileActivePane::LeftLocal => {
            let Some(left) = session.left_local.as_mut() else {
                return PathBarAction::default();
            };
            let cwd = left.cwd.display().to_string();
            let show_hidden = left.show_hidden;
            paint_pane_path_row(
                ui,
                &mut session.path_autocomplete,
                FileActivePane::LeftLocal,
                &mut left.path_edit,
                &cwd,
                false,
                show_hidden,
            )
        }
        FileActivePane::Right => {
            let cwd = session.right.cwd.display().to_string();
            let show_hidden = session.right.show_hidden;
            paint_pane_path_row(
                ui,
                &mut session.path_autocomplete,
                FileActivePane::Right,
                &mut session.right.path_edit,
                &cwd,
                false,
                show_hidden,
            )
        }
    }
}

fn paint_pane_path_row(
    ui: &mut egui::Ui,
    path_ac: &mut PathAutocompleteState,
    pane: FileActivePane,
    path_edit: &mut String,
    cwd: &str,
    remote: bool,
    show_hidden: bool,
) -> PathBarAction {
    use rsterm_uiframe::components::toolbar_button::text_toolbar_button;

    let mut action = PathBarAction::default();
    let labels = labels::labels();
    // Stable id — must not change when switching left/right pane focus.
    let path_id = egui::Id::new("fm_path_bar_edit");
    let up_id = egui::Id::new("fm_path_bar_up");

    ui.style_mut().spacing.item_spacing.x = tokens::space::XS;

    if text_toolbar_button(ui, up_id, "↑")
        .on_hover_text(&labels.parent_folder)
        .clicked()
    {
        action.go_up = true;
    }

    let focused = ui.memory(|m| m.has_focus(path_id));
    if !focused && path_edit.as_str() != cwd {
        *path_edit = cwd.to_string();
    }

    let pane_w = ui.ctx().content_rect().width();
    let path_w = ui
        .available_width()
        .min((pane_w * PATH_BAR_WIDTH_FRAC).clamp(PATH_BAR_MIN_WIDTH, PATH_BAR_MAX_WIDTH));

    let path_edit_resp = ui.add(
        egui::TextEdit::singleline(path_edit)
            .id(path_id)
            .desired_width(path_w)
            .font(egui::TextStyle::Small)
            .margin(egui::Margin::symmetric(4, 0))
            .hint_text(&labels.path_placeholder),
    );

    let popup_id = path_id.with("ac");
    let popup_open = egui::Popup::is_id_open(ui.ctx(), popup_id);

    if path_edit_resp.lost_focus() && !popup_open {
        cancel_path_autocomplete(path_ac);
    }

    if path_edit_resp.has_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
        cancel_path_autocomplete(path_ac);
        action.path_submitted = Some(path_edit.clone());
    }

    if path_edit_resp.has_focus() {
        if let Some(parsed) = parse_path_input(path_edit, cwd, remote) {
            request_path_autocomplete(path_ac, pane, path_edit, &parsed, remote, show_hidden);

            let suggestions = build_suggestions(&path_ac.entries, &parsed, remote, show_hidden);
            let loading = path_ac.loading;
            let show_popup = loading || !suggestions.is_empty();

            if show_popup {
                egui::Popup::open_id(ui.ctx(), popup_id);
                let mut popup_labels: Vec<String> = suggestions.clone();
                if loading {
                    popup_labels.push(labels.path_autocomplete_loading.clone());
                }
                let label_refs: Vec<&str> = popup_labels.iter().map(|s| s.as_str()).collect();
                let width = Some(rsterm_uiframe::measure_menu_width(
                    ui.ctx(),
                    &label_refs,
                    false,
                ));
                let loading_label = labels.path_autocomplete_loading.clone();
                rsterm_uiframe::show_anchor_popup(&path_edit_resp, popup_id, width, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(160.0)
                        .show(ui, |ui| {
                            if loading {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label(&loading_label);
                                });
                            }
                            for s in &suggestions {
                                if ui.selectable_label(false, s).clicked() {
                                    *path_edit = s.clone();
                                    action.path_submitted = Some(s.clone());
                                }
                            }
                        });
                });
                if action.path_submitted.is_some() {
                    cancel_path_autocomplete(path_ac);
                }
            }
        } else {
            cancel_path_autocomplete(path_ac);
        }
    } else if !popup_open {
        cancel_path_autocomplete(path_ac);
    }

    action
}

/// Paint the expandable advanced search strip under the top bar.
pub fn paint_search_panel(
    ui: &mut egui::Ui,
    session: &mut FileManagerSession,
    searching: bool,
) -> PathBarAction {
    let mut action = PathBarAction::default();
    let labels = labels::labels();

    let Some((filter, case_s, regex, recursive)) = (match session.active_pane {
        FileActivePane::Remote => session.remote.as_mut().map(|remote| {
            (
                &mut remote.filter,
                &mut remote.filter_case_sensitive,
                &mut remote.filter_regex,
                &mut remote.filter_recursive,
            )
        }),
        FileActivePane::LeftLocal => session.left_local.as_mut().map(|left| {
            (
                &mut left.filter,
                &mut left.filter_case_sensitive,
                &mut left.filter_regex,
                &mut left.filter_recursive,
            )
        }),
        FileActivePane::Right => Some((
            &mut session.right.filter,
            &mut session.right.filter_case_sensitive,
            &mut session.right.filter_regex,
            &mut session.right.filter_recursive,
        )),
    }) else {
        return action;
    };

    // Stable ids — must not change when switching left/right pane focus.
    let filter_id = egui::Id::new("fm_search_filter");
    let case_id = egui::Id::new("fm_search_case");
    let regex_id = egui::Id::new("fm_search_regex");
    let recursive_id = egui::Id::new("fm_search_recursive");

    ui.horizontal(|ui| {
        ui.style_mut().spacing.item_spacing.x = tokens::space::SM;
        ui.label(egui::RichText::new(&labels.search_query).size(tokens::text::CAPTION));

        let edit = ui.add(
            egui::TextEdit::singleline(filter)
                .id(filter_id)
                .desired_width(180.0)
                .hint_text(&labels.filter_placeholder),
        );
        if edit.changed() {
            action.listing_changed = true;
            if *recursive {
                action.kick_recursive_search = true;
            }
        }

        if ui
            .push_id(case_id, |ui| ui.checkbox(case_s, &labels.search_match_case))
            .inner
            .changed()
        {
            action.listing_changed = true;
            if *recursive {
                action.kick_recursive_search = true;
            }
        }
        let regex_changed = ui
            .push_id(regex_id, |ui| ui.checkbox(regex, &labels.search_regex))
            .inner
            .changed();
        if *regex && !filter.trim().is_empty() {
            let pattern = filter.trim();
            let ok = if *case_s {
                regex::Regex::new(pattern).is_ok()
            } else {
                regex::Regex::new(&format!("(?i){pattern}")).is_ok()
            };
            if !ok {
                ui.colored_label(rsterm_uiframe::style::RED, &labels.search_regex_invalid);
            }
        }
        if regex_changed {
            action.listing_changed = true;
            if *recursive {
                action.kick_recursive_search = true;
            }
        }
        if ui
            .push_id(recursive_id, |ui| {
                ui.checkbox(recursive, &labels.search_recursive)
            })
            .inner
            .changed()
        {
            action.listing_changed = true;
            if *recursive {
                action.kick_recursive_search = true;
            }
        }

        if searching {
            if ui.button(&labels.search_stop).clicked() {
                action.cancel_recursive_search = true;
            }
            ui.spinner();
        } else if ui.button(&labels.search_clear).clicked() {
            filter.clear();
            action.listing_changed = true;
        }
    });

    action
}

/// Apply a submitted path to a local pane.
pub fn apply_local_path(
    pane: &mut rsterm_session_core::FilePaneState,
    raw: &str,
) -> Result<(), String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Path is empty".into());
    }
    let path = if trimmed.starts_with('~') {
        let home = rsterm_fs::home_dir();
        if trimmed == "~" {
            home
        } else if let Some(rest) = trimmed.strip_prefix("~/") {
            home.join(rest)
        } else {
            PathBuf::from(trimmed)
        }
    } else {
        PathBuf::from(trimmed)
    };
    let path = if path.is_absolute() {
        path
    } else {
        pane.cwd.join(path)
    };
    if !path.is_dir() {
        return Err(format!("Not a directory: {}", path.display()));
    }
    pane.cwd = path;
    pane.sync_path_edit_from_cwd();
    pane.loading = true;
    pane.selected.clear();
    pane.focus_index = None;
    pane.filter_recursive = false;
    Ok(())
}

/// Apply a submitted path to a remote pane.
pub fn apply_remote_path(
    remote: &mut rsterm_session_core::RemotePane,
    raw: &str,
) -> Result<(), String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Path is empty".into());
    }
    let path = if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        let base = remote.cwd.trim_end_matches('/');
        format!("{base}/{trimmed}")
    };
    remote.client.list_dir(&path)?;
    remote.cwd = path;
    remote.sync_path_edit_from_cwd();
    remote.loading = true;
    remote.selected.clear();
    remote.focus_index = None;
    remote.filter_recursive = false;
    Ok(())
}
