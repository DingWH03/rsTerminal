use rsterm_session_core::{ActiveSession, ConnectionViewAction};
use rsterm_uiframe::PaneChrome;
use rsterm_uiframe::clipboard::read_text;
use rsterm_uiframe::components::pane_header::PaneHeader;
use rsterm_uiframe::components::toolbar_button::{
    icon_toolbar_button, icon_toolbar_danger, icon_toolbar_toggle, text_toolbar_button,
};
use rsterm_uiframe::keyboard::VirtualKeyboard;
use rsterm_uiframe::tokens;
use rsterm_uiframe::vector_icons::Icon;

use super::context_menu::copy_selection_to_clipboard;
use super::paste_to_session;
use crate::labels;

pub(super) fn render(
    ui: &mut egui::Ui,
    session: &mut Option<&mut ActiveSession>,
    keyboard: &mut VirtualKeyboard,
    chrome: &mut PaneChrome<'_>,
    pane_id: u64,
    in_split: bool,
    ctx: &egui::Context,
    action: &mut ConnectionViewAction,
) {
    let labels = labels::labels();
    let show_hamburger = chrome.show_hamburger;
    let show_actions = session
        .as_ref()
        .is_some_and(|s| s.view.touch_state.show_handles);
    let show_title = ui.available_width() > 320.0 && !show_actions;
    let title = if show_title {
        session.as_ref().map(|s| s.tab_label()).unwrap_or_default()
    } else {
        String::new()
    };

    let mut copy = false;
    let mut paste = false;
    let mut cancel_sel = false;
    let mut close = false;
    let mut minimize = false;
    let mut toggle_kb = false;
    let mut toggle_kb_mode = false;
    let mut switch_port: Option<u8> = None;

    let mut center = |ui: &mut egui::Ui| {
        if show_actions {
            ui.scope(|ui| {
                ui.style_mut().spacing.button_padding =
                    egui::vec2(tokens::space::MD, tokens::space::XS * 0.5);
                if text_toolbar_button(ui, ui.id().with("sel_copy"), &labels.copy)
                    .clicked()
                {
                    copy = true;
                }
                if text_toolbar_button(ui, ui.id().with("sel_paste"), &labels.paste)
                    .clicked()
                {
                    paste = true;
                }
                if text_toolbar_button(ui, ui.id().with("sel_cancel"), &labels.cancel)
                    .clicked()
                {
                    cancel_sel = true;
                }
            });
        } else if show_title {
            ui.label(
                egui::RichText::new(&title)
                    .size(tokens::text::COMPACT)
                    .strong()
                    .color(ui.visuals().text_color()),
            );
        }

        if let Some(session) = session.as_ref()
            && session.core.ports.len() > 1
        {
            ui.separator();
            for p in &session.core.ports {
                let selected = p.port == session.core.active_port;
                let unread = *session.core.port_unread.get(&p.port).unwrap_or(&0);
                let text = if unread > 0 && !selected {
                    format!("{} •", p.name)
                } else {
                    p.name.clone()
                };
                if ui
                    .selectable_label(
                        selected,
                        egui::RichText::new(text).size(tokens::text::SMALL),
                    )
                    .clicked()
                {
                    switch_port = Some(p.port);
                }
            }
        }
    };

    let mut trailing = |ui: &mut egui::Ui| {
        if in_split {
            if icon_toolbar_danger(ui, ui.id().with(("hdr_close", pane_id)), Icon::Close)
                .on_hover_text(&labels.close_pane)
                .clicked()
            {
                close = true;
            }
            if icon_toolbar_button(ui, ui.id().with(("hdr_hide", pane_id)), Icon::Minimize)
                .on_hover_text(&labels.minimize_pane)
                .clicked()
            {
                minimize = true;
            }
        } else {
            if icon_toolbar_danger(ui, ui.id().with(("hdr_close", pane_id)), Icon::Close)
                .on_hover_text(&labels.close_pane)
                .clicked()
            {
                close = true;
            }

            let mode_label = match keyboard.mode {
                rsterm_uiframe::keyboard::KeyboardMode::Special => "Sp",
                rsterm_uiframe::keyboard::KeyboardMode::Full => "Full",
            };
            if text_toolbar_button(ui, ui.id().with(("hdr_kbmode", pane_id)), mode_label)
                .on_hover_text(&labels.settings_default_keyboard)
                .clicked()
            {
                toggle_kb_mode = true;
            }

            if icon_toolbar_toggle(
                ui,
                ui.id().with(("hdr_kb", pane_id)),
                Icon::Keyboard,
                keyboard.visible,
            )
            .on_hover_text(&labels.settings_default_keyboard)
            .clicked()
            {
                toggle_kb = true;
            }
        }
    };

    let outcome = PaneHeader {
        show_hamburger,
        hamburger_id: Some(ui.id().with(("hdr_menu", pane_id))),
        title: None,
        center: Some(&mut center),
        trailing: Some(&mut trailing),
    }
    .show(ui);
    if outcome.hamburger_clicked {
        (chrome.on_hamburger)();
    }

    if let Some(port) = switch_port
        && let Some(session) = session.as_mut()
    {
        session.switch_to_port(port);
        ctx.request_repaint();
    }
    if copy && let Some(session) = session.as_mut() {
        copy_selection_to_clipboard(session, ctx);
        ctx.request_repaint();
    }
    if paste
        && let Some(session) = session.as_mut()
        && let Some(text) = read_text()
    {
        paste_to_session(session, &text, ctx, action);
    }
    if cancel_sel && let Some(session) = session.as_mut() {
        session.view.touch_state.show_handles = false;
        session.view.touch_state.touch_select_mode = false;
        session.view.selection = None;
        session.view.selection_pointer = None;
        ctx.request_repaint();
    }
    if close {
        *action = ConnectionViewAction::CloseSession;
    }
    if minimize {
        *action = ConnectionViewAction::MinimizePane;
    }
    if toggle_kb_mode {
        keyboard.toggle_mode();
    }
    if toggle_kb {
        keyboard.toggle();
        #[cfg(target_os = "android")]
        if keyboard.visible {
            keyboard.terminal_ime_enabled = false;
            crate::page::input::hide_android_terminal_ime(ui.ctx());
        }
    }
}
