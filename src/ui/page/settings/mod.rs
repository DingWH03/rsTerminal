//! 设置页面 — 管理应用配置、终端主题、键盘快捷键和行为设置。
//!
//! 提供全屏设置页（`settings_page`）和工作区侧面板（`settings_side_panel`）两种布局。
//! 设置按标签页组织：通用、配置文件、外观、主题、行为、高级。

use crate::config::{BellStyle, CursorStyle, TerminalTheme, TerminalType};
use crate::fonts;
use crate::i18n::Language;
use crate::settings::{AppSettings, Profile};
use crate::ui::uiframe::keyboard::KeyboardMode;
use crate::ui::uiframe::style;

// ─── 标签页标识 ─────────────────────────────────────────────────────────

/// 设置页面的标签页枚举。
#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    /// 通用设置（语言、UI 主题）
    General,
    /// 配置文件管理
    Profiles,
    /// 外观设置（字体、字号、光标样式）
    Appearance,
    /// 主题颜色设置
    Theme,
    /// 终端行为设置
    Behavior,
    /// 高级设置（环境变量）
    Advanced,
}

impl SettingsTab {
    const ALL: [Self; 6] = [
        Self::General,
        Self::Profiles,
        Self::Appearance,
        Self::Theme,
        Self::Behavior,
        Self::Advanced,
    ];

    fn label(self) -> String {
        match self {
            Self::General => rust_i18n::t!("settings_tab_general").into_owned(),
            Self::Profiles => rust_i18n::t!("settings_tab_profiles").into_owned(),
            Self::Appearance => rust_i18n::t!("settings_tab_appearance").into_owned(),
            Self::Theme => rust_i18n::t!("settings_tab_theme").into_owned(),
            Self::Behavior => rust_i18n::t!("settings_tab_behavior").into_owned(),
            Self::Advanced => rust_i18n::t!("settings_tab_advanced").into_owned(),
        }
    }
}

// ─── 公开入口函数 ──────────────────────────────────────────────────────

/// Settings as a centered dialog window. Returns `true` when the user closes it.
pub fn settings_dialog(ctx: &egui::Context, settings: &mut AppSettings) -> bool {
    use crate::ui::uiframe::{DialogFrame, DialogOutcome};

    let frame = DialogFrame::new(rust_i18n::t!("settings").to_string())
        .width(560.0)
        .height(Some(520.0))
        .resizable(true);

    let outcome = frame.show(ctx, "settings_dialog", |ui| {
        settings_scroll_body(ui, settings, SettingsLayout::Dialog);
    });
    matches!(outcome, DialogOutcome::Closed)
}

/// 全屏设置页（遗留）：包含"返回"按钮，返回 `true` 表示关闭设置页。
#[allow(dead_code)]
pub fn settings_page(ui: &mut egui::Ui, settings: &mut AppSettings) -> bool {
    let mut close = false;
    ui.horizontal(|ui| {
        let back_btn = egui::Button::new(
            egui::RichText::new("\u{2190}  ".to_string() + &rust_i18n::t!("back"))
                .size(14.0)
                .color(ui.visuals().text_color()),
        )
        .frame(false)
        .corner_radius(style::CORNER_RADIUS_XS);
        if ui.add(back_btn).clicked() {
            close = true;
        }
    });
    ui.add_space(8.0);
    settings_scroll_body(ui, settings, SettingsLayout::Home);
    close
}

/// 工作区右侧面板设置视图；返回 `true` 表示关闭面板。
#[allow(dead_code)]
pub fn settings_side_panel(ui: &mut egui::Ui, settings: &mut AppSettings) -> bool {
    let mut close = false;
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(rust_i18n::t!("settings"))
                .size(17.0)
                .strong()
                .color(ui.visuals().text_color()),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close_btn = egui::Button::new(
                egui::RichText::new("\u{2715}").size(14.0).color(ui.visuals().weak_text_color()),
            )
            .frame(false)
            .corner_radius(style::CORNER_RADIUS_XS);
            if ui.add(close_btn)
                .on_hover_text(rust_i18n::t!("close"))
                .clicked()
            {
                close = true;
            }
        });
    });
    ui.add_space(2.0);
    settings_scroll_body(ui, settings, SettingsLayout::Workspace);
    close
}

// ─── 内部布局 ──────────────────────────────────────────────────────────

/// 设置页面的布局模式。
#[derive(Clone, Copy)]
enum SettingsLayout {
    /// 全屏首页模式
    Home,
    /// 工作区侧面板模式
    Workspace,
    /// Centered dialog
    Dialog,
}

fn settings_scroll_body(ui: &mut egui::Ui, settings: &mut AppSettings, layout: SettingsLayout) {
    let scroll_w = page_content_width(ui);
    egui::ScrollArea::vertical()
        .id_salt(match layout {
            SettingsLayout::Home => "settings_page_scroll",
            SettingsLayout::Workspace => "settings_side_panel_scroll",
            SettingsLayout::Dialog => "settings_dialog_scroll",
        })
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.set_width(scroll_w);
            ui.set_max_width(scroll_w);
            ui.push_id(match layout {
                SettingsLayout::Home => "home",
                SettingsLayout::Workspace => "workspace",
                SettingsLayout::Dialog => "dialog",
            }, |ui| {
                fill_page_width(ui);
                settings_page_body(ui, settings);
            });
        });
}

