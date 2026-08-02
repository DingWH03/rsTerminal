use crate::session::{ActiveSession, ConnectionViewAction};
use crate::ui::function_pane::FunctionPane;
use crate::ui::uiframe::clipboard::read_text;
use crate::ui::uiframe::components::toolbar_button::{
    icon_toolbar_button, icon_toolbar_danger, icon_toolbar_toggle, text_toolbar_button,
};
use crate::ui::uiframe::keyboard::VirtualKeyboard;
use crate::ui::uiframe::vector_icons::Icon;

use super::context_menu::copy_selection_to_clipboard;
use super::paste_to_session;

pub(super) fn render(
    ui: &mut egui::Ui,
    session: &mut Option<&mut ActiveSession>,
    keyboard: &mut VirtualKeyboard,
    function_pane: &mut FunctionPane,
    pane_id: u64,
    in_split: bool,
    ctx: &egui::Context,
    action: &mut ConnectionViewAction,
) {
    let show_hamburger = !in_split && function_pane.show_content_hamburger();
    let show_actions = session
        .as_ref()
        .is_some_and(|s| s.view.touch_state.show_handles);
    let show_title = ui.available_width() > 320.0 && !show_actions;

    ui.horizontal(|ui| {
        ui.style_mut().spacing.button_padding = egui::vec2(2.0, 1.0);
        ui.style_mut().spacing.item_spacing.x = 2.0;

        if show_hamburger
            && icon_toolbar_button(ui, ui.id().with(("hdr_menu", pane_id)), Icon::Hamburger)
                .clicked()
        {
            function_pane.hamburger_click();
        }

        if show_actions {
            render_selection_actions(ui, session, ctx, action);
        } else if show_title {
            let title = session.as_ref().map(|s| s.tab_label()).unwrap_or_default();
            ui.label(
                egui::RichText::new(title)
                    .size(12.0)
                    .strong()
                    .color(ui.visuals().text_color()),
            );
        }

        render_port_switcher(ui, session, ctx);
        render_toolbar(ui, keyboard, pane_id, in_split, action);
    });
    ui.add(egui::Separator::default().spacing(2.0));
}

fn render_selection_actions(
    ui: &mut egui::Ui,
    session: &mut Option<&mut ActiveSession>,
    ctx: &egui::Context,
    action: &mut ConnectionViewAction,
) {
    let Some(session) = session.as_mut() else {
        return;
    };
    ui.scope(|ui| {
        ui.style_mut().spacing.button_padding = egui::vec2(5.0, 1.0);
        if ui
            .button(
                egui::RichText::new(rust_i18n::t!("copy"))
                    .size(11.0)
                    .strong(),
            )
            .clicked()
        {
            copy_selection_to_clipboard(session, ctx);
            ctx.request_repaint();
        }
        if ui
            .button(egui::RichText::new(rust_i18n::t!("paste")).size(11.0))
            .clicked()
        {
            if let Some(text) = read_text() {
                paste_to_session(session, &text, ctx, action);
            }
        }
        if ui
            .button(egui::RichText::new(rust_i18n::t!("cancel")).size(11.0))
            .clicked()
        {
            session.view.touch_state.show_handles = false;
            session.view.touch_state.touch_select_mode = false;
            session.view.selection = None;
            session.view.selection_pointer = None;
            ctx.request_repaint();
        }
    });
}

fn render_port_switcher(
    ui: &mut egui::Ui,
    session: &mut Option<&mut ActiveSession>,
    ctx: &egui::Context,
) {
    let Some(session) = session.as_mut() else {
        return;
    };
    if session.core.ports.len() <= 1 {
        return;
    }

    ui.separator();
    let port_buttons: Vec<(u8, String, bool, usize)> = session
        .core
        .ports
        .iter()
        .map(|p| {
            (
                p.port,
                p.name.clone(),
                p.port == session.core.active_port,
                *session.core.port_unread.get(&p.port).unwrap_or(&0),
            )
        })
        .collect();
    for (port, label, selected, unread) in port_buttons {
        let text = if unread > 0 && !selected {
            format!("{label} •")
        } else {
            label
        };
        if ui
            .selectable_label(selected, egui::RichText::new(text).size(11.0))
            .clicked()
        {
            session.switch_to_port(port);
            ctx.request_repaint();
        }
    }
}

fn render_toolbar(
    ui: &mut egui::Ui,
    keyboard: &mut VirtualKeyboard,
    pane_id: u64,
    in_split: bool,
    action: &mut ConnectionViewAction,
) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.style_mut().spacing.item_spacing.x = 2.0;

        if icon_toolbar_danger(ui, ui.id().with(("hdr_close", pane_id)), Icon::Close)
            .on_hover_text(rust_i18n::t!("close_pane"))
            .clicked()
        {
            *action = ConnectionViewAction::CloseSession;
        }

        if in_split {
            if icon_toolbar_button(ui, ui.id().with(("hdr_hide", pane_id)), Icon::Minimize)
                .on_hover_text(rust_i18n::t!("minimize_pane"))
                .clicked()
            {
                *action = ConnectionViewAction::MinimizePane;
            }
            return;
        }

        let mode_label = match keyboard.mode {
            crate::ui::uiframe::keyboard::KeyboardMode::Special => "Sp",
            crate::ui::uiframe::keyboard::KeyboardMode::Full => "Full",
        };
        if text_toolbar_button(ui, ui.id().with(("hdr_kbmode", pane_id)), mode_label)
            .on_hover_text(rust_i18n::t!("settings_default_keyboard"))
            .clicked()
        {
            keyboard.toggle_mode();
        }

        if icon_toolbar_toggle(
            ui,
            ui.id().with(("hdr_kb", pane_id)),
            Icon::Keyboard,
            keyboard.visible,
        )
        .on_hover_text(rust_i18n::t!("settings_default_keyboard"))
        .clicked()
        {
            keyboard.toggle();
            #[cfg(target_os = "android")]
            if keyboard.visible {
                keyboard.terminal_ime_enabled = false;
                super::input::hide_android_terminal_ime(ui.ctx());
            }
        }
    });
}
