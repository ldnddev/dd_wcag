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
            InputTarget::None => String::new(),
        };
    }

    pub fn sync_active_input(&mut self) {
        match self.input_target {
            InputTarget::Foreground => self.foreground_input = self.current_input.clone(),
            InputTarget::Background => self.background_input = self.current_input.clone(),
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

        let parsed = Color::parse_hex(&self.current_input)
            .or_else(|_| Color::parse_rgb(&self.current_input))
            .or_else(|_| Color::parse_hsl(&self.current_input));

        match parsed {
            Ok(color) => {
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
                    InputTarget::None => {}
                }
                self.error = None;
                self.update_contrast();
                true
            }
            Err(err) => {
                self.error = Some(format!("Invalid color input: {err}"));
                false
            }
        }
    }
}