#[derive(Clone, Copy)]
struct TabState {
    active_tab: SettingsTab,
    selected_profile: usize,
}

impl Default for TabState {
    fn default() -> Self {
        Self { active_tab: SettingsTab::General, selected_profile: 0 }
    }
}

fn paint_tab_buttons(ui: &mut egui::Ui, state: &mut TabState) {
    for tab in SettingsTab::ALL {
        let label = tab.label();
        let selected = state.active_tab == tab;
        let text_color = if selected { ui.visuals().selection.stroke.color } else { ui.visuals().weak_text_color() };

        let btn = egui::Button::new(
            egui::RichText::new(&label).size(13.0).color(text_color).strong(),
        )
        .fill(egui::Color32::TRANSPARENT)
        .corner_radius(style::CORNER_RADIUS_SM)
        .min_size(egui::vec2(0.0, 30.0));

        let resp = ui.add(btn);
        if selected {
            // Draw a small indicator line under the selected tab
            let painter = ui.painter();
            let line_y = resp.rect.bottom();
            let line_x = resp.rect.left() + 4.0;
            let line_w = resp.rect.width() - 8.0;
            painter.line_segment(
                [egui::pos2(line_x, line_y), egui::pos2(line_x + line_w, line_y)],
                egui::Stroke::new(2.0, ui.visuals().selection.stroke.color),
            );
        }
        if resp.clicked() {
            state.active_tab = tab;
        }
    }
}

fn tab_bar(ui: &mut egui::Ui, layout: SettingsFormLayout, state: &mut TabState) {
    if layout.use_tab_scroll() {
        // Shrink height to tab row only; [false, true] avoids eating space below the tabs.
        egui::ScrollArea::horizontal()
            .id_salt("settings_tabs_scroll")
            .auto_shrink(egui::Vec2b::new(false, true))
            .max_height(36.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    paint_tab_buttons(ui, state);
                });
            });
    } else {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            paint_tab_buttons(ui, state);
        });
    }
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);
}

pub fn settings_page_body(ui: &mut egui::Ui, settings: &mut AppSettings) {
    fill_page_width(ui);
    let layout = SettingsFormLayout::from_ui(ui);

    let mut state: TabState = ui.memory_mut(|mem|
        mem.data.get_persisted::<TabState>(ui.id().with("settings_tab_state"))
    ).unwrap_or_default();

    tab_bar(ui, layout, &mut state);

    if state.selected_profile >= settings.profiles.len() {
        state.selected_profile = 0;
    }

    match state.active_tab {
        SettingsTab::General => general_tab(ui, settings),
        SettingsTab::Profiles => profiles_tab(ui, settings, &mut state),
        SettingsTab::Appearance => appearance_tab(ui, settings, state.selected_profile),
        SettingsTab::Theme => theme_tab(ui, settings, state.selected_profile),
        SettingsTab::Behavior => behavior_tab(ui, settings, state.selected_profile),
        SettingsTab::Advanced => advanced_tab(ui, settings, state.selected_profile),
    }

    ui.memory_mut(|mem| {
        mem.data.insert_persisted(ui.id().with("settings_tab_state"), state);
    });
}

// ═══════════════════════════════════════════════════════════════════════════════
//  统一表单布局系统
// ═══════════════════════════════════════════════════════════════════════════════

/// 表单标签字体大小
const FORM_LABEL_SIZE: f32 = 13.0;
/// 表单分组标题字体大小
const FORM_GROUP_SIZE: f32 = 12.0;
/// 窄布局行间距
const FORM_ROW_GAP_NARROW: f32 = 8.0;

/// 表单布局模式（宽/窄）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum FormLayoutMode {
    /// 宽布局：两列（标签 + 控件）
    Wide,
    /// 窄布局：单列（标签在上，控件在下）
    Narrow,
}

/// 设置表单布局参数。
#[derive(Clone, Copy)]
struct SettingsFormLayout {
    /// 可用宽度
    width: f32,
    /// 布局模式
    mode: FormLayoutMode,
    /// 标签列最大宽度
    label_col_max: f32,
}

impl SettingsFormLayout {
    const NARROW_BREAKPOINT: f32 = 480.0;
    const TAB_SCROLL_BREAKPOINT: f32 = 560.0;

    fn from_ui(ui: &egui::Ui) -> Self {
        let width = ui.available_width().max(1.0);
        let mode = if width < Self::NARROW_BREAKPOINT {
            FormLayoutMode::Narrow
        } else {
            FormLayoutMode::Wide
        };
        let label_col_max = (width * 0.36).clamp(88.0, 180.0);
        Self { width, mode, label_col_max }
    }

    fn is_wide(self) -> bool {
        self.mode == FormLayoutMode::Wide
    }

    fn use_tab_scroll(self) -> bool {
        self.width < Self::TAB_SCROLL_BREAKPOINT
    }

    fn form_label(text: &str) -> egui::RichText {
        egui::RichText::new(text).size(FORM_LABEL_SIZE)
    }

    fn control_width(ui: &egui::Ui) -> f32 {
        ui.available_width().max(48.0)
    }

