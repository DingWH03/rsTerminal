//! Shell-level UI animations.

use std::collections::HashMap;

use crate::ui::shell::layout_state::PaneId;
use crate::ui::uiframe::animation::Spring;

pub struct ShellAnimations {
    pub page_slide: Spring,
    pub session_fade: HashMap<PaneId, Spring>,
    pub preview_ratio: Spring,
    pub preview_active: bool,
}

impl ShellAnimations {
    pub fn new() -> Self {
        Self {
            page_slide: Spring::new(0.0),
            session_fade: HashMap::new(),
            preview_ratio: Spring::new(0.5),
            preview_active: false,
        }
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        let mut repaint = false;
        repaint |= self.page_slide.tick(dt);
        if self.preview_active {
            repaint |= self.preview_ratio.tick(dt);
        } else {
            self.preview_ratio.snap_to(0.5);
        }
        for spring in self.session_fade.values_mut() {
            repaint |= spring.tick(dt);
        }
        self.session_fade.retain(|_, s| s.is_animating());
        repaint
    }

    pub fn on_page_connections(&mut self) {
        self.page_slide.set_target(1.0);
    }

    pub fn on_page_workspace(&mut self) {
        self.page_slide.set_target(0.0);
    }

    pub fn fade_in_pane(&mut self, pane: PaneId) {
        let mut spring = Spring::new(0.0);
        spring.set_target(1.0);
        self.session_fade.insert(pane, spring);
    }

    pub fn session_fade_value(&self, pane: PaneId) -> f32 {
        self.session_fade
            .get(&pane)
            .map(|s| s.current)
            .unwrap_or(1.0)
    }

    pub fn begin_preview(&mut self) {
        self.preview_active = true;
        self.preview_ratio.set_target(crate::ui::workspace_pane::drag_drop::PREVIEW_INSERT_RATIO);
    }

    pub fn end_preview(&mut self) {
        self.preview_active = false;
        self.preview_ratio.set_target(0.5);
    }
}

impl Default for ShellAnimations {
    fn default() -> Self {
        Self::new()
    }
}
