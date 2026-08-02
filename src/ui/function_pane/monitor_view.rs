//! Sidebar Monitor tab — one-minute (60 sample) performance charts for SSH sessions.

use crate::data::persist::types::ConnectionType;
use crate::remote::{METRICS_HISTORY_LEN, MetricsSample};
use crate::session::WorkspaceSession;
use crate::ui::shell::messages::FunctionAction;
use crate::ui::uiframe::components::empty_state::{EmptyStateConfig, paint_empty_state};
use crate::ui::uiframe::style;

const CHART_H: f32 = 72.0;
const CHART_PAD: f32 = 4.0;

pub fn render(
    ui: &mut egui::Ui,
    sessions: &[WorkspaceSession],
    focused_session_id: Option<&str>,
) -> FunctionAction {
    let Some(id) = focused_session_id else {
        paint_empty(
            ui,
            "\u{1F4CA}",
            &rust_i18n::t!("sidebar_monitor_no_terminal"),
            Some(&rust_i18n::t!("sidebar_monitor_no_terminal_hint")),
        );
        return FunctionAction::empty();
    };

    let Some(WorkspaceSession::Terminal(term)) = sessions.iter().find(|s| s.id() == id) else {
        paint_empty(
            ui,
            "\u{1F4CA}",
            &rust_i18n::t!("sidebar_monitor_no_terminal"),
            Some(&rust_i18n::t!("sidebar_monitor_no_terminal_hint")),
        );
        return FunctionAction::empty();
    };

    if term.core.conn_type != ConnectionType::Ssh {
        paint_empty(
            ui,
            "\u{1F4CA}",
            &rust_i18n::t!("sidebar_monitor_ssh_only"),
            Some(&rust_i18n::t!("sidebar_monitor_ssh_only_hint")),
        );
        return FunctionAction::empty();
    }

    let history = term.core.metrics.history();
    let snap = term.core.metrics.snapshot();

    if history.is_empty() {
        paint_empty(
            ui,
            "\u{23F3}",
            &rust_i18n::t!("sidebar_monitor_waiting"),
            Some(&rust_i18n::t!("sidebar_monitor_waiting_hint")),
        );
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(500));
        return FunctionAction::empty();
    }

    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(1000));

    let text = ui.visuals().text_color();
    let muted = ui.visuals().weak_text_color();

    egui::ScrollArea::vertical()
        .id_salt("sidebar_monitor_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(2.0);

            let host = snap
                .as_ref()
                .and_then(|s| s.hostname.as_deref())
                .unwrap_or("—");
            let uptime_label = snap
                .as_ref()
                .and_then(|s| s.uptime_secs)
                .map(|secs| {
                    let formatted = fmt_uptime(secs);
                    rust_i18n::t!("sidebar_monitor_uptime", time = formatted).to_string()
                })
                .unwrap_or_else(|| rust_i18n::t!("sidebar_monitor_uptime_unknown").to_string());

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(host).strong().color(text).size(13.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(uptime_label).color(muted).size(11.0));
                });
            });
            ui.add_space(2.0);

            let last = history.last().copied().unwrap_or_default();
            let cpu_title = rust_i18n::t!("sidebar_monitor_cpu");
            let mem_title = rust_i18n::t!("sidebar_monitor_mem");
            let disk_title = rust_i18n::t!("sidebar_monitor_disk");

            paint_series_card(
                ui,
                cpu_title.as_ref(),
                format!("{:.2}", last.load1),
                accent_for(ui, SeriesKind::Load),
                &history,
                SeriesKind::Load,
            );
            ui.add_space(4.0);
            paint_series_card(
                ui,
                mem_title.as_ref(),
                format!(
                    "{:.0}% · {}",
                    last.mem_used_ratio * 100.0,
                    fmt_bytes(last.mem_used)
                ),
                accent_for(ui, SeriesKind::Mem),
                &history,
                SeriesKind::Mem,
            );
            ui.add_space(4.0);
            paint_series_card(
                ui,
                disk_title.as_ref(),
                format!(
                    "{:.0}% · {} free",
                    last.disk_used_ratio * 100.0,
                    fmt_bytes(last.disk_avail)
                ),
                accent_for(ui, SeriesKind::Disk),
                &history,
                SeriesKind::Disk,
            );
        });

    FunctionAction::empty()
}

fn paint_empty(ui: &mut egui::Ui, icon: &str, title: &str, subtitle: Option<&str>) {
    paint_empty_state(
        ui,
        EmptyStateConfig {
            icon,
            title,
            subtitle,
            ..Default::default()
        },
    );
}

#[derive(Clone, Copy)]
enum SeriesKind {
    Load,
    Mem,
    Disk,
}

/// Series colors that stay readable on both light and dark themes.
fn accent_for(ui: &egui::Ui, kind: SeriesKind) -> egui::Color32 {
    let dark = ui.visuals().dark_mode;
    match kind {
        SeriesKind::Load => {
            if dark {
                style::ACCENT
            } else {
                egui::Color32::from_rgb(30, 110, 210)
            }
        }
        SeriesKind::Mem => {
            if dark {
                style::GREEN
            } else {
                egui::Color32::from_rgb(20, 140, 80)
            }
        }
        SeriesKind::Disk => {
            if dark {
                style::AMBER
            } else {
                egui::Color32::from_rgb(180, 110, 10)
            }
        }
    }
}

