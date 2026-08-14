use rsterm_session_core::{ActiveSession, ConnectionViewAction};
use rsterm_uiframe::clipboard::{read_text, write_text};

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
    let menu_id = resp.id.with("terminal_ctx_popup");
    let is_touch = ui.input(|i| i.has_touch_screen());

    if !is_touch {
        resp.context_menu(|ui| contents(ui, has_selection, action));
    }

    let touch_open = force_popup.then_some(egui::SetOpenCommand::Bool(true));
    egui::Popup::from_response(resp)
        .id(menu_id)
        .open_memory(touch_open)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_min_width(150.0);
            contents(ui, has_selection, action);
        });
}

fn contents(ui: &mut egui::Ui, has_selection: bool, action: &mut TerminalMenuAction) {
    let labels = crate::labels::labels();
    if ui
        .add_enabled(has_selection, egui::Button::new(&labels.copy))
        .clicked()
    {
        action.copy = true;
        ui.close();
    }
    if ui.button(&labels.paste).clicked() {
        action.paste = true;
        ui.close();
    }
    if ui
        .add_enabled(
            has_selection,
            egui::Button::new(&labels.clear_selection),
        )
        .clicked()
    {
        action.clear_selection = true;
        ui.close();
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