        /// 在区块卡片内使用标准两列表单网格（宽布局时）。
    fn with_form_grid(
        self,
        ui: &mut egui::Ui,
        id_salt: impl std::hash::Hash,
        add_body: impl FnOnce(&mut egui::Ui, Self),
    ) {
        fill_page_width(ui);
        if self.is_wide() {
            egui::Grid::new(id_salt)
                .num_columns(2)
                .spacing([12.0, 6.0])
                .min_col_width(48.0)
                .max_col_width(self.label_col_max)
                .show(ui, |ui| {
                    fill_page_width(ui);
                    add_body(ui, self);
                });
        } else {
            add_body(ui, self);
        }
    }

    /// 渲染一行表单：宽布局时两列（标签 + 控件），窄布局时单列堆叠。
    fn form_row(self, ui: &mut egui::Ui, label: &str, add_widget: impl FnOnce(&mut egui::Ui, Self)) {
        if self.is_wide() {
            ui.label(Self::form_label(label));
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                add_widget(ui, self);
            });
            ui.end_row();
        } else {
            ui.label(Self::form_label(label));
            ui.add_space(4.0);
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                ui.set_max_width(ui.available_width());
                add_widget(ui, self);
            });
            ui.add_space(FORM_ROW_GAP_NARROW);
        }
    }

    /// 行组之间的全宽分隔线（在同一网格内）。
    fn form_divider(self, ui: &mut egui::Ui) {
        if self.is_wide() {
            ui.label("");
            ui.separator();
            ui.end_row();
        } else {
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);
        }
    }

    /// 表单区块内的子标题（例如颜色分组）。
    fn form_group_heading(self, ui: &mut egui::Ui, text: &str) {
        if self.is_wide() {
            ui.label(egui::RichText::new(text).size(FORM_GROUP_SIZE).weak());
            ui.label("");
            ui.end_row();
        } else {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(text).size(FORM_GROUP_SIZE).weak());
            ui.add_space(2.0);
        }
    }

    /// 操作按钮行，跨越值列（复制/导出/创建等）。
    fn form_actions_row(self, ui: &mut egui::Ui, add_buttons: impl FnOnce(&mut egui::Ui)) {
        if self.is_wide() {
            ui.label("");
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.horizontal_wrapped(add_buttons);
            });
            ui.end_row();
        } else {
            ui.add_space(4.0);
            ui.horizontal_wrapped(add_buttons);
            ui.add_space(FORM_ROW_GAP_NARROW);
        }
    }
}

/// 当前列中所有设置卡片共享的宽度。
fn page_content_width(ui: &egui::Ui) -> f32 {
    ui.available_width().max(1.0)
}

/// 将 UI 宽度设置为页面内容宽度。
fn fill_page_width(ui: &mut egui::Ui) {
    let w = page_content_width(ui);
    ui.set_width(w);
    ui.set_max_width(w);
}

/// 分配一个全宽区块外壳，确保 Frame + Grid 不会缩得比兄弟元素窄。
fn section_shell(ui: &mut egui::Ui, title: &str, subtitle: &str, add_body: impl FnOnce(&mut egui::Ui)) {
    let outer_w = page_content_width(ui);
    ui.allocate_ui_with_layout(
        egui::vec2(outer_w, 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_width(outer_w);
            ui.set_max_width(outer_w);
            egui::Frame::new()
                .fill(ui.visuals().extreme_bg_color)
                .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                .corner_radius(style::CORNER_RADIUS_SM)
                .inner_margin(egui::Margin::symmetric(16, 14))
                .show(ui, |ui| {
                    fill_page_width(ui);
                    ui.label(
                        egui::RichText::new(title)
                            .size(15.0)
                            .strong()
                            .color(ui.visuals().text_color()),
                    );
                    if !subtitle.is_empty() {
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(subtitle)
                                .size(11.0)
                                .color(ui.visuals().weak_text_color()),
                        );
                        ui.add_space(8.0);
                    } else {
                        ui.add_space(10.0);
                    }
                    fill_page_width(ui);
                    add_body(ui);
                });
            ui.add_space(12.0);
        },
    );
}

/// 渲染一个设置区块卡片（标题 + 副标题 + 表单内容）。
fn section_card(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: &str,
    add_body: impl FnOnce(&mut egui::Ui, SettingsFormLayout),
) {
    section_shell(ui, title, subtitle, |ui| {
        let layout = SettingsFormLayout::from_ui(ui);
        add_body(ui, layout);
    });
}

/// 环境变量卡片（简单行布局 — 不使用表单网格）。
fn section_env_card(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: &str,
    id_salt: &str,
    vars: &mut std::collections::HashMap<String, String>,
) {
    section_shell(ui, title, subtitle, |ui| {
        env_var_editor(ui, ui.id().with(id_salt), vars);
    });
}

/// 区块内容完全由标准表单行组成的网格。
fn section_form(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: &str,
    grid_id: &str,
    add_rows: impl FnOnce(&mut egui::Ui, SettingsFormLayout),
) {
    section_card(ui, title, subtitle, |ui, layout| {
        layout.with_form_grid(ui, grid_id, add_rows);
    });
}

// ─── Setting row helpers (always via form_row) ────────────────────────────────

fn slider_row(
    ui: &mut egui::Ui,
    layout: SettingsFormLayout,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) {
    layout.form_row(ui, label, |ui, _| {
        ui.set_max_width(SettingsFormLayout::control_width(ui));
        ui.add(egui::Slider::new(value, range).show_value(true));
    });
}