fn paint_series_card(
    ui: &mut egui::Ui,
    title: &str,
    value: String,
    color: egui::Color32,
    samples: &[MetricsSample],
    kind: SeriesKind,
) {
    let text = ui.visuals().text_color();
    let muted = ui.visuals().weak_text_color();
    let fill = if ui.visuals().dark_mode {
        ui.visuals().extreme_bg_color.gamma_multiply(0.55)
    } else {
        ui.visuals().widgets.noninteractive.bg_fill
    };
    let border = if ui.visuals().dark_mode {
        style::BORDER_SUBTLE
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke.color
    };

    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, border))
        .corner_radius(style::CORNER_RADIUS_SM)
        .inner_margin(egui::Margin::symmetric(4, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(title).color(muted).size(11.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(value).color(color).strong().size(12.0));
                });
            });
            ui.add_space(4.0);
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), CHART_H),
                egui::Sense::hover(),
            );
            if ui.is_rect_visible(rect) {
                paint_sparkline(ui, rect, samples, kind, color, text);
            }
        });
}

fn paint_sparkline(
    ui: &egui::Ui,
    rect: egui::Rect,
    samples: &[MetricsSample],
    kind: SeriesKind,
    color: egui::Color32,
    _text: egui::Color32,
) {
    let painter = ui.painter_at(rect);
    let plot = rect.shrink2(egui::vec2(CHART_PAD, CHART_PAD));
    if plot.width() < 8.0 || plot.height() < 8.0 || samples.is_empty() {
        return;
    }

    let grid = if ui.visuals().dark_mode {
        style::BORDER_SUBTLE
    } else {
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 28)
    };
    painter.line_segment(
        [plot.left_bottom(), plot.right_bottom()],
        egui::Stroke::new(1.0, grid),
    );
    let mid_y = plot.center().y;
    painter.line_segment(
        [
            egui::pos2(plot.left(), mid_y),
            egui::pos2(plot.right(), mid_y),
        ],
        egui::Stroke::new(1.0, grid.gamma_multiply(0.85)),
    );

    let n = samples.len();
    let slots = METRICS_HISTORY_LEN.max(n);
    let y_max = match kind {
        SeriesKind::Load => {
            let peak = samples.iter().map(|s| s.load1).fold(0.0_f32, f32::max);
            peak.max(1.0) * 1.15
        }
        SeriesKind::Mem | SeriesKind::Disk => 1.0,
    };

    let value_of = |s: &MetricsSample| -> f32 {
        match kind {
            SeriesKind::Load => s.load1,
            SeriesKind::Mem => s.mem_used_ratio,
            SeriesKind::Disk => s.disk_used_ratio,
        }
    };

    let offset = slots - n;
    let mut points: Vec<egui::Pos2> = Vec::with_capacity(n);
    for (i, s) in samples.iter().enumerate() {
        let xi = (offset + i) as f32;
        let t = if slots <= 1 {
            1.0
        } else {
            xi / (slots - 1) as f32
        };
        let v = value_of(s).clamp(0.0, y_max);
        let y_t = 1.0 - (v / y_max);
        points.push(egui::pos2(
            plot.left() + t * plot.width(),
            plot.top() + y_t * plot.height(),
        ));
    }

    if points.len() >= 2 {
        let fill_alpha = if ui.visuals().dark_mode { 36 } else { 48 };
        let fill_color =
            egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), fill_alpha);
        let bottom = plot.bottom();
        for w in points.windows(2) {
            let a = w[0];
            let b = w[1];
            painter.add(egui::Shape::convex_polygon(
                vec![a, b, egui::pos2(b.x, bottom), egui::pos2(a.x, bottom)],
                fill_color,
                egui::Stroke::NONE,
            ));
        }
        painter.add(egui::Shape::line(
            points.clone(),
            egui::Stroke::new(1.8, color),
        ));
        if let Some(&p) = points.last() {
            painter.circle_filled(p, 2.6, color);
            painter.circle_stroke(p, 2.6, egui::Stroke::new(1.0, ui.visuals().panel_fill));
        }
    } else if let Some(&p) = points.first() {
        painter.circle_filled(p, 2.6, color);
    }
}

fn fmt_bytes(n: u64) -> String {
    const K: f64 = 1024.0;
    let v = n as f64;
    if v >= K * K * K {
        format!("{:.1}G", v / (K * K * K))
    } else if v >= K * K {
        format!("{:.0}M", v / (K * K))
    } else if v >= K {
        format!("{:.0}K", v / K)
    } else {
        format!("{n}B")
    }
}

fn fmt_uptime(secs: u64) -> String {
    let d = secs / 86_400;
    let h = (secs % 86_400) / 3_600;
    let m = (secs % 3_600) / 60;
    if d > 0 {
        format!("{d}d {h}h")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}
