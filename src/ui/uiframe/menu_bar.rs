//! Declarative menu bar chrome — no app-specific menu trees.
//!
//! Business modules build a [`MenuBarSpec`] and call [`MenuBar::show`]; this
//! module only paints and returns entry ids.

/// Identifier for a leaf menu entry (button / checkbox).
pub type MenuEntryId = u16;

/// One leaf or separator inside a dropdown menu.
#[derive(Clone, Copy)]
pub enum MenuEntry<'a> {
    Button {
        id: MenuEntryId,
        label: &'a str,
    },
    Checkbox {
        id: MenuEntryId,
        label: &'a str,
        checked: bool,
        enabled: bool,
    },
    Separator,
}

/// One top-level menu (e.g. "Connection", "View").
#[derive(Clone, Copy)]
pub struct MenuGroup<'a> {
    pub title: &'a str,
    pub entries: &'a [MenuEntry<'a>],
}

/// Full menu bar specification for one frame.
#[derive(Clone, Copy)]
pub struct MenuBarSpec<'a> {
    pub groups: &'a [MenuGroup<'a>],
}

/// Fixed-height text menu bar painted at the top of the shell.
pub struct MenuBar;

impl MenuBar {
    pub const HEIGHT: f32 = 28.0;

    /// Paint the menu bar. Returns the id of a leaf entry the user activated.
    pub fn show(ui: &mut egui::Ui, spec: MenuBarSpec<'_>) -> Option<MenuEntryId> {
        let mut activated = None;

        egui::MenuBar::new().ui(ui, |ui| {
            for group in spec.groups {
                ui.menu_button(group.title, |ui| {
                    for entry in group.entries {
                        match *entry {
                            MenuEntry::Separator => {
                                ui.separator();
                            }
                            MenuEntry::Button { id, label } => {
                                if ui.button(label).clicked() {
                                    activated = Some(id);
                                    ui.close();
                                }
                            }
                            MenuEntry::Checkbox {
                                id,
                                label,
                                checked,
                                enabled,
                            } => {
                                let mut checked = checked;
                                let resp = ui
                                    .add_enabled(enabled, egui::Checkbox::new(&mut checked, label));
                                if enabled && resp.changed() {
                                    activated = Some(id);
                                    ui.close();
                                }
                            }
                        }
                    }
                });
            }
        });

        activated
    }
}
