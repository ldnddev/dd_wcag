//! # App Module (Phase 2)
//!
//! This module manages the application state for dd_wcag.
//! It includes the App struct with all fields for color management,
//! input handling, and UI state as per the architecture spec.

use crate::color::Color;
use palette::Srgb;

// Enum for active input target
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputTarget {
    Foreground,
    Background,
    PreviewText,
    None,
}

// Enum for active tab
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Input,
    Conversions,
    Contrast,
    Preview,
}

// Main application state (updated with all spec fields)
#[derive(Debug)]
pub struct App {
    pub foreground: Color, // Current foreground color
    pub background: Color, // Current background color
    pub foreground_input: String,
    pub background_input: String,
    pub input_target: InputTarget,
    pub current_input: String,
    pub parsed_fg: Option<Color>,
    pub parsed_bg: Option<Color>,
    pub last_parsed_format: Option<String>,
    pub contrast_ratio: f64,
    pub preview_text: String,
    pub font_size_px: u16,
    pub is_bold: bool,
    pub error: Option<String>,
    pub active_tab: ActiveTab,
}

impl App {
    pub fn new() -> Self {
        let foreground = Color(Srgb::new(0.0, 0.0, 0.0));
        let background = Color(Srgb::new(1.0, 1.0, 1.0));

        App {
            foreground, // Default black
            background, // Default white
            foreground_input: foreground.to_hex(),
            background_input: background.to_hex(),
            input_target: InputTarget::Foreground,       // Start with FG active
            current_input: foreground.to_hex(),
            parsed_fg: None,
            parsed_bg: None,
            last_parsed_format: None,
            contrast_ratio: 21.0, // Default black on white
            preview_text: "The quick brown fox jumps over the lazy dog.".to_string(),
            font_size_px: 12,
            is_bold: false,
            error: None,
            active_tab: ActiveTab::Input,
        }
    }

    pub fn adjust_font_size(&mut self, delta: i16) {
        let updated = self.font_size_px as i16 + delta;
        self.font_size_px = updated.clamp(6, 120) as u16;
    }

    pub fn set_input_target(&mut self, target: InputTarget) {
        self.input_target = target;
        self.current_input = match target {
            InputTarget::Foreground => self.foreground_input.clone(),
            InputTarget::Background => self.background_input.clone(),
            InputTarget::PreviewText => self.preview_text.clone(),
            InputTarget::None => String::new(),
        };
    }

    pub fn sync_active_input(&mut self) {
        match self.input_target {
            InputTarget::Foreground => self.foreground_input = self.current_input.clone(),
            InputTarget::Background => self.background_input = self.current_input.clone(),
            InputTarget::PreviewText => self.preview_text = self.current_input.clone(),
            InputTarget::None => {}
        }
    }

    // Updates contrast ratio if both colors are parsed
    pub fn update_contrast(&mut self) {
        self.contrast_ratio = self.foreground.contrast_ratio(&self.background);
    }

    // Checks if ratio passes WCAG AA for given size/bold
    pub fn passes_aa(&self, size: f32, bold: bool, ratio: f64) -> bool {
        if (bold && size >= 14.0) || (!bold && size >= 18.0) {
            ratio >= 3.0
        } else {
            ratio >= 4.5
        }
    }

