//! egui color conversions for neutral [`rsterm_config::Rgba`].

use rsterm_config::Rgba;

pub fn to_egui(c: Rgba) -> egui::Color32 {
    egui::Color32::from_rgba_premultiplied(c.r, c.g, c.b, c.a)
}

pub fn from_egui(c: egui::Color32) -> Rgba {
    let [r, g, b, a] = c.to_array();
    Rgba { r, g, b, a }
}
