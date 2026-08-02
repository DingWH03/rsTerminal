//! Favorite commands quick-input page in the function pane.

use crate::data::persist::types::FavoriteCommand;
use crate::ui::shell::messages::FunctionAction;
use crate::ui::uiframe::components::empty_state::{EmptyStateConfig, paint_empty_state};
use crate::ui::uiframe::style;
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
            EmptyStateConfig {
                vector_icon: Some(Icon::Commands),
                vector_icon_size: 44.0,
                title: &rust_i18n::t!("cmd_empty"),
                title_size: 13.0,
                ..Default::default()
            },
        );
        return action;
    }

    ui.style_mut().spacing.scroll.bar_width = 6.0;
    ui.style_mut().spacing.scroll.bar_outer_margin = 0.0;
    let menu_id_key = egui::Id::new(format!("{id_salt}_menu_id"));
    let menu_state: Option<String> = ui.data(|d| d.get_temp(menu_id_key)).unwrap_or(None);

    if menu_state.is_some() && ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary))
    {
        ui.data_mut(|d| d.insert_temp(menu_id_key, None::<String>));
    }

    egui::ScrollArea::vertical()
        .id_salt(format!("{id_salt}_list_scroll"))
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for cmd in commands {
                paint_command_row(ui, cmd, &menu_id_key, &menu_state, &mut action);
            }
        });

    action
}

fn paint_command_row(
    ui: &mut egui::Ui,
    cmd: &FavoriteCommand,
    menu_id_key: &egui::Id,
    menu_state: &Option<String>,
    action: &mut FunctionAction,
) {
    let row_h = 34.0;
    let row_rect =
        egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), row_h));
    let row_resp = ui.allocate_rect(row_rect, egui::Sense::click());

    let dots_rect = egui::Rect::from_min_size(
        egui::pos2(row_rect.right() - 24.0, row_rect.top()),
        egui::vec2(24.0, row_h),
    );
    let dots_id = ui.id().with(("dots", &cmd.id));
    let dots_resp = ui.interact(dots_rect, dots_id, egui::Sense::click());

    if row_resp.clicked() && !dots_resp.clicked() && !row_resp.long_touched() {
        ui.data_mut(|d| d.insert_temp(*menu_id_key, None::<String>));
        action.run_favorite_command = Some(cmd.id.clone());
    }

    row_resp.context_menu(|ui| {
        ui.data_mut(|d| d.insert_temp(*menu_id_key, None::<String>));
        paint_cmd_menu(ui, cmd, action);
    });
    if row_resp.long_touched() || dots_resp.clicked() {
        ui.data_mut(|d| d.insert_temp(*menu_id_key, Some(cmd.id.clone())));
    }

    if ui.is_rect_visible(row_rect) {
        let painter = ui.painter_at(row_rect);
        if row_resp.hovered() || menu_state.as_deref() == Some(cmd.id.as_str()) {
            painter.rect_filled(
                row_rect,
                style::CORNER_RADIUS_XS,
                ui.visuals().widgets.hovered.bg_fill,
            );
        }

        let text_left = row_rect.left() + 6.0;
        let name_w = row_rect.right() - text_left - 30.0;
        let name_g = ui.fonts_mut(|f| {
            f.layout(
                cmd.name.clone(),
                egui::FontId::proportional(13.0),
                ui.visuals().text_color(),
                name_w,
            )
        });
        painter.galley(
            egui::pos2(text_left, row_rect.top() + 2.0),
            name_g,
            ui.visuals().text_color(),
        );

        let mut subtitle = cmd.command.clone();
        if subtitle.len() > 48 {
            subtitle = format!("{}…", &subtitle[..48]);
        }
        if cmd.auto_execute {
            subtitle = format!("↵ {subtitle}");
        }
        let det_g = ui.fonts_mut(|f| {
            f.layout(
                subtitle,
                egui::FontId::proportional(10.0),
                ui.visuals().weak_text_color(),
                name_w,
            )
        });
        painter.galley(
            egui::pos2(text_left, row_rect.top() + 18.0),
            det_g,
            ui.visuals().weak_text_color(),
        );

        let dots_g = ui.fonts_mut(|f| {
            f.layout(
                "\u{22EE}".to_string(),
                egui::FontId::proportional(16.0),
                if dots_resp.hovered() {
                    ui.visuals().text_color()
                } else {
                    ui.visuals().weak_text_color()
                },
                f32::INFINITY,
            )
        });
        painter.galley(
            egui::pos2(
                dots_rect.center().x - dots_g.size().x / 2.0,
                dots_rect.center().y - dots_g.size().y / 2.0,
            ),
            dots_g,
            if dots_resp.hovered() {
                ui.visuals().text_color()
            } else {
                ui.visuals().weak_text_color()
            },
        );
    }

    if menu_state.as_deref() == Some(cmd.id.as_str()) {
        egui::Popup::from_response(&dots_resp)
            .id(dots_id.with("ctx"))
            .show(|ui| {
                ui.set_min_width(120.0);
                paint_cmd_menu(ui, cmd, action);
            });
    }

    ui.add_space(2.0);
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
