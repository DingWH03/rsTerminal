use rsterm_session_core::{ActiveSession, ConnectionViewAction};
use rsterm_uiframe::clipboard::{read_text, write_text};
use rsterm_uiframe::{install_context_popup, menu_action, menu_action_enabled};

#[derive(Default, Clone, Copy)]
pub(super) struct TerminalMenuAction {
    copy: bool,
    paste: bool,
    clear_selection: bool,
}

pub(super) fn install(
    ui: &egui::Ui,
    resp: &egui::Response,
    has_selection: bool,
    force_popup: bool,
    action: &mut TerminalMenuAction,
) {
    let is_touch = ui.input(|i| i.has_touch_screen());
    let touch_open = is_touch
        .then_some(force_popup)
        .filter(|&open| open)
        .map(|_| egui::SetOpenCommand::Bool(true));
    if !is_touch {
        install_context_popup(resp, true, None, None, |ui| {
            contents(ui, has_selection, action)
        });
    } else {
        install_context_popup(resp, false, touch_open, None, |ui| {
            contents(ui, has_selection, action)
        });
    }
}

fn contents(ui: &mut egui::Ui, has_selection: bool, action: &mut TerminalMenuAction) {
    let labels = crate::labels::labels();
    if menu_action_enabled(ui, &labels.copy, has_selection) {
        action.copy = true;
    }
    if menu_action(ui, &labels.paste) {
        action.paste = true;
    }
    if menu_action_enabled(ui, &labels.clear_selection, has_selection) {
        action.clear_selection = true;
    }
}

pub(super) fn apply(
    session: &mut ActiveSession,
    ctx: &egui::Context,
    action: &mut ConnectionViewAction,
    menu_action: TerminalMenuAction,
) {
    if menu_action.copy {
        copy_selection_to_clipboard(session, ctx);
    }
    if menu_action.paste
        && let Some(text) = read_text()
    {
        super::paste_to_session(session, &text, ctx, action);
    }
    if menu_action.clear_selection {
        clear_selection(session);
    }
}

pub(super) fn copy_selection_to_clipboard(session: &mut ActiveSession, ctx: &egui::Context) {
    if let Some(selection) = session.view.selection.as_ref() {
        let text = selection.text(&session.core.terminal.screen);
        if !text.is_empty() {
            write_text(&text);
            ctx.copy_text(text);
        }
    }
    clear_selection(session);
}

fn clear_selection(session: &mut ActiveSession) {
    session.view.selection = None;
    session.view.selection_pointer = None;
    session.view.touch_state.show_handles = false;
    session.view.touch_state.touch_select_mode = false;
}
