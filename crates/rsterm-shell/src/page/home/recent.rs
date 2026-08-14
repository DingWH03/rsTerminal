//! 最近连接视图 — 在侧边栏中显示最近使用的连接列表。

use rsterm_data::persist::types::SavedConnection;
use crate::PaneChrome;
use crate::connection_display::connection_type_icon;
use crate::uiframe::components::compact_list_row::{CompactListRow, ListRowDensity};
use crate::uiframe::components::empty_state::{self, EmptyStateConfig};
use crate::uiframe::components::pane_header::PaneHeader;
use crate::uiframe::components::toolbar_button::{icon_toolbar_button, icon_toolbar_danger};
use crate::uiframe::style;
use crate::uiframe::tokens;
use crate::uiframe::vector_icons::Icon;

/// 最近连接最大显示数量
const MAX_RECENT_CONNECTIONS: usize = 20;
/// 底部"查看全部"按钮区域高度
const RECENT_FOOTER_HEIGHT: f32 = tokens::size::BUTTON;

/// 工作区窗格顶栏操作。
pub struct SplitPaneChrome<'a> {
    pub hide_pane: Option<&'a mut bool>,
    pub close_pane: Option<&'a mut bool>,
}

/// 渲染最近连接列表视图。
pub fn recent_connections_view(
    ui: &mut egui::Ui,
    chrome: &mut PaneChrome<'_>,
    connections: &[SavedConnection],
    connect_clicked: &mut Option<String>,
    more_clicked: &mut bool,
    split_chrome: Option<SplitPaneChrome<'_>>,
) {
    let mut recent: Vec<&SavedConnection> = connections.iter().collect();
    recent.sort_by(|a, b| {
        b.last_connected
            .as_deref()
            .unwrap_or("")
            .cmp(a.last_connected.as_deref().unwrap_or(""))
            .then_with(|| a.name.cmp(&b.name))
    });

    let show_count = recent.len().min(MAX_RECENT_CONNECTIONS);
    let recent = &recent[..show_count];

    let show_hamburger = chrome.show_hamburger;
    let title = crate::i18n_bridge::tr("recent_connections");
    let (mut hide_pane, mut close_pane) = match split_chrome {
        Some(c) => (c.hide_pane, c.close_pane),
        None => (None, None),
    };
    let mut trailing = |ui: &mut egui::Ui| {
        if let Some(close) = close_pane.as_deref_mut()
            && icon_toolbar_danger(ui, ui.id().with("recent_close"), Icon::Close)
                .on_hover_text(crate::i18n_bridge::tr("close_pane"))
                .clicked()
        {
            *close = true;
        }
        if let Some(hide) = hide_pane.as_deref_mut()
            && icon_toolbar_button(ui, ui.id().with("recent_hide"), Icon::Minimize)
                .on_hover_text(crate::i18n_bridge::tr("minimize_pane"))
                .clicked()
        {
            *hide = true;
        }
    };
    let header = PaneHeader {
        show_hamburger,
        hamburger_id: Some(ui.id().with("recent_menu")),
        title: Some(title.as_ref()),
        center: None,
        trailing: Some(&mut trailing),
    }
    .show(ui);
    if header.hamburger_clicked {
        (chrome.on_hamburger)();
    }

    if recent.is_empty() {
        empty_state::paint_empty_state(
            ui,
            EmptyStateConfig::compact(
                Icon::Connections,
                &crate::i18n_bridge::tr("home_no_connections"),
                Some(&crate::i18n_bridge::tr("open_terminal_hint")),
            ),
        );
        return;
    }

    let row_h = ListRowDensity::Standard.height();
    let row_step = row_h + tokens::space::XS;
    let desired_list_height = recent.len() as f32 * row_step;
    let available_list_height = (ui.available_height() - RECENT_FOOTER_HEIGHT).max(row_h);
    let list_height = desired_list_height.min(available_list_height);

    egui::ScrollArea::vertical()
        .id_salt("home_recent_connections")
        .auto_shrink([false, false])
        .max_height(list_height)
        .show(ui, |ui| {
            ui.style_mut().spacing.scroll.bar_width = 6.0;
            for conn in recent {
                let subtitle = crate::page::home::conn_subtitle(conn);
                let icon = connection_type_icon(conn.conn_type);
                let outcome = CompactListRow {
                    id: ui.id().with(("recent_row", &conn.id)),
                    density: ListRowDensity::Standard,
                    title: &conn.name,
                    subtitle: Some(&subtitle),
                    leading: Some(icon),
                    selected: false,
                    accent_stripe: None,
                    sense: egui::Sense::click(),
                    trailing_width: 0.0,
                    menu_open: false,
                }
                .show(ui);
                if outcome.response.as_ref().is_some_and(|r| r.clicked()) {
                    *connect_clicked = Some(conn.id.clone());
                }
            }
        });

    ui.add_space(tokens::space::SM);
    ui.horizontal(|ui| {
        ui.add_space(tokens::space::LG);
        let more_label = format!("{}  →", crate::i18n_bridge::tr("view_all"));
        if ui
            .button(
                egui::RichText::new(&more_label)
                    .size(tokens::text::COMPACT)
                    .color(style::ACCENT),
            )
            .clicked()
        {
            *more_clicked = true;
        }
    });
}