fn slider_row_usize(
    ui: &mut egui::Ui,
    layout: SettingsFormLayout,
    label: &str,
    value: &mut usize,
    range: std::ops::RangeInclusive<usize>,
    logarithmic: bool,
) {
    layout.form_row(ui, label, |ui, _| {
        ui.set_max_width(SettingsFormLayout::control_width(ui));
        let mut s = egui::Slider::new(value, range).show_value(true);
        if logarithmic {
            s = s.logarithmic(true);
        }
        ui.add(s);
    });
}

fn toggle_row(ui: &mut egui::Ui, layout: SettingsFormLayout, label: &str, value: &mut bool) {
    layout.form_row(ui, label, |ui, _| {
        ui.checkbox(value, "");
    });
}

fn combo_row<T: PartialEq + Copy + 'static>(
    ui: &mut egui::Ui,
    layout: SettingsFormLayout,
    label: &str,
    current: &mut T,
    options: &[T],
    display: fn(T) -> String,
    id_salt: &str,
) {
    layout.form_row(ui, label, |ui, _| {
        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(display(*current))
            .width(SettingsFormLayout::control_width(ui))
            .show_ui(ui, |ui| {
                for &opt in options {
                    let d = display(opt);
                    if ui.selectable_label(*current == opt, d).clicked() {
                        *current = opt;
                    }
                }
            });
    });
}

fn radio_group<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    layout: SettingsFormLayout,
    label: &str,
    current: &mut T,
    options: &[T],
    display: fn(T) -> String,
) {
    layout.form_row(ui, label, |ui, _| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            for &opt in options {
                let d = display(opt);
                if ui.selectable_label(*current == opt, d).clicked() {
                    *current = opt;
                }
            }
        });
    });
}

fn text_row(ui: &mut egui::Ui, layout: SettingsFormLayout, label: &str, value: &mut String, hint: &str) {
    layout.form_row(ui, label, |ui, _| {
        ui.add(
            egui::TextEdit::singleline(value)
                .hint_text(hint)
                .desired_width(SettingsFormLayout::control_width(ui)),
        );
    });
}

fn pane_colors_section(ui: &mut egui::Ui, layout: SettingsFormLayout, settings: &mut AppSettings) {
    use crate::ui::pane_colors::palette_for_theme;

    layout.form_row(ui, &rust_i18n::t!("settings_pane_colors"), |ui, _| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(rust_i18n::t!("settings_pane_colors_desc"))
                    .size(11.0)
                    .color(ui.visuals().weak_text_color()),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui
                    .button(rust_i18n::t!("settings_pane_colors_theme"))
                    .clicked()
                {
                    settings.pane_accent_colors = palette_for_theme(settings.ui_theme);
                }
                if ui
                    .button(rust_i18n::t!("settings_pane_colors_reset"))
                    .clicked()
                {
                    settings.pane_accent_colors.clear();
                }
            });
            if settings.pane_accent_colors.is_empty() {
                ui.horizontal(|ui| {
                    for c in palette_for_theme(settings.ui_theme) {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
                        if ui.is_rect_visible(rect) {
                            ui.painter().rect_filled(
                                rect,
                                3.0,
                                egui::Color32::from_rgb(c[0], c[1], c[2]),
                            );
                        }
                    }
                });
            } else {
                let mut remove_at = None;
                for (i, rgb) in settings.pane_accent_colors.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{} {}",
                            rust_i18n::t!("settings_pane_color"),
                            i + 1
                        ));
                        let mut floats = [
                            rgb[0] as f32 / 255.0,
                            rgb[1] as f32 / 255.0,
                            rgb[2] as f32 / 255.0,
                        ];
                        if ui.color_edit_button_rgb(&mut floats).changed() {
                            *rgb = [
                                (floats[0] * 255.0) as u8,
                                (floats[1] * 255.0) as u8,
                                (floats[2] * 255.0) as u8,
                            ];
                        }
                        if ui.small_button("✕").clicked() {
                            remove_at = Some(i);
                        }
                    });
                }
                if let Some(i) = remove_at {
                    settings.pane_accent_colors.remove(i);
                }
                if settings.pane_accent_colors.len() < 12
                    && ui.button(rust_i18n::t!("add")).clicked()
                {
                    let theme = palette_for_theme(settings.ui_theme);
                    let pick = theme[settings.pane_accent_colors.len() % theme.len()];
                    settings.pane_accent_colors.push(pick);
                }
            }
        });
    });
}

fn color_row(ui: &mut egui::Ui, layout: SettingsFormLayout, label: &str, color: &mut egui::Color32) {
    layout.form_row(ui, label, |ui, _| {
        ui.horizontal(|ui| {
            let mut rgb = [
                color.r() as f32 / 255.0,
                color.g() as f32 / 255.0,
                color.b() as f32 / 255.0,
            ];
            if ui.color_edit_button_rgb(&mut rgb).changed() {
                *color = egui::Color32::from_rgb(
                    (rgb[0] * 255.0) as u8,
                    (rgb[1] * 255.0) as u8,
                    (rgb[2] * 255.0) as u8,
                );
            }
            ui.monospace(format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b()));
        });
    });
}

fn preset_combo_row(ui: &mut egui::Ui, layout: SettingsFormLayout, label: &str, profile: &mut Profile) {
    layout.form_row(ui, label, |ui, _| {
        let presets = TerminalTheme::presets();
        egui::ComboBox::from_id_salt("theme_preset_combo")
            .selected_text("—")
            .width(SettingsFormLayout::control_width(ui))
            .show_ui(ui, |ui| {
                for (name, preset_fn) in &presets {
                    if ui.selectable_label(false, *name).clicked() {
                        profile.theme = preset_fn();
                    }
                }
            });
    });
}

