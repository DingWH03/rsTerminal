//! Application top menu bar — text-only Connection / Preferences / Help.

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TopBarAction {
    #[default]
    None,
    /// Connection → New
    NewConnection,
    /// Connection → Open (saved connections browser)
    OpenConnections,
    OpenSettings,
    OpenHelp,
}

/// Fixed-height text menu bar painted at the top of the shell.
pub struct TopBar;

impl TopBar {
    pub const HEIGHT: f32 = 28.0;

    pub fn show(ui: &mut egui::Ui) -> TopBarAction {
        let mut action = TopBarAction::None;

        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(rust_i18n::t!("menu_connection"), |ui| {
                if ui.button(rust_i18n::t!("menu_connection_new")).clicked() {
                    action = TopBarAction::NewConnection;
                    ui.close();
                }
                if ui.button(rust_i18n::t!("menu_connection_open")).clicked() {
                    action = TopBarAction::OpenConnections;
                    ui.close();
                }
            });
            ui.menu_button(rust_i18n::t!("menu_preferences"), |ui| {
                if ui.button(rust_i18n::t!("menu_settings")).clicked() {
                    action = TopBarAction::OpenSettings;
                    ui.close();
                }
            });
            ui.menu_button(rust_i18n::t!("menu_help"), |ui| {
                if ui.button(rust_i18n::t!("menu_about")).clicked() {
                    action = TopBarAction::OpenHelp;
                    ui.close();
                }
            });
        });

        action
    }
}
