//! Minimal chrome callbacks for content pane headers (hamburger only).

/// Header chrome passed into content views that need a hamburger menu.
pub struct PaneChrome<'a> {
    pub show_hamburger: bool,
    pub on_hamburger: &'a mut dyn FnMut(),
}
