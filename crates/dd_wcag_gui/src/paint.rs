//! Packed theme colors. Hex literals here are only the fallback when a token is missing.

use gpui::Hsla;
use ldnddev_theme::{Palette, Rgb};

#[derive(Clone, Copy, Debug)]
pub struct Packed {
    pub base: Hsla,
    pub panel: Hsla,
    pub raised: Hsla,
    pub text: Hsla,
    pub secondary: Hsla,
    pub border: Hsla,
    pub selection: Hsla,
    pub accent: Hsla,
    pub success: Hsla,
    pub error: Hsla,
}

pub fn pack(palette: &Palette) -> Packed {
    Packed {
        base: token(palette, "base_background", Rgb::new(0x0f, 0x11, 0x14)),
        panel: token(palette, "modal_background", Rgb::new(0x1c, 0x1e, 0x21)),
        raised: token(palette, "body_background", Rgb::new(0x2a, 0x2d, 0x31)),
        text: token(palette, "text_primary", Rgb::new(0xf5, 0xf6, 0xf7)),
        secondary: token(palette, "text_secondary", Rgb::new(0x9e, 0xa3, 0xaa)),
        border: token(palette, "border_default", Rgb::new(0xf5, 0xf6, 0xf7)),
        selection: token(palette, "selected_background", Rgb::new(0x0f, 0x11, 0x14)),
        accent: token(palette, "text_active_focus", Rgb::new(0x64, 0xb4, 0xf5)),
        success: token(palette, "success", Rgb::new(0x82, 0xe0, 0xaa)),
        error: token(palette, "error", Rgb::new(0xe5, 0x73, 0x73)),
    }
}

fn token(palette: &Palette, key: &str, fallback: Rgb) -> Hsla {
    hsla(palette.get(key).unwrap_or(fallback))
}

pub fn hsla(color: Rgb) -> Hsla {
    gpui::Rgba {
        r: f32::from(color.r) / 255.0,
        g: f32::from(color.g) / 255.0,
        b: f32::from(color.b) / 255.0,
        a: 1.0,
    }
    .into()
}

pub fn hsla_bytes(rgb: (u8, u8, u8)) -> Hsla {
    hsla(Rgb::new(rgb.0, rgb.1, rgb.2))
}
