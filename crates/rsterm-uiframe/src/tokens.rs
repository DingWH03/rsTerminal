//! Compact UI design tokens and semantic theme palettes.

use egui::{Color32, CornerRadius};

pub mod space {
    pub const XS: f32 = 2.0;
    pub const SM: f32 = 4.0;
    pub const MD: f32 = 6.0;
    pub const LG: f32 = 8.0;
    pub const XL: f32 = 12.0;
}

pub mod text {
    pub const CAPTION: f32 = 10.0;
    pub const SMALL: f32 = 11.0;
    pub const COMPACT: f32 = 12.0;
    pub const BODY: f32 = 13.0;
    pub const EMPHASIS: f32 = 14.0;
    pub const HEADING: f32 = 18.0;
}

pub mod radius {
    use egui::CornerRadius;

    pub const XS: CornerRadius = CornerRadius::same(4);
    pub const SM: CornerRadius = CornerRadius::same(6);
    pub const LG: CornerRadius = CornerRadius::same(10);
}

pub mod size {
    pub const TOOLBAR_WIDTH: f32 = 24.0;
    pub const TOOLBAR_HEIGHT: f32 = 22.0;
    pub const NAV_ROW: f32 = 28.0;
    pub const RESOURCE_ROW: f32 = 32.0;
    pub const BUTTON: f32 = 30.0;
    pub const BOTTOM_BAR: f32 = 36.0;
}

pub mod stroke {
    pub const HAIRLINE: f32 = 1.0;
    pub const EMPHASIS: f32 = 1.5;
    pub const STRONG: f32 = 2.0;
}

pub const ACCENT: Color32 = Color32::from_rgb(74, 158, 255);
pub const GREEN: Color32 = Color32::from_rgb(61, 220, 132);
pub const RED: Color32 = Color32::from_rgb(255, 82, 82);
pub const AMBER: Color32 = Color32::from_rgb(255, 215, 64);

/// Theme-resolved colors used by application chrome (not terminal rendering).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticPalette {
    pub surface_0: Color32,
    pub surface_1: Color32,
    pub surface_2: Color32,
    pub surface_3: Color32,
    pub surface_4: Color32,
    pub border_subtle: Color32,
    pub border: Color32,
    pub border_accent: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_tertiary: Color32,
    pub selection: Color32,
    pub accent: Color32,
}

impl SemanticPalette {
    pub const DARK: Self = Self {
        surface_0: Color32::from_rgb(13, 13, 15),
        surface_1: Color32::from_rgb(19, 19, 23),
        surface_2: Color32::from_rgb(26, 26, 32),
        surface_3: Color32::from_rgb(32, 32, 40),
        surface_4: Color32::from_rgb(38, 38, 48),
        // Premultiplied correctly: rgb *= a/255 (from_rgba_unmultiplied is not const).
        border_subtle: Color32::from_rgba_premultiplied(18, 18, 18, 18),
        border: Color32::from_rgba_premultiplied(32, 32, 32, 32),
        border_accent: Color32::from_rgba_premultiplied(26, 55, 90, 90),
        text_primary: Color32::from_rgb(232, 232, 236),
        text_secondary: Color32::from_rgb(158, 158, 166),
        text_tertiary: Color32::from_rgb(120, 120, 130),
        selection: Color32::from_rgba_premultiplied(14, 30, 48, 48),
        accent: ACCENT,
    };

    pub const LIGHT: Self = Self {
        surface_0: Color32::from_rgb(246, 247, 249),
        surface_1: Color32::from_rgb(255, 255, 255),
        surface_2: Color32::from_rgb(249, 250, 252),
        surface_3: Color32::from_rgb(239, 243, 248),
        surface_4: Color32::from_rgb(229, 237, 248),
        border_subtle: Color32::from_rgba_premultiplied(2, 4, 5, 28),
        border: Color32::from_rgba_premultiplied(4, 6, 9, 48),
        border_accent: Color32::from_rgba_premultiplied(17, 54, 94, 120),
        text_primary: Color32::from_rgb(31, 36, 45),
        text_secondary: Color32::from_rgb(82, 91, 105),
        text_tertiary: Color32::from_rgb(112, 122, 138),
        selection: Color32::from_rgba_premultiplied(6, 19, 33, 42),
        accent: Color32::from_rgb(36, 114, 200),
    };

    pub const fn for_dark_mode(dark_mode: bool) -> Self {
        if dark_mode { Self::DARK } else { Self::LIGHT }
    }
}

pub const CORNER_RADIUS: CornerRadius = radius::LG;