// ═══════════════════════════════════════════════════════════════════════════════
//  标签页内容
// ═══════════════════════════════════════════════════════════════════════════════

/// 通用设置标签页：语言和 UI 主题选择。
fn general_tab(ui: &mut egui::Ui, settings: &mut AppSettings) {
    section_form(ui, &rust_i18n::t!("settings_tab_general"), "", "general_main", |ui, layout| {
        layout.form_row(ui, &rust_i18n::t!("language"), |ui, _layout| {
            egui::ComboBox::from_id_salt("language_selector")
                .selected_text(settings.language.label())
                .width(SettingsFormLayout::control_width(ui))
                .show_ui(ui, |ui| {
                    for lang in Language::ALL {
                        let label = lang.label();
                        if ui.selectable_label(settings.language == lang, label).clicked() {
                            settings.language = lang;
                            lang.apply();
                        }
                    }
                });
        });

        use crate::i18n::UiTheme;
        combo_row(
            ui,
            layout,
            &rust_i18n::t!("ui_theme"),
            &mut settings.ui_theme,
            &UiTheme::ALL,
            |theme| theme.label(),
            "ui_theme_selector",
        );

        pane_colors_section(ui, layout, settings);
    });

    section_env_card(
        ui,
        &rust_i18n::t!("settings_ssh_env"),
        &rust_i18n::t!("settings_ssh_env_desc"),
        "ssh_env",
        &mut settings.ssh_env_vars,
    );
}

/// 配置文件标签页：管理终端配置文件（创建、选择、删除、设为默认）。
fn profiles_tab(ui: &mut egui::Ui, settings: &mut AppSettings, state: &mut TabState) {
    section_card(
        ui,
        &rust_i18n::t!("settings_section_profiles"),
        &rust_i18n::t!("settings_section_profiles_desc"),
        |ui, layout| profile_selector(ui, layout, settings, state),
    );

    if settings.profiles.is_empty() {
        return;
    }

    let profile_idx = state.selected_profile;
    let is_default = settings.default_profile_name == settings.profiles[profile_idx].name;

    if let Some(profile) = settings.profiles.get_mut(profile_idx) {
        let title = format!("{}: {}", rust_i18n::t!("settings_profile_detail"), profile.name);
        section_form(ui, &title, "", "profile_detail", |ui, layout| {
            profile_detail_rows(ui, layout, profile, is_default);
        });
    }
}

/// 配置文件详细信息行：名称、描述、主题预设、复制/导出操作。
fn profile_detail_rows(
    ui: &mut egui::Ui,
    layout: SettingsFormLayout,
    profile: &mut Profile,
    is_default: bool,
) {
    text_row(ui, layout, &rust_i18n::t!("settings_profile_name"), &mut profile.name, "");
    text_row(ui, layout, &rust_i18n::t!("settings_profile_description"), &mut profile.description, "");
    preset_combo_row(ui, layout, &rust_i18n::t!("settings_theme_preset"), profile);

    if is_default {
        layout.form_row(ui, "", |ui, _| {
            ui.label(
                egui::RichText::new(rust_i18n::t!("settings_current_default"))
                    .size(FORM_GROUP_SIZE)
                    .color(ui.visuals().weak_text_color()),
            );
        });
    }

    layout.form_actions_row(ui, |ui| {
        if ui.button(rust_i18n::t!("settings_duplicate_profile")).clicked() {
            let new_name = format!("{} (Copy)", profile.name);
            let _copy = profile.duplicate(&new_name);
        }
        if ui.button(rust_i18n::t!("settings_export_profile")).clicked() {
            if let Ok(json) = profile.export_json() {
                ui.ctx().copy_text(json);
            }
        }
    });
}

