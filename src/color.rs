//! # Color Module (Phase 2)
//!
//! Extended for Phase 2 with RGB/HSL parsing, conversions, contrast ratio, and style generation.
//! All methods are documented for learning.

use anyhow::{anyhow, Result};
use palette::{FromColor, Hsl, IntoColor, LinSrgb, RgbHue, Srgb};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color(pub Srgb);

impl Color {
    pub fn parse_hex(s: &str) -> Result<Self> {
        let s = s.trim().strip_prefix('#').unwrap_or(s);
        let len = s.len();
        if len != 3 && len != 6 {
            return Err(anyhow!("Invalid hex length: must be 3 or 6 digits"));
        }

        let (r_str, g_str, b_str) = if len == 3 {
            (&s[0..1].repeat(2), &s[1..2].repeat(2), &s[2..3].repeat(2))
        } else {
            (&s[0..2], &s[2..4], &s[4..6])
        };

        let r = u8::from_str_radix(r_str, 16)?;
        let g = u8::from_str_radix(g_str, 16)?;
        let b = u8::from_str_radix(b_str, 16)?;

        Ok(Color(Srgb::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
        )))
    }

    pub fn parse_rgb(s: &str) -> Result<Self> {
        let s = s.trim().to_lowercase().replace("rgb(", "").replace(")", "");
        let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();
        if parts.len() != 3 {
            return Err(anyhow!("Invalid RGB format"));
        }

        let r = parts[0].parse::<u8>()? as f32 / 255.0;
        let g = parts[1].parse::<u8>()? as f32 / 255.0;
        let b = parts[2].parse::<u8>()? as f32 / 255.0;

        Ok(Color(Srgb::new(r, g, b)))
    }

    pub fn parse_hsl(s: &str) -> Result<Self> {
        let s = s.trim().to_lowercase().replace("hsl(", "").replace(")", "");
        let parts: Vec<&str> = s
            .split(',')
            .map(|p| p.trim().trim_end_matches('%'))
            .collect();
        if parts.len() != 3 {
            return Err(anyhow!("Invalid HSL format"));
        }

        let h = parts[0].parse::<f32>()?;
        let s = parts[1].parse::<f32>()? / 100.0;
        let l = parts[2].parse::<f32>()? / 100.0;

        let hsl = Hsl::new(h, s, l);
        Ok(Color(hsl.into_color()))
    }

    pub fn to_hex(&self) -> String {
        let r = (self.0.red * 255.0) as u8;
        let g = (self.0.green * 255.0) as u8;
        let b = (self.0.blue * 255.0) as u8;
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    }

    pub fn to_rgb_str(&self) -> String {
        let r = (self.0.red * 255.0) as u8;
        let g = (self.0.green * 255.0) as u8;
        let b = (self.0.blue * 255.0) as u8;
        format!("rgb({},{},{})", r, g, b)
    }

    pub fn to_hsl_str(&self) -> String {
        let hsl: Hsl = self.0.into_color();
        format!(
            "hsl({:.0},{:.0}%,{:.0}%)",
            hsl.hue.into_positive_degrees().round(),
            (hsl.saturation * 100.0).round(),
            (hsl.lightness * 100.0).round()
        )
    }

    pub fn luminance(&self) -> f64 {
        let lin: LinSrgb = self.0.into_color();
        0.2126 * lin.red as f64 + 0.7152 * lin.green as f64 + 0.0722 * lin.blue as f64
    }

    pub fn contrast_ratio(&self, other: &Color) -> f64 {
        let l1 = self.luminance();
        let l2 = other.luminance();
        let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
        (lighter + 0.05) / (darker + 0.05)
    }

    pub fn to_style(&self) -> ratatui::style::Style {
        use ratatui::style::{Color as TuiColor, Style};
        Style::new().fg(TuiColor::Rgb(
            (self.0.red * 255.0) as u8,
            (self.0.green * 255.0) as u8,
            (self.0.blue * 255.0) as u8,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex() {
        let color = Color::parse_hex("#ff0000").unwrap();
        assert_eq!(color.0.red, 1.0);
        assert_eq!(color.0.green, 0.0);
        assert_eq!(color.0.blue, 0.0);

        let short = Color::parse_hex("#f00").unwrap();
        assert_eq!(short.0.red, 1.0);
    }

    #[test]
    fn test_parse_rgb() {
        let color = Color::parse_rgb("rgb(255,0,0)").unwrap();
        assert_eq!(color.0.red, 1.0);

        let color = Color::parse_rgb("0,255,0").unwrap();
        assert_eq!(color.0.green, 1.0);
    }

    #[test]
    fn test_parse_hsl() {
        let color = Color::parse_hsl("hsl(0,100,50)").unwrap(); // Red
        assert!((color.0.red - 1.0).abs() < 0.01);

        let color = Color::parse_hsl("hsl(120,100,50)").unwrap(); // Green
        assert!((color.0.green - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_to_hex() {
        let color = Color(Srgb::new(1.0, 0.0, 0.0));
        assert_eq!(color.to_hex(), "#ff0000");
    }

    #[test]
    #[test]
    fn test_to_rgb_str() {
        let color = Color(Srgb::new(1.0, 0.0, 0.0));
        assert_eq!(color.to_rgb_str(), "rgb(255,0,0)");
    }

    #[test]
    fn test_to_hsl_str() {
        let color = Color(Srgb::new(1.0, 0.0, 0.0));
        assert_eq!(color.to_hsl_str(), "hsl(0,100%,50%)");
    }

    #[test]
    fn test_luminance() {
        let white = Color(Srgb::new(1.0, 1.0, 1.0));
        assert!((white.luminance() - 1.0).abs() < 0.001);

        let black = Color(Srgb::new(0.0, 0.0, 0.0));
        assert_eq!(black.luminance(), 0.0);
    }

    #[test]
    fn test_contrast_ratio() {
        let black = Color(Srgb::new(0.0, 0.0, 0.0));
        let white = Color(Srgb::new(1.0, 1.0, 1.0));
        assert!((black.contrast_ratio(&white) - 21.0).abs() < 0.01);
    }

    #[test]
    fn test_to_style() {
        let red = Color(Srgb::new(1.0, 0.0, 0.0));
        let style = red.to_style();
        assert_eq!(style.fg, Some(ratatui::style::Color::Rgb(255, 0, 0)));
    }
}
