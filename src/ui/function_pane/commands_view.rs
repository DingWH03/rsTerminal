//! Favorite commands quick-input page in the function pane.

use crate::data::persist::types::FavoriteCommand;
use crate::ui::shell::messages::FunctionAction;
use crate::ui::uiframe::components::compact_list_row::{CompactListRow, ListRowDensity};
use crate::ui::uiframe::components::empty_state::{EmptyStateConfig, paint_empty_state};
use crate::ui::uiframe::components::overflow_menu::{self, OverflowMenuState};
use crate::ui::uiframe::vector_icons::Icon;

pub fn render(ui: &mut egui::Ui, commands: &[FavoriteCommand]) -> FunctionAction {
    render_with_id(ui, commands, "function_cmds")
}

pub fn render_with_id(
    ui: &mut egui::Ui,
    commands: &[FavoriteCommand],
    id_salt: &str,
) -> FunctionAction {
    let mut action = FunctionAction::empty();

    if commands.is_empty() {
        paint_empty_state(
            ui,
            EmptyStateConfig::compact(Icon::Commands, &rust_i18n::t!("cmd_empty"), None),
        );
        return action;
    }

    ui.style_mut().spacing.scroll.bar_width = 6.0;
    ui.style_mut().spacing.scroll.bar_outer_margin = 0.0;
    let menu_id_key = egui::Id::new(format!("{id_salt}_menu_id"));
    let mut menu_state = OverflowMenuState::load(ui, menu_id_key);

    if menu_state.open_id.is_some()
        && ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary))
    {
        menu_state.close();
    }

    egui::ScrollArea::vertical()
        .id_salt(format!("{id_salt}_list_scroll"))
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for cmd in commands {
                paint_command_row(ui, cmd, &mut menu_state, &mut action);
            }
        });

    menu_state.store(ui, menu_id_key);
    action
}

fn paint_command_row(
    ui: &mut egui::Ui,
    cmd: &FavoriteCommand,
    menu_state: &mut OverflowMenuState,
    action: &mut FunctionAction,
) {
    let mut subtitle = cmd.command.clone();
    if subtitle.len() > 48 {
        subtitle = format!("{}…", &subtitle[..48]);
    }
    if cmd.auto_execute {
        subtitle = format!("↵ {subtitle}");
    }

    let outcome = CompactListRow {
        id: ui.id().with(("cmd_row", &cmd.id)),
        density: ListRowDensity::Standard,
        title: &cmd.name,
        subtitle: Some(&subtitle),
        leading: None,
        selected: false,
        accent_stripe: None,
        sense: egui::Sense::click(),
        trailing_width: 24.0,
        menu_open: menu_state.is_open(&cmd.id),
    }
    .show(ui);

    let Some(row_resp) = outcome.response else {
        return;
    };
    let Some(dots_resp) = outcome.trailing_response else {
        return;
    };
    let dots_id = ui.id().with(("dots", &cmd.id));

    if row_resp.clicked() && !dots_resp.clicked() && !row_resp.long_touched() {
        menu_state.close();
        action.run_favorite_command = Some(cmd.id.clone());
    }

    row_resp.context_menu(|ui| {
        menu_state.close();
        paint_cmd_menu(ui, cmd, action);
    });
    overflow_menu::overflow_trigger(ui, &dots_resp, &row_resp, &cmd.id, menu_state);
    overflow_menu::show_if_open(ui, &dots_resp, dots_id, &cmd.id, menu_state, 120.0, |ui| {
        paint_cmd_menu(ui, cmd, action);
    });
}

fn paint_cmd_menu(ui: &mut egui::Ui, cmd: &FavoriteCommand, action: &mut FunctionAction) {
    if ui.button(rust_i18n::t!("cmd_run")).clicked() {
        action.run_favorite_command = Some(cmd.id.clone());
        ui.close();
    }
    if ui.button(rust_i18n::t!("edit")).clicked() {
        action.edit_favorite_command = Some(cmd.id.clone());
        ui.close();
    }
    if ui.button(rust_i18n::t!("delete")).clicked() {
        action.delete_favorite_command = Some(cmd.id.clone());
        ui.close();
    }
}