/// 配置文件选择器列表：显示所有配置文件，支持选择、删除、设为默认和创建新配置。
fn profile_selector(
    ui: &mut egui::Ui,
    layout: SettingsFormLayout,
    settings: &mut AppSettings,
    state: &mut TabState,
) {
    fill_page_width(ui);
    if settings.profiles.is_empty() {
        ui.label(egui::RichText::new(rust_i18n::t!("settings_no_profiles")).size(12.0).color(egui::Color32::GRAY));
        layout.form_actions_row(ui, |ui| {
            if ui.button(rust_i18n::t!("settings_create_profile")).clicked() {
                let mut p = Profile::default();
                p.name = format!("Profile {}", settings.profiles.len() + 1);
                settings.profiles.push(p);
                state.selected_profile = settings.profiles.len() - 1;
            }
        });
        return;
    }

    let mut to_delete: Option<usize> = None;
    let names: Vec<String> = settings.profiles.iter().map(|p| p.name.clone()).collect();

    for (i, name) in names.iter().enumerate() {
        let is_default = *name == settings.default_profile_name;
        let is_selected = i == state.selected_profile;
        let bg = if is_selected {
            ui.visuals().selection.bg_fill.gamma_multiply(0.3)
        } else {
            egui::Color32::TRANSPARENT
        };

        egui::Frame::new()
            .fill(bg)
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .corner_radius(style::CORNER_RADIUS_XS)
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let label = if is_default {
                        format!("\u{25CF} {name}")
                    } else {
                        name.clone()
                    };
                    let text_color = if is_selected {
                        ui.visuals().selection.stroke.color
                    } else {
                        ui.visuals().text_color()
                    };
                    if ui
                        .selectable_label(is_selected, egui::RichText::new(&label).size(13.0).color(text_color))
                        .clicked()
                    {
                        state.selected_profile = i;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !is_default && settings.profiles.len() > 1 {
                            let del_btn = egui::Button::new(
                                egui::RichText::new("\u{2715}").size(11.0).color(ui.visuals().weak_text_color()),
                            )
                            .frame(false)
                            .corner_radius(style::CORNER_RADIUS_XS);
                            if ui.add(del_btn).on_hover_text(rust_i18n::t!("delete")).clicked() {
                                to_delete = Some(i);
                            }
                        }
                        if !is_default {
                            let def_btn = egui::Button::new(
                                egui::RichText::new("\u{2605}").size(12.0).color(ui.visuals().weak_text_color()),
                            )
                            .frame(false)
                            .corner_radius(style::CORNER_RADIUS_XS);
                            if ui.add(def_btn).on_hover_text(rust_i18n::t!("settings_set_default")).clicked() {
                                settings.default_profile_name = name.clone();
                            }
                        }
                    });
                });
            });
        ui.add_space(4.0);
    }

    if let Some(idx) = to_delete {
        settings.profiles.remove(idx);
        if state.selected_profile >= settings.profiles.len() {
            state.selected_profile = settings.profiles.len().saturating_sub(1);
        }
    }

    ui.add_space(4.0);
    let new_name_id = ui.id().with("new_profile_name");
    let mut new_name: String = ui.data_mut(|d| {
        d.get_temp_mut_or_insert_with(new_name_id, String::new).clone()
    });
    layout.with_form_grid(ui, "profile_create", |ui, layout| {
        layout.form_row(ui, &rust_i18n::t!("settings_create_profile"), |ui, _| {
            ui.horizontal(|ui| {
                let field_w = (SettingsFormLayout::control_width(ui) - 88.0).max(80.0);
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut new_name)
                        .hint_text(rust_i18n::t!("settings_new_profile_hint"))
                        .desired_width(field_w),
                );
                if resp.changed() {
                    ui.data_mut(|d| *d.get_temp_mut_or_insert_with(new_name_id, String::new) = new_name.clone());
                }
                if ui.button(rust_i18n::t!("settings_create_profile")).clicked() && !new_name.is_empty() {
                    let mut p = Profile::default();
                    p.name = new_name.clone();
                    settings.profiles.push(p);
                    state.selected_profile = settings.profiles.len() - 1;
                    new_name.clear();
                    ui.data_mut(|d| d.get_temp_mut_or_insert_with(new_name_id, String::new).clear());
                }
            });
        });
    });
}

/// 获取终端字体显示标签（自动检测时显示"自动"）。
fn terminal_font_display_label(profile: &Profile) -> String {
    if profile.terminal_font.is_empty() {
        return rust_i18n::t!("settings_terminal_font_auto").into_owned();
    }
    std::path::Path::new(&profile.terminal_font)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_owned())
        .unwrap_or_else(|| profile.terminal_font.clone())
}

/// 终端字体选择行：加载等宽字体列表并支持选择。
fn terminal_font_row(ui: &mut egui::Ui, layout: SettingsFormLayout, profile: &mut Profile) {
    layout.form_row(ui, &rust_i18n::t!("settings_terminal_font"), |ui, _| {
        match fonts::monospace_catalog_status() {
            fonts::MonospaceCatalogStatus::Loading => {
                ui.add_enabled_ui(false, |ui| {
                    egui::ComboBox::from_id_salt("settings_terminal_font")
                        .selected_text(terminal_font_display_label(profile))
                        .width(ui.available_width())
                        .show_ui(ui, |_| {});
                });
                ui.label(
                    egui::RichText::new(rust_i18n::t!("settings_terminal_font_loading"))
                        .small()
                        .weak(),
                );
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(150));
            }
            fonts::MonospaceCatalogStatus::Ready(entries) => {
                let entries = entries.as_slice();
                let selected = entries
                    .iter()
                    .position(|e| e.path == profile.terminal_font)
                    .unwrap_or(0);
                let selected_label = entries[selected].label.as_str();
                let mut changed = false;
                egui::ComboBox::from_id_salt("settings_terminal_font")
                    .selected_text(selected_label)
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        ui.set_min_width(280.0);
                        for (idx, entry) in entries.iter().enumerate() {
                            if ui
                                .selectable_label(selected == idx, &entry.label)
                                .clicked()
                            {
                                profile.terminal_font = entry.path.clone();
                                changed = true;
                            }
                        }
                    });
                if changed {
                    fonts::apply_terminal_fonts(ui.ctx(), &profile.terminal_font);
                    ui.ctx().request_repaint();
                }
            }
        }
    });
}

