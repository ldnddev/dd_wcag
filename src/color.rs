//! # Color Module (Phase 2)
//!
//! Extended for Phase 2 with RGB/HSL parsing, conversions, contrast ratio, and style generation.
//! All methods are documented for learning.

use anyhow::{anyhow, Result};
use palette::{Hsl, IntoColor, LinSrgb, Srgb};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color(pub Srgb);

impl Color {
    pub fn parse_hex(s: &str) -> Result<Self> {
        let s = s.trim().strip_prefix('#').unwrap_or(s);
        let len = s.len();
        if len != 3 && len != 6 {
            return Err(anyhow!("Invalid hex length: must be 3 or 6 digits"));
        }

        let (r_str, g_str, b_str): (String, String, String) = if len == 3 {
            (
                s[0..1].repeat(2),
                s[1..2].repeat(2),
                s[2..3].repeat(2),
            )
        } else {
            (
                s[0..2].to_string(),
                s[2..4].to_string(),
                s[4..6].to_string(),
            )
        };

        let r = u8::from_str_radix(&r_str, 16)?;
        let g = u8::from_str_radix(&g_str, 16)?;
        let b = u8::from_str_radix(&b_str, 16)?;

        Ok(Color(Srgb::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
        )))
    }

    pub fn parse_rgb(s: &str) -> Result<Self> {
        let normalized = s.trim().to_lowercase();
        let raw = if normalized.starts_with("rgba(") && normalized.ends_with(')') {
            &normalized[5..normalized.len() - 1]
        } else if normalized.starts_with("rgb(") && normalized.ends_with(')') {
            &normalized[4..normalized.len() - 1]
        } else {
            normalized.as_str()
        };

        let parts: Vec<&str> = raw.split(',').map(|p| p.trim()).collect();
        if parts.len() != 3 && parts.len() != 4 {
            return Err(anyhow!("Invalid RGB/RGBA format"));
        }

        let r = parts[0].parse::<u8>()? as f32 / 255.0;
        let g = parts[1].parse::<u8>()? as f32 / 255.0;
        let b = parts[2].parse::<u8>()? as f32 / 255.0;
        if parts.len() == 4 {
            let alpha = parts[3].parse::<f32>()?;
            if !(0.0..=1.0).contains(&alpha) {
                return Err(anyhow!("RGBA alpha must be between 0 and 1"));
            }
        }

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

    // New: APCA contrast calculation (ported from Myndex/SAPC-APCA 0.98G)
    pub fn apca_lc(&self, other: &Color) -> f64 {
        let fg_y = self.luminance();
        let bg_y = other.luminance();

        let (txt_y, bg_y) = if fg_y < bg_y { (bg_y, fg_y) } else { (fg_y, bg_y) };

        if txt_y <= 0.0 || bg_y <= 0.0 {
            return 0.0;
        }
        if txt_y == bg_y {
            return 0.0;
        }

        const BLK_THRS: f64 = 0.022;
        const BLK_CLMP: f64 = 1.414;

        let mut bg_y_clamped = bg_y;
        if bg_y < BLK_THRS {
            bg_y_clamped = bg_y + (BLK_THRS - bg_y).powf(BLK_CLMP);
        }

        let mut txt_y_clamped = txt_y;
        if txt_y < BLK_THRS {
            txt_y_clamped = txt_y + (BLK_THRS - txt_y).powf(BLK_CLMP);
        }

        let mut output = txt_y_clamped.powf(0.38) - bg_y_clamped.powf(0.40);
        const DELTA_Y_MIN: f64 = 0.0005;
        if output.abs() < DELTA_Y_MIN {
            return 0.0;
        }

        const SCALE_BO: f64 = 1.14;
        const SCALE_CO: f64 = 1.14;
        const LO_BO_CLIP: f64 = 0.1;
        const LO_BO_CTRST: f64 = 0.007;
        const LO_CO_CLIP: f64 = 0.06;
        const LO_CO_CTRST: f64 = 0.007;

        if output > 0.0 {
            output *= SCALE_BO;
            if output < LO_BO_CLIP {
                return 0.0;
            }
            if output < LO_BO_CTRST {
                output = LO_BO_CTRST;
            }
        } else {
            output *= SCALE_CO;
            if output > -LO_CO_CLIP {
                return 0.0;
            }
            if output > -LO_CO_CTRST {
                output = -LO_CO_CTRST;
            }
        }

        output * 100.0 // Scale to Lc 0-100+
    }

    // New: Determine if APCA passes based on font size and weight (approximate thresholds)
    pub fn apca_passes(&self, other: &Color, font_size_px: u32, is_bold: bool) -> bool {
        let lc = self.apca_lc(other).abs();
        let threshold = match font_size_px {
            0..=12 => if is_bold { 75.0 } else { 90.0 },
            13..=18 => if is_bold { 60.0 } else { 75.0 },
            19..=24 => if is_bold { 45.0 } else { 60.0 },
            _ => if is_bold { 30.0 } else { 45.0 }, // For larger text
        };
        lc >= threshold
    }

    pub fn to_style(&self) -> ratatui::style::Style {
        use ratatui::style::{Color as TuiColor, Style};
        Style::new().fg(TuiColor::Rgb(
            (self.0.red * 255.0) as u8,
            (self.0.green * 255.0) as u8,
            (self.0.blue * 255.0) as u8,
        ))
    }

    pub fn to_tui_color(&self) -> ratatui::style::Color {
        ratatui::style::Color::Rgb(
            (self.0.red * 255.0) as u8,
            (self.0.green * 255.0) as u8,
            (self.0.blue * 255.0) as u8,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Existing tests...

    #[test]
    fn test_apca_lc() {
        let white = Color(Srgb::new(1.0, 1.0, 1.0));
        let black = Color(Srgb::new(0.0, 0.0, 0.0));
        let lc = white.apca_lc(&black);
        assert!(lc > 100.0); // Should be high contrast

        let gray = Color(Srgb::new(0.5, 0.5, 0.5));
        let lc = white.apca_lc(&gray);
        assert!(lc > 50.0);
    }

    #[test]
    fn test_apca_passes() {
        let fg = Color(Srgb::new(0.0, 0.0, 0.0));
        let bg = Color(Srgb::new(1.0, 1.0, 1.0));
        assert!(fg.apca_passes(&bg, 16, false)); // Should pass for normal text
        assert!(!fg.apca_passes(&Color(Srgb::new(0.8, 0.8, 0.8)), 10, false)); // Low contrast small text should fail
    }
}