    pub fn submit_input(&mut self) -> bool {
        if self.current_input.trim().is_empty() {
            return true;
        }

        if self.input_target == InputTarget::PreviewText {
            self.preview_text = self.current_input.clone();
            self.error = None;
            return true;
        }

        let input = self.current_input.trim();
        let lower = input.to_lowercase();

        let parse_result = if lower.starts_with("rgba(") {
            Color::parse_rgb(input)
                .map(|color| (color, "RGBA".to_string()))
                .map_err(|err| format!("Invalid RGBA format: {err}"))
        } else if lower.starts_with("rgb(") {
            Color::parse_rgb(input)
                .map(|color| (color, "RGB".to_string()))
                .map_err(|err| format!("Invalid RGB format: {err}"))
        } else if lower.starts_with("hsl(") {
            Color::parse_hsl(input)
                .map(|color| (color, "HSL".to_string()))
                .map_err(|err| format!("Invalid HSL format: {err}"))
        } else if input.starts_with('#') {
            Color::parse_hex(input)
                .map(|color| (color, "HEX".to_string()))
                .map_err(|err| format!("Invalid HEX format: {err}"))
        } else {
            let maybe_hex = input.strip_prefix('#').unwrap_or(input);
            if !maybe_hex.is_empty()
                && maybe_hex.len() <= 6
                && !input.contains('(')
                && !input.contains(',')
            {
                Color::parse_hex(input)
                    .map(|color| (color, "HEX".to_string()))
                    .map_err(|err| format!("Invalid HEX format: {err}"))
            } else {
                let parsed = Color::parse_hex(input)
                    .map(|color| (color, "HEX".to_string()))
                    .or_else(|_| Color::parse_rgb(input).map(|color| (color, "RGB".to_string())))
                    .or_else(|_| Color::parse_hsl(input).map(|color| (color, "HSL".to_string())));
                parsed.map_err(|_| {
                    "Invalid color input. Supported formats: HEX (#rgb/#rrggbb), RGB/RGBA, HSL."
                        .to_string()
                })
            }
        };

        match parse_result {
            Ok((color, format_label)) => {
                match self.input_target {
                    InputTarget::Foreground => {
                        self.foreground = color;
                        self.parsed_fg = Some(color);
                        self.foreground_input = self.current_input.clone();
                    }
                    InputTarget::Background => {
                        self.background = color;
                        self.parsed_bg = Some(color);
                        self.background_input = self.current_input.clone();
                    }
                    InputTarget::PreviewText => {}
                    InputTarget::None => {}
                }
                self.last_parsed_format = Some(format_label);
                self.error = None;
                self.update_contrast();
                true
            }
            Err(err) => {
                self.error = Some(err);
                self.last_parsed_format = None;
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjust_font_size_clamps_to_bounds() {
        let mut app = App::new();
        assert_eq!(app.font_size_px, 12);

        app.adjust_font_size(-200);
        assert_eq!(app.font_size_px, 6);

        app.adjust_font_size(500);
        assert_eq!(app.font_size_px, 120);
    }

    #[test]
    fn submit_input_updates_foreground_and_background() {
        let mut app = App::new();

        app.set_input_target(InputTarget::Foreground);
        app.current_input = "#00ff00".to_string();
        assert!(app.submit_input());
        assert_eq!(app.foreground.to_hex(), "#00ff00");
        assert_eq!(app.foreground_input, "#00ff00");

        app.set_input_target(InputTarget::Background);
        app.current_input = "rgb(255,0,0)".to_string();
        assert!(app.submit_input());
        assert_eq!(app.background.to_hex(), "#ff0000");
        assert_eq!(app.background_input, "rgb(255,0,0)");
        assert_eq!(app.last_parsed_format.as_deref(), Some("RGB"));

        app.current_input = "rgba(0,0,255,0.8)".to_string();
        assert!(app.submit_input());
        assert_eq!(app.last_parsed_format.as_deref(), Some("RGBA"));
    }

    #[test]
    fn invalid_submit_does_not_change_committed_color() {
        let mut app = App::new();
        let original = app.foreground.to_hex();

        app.set_input_target(InputTarget::Foreground);
        app.current_input = "not-a-color".to_string();
        assert!(!app.submit_input());
        assert_eq!(app.foreground.to_hex(), original);
        assert!(app.error.is_some());
    }

    #[test]
    fn invalid_hex_reports_hex_error() {
        let mut app = App::new();
        app.set_input_target(InputTarget::Foreground);
        app.current_input = "#12zz34".to_string();

        assert!(!app.submit_input());
        let error = app.error.unwrap_or_default();
        assert!(error.contains("HEX"));
    }

    #[test]
    fn tab_apply_flow_keeps_independent_field_drafts() {
        let mut app = App::new();

        app.current_input = "hsl(120,100,50)".to_string();
        app.sync_active_input();
        assert!(app.submit_input());
        app.set_input_target(InputTarget::Background);

        assert_eq!(app.foreground.to_hex(), "#00ff00");
        assert_eq!(app.foreground_input, "hsl(120,100,50)");

        app.current_input = "rgba(0,0,255,0.8)".to_string();
        app.sync_active_input();
        assert!(app.submit_input());
        app.set_input_target(InputTarget::Foreground);

        assert_eq!(app.background.to_hex(), "#0000ff");
        assert_eq!(app.background_input, "rgba(0,0,255,0.8)");
        assert_eq!(app.current_input, "hsl(120,100,50)");
    }

    #[test]
    fn preview_text_target_updates_preview_text() {
        let mut app = App::new();
        app.set_input_target(InputTarget::PreviewText);
        app.current_input = "Custom preview sample".to_string();
        app.sync_active_input();
        assert!(app.submit_input());
        assert_eq!(app.preview_text, "Custom preview sample");
    }
}