/// 外观设置标签页：字体、字号、行间距、单元格宽高比、光标样式、回滚行数和键盘模式。
fn appearance_tab(ui: &mut egui::Ui, settings: &mut AppSettings, profile_idx: usize) {
    let Some(profile) = settings.profiles.get_mut(profile_idx) else {
        ui.colored_label(egui::Color32::YELLOW, rust_i18n::t!("settings_profile_not_found"));
        return;
    };
    section_form(
        ui,
        &rust_i18n::t!("settings_tab_appearance"),
        "",
        "appearance",
        |ui, layout| {
            terminal_font_row(ui, layout, profile);
            slider_row(ui, layout, &rust_i18n::t!("settings_font_size"), &mut profile.font_size, 8.0..=32.0);
            slider_row(ui, layout, &rust_i18n::t!("settings_line_spacing"), &mut profile.line_spacing, 0.5..=2.0);
            slider_row(ui, layout, &rust_i18n::t!("settings_cell_width"), &mut profile.cell_width_scale, 0.5..=1.5);

            layout.form_divider(ui);

            radio_group(
                ui,
                layout,
                &rust_i18n::t!("settings_cursor_style"),
                &mut profile.cursor_style,
                &CursorStyle::ALL,
                |cs| cs.label(),
            );
            toggle_row(ui, layout, &rust_i18n::t!("settings_bold_is_bright"), &mut profile.bold_is_bright);

            layout.form_divider(ui);

            slider_row_usize(
                ui,
                layout,
                &rust_i18n::t!("settings_scrollback_lines"),
                &mut profile.scrollback_lines,
                100..=100_000,
                true,
            );

            layout.form_divider(ui);

            radio_group(
                ui,
                layout,
                &rust_i18n::t!("settings_default_keyboard"),
                &mut profile.keyboard_mode,
                &[KeyboardMode::Full, KeyboardMode::Special],
                |km| match km {
                    KeyboardMode::Full => rust_i18n::t!("settings_keyboard_full").into_owned(),
                    KeyboardMode::Special => rust_i18n::t!("settings_keyboard_special").into_owned(),
                },
            );
        },
    );
}

/// 主题设置标签页：主题预设和颜色自定义（基础色、标准色、亮色）。
fn theme_tab(ui: &mut egui::Ui, settings: &mut AppSettings, profile_idx: usize) {
    let Some(profile) = settings.profiles.get_mut(profile_idx) else {
        ui.colored_label(egui::Color32::YELLOW, rust_i18n::t!("settings_profile_not_found"));
        return;
    };
    section_form(
        ui,
        &rust_i18n::t!("settings_theme_preset"),
        &rust_i18n::t!("settings_theme_preset_desc"),
        "theme_preset",
        |ui, layout| {
            preset_combo_row(ui, layout, &rust_i18n::t!("settings_theme_preset"), profile);
        },
    );
    section_form(ui, &rust_i18n::t!("settings_theme_colors"), "", "theme_colors", |ui, layout| {
        theme_color_rows(ui, layout, profile);
    });
}

fn theme_color_rows(ui: &mut egui::Ui, layout: SettingsFormLayout, profile: &mut Profile) {
    layout.form_group_heading(ui, &rust_i18n::t!("theme_basic"));
    color_row(ui, layout, &rust_i18n::t!("theme_bg"), &mut profile.theme.bg);
    color_row(ui, layout, &rust_i18n::t!("theme_fg"), &mut profile.theme.fg);
    color_row(ui, layout, &rust_i18n::t!("theme_cursor"), &mut profile.theme.cursor);
    color_row(ui, layout, &rust_i18n::t!("theme_selection"), &mut profile.theme.selection);

    layout.form_divider(ui);

    layout.form_group_heading(ui, &rust_i18n::t!("theme_standard"));
    color_row(ui, layout, &rust_i18n::t!("theme_black"), &mut profile.theme.black);
    color_row(ui, layout, &rust_i18n::t!("theme_red"), &mut profile.theme.red);
    color_row(ui, layout, &rust_i18n::t!("theme_green"), &mut profile.theme.green);
    color_row(ui, layout, &rust_i18n::t!("theme_yellow"), &mut profile.theme.yellow);
    color_row(ui, layout, &rust_i18n::t!("theme_blue"), &mut profile.theme.blue);
    color_row(ui, layout, &rust_i18n::t!("theme_magenta"), &mut profile.theme.magenta);
    color_row(ui, layout, &rust_i18n::t!("theme_cyan"), &mut profile.theme.cyan);
    color_row(ui, layout, &rust_i18n::t!("theme_white"), &mut profile.theme.white);

    layout.form_divider(ui);

    layout.form_group_heading(ui, &rust_i18n::t!("theme_bright"));
    color_row(ui, layout, &rust_i18n::t!("theme_bright_black"), &mut profile.theme.bright_black);
    color_row(ui, layout, &rust_i18n::t!("theme_bright_red"), &mut profile.theme.bright_red);
    color_row(ui, layout, &rust_i18n::t!("theme_bright_green"), &mut profile.theme.bright_green);
    color_row(ui, layout, &rust_i18n::t!("theme_bright_yellow"), &mut profile.theme.bright_yellow);
    color_row(ui, layout, &rust_i18n::t!("theme_bright_blue"), &mut profile.theme.bright_blue);
    color_row(ui, layout, &rust_i18n::t!("theme_bright_magenta"), &mut profile.theme.bright_magenta);
    color_row(ui, layout, &rust_i18n::t!("theme_bright_cyan"), &mut profile.theme.bright_cyan);
    color_row(ui, layout, &rust_i18n::t!("theme_bright_white"), &mut profile.theme.bright_white);
}

