//! egui color conversions for neutral [`crate::config::Rgba`].

use crate::config::Rgba;

pub fn to_egui(c: Rgba) -> egui::Color32 {
    egui::Color32::from_rgba_premultiplied(c.r, c.g, c.b, c.a)
}

pub fn from_egui(c: egui::Color32) -> Rgba {
    let [r, g, b, a] = c.to_array();
    Rgba { r, g, b, a }
}

impl From<Rgba> for egui::Color32 {
    fn from(c: Rgba) -> Self {
        to_egui(c)
    }
}

impl From<egui::Color32> for Rgba {
    fn from(c: egui::Color32) -> Self {
        from_egui(c)
    }
}
