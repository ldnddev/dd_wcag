//! # App Module (Phase 2)
//!
//! This module manages the application state for dd_wcag.
//! It includes the App struct with all fields for color management,
//! input handling, and UI state as per the architecture spec.

use crate::color::Color;

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

// Font sizes from architecture spec
pub const FONT_SIZES: [f32; 4] = [12.0, 14.0, 16.0, 18.0];

// Main application state (updated with all spec fields)
#[derive(Debug)]
pub struct App {
    pub foreground: Color, // Current foreground color
    pub background: Color, // Current background color
    pub input_target: InputTarget,
    pub current_input: String,
    pub parsed_fg: Option<Color>,
    pub parsed_bg: Option<Color>,
    pub contrast_ratio: f64,
    pub preview_text: String,
    pub font_size_idx: usize,
    pub is_bold: bool,
    pub error: Option<String>,
    pub active_tab: ActiveTab,
}

impl App {
    pub fn new() -> Self {
        App {
            foreground: Color(Srgb::new(0.0, 0.0, 0.0)), // Default black
            background: Color(Srgb::new(1.0, 1.0, 1.0)), // Default white
            input_target: InputTarget::Foreground,       // Start with FG active
            current_input: String::new(),
            parsed_fg: None,
            parsed_bg: None,
            contrast_ratio: 21.0, // Default black on white
            preview_text: "The quick brown fox jumps over the lazy dog.".to_string(),
            font_size_idx: 2, // Default 16.0
            is_bold: false,
            error: None,
            active_tab: ActiveTab::Input,
        }
    }

    // Updates contrast ratio if both colors are parsed
    pub fn update_contrast(&mut self) {
        if let (Some(fg), Some(bg)) = (self.parsed_fg, self.parsed_bg) {
            self.contrast_ratio = fg.contrast_ratio(&bg);
        }
    }

    // Checks if ratio passes WCAG AA for given size/bold
    pub fn passes_aa(&self, size: f32, bold: bool, ratio: f64) -> bool {
        if (bold && size >= 14.0) || (!bold && size >= 18.0) {
            ratio >= 3.0
        } else {
            ratio >= 4.5
        }
    }
}