/// 行为设置标签页：终端类型、响铃样式、括号粘贴、SGR 鼠标、自动换行和单词分隔符。
fn behavior_tab(ui: &mut egui::Ui, settings: &mut AppSettings, profile_idx: usize) {
    let Some(profile) = settings.profiles.get_mut(profile_idx) else {
        ui.colored_label(egui::Color32::YELLOW, rust_i18n::t!("settings_profile_not_found"));
        return;
    };
    section_form(
        ui,
        &rust_i18n::t!("settings_terminal_behavior"),
        &rust_i18n::t!("settings_terminal_behavior_desc"),
        "behavior",
        |ui, layout| {
            combo_row(
                ui,
                layout,
                &rust_i18n::t!("settings_terminal_type"),
                &mut profile.terminal_type,
                &TerminalType::ALL,
                |tt| tt.label().to_string(),
                "terminal_type_combo",
            );

            layout.form_divider(ui);

            radio_group(
                ui,
                layout,
                &rust_i18n::t!("settings_bell"),
                &mut profile.bell,
                &BellStyle::ALL,
                |bs| bs.label().to_string(),
            );

            layout.form_divider(ui);

            toggle_row(ui, layout, &rust_i18n::t!("settings_bracketed_paste"), &mut profile.enable_bracketed_paste);
            toggle_row(ui, layout, &rust_i18n::t!("settings_sgr_mouse"), &mut profile.enable_sgr_mouse);
            toggle_row(ui, layout, &rust_i18n::t!("settings_auto_wrap"), &mut profile.auto_wrap);

            layout.form_divider(ui);

            text_row(
                ui,
                layout,
                &rust_i18n::t!("settings_word_separators"),
                &mut profile.word_separators,
                "",
            );
        },
    );
}

/// 高级设置标签页：配置文件环境变量管理。
fn advanced_tab(ui: &mut egui::Ui, settings: &mut AppSettings, profile_idx: usize) {
    let Some(profile) = settings.profiles.get_mut(profile_idx) else {
        ui.colored_label(egui::Color32::YELLOW, rust_i18n::t!("settings_profile_not_found"));
        return;
    };
    section_env_card(
        ui,
        &rust_i18n::t!("settings_env_vars"),
        &rust_i18n::t!("settings_env_vars_desc"),
        "profile_env",
        &mut profile.env_vars,
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
//  共享编辑器
// ═══════════════════════════════════════════════════════════════════════════════

/// 环境变量编辑器：KEY = VALUE 行（固定字段宽度，保持在卡片内）。
fn env_var_editor(
    ui: &mut egui::Ui,
    scope_id: egui::Id,
    vars: &mut std::collections::HashMap<String, String>,
) {
    const VALUE_WIDTH_MAX: f32 = 140.0;
    const NEW_KEY_WIDTH_MAX: f32 = 88.0;
    const NEW_VAL_WIDTH_MAX: f32 = 110.0;

    let mut to_remove: Option<String> = None;
    let new_key_id = scope_id.with("new_key");
    let new_val_id = scope_id.with("new_val");
    let mut new_key: String = ui.data_mut(|d| d.get_temp_mut_or_insert_with(new_key_id, String::new).clone());
    let mut new_val: String = ui.data_mut(|d| d.get_temp_mut_or_insert_with(new_val_id, String::new).clone());

    let mut keys: Vec<String> = vars.keys().cloned().collect();
    keys.sort();

    if keys.is_empty() {
        ui.label(
            egui::RichText::new(rust_i18n::t!("settings_no_variables"))
                .size(FORM_GROUP_SIZE)
                .color(egui::Color32::GRAY),
        );
    }

    for key in &keys {
        let key = key.clone();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&key).monospace());
            ui.label("=");
            if let Some(val) = vars.get(&key) {
                let mut v = val.clone();
                let w = ui.available_width().max(48.0).min(VALUE_WIDTH_MAX);
                let resp = ui.add(egui::TextEdit::singleline(&mut v).desired_width(w));
                if resp.changed() {
                    vars.insert(key.clone(), v);
                }
            }
            if ui.small_button("\u{2715}").on_hover_text(rust_i18n::t!("delete")).clicked() {
                to_remove = Some(key);
            }
        });
        ui.add_space(4.0);
    }

    if let Some(key) = to_remove {
        vars.remove(&key);
    }

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let total = ui.available_width().max(120.0);
        let key_w = (total * 0.34).clamp(56.0, NEW_KEY_WIDTH_MAX);
        let val_w = (total - key_w - 58.0).max(48.0).min(NEW_VAL_WIDTH_MAX);
        let key_resp = ui.add(
            egui::TextEdit::singleline(&mut new_key)
                .hint_text("KEY")
                .desired_width(key_w),
        );
        if key_resp.changed() {
            ui.data_mut(|d| *d.get_temp_mut_or_insert_with(new_key_id, String::new) = new_key.clone());
        }
        ui.label("=");
        let val_resp = ui.add(
            egui::TextEdit::singleline(&mut new_val)
                .hint_text("value")
                .desired_width(val_w),
        );
        if val_resp.changed() {
            ui.data_mut(|d| *d.get_temp_mut_or_insert_with(new_val_id, String::new) = new_val.clone());
        }
        if ui.button(rust_i18n::t!("add")).clicked() && !new_key.is_empty() {
            vars.insert(new_key.clone(), new_val.clone());
            ui.data_mut(|d| {
                d.get_temp_mut_or_insert_with(new_key_id, String::new).clear();
                d.get_temp_mut_or_insert_with(new_val_id, String::new).clear();
            });
        }
    });
}
