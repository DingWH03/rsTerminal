//! Glyph cache for terminal cell painting.

use std::collections::HashMap;
use std::sync::Arc;

use egui::text::Galley;

#[derive(Default)]
pub struct RowGalleyCache {
    font_size: f32,
    entries: HashMap<u64, Arc<Galley>>,
}

impl RowGalleyCache {
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn ensure_font(&mut self, font_size: f32) {
        if (self.font_size - font_size).abs() > f32::EPSILON {
            self.font_size = font_size;
            self.entries.clear();
        }
    }

    pub fn get(&self, key: u64) -> Option<Arc<Galley>> {
        self.entries.get(&key).cloned()
    }

    pub fn insert(&mut self, key: u64, galley: Arc<Galley>) {
        if self.entries.len() > 4096 {
            self.entries.clear();
        }
        self.entries.insert(key, galley);
    }
}
