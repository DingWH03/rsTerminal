//! Pluggable workspace pane content.

use std::any::Any;

pub struct ContentTickCtx {
    pub request_repaint: bool,
}

pub struct ContentUiCtx<'a> {
    pub pane_id: u64,
    pub is_focused: bool,
    pub in_split: bool,
    pub suppress_terminal_input: bool,
    pub show_hamburger: bool,
    pub hamburger_pending: &'a mut bool,
    pub pane_focus_click: &'a mut bool,
    /// Host-specific payload (e.g. terminal profiles). Adapters downcast.
    pub extras: &'a mut dyn Any,
}

#[derive(Debug, Default)]
pub enum ContentAction {
    #[default]
    None,
    Close,
    MinimizePane,
}

pub trait WorkspaceContent: Send {
    fn id(&self) -> &str;
    fn tab_label(&self) -> String;
    fn sidebar_has_new_window(&self) -> bool;
    fn tick(&mut self, _ctx: &mut ContentTickCtx) {}
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut ContentUiCtx<'_>) -> ContentAction;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct WorkspaceHost {
    pub layout: crate::layout::WorkspaceLayout,
}
