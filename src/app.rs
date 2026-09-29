//! # App Module (Phase 2)
//!
//! This module manages the application state for dd_wcag.
//! It includes the App struct with all fields for color management,
//! input handling, and UI state as per the architecture spec.

use crate::color::{Color, is_large_text};
use crate::fix::{FixAxis, FixState};
use crate::layout::{Hit, LayoutMap};
use crate::palette::{
    GeneratedPalette, MatrixAxis, PaletteInput, PaletteState, generate_palette, matrix_cell,
    parse_palette_color, validate_export,
};
use crate::theme::{Theme, ThemeSource};
use palette::Srgb;
use std::time::{Duration, Instant};

pub const TOAST_TTL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Contrast,
    Palette,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WcagLevel {
    #[default]
    Aa,
    Aaa,
}

impl WcagLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Aa => "AA",
            Self::Aaa => "AAA",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::Aa => Self::Aaa,
            Self::Aaa => Self::Aa,
        }
    }

    pub fn cycle_back(self) -> Self {
        self.cycle()
    }

    pub fn text_threshold(self, large: bool) -> f64 {
        match (self, large) {
            (Self::Aa, false) => 4.5,
            (Self::Aa, true) => 3.0,
            (Self::Aaa, false) => 7.0,
            (Self::Aaa, true) => 4.5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApcaTarget {
    Lc45,
    Lc60,
    #[default]
    Lc75,
    Lc90,
}

impl ApcaTarget {
    pub fn label(self) -> &'static str {
        match self {
            Self::Lc45 => "Lc45",
            Self::Lc60 => "Lc60",
            Self::Lc75 => "Lc75",
            Self::Lc90 => "Lc90",
        }
    }

    pub fn value(self) -> f64 {
        match self {
            Self::Lc45 => 45.0,
            Self::Lc60 => 60.0,
            Self::Lc75 => 75.0,
            Self::Lc90 => 90.0,
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::Lc45 => Self::Lc60,
            Self::Lc60 => Self::Lc75,
            Self::Lc75 => Self::Lc90,
            Self::Lc90 => Self::Lc45,
        }
    }

    pub fn cycle_back(self) -> Self {
        match self {
            Self::Lc45 => Self::Lc90,
            Self::Lc60 => Self::Lc45,
            Self::Lc75 => Self::Lc60,
            Self::Lc90 => Self::Lc75,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Targets {
    pub wcag: WcagLevel,
    pub apca: ApcaTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FixSource {
    #[default]
    Contrast,
    Palette {
        text: MatrixAxis,
        surface: MatrixAxis,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusId {
    #[default]
    FgHex,
    BgHex,
    Size,
    Weight,
    Style,
    PreviewText,
    FontFamily,
    Swap,
    CopyHex,
    FixBtn,
    OpenPreview,
    Role(usize),
    Generate,
    Matrix,
    Detail,
    NudgeFg,
    NudgeBg,
    SendFg,
    SendBg,
    ApplyFix,
    NextFix,
    CloseFix,
    TargetWcag,
    TargetApca,
}

impl FocusId {
    pub fn is_text_field(self) -> bool {
        matches!(
            self,
            Self::FgHex | Self::BgHex | Self::PreviewText | Self::FontFamily
        )
    }

    pub fn contrast_order() -> &'static [FocusId] {
        &[
            FocusId::FgHex,
            FocusId::BgHex,
            FocusId::Size,
            FocusId::Weight,
            FocusId::Style,
            FocusId::PreviewText,
            FocusId::FontFamily,
            FocusId::Swap,
            FocusId::CopyHex,
            FocusId::FixBtn,
            FocusId::OpenPreview,
            FocusId::TargetWcag,
            FocusId::TargetApca,
        ]
    }

    pub fn palette_order() -> &'static [FocusId] {
        &[
            FocusId::Role(0),
            FocusId::Role(1),
            FocusId::Role(2),
            FocusId::Role(3),
            FocusId::Size,
            FocusId::Weight,
            FocusId::Generate,
            FocusId::Matrix,
            FocusId::Detail,
        ]
    }

    pub fn fix_order() -> &'static [FocusId] {
        &[
            FocusId::NudgeFg,
            FocusId::NudgeBg,
            FocusId::ApplyFix,
            FocusId::NextFix,
            FocusId::CloseFix,
            FocusId::SendFg,
            FocusId::SendBg,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StylePreset {
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

impl StylePreset {
    pub const ALL: [Self; 4] = [Self::Regular, Self::Bold, Self::Italic, Self::BoldItalic];

    pub fn label(self) -> &'static str {
        match self {
            Self::Regular => "Reg",
            Self::Bold => "Bld",
            Self::Italic => "Itl",
            Self::BoldItalic => "B+I",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Regular => 0,
            Self::Bold => 1,
            Self::Italic => 2,
            Self::BoldItalic => 3,
        }
    }

    pub fn from_index(index: usize) -> Self {
        Self::ALL[index.min(3)]
    }

    pub fn apply(self) -> (u16, bool) {
        match self {
            Self::Regular => (400, false),
            Self::Bold => (700, false),
            Self::Italic => (400, true),
            Self::BoldItalic => (700, true),
        }
    }
}

// Enum for active input target
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputTarget {
    Foreground,
    Background,
    PreviewText,
    FontFamily,
    None,
}

// Enum for active tab (legacy; Mode is the displayed tab)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ActiveTab {
    Input,
    Conversions,
    Contrast,
    Preview,
    Palette,
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
    pub cursor_char_idx: usize,
    pub parsed_fg: Option<Color>,
    pub parsed_bg: Option<Color>,
    pub last_parsed_format: Option<String>,
    pub contrast_ratio: f64,
    pub preview_text: String,
    pub preview_font_family: String,
    pub font_size_px: u16,
    pub is_bold: bool,
    pub weight: u16,
    pub italic: bool,
    pub style_chip: usize,
    pub mode: Mode,
    pub focus: FocusId,
    pub editing: bool,
    pub fix_open: bool,
    pub fix: FixState,
    pub fix_source: FixSource,
    pub nudge_dragging: Option<FixAxis>,
    pub fix_send_chip: usize,
    pub targets: Targets,
    pub layout: LayoutMap,
    pub hovered: Option<Hit>,
    pub mouse_pos: Option<(u16, u16)>,
    pub scrollbar_dragging: bool,
    pub contrast_scroll: u16,
    pub contrast_max_scroll: u16,
    pub contrast_scroll_focus: Option<FocusId>,
    pub last_mouse_click_pos: Option<(u16, u16, Instant)>,
    pub error: Option<String>,
    pub status: Option<String>,
    pub notification_updated_at: Option<Instant>,
    pub show_keybindings: bool,
    pub show_theme_debug: bool,
    pub theme_editor: Option<ldnddev_theme::ThemeEditor>,
    pub active_tab: ActiveTab,
    pub theme: Theme,
    pub theme_source: ThemeSource,
    pub palette: PaletteState,
    pub copied_palette: Option<String>,
}

impl App {
    #[cfg(test)]
    pub fn new() -> Self {
        Self::with_theme(Theme::default(), ThemeSource::Default)
    }

    pub fn with_theme(theme: Theme, theme_source: ThemeSource) -> Self {
        let foreground = Color(Srgb::new(0.0, 0.0, 0.0));
        let background = Color(Srgb::new(1.0, 1.0, 1.0));

        App {
            foreground, // Default black
            background, // Default white
            foreground_input: foreground.to_hex(),
            background_input: background.to_hex(),
            input_target: InputTarget::Foreground, // Start with FG active
            current_input: foreground.to_hex(),
            cursor_char_idx: foreground.to_hex().chars().count(),
            parsed_fg: None,
            parsed_bg: None,
            last_parsed_format: None,
            contrast_ratio: 21.0, // Default black on white
            preview_text: "The quick brown fox jumps over the lazy dog.".to_string(),
            preview_font_family: "Roboto".to_string(),
            font_size_px: 16,
            is_bold: false,
            weight: 400,
            italic: false,
            style_chip: 0,
            mode: Mode::Contrast,
            focus: FocusId::FgHex,
            editing: true,
            fix_open: false,
            fix: FixState::default(),
            fix_source: FixSource::Contrast,
            nudge_dragging: None,
            fix_send_chip: 0,
            targets: Targets::default(),
            layout: LayoutMap::default(),
            hovered: None,
            mouse_pos: None,
            scrollbar_dragging: false,
            contrast_scroll: 0,
            contrast_max_scroll: 0,
            contrast_scroll_focus: None,
            last_mouse_click_pos: None,
            error: None,
            status: None,
            notification_updated_at: None,
            show_keybindings: false,
            show_theme_debug: false,
            theme_editor: None,
            active_tab: ActiveTab::Contrast,
            theme,
            theme_source,
            palette: PaletteState::default(),
            copied_palette: None,
        }
    }

    pub fn adjust_font_size(&mut self, delta: i16) {
        let updated = self.font_size_px as i16 + delta;
        self.font_size_px = updated.clamp(6, 120) as u16;
    }

    pub fn adjust_weight(&mut self, delta: i16) {
        let updated = self.weight as i16 + delta;
        self.weight = ((updated / 100) * 100).clamp(100, 900) as u16;
        self.is_bold = self.weight >= 700;
    }

    pub fn apply_style_preset(&mut self, preset: StylePreset) {
        let (weight, italic) = preset.apply();
        self.weight = weight;
        self.italic = italic;
        self.is_bold = weight >= 700;
        self.style_chip = preset.index();
    }

    pub fn move_style_chip(&mut self, delta: i16) {
        let len = StylePreset::ALL.len() as i16;
        let next = (self.style_chip as i16 + delta).rem_euclid(len) as usize;
        self.apply_style_preset(StylePreset::from_index(next));
    }

    pub fn active_style_preset(&self) -> Option<StylePreset> {
        match (self.weight, self.italic) {
            (400, false) => Some(StylePreset::Regular),
            (700, false) => Some(StylePreset::Bold),
            (400, true) => Some(StylePreset::Italic),
            (700, true) => Some(StylePreset::BoldItalic),
            _ => None,
        }
    }

    pub fn cycle_style(&mut self) {
        let next = match self.active_style_preset() {
            Some(StylePreset::Regular) => StylePreset::Bold,
            Some(StylePreset::Bold) => StylePreset::Italic,
            Some(StylePreset::Italic) => StylePreset::BoldItalic,
            Some(StylePreset::BoldItalic) | None => StylePreset::Regular,
        };
        self.apply_style_preset(next);
    }

    pub fn toggle_bold_preset(&mut self) {
        if self.weight >= 700 {
            self.weight = 400;
        } else {
            self.weight = 700;
        }
        self.is_bold = self.weight >= 700;
        self.style_chip = self
            .active_style_preset()
            .map(StylePreset::index)
            .unwrap_or(self.style_chip);
    }

    pub fn swap_colors(&mut self) {
        std::mem::swap(&mut self.foreground, &mut self.background);
        std::mem::swap(&mut self.foreground_input, &mut self.background_input);
        std::mem::swap(&mut self.parsed_fg, &mut self.parsed_bg);
        match self.input_target {
            InputTarget::Foreground => {
                self.current_input = self.foreground_input.clone();
                self.cursor_char_idx = self.current_input.chars().count();
            }
            InputTarget::Background => {
                self.current_input = self.background_input.clone();
                self.cursor_char_idx = self.current_input.chars().count();
            }
            _ => {}
        }
        self.update_contrast();
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.active_tab = match mode {
            Mode::Contrast => ActiveTab::Contrast,
            Mode::Palette => ActiveTab::Palette,
        };
        self.focus = match mode {
            Mode::Contrast => FocusId::FgHex,
            Mode::Palette => FocusId::Role(0),
        };
        self.sync_focus_input();
    }

    pub fn contrast_thresholds(&self) -> (f64, f64) {
        let large = is_large_text(self.font_size_px, self.weight);
        (
            self.targets.wcag.text_threshold(large),
            self.targets.apca.value(),
        )
    }

    fn refresh_fix_search(&mut self) {
        let (wcag, apca) = self.contrast_thresholds();
        self.fix
            .search(self.foreground, self.background, wcag, apca);
    }

    pub fn toggle_fix(&mut self) {
        if self.fix_open {
            self.close_fix();
        } else {
            self.open_fix();
        }
    }

    pub fn open_fix(&mut self) {
        if self.mode == Mode::Palette {
            self.open_fix_for_matrix();
            return;
        }
        self.fix_source = FixSource::Contrast;
        self.fix_open = true;
        self.nudge_dragging = None;
        self.fix.axis = FixAxis::Fg;
        self.refresh_fix_search();
        self.set_focus(FocusId::NudgeFg);
    }

    pub fn open_fix_for_matrix(&mut self) {
        let (wcag, apca) = self.contrast_thresholds();
        let Some(cell) = matrix_cell(
            &self.palette,
            self.palette.matrix_row,
            self.palette.matrix_col,
            wcag,
            apca,
        ) else {
            self.notify_error("Cannot open Fix: that matrix pair has an invalid color.");
            return;
        };
        self.fix_source = FixSource::Palette {
            text: cell.text,
            surface: cell.surface,
        };
        self.fix_open = true;
        self.nudge_dragging = None;
        self.fix.axis = if cell.text.as_palette_input().is_none() {
            FixAxis::Bg
        } else {
            FixAxis::Fg
        };
        self.fix.search(cell.fg, cell.bg, wcag, apca);
        self.set_focus(match self.fix.axis {
            FixAxis::Fg => FocusId::NudgeFg,
            FixAxis::Bg => FocusId::NudgeBg,
        });
    }

    pub fn close_fix(&mut self) {
        self.fix_open = false;
        self.nudge_dragging = None;
        if matches!(
            self.focus,
            FocusId::NudgeFg
                | FocusId::NudgeBg
                | FocusId::SendFg
                | FocusId::SendBg
                | FocusId::ApplyFix
                | FocusId::NextFix
                | FocusId::CloseFix
        ) {
            if self.mode == Mode::Palette {
                self.set_focus(FocusId::Matrix);
            } else {
                self.set_focus(FocusId::FixBtn);
            }
        }
    }

    pub fn apply_fix(&mut self) {
        if !self.fix_open {
            return;
        }
        match self.fix_source {
            FixSource::Contrast => {
                self.foreground = self.fix.candidate_fg;
                self.background = self.fix.candidate_bg;
                self.foreground_input = self.foreground.to_hex();
                self.background_input = self.background.to_hex();
                match self.focus {
                    FocusId::FgHex => {
                        self.current_input = self.foreground_input.clone();
                        self.cursor_char_idx = self.current_input.chars().count();
                    }
                    FocusId::BgHex => {
                        self.current_input = self.background_input.clone();
                        self.cursor_char_idx = self.current_input.chars().count();
                    }
                    _ => {}
                }
                self.update_contrast();
                self.refresh_fix_search();
                self.notify_status(format!(
                    "Applied {} on {}",
                    self.foreground.to_hex(),
                    self.background.to_hex()
                ));
            }
            FixSource::Palette { text, surface } => {
                if let Some(role) = text.as_palette_input() {
                    self.palette
                        .set_role_hex(role, self.fix.candidate_fg.to_hex());
                }
                if let Some(role) = surface.as_palette_input() {
                    self.palette
                        .set_role_hex(role, self.fix.candidate_bg.to_hex());
                }
                let (wcag, apca) = self.contrast_thresholds();
                if let Some(cell) =
                    matrix_cell(&self.palette, text.index(), surface.index(), wcag, apca)
                {
                    self.fix.search(cell.fg, cell.bg, wcag, apca);
                }
                self.notify_status(format!(
                    "Applied {} on {} to palette bases",
                    self.fix.original_fg.to_hex(),
                    self.fix.original_bg.to_hex()
                ));
            }
        }
    }

    pub fn next_fix_candidate(&mut self) {
        if !self.fix_open {
            return;
        }
        self.fix.next();
    }

    pub fn nudge_fix(&mut self, axis: FixAxis, delta: f32) {
        if !self.fix_open {
            return;
        }
        self.fix.nudge(axis, delta);
        self.focus = match axis {
            FixAxis::Fg => FocusId::NudgeFg,
            FixAxis::Bg => FocusId::NudgeBg,
        };
        self.editing = false;
    }

    pub fn nudge_fix_hue(&mut self, axis: FixAxis, degrees: f32) {
        if !self.fix_open {
            return;
        }
        self.fix.nudge_hue(axis, degrees);
        self.focus = match axis {
            FixAxis::Fg => FocusId::NudgeFg,
            FixAxis::Bg => FocusId::NudgeBg,
        };
        self.editing = false;
    }

    pub fn move_fix_send_chip(&mut self, delta: i32) {
        let next = i32::from(self.fix_send_chip as u16) + delta;
        self.fix_send_chip = next.clamp(0, 3) as usize;
    }

    pub fn send_fixed_to_role(&mut self, axis: FixAxis, role: PaletteInput) {
        if !self.fix_open {
            return;
        }
        let color = match axis {
            FixAxis::Fg => self.fix.candidate_fg,
            FixAxis::Bg => self.fix.candidate_bg,
        };
        let hex = color.to_hex();
        self.palette.set_role_hex(role, hex.clone());
        self.fix_send_chip = role.index();
        self.notify_status(format!("FIXED {} {hex} → {}", axis.label(), role.label()));
    }

    pub fn send_fixed_selected_chip(&mut self) {
        let axis = match self.focus {
            FocusId::SendBg | FocusId::NudgeBg => FixAxis::Bg,
            _ => FixAxis::Fg,
        };
        self.send_fixed_to_role(axis, PaletteInput::from_index(self.fix_send_chip));
    }

    pub fn set_fix_l_from_x(&mut self, axis: FixAxis, x: u16, gauge: ratatui::layout::Rect) {
        if gauge.width <= 1 {
            return;
        }
        let rel = f32::from(x.saturating_sub(gauge.x));
        let l = (rel / f32::from(gauge.width.saturating_sub(1))).clamp(0.0, 1.0);
        self.fix.set_axis_l(axis, l);
    }

    pub fn scroll_contrast_by(&mut self, delta: i32) {
        let next = i32::from(self.contrast_scroll) + delta;
        self.contrast_scroll = next.clamp(0, i32::from(self.contrast_max_scroll)) as u16;
    }

    pub fn set_focus(&mut self, focus: FocusId) {
        self.focus = focus;
        if focus == FocusId::Style {
            self.style_chip = self
                .active_style_preset()
                .map(StylePreset::index)
                .unwrap_or(self.style_chip);
        }
        self.sync_focus_input();
    }

    fn sync_focus_input(&mut self) {
        let target = match self.focus {
            FocusId::FgHex => InputTarget::Foreground,
            FocusId::BgHex => InputTarget::Background,
            FocusId::PreviewText => InputTarget::PreviewText,
            FocusId::FontFamily => InputTarget::FontFamily,
            _ => InputTarget::None,
        };
        if target != self.input_target {
            self.set_input_target(target);
        }
        self.editing = self.focus.is_text_field();
    }

    pub fn focus_order(&self) -> Vec<FocusId> {
        let mut order = match self.mode {
            Mode::Contrast => FocusId::contrast_order().to_vec(),
            Mode::Palette => FocusId::palette_order().to_vec(),
        };
        if self.fix_open {
            order.extend_from_slice(FocusId::fix_order());
        }
        order
    }

    pub fn cycle_focus(&mut self, reverse: bool) -> bool {
        let order = self.focus_order();
        if order.is_empty() {
            return true;
        }
        let current = order.iter().position(|id| *id == self.focus).unwrap_or(0);
        let next = if reverse {
            if current == 0 {
                order.len() - 1
            } else {
                current - 1
            }
        } else {
            (current + 1) % order.len()
        };
        self.set_focus(order[next]);
        true
    }

    pub fn copy_focused_hex(&self) -> Option<String> {
        match self.focus {
            FocusId::FgHex => Some(self.foreground.to_hex()),
            FocusId::BgHex => Some(self.background.to_hex()),
            FocusId::Role(idx) => {
                let input = match idx {
                    0 => self.palette.primary_input.as_str(),
                    1 => self.palette.secondary_input.as_str(),
                    2 => self.palette.tertiary_input.as_str(),
                    3 => self.palette.support_input.as_str(),
                    _ => return None,
                };
                parse_palette_color(input).ok().map(|c| c.to_hex())
            }
            _ => None,
        }
    }

    pub fn set_input_target(&mut self, target: InputTarget) {
        self.input_target = target;
        self.current_input = match target {
            InputTarget::Foreground => self.foreground_input.clone(),
            InputTarget::Background => self.background_input.clone(),
            InputTarget::PreviewText => self.preview_text.clone(),
            InputTarget::FontFamily => self.preview_font_family.clone(),
            InputTarget::None => String::new(),
        };
        self.cursor_char_idx = self.current_input.chars().count();
    }

    pub fn sync_active_input(&mut self) {
        match self.input_target {
            InputTarget::Foreground => self.foreground_input = self.current_input.clone(),
            InputTarget::Background => self.background_input = self.current_input.clone(),
            InputTarget::PreviewText => self.preview_text = self.current_input.clone(),
            InputTarget::FontFamily => self.preview_font_family = self.current_input.clone(),
            InputTarget::None => {}
        }
    }

    pub fn cycle_font_family(&mut self) {
        const PRESET_FONTS: [&str; 5] = ["Roboto", "Open Sans", "Lato", "Montserrat", "Poppins"];
        let current = self.preview_font_family.trim();
        let idx = PRESET_FONTS
            .iter()
            .position(|font| font.eq_ignore_ascii_case(current))
            .unwrap_or(0);
        let next = PRESET_FONTS[(idx + 1) % PRESET_FONTS.len()];
        self.preview_font_family = next.to_string();
        if self.input_target == InputTarget::FontFamily {
            self.current_input = self.preview_font_family.clone();
            self.cursor_char_idx = self.current_input.chars().count();
        }
    }

    fn clamp_cursor(&mut self) {
        let len = self.current_input.chars().count();
        if self.cursor_char_idx > len {
            self.cursor_char_idx = len;
        }
    }

    fn byte_index_at_char(s: &str, char_idx: usize) -> usize {
        s.char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(s.len())
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_char_idx > 0 {
            self.cursor_char_idx -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        let len = self.current_input.chars().count();
        if self.cursor_char_idx < len {
            self.cursor_char_idx += 1;
        }
    }

    pub fn insert_char_at_cursor(&mut self, c: char) {
        self.clamp_cursor();
        let byte_idx = Self::byte_index_at_char(&self.current_input, self.cursor_char_idx);
        self.current_input.insert(byte_idx, c);
        self.cursor_char_idx += 1;
    }

    pub fn insert_newline_at_cursor(&mut self) {
        self.insert_char_at_cursor('\n');
    }

    pub fn insert_str_at_cursor(&mut self, s: &str) {
        for c in s.chars() {
            if c == '\r' {
                continue;
            }
            self.insert_char_at_cursor(c);
        }
    }

    pub fn move_cursor_home(&mut self) {
        self.cursor_char_idx = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.cursor_char_idx = self.current_input.chars().count();
    }

    pub fn backspace_at_cursor(&mut self) {
        self.clamp_cursor();
        if self.cursor_char_idx == 0 {
            return;
        }
        let end = Self::byte_index_at_char(&self.current_input, self.cursor_char_idx);
        let start = Self::byte_index_at_char(&self.current_input, self.cursor_char_idx - 1);
        self.current_input.replace_range(start..end, "");
        self.cursor_char_idx -= 1;
    }

    pub fn delete_at_cursor(&mut self) {
        self.clamp_cursor();
        let len = self.current_input.chars().count();
        if self.cursor_char_idx >= len {
            return;
        }
        let start = Self::byte_index_at_char(&self.current_input, self.cursor_char_idx);
        let end = Self::byte_index_at_char(&self.current_input, self.cursor_char_idx + 1);
        self.current_input.replace_range(start..end, "");
    }

    /// Apply FG/BG draft when it parses. Incomplete input keeps the last committed color.
    pub fn try_live_apply_color(&mut self) -> bool {
        if !matches!(self.focus, FocusId::FgHex | FocusId::BgHex) {
            return false;
        }
        let Ok((color, format_label)) = Color::parse_input(&self.current_input) else {
            return false;
        };
        match self.focus {
            FocusId::FgHex => {
                self.foreground = color;
                self.parsed_fg = Some(color);
                self.foreground_input = self.current_input.clone();
            }
            FocusId::BgHex => {
                self.background = color;
                self.parsed_bg = Some(color);
                self.background_input = self.current_input.clone();
            }
            _ => return false,
        }
        self.last_parsed_format = Some(format_label.to_string());
        if self.error.is_some() {
            self.error = None;
            self.notification_updated_at = None;
        }
        self.update_contrast();
        true
    }

    // Updates contrast ratio if both colors are parsed
    pub fn update_contrast(&mut self) {
        self.contrast_ratio = self.foreground.contrast_ratio(&self.background);
    }

    pub fn submit_input(&mut self) -> bool {
        if self.current_input.trim().is_empty() {
            return true;
        }

        if self.input_target == InputTarget::PreviewText {
            self.preview_text = self.current_input.clone();
            self.error = None;
            self.notification_updated_at = None;
            self.clamp_cursor();
            return true;
        }
        if self.input_target == InputTarget::FontFamily {
            if self.current_input.trim().is_empty() {
                self.notify_error("Font family cannot be empty");
                return false;
            }
            self.preview_font_family = self.current_input.clone();
            self.error = None;
            self.notification_updated_at = None;
            self.clamp_cursor();
            return true;
        }

        match Color::parse_input(&self.current_input) {
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
                    InputTarget::FontFamily => {}
                    InputTarget::None => {}
                }
                self.last_parsed_format = Some(format_label.to_string());
                self.error = None;
                self.notification_updated_at = None;
                self.update_contrast();
                self.clamp_cursor();
                true
            }
            Err(err) => {
                self.notify_error(err);
                self.last_parsed_format = None;
                self.clamp_cursor();
                false
            }
        }
    }

    pub fn generate_palette(&mut self) -> bool {
        match generate_palette(&self.palette) {
            Ok(generated) => {
                let blocking_count = generated.blocking_failures().len();
                let advisory_count = generated.advisory_failures().len();
                self.palette.generated = Some(generated);
                self.palette.detail_scroll = 0;
                let (wcag, apca) = self.contrast_thresholds();
                self.palette.select_worst_failing_pair(wcag, apca);
                self.error = None;
                self.notify_status(format!(
                    "Palette generated: {blocking_count} blocking failure(s), {advisory_count} advisory warning(s)."
                ));
                blocking_count == 0
            }
            Err(err) => {
                self.notify_error(err);
                false
            }
        }
    }

    pub fn prepare_palette_export(&self, action: &str) -> Result<&GeneratedPalette, String> {
        validate_export(self.palette.generated.as_ref(), action)
    }

    pub fn notify_status(&mut self, message: impl Into<String>) {
        self.status = Some(message.into());
        self.error = None;
        self.notification_updated_at = Some(Instant::now());
    }

    pub fn notify_error(&mut self, message: impl Into<String>) {
        self.error = Some(message.into());
        self.status = None;
        self.notification_updated_at = Some(Instant::now());
    }

    pub fn clear_notification(&mut self) {
        self.error = None;
        self.status = None;
        self.notification_updated_at = None;
    }

    pub fn expire_notification(&mut self, now: Instant) -> bool {
        if self
            .notification_updated_at
            .is_some_and(|updated_at| now.duration_since(updated_at) >= TOAST_TTL)
        {
            self.clear_notification();
            true
        } else {
            false
        }
    }

    pub fn toast_remaining(&self, now: Instant) -> Option<Duration> {
        let updated_at = self.notification_updated_at?;
        Some(TOAST_TTL.saturating_sub(now.duration_since(updated_at)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn adjust_font_size_clamps_to_bounds() {
        let mut app = App::new();
        assert_eq!(app.font_size_px, 16);

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
        app.cursor_char_idx = app.current_input.chars().count();
        app.sync_active_input();
        assert!(app.submit_input());
        assert_eq!(app.preview_text, "Custom preview sample");
    }

    #[test]
    fn font_family_target_updates_preview_font_family() {
        let mut app = App::new();
        app.set_input_target(InputTarget::FontFamily);
        app.current_input = "Inter".to_string();
        app.cursor_char_idx = app.current_input.chars().count();
        app.sync_active_input();
        assert!(app.submit_input());
        assert_eq!(app.preview_font_family, "Inter");
    }

    #[test]
    fn cycle_font_family_rotates_presets() {
        let mut app = App::new();
        app.preview_font_family = "Roboto".to_string();
        app.cycle_font_family();
        assert_eq!(app.preview_font_family, "Open Sans");
    }

    #[test]
    fn cursor_insert_and_backspace_edit_at_position() {
        let mut app = App::new();
        app.set_input_target(InputTarget::PreviewText);
        app.current_input = "ab".to_string();
        app.cursor_char_idx = 1;

        app.insert_char_at_cursor('X');
        assert_eq!(app.current_input, "aXb");
        assert_eq!(app.cursor_char_idx, 2);

        app.backspace_at_cursor();
        assert_eq!(app.current_input, "ab");
        assert_eq!(app.cursor_char_idx, 1);
    }

    #[test]
    fn generate_palette_creates_exportable_scss() {
        let mut app = App::new();

        assert!(app.generate_palette());
        let export = app
            .prepare_palette_export("saving")
            .expect("generated palette exports");

        assert!(export.scss.contains("$c_primary_default"));
        assert!(export.tokens_json.contains("\"c_primary_default\""));
        assert!(export.tokens_json.contains("\"$type\": \"color\""));
        assert!(app.error.is_none());
    }

    #[test]
    fn invalid_palette_base_color_sets_error() {
        let mut app = App::new();
        app.palette.primary_input = "bad-color".to_string();

        assert!(!app.generate_palette());
        assert!(app.error.as_deref().unwrap_or("").contains("Primary"));
    }

    #[test]
    fn notifications_expire_after_toast_ttl() {
        let mut app = App::new();
        let now = Instant::now();
        app.notify_status("Saved");
        app.notification_updated_at = Some(now);

        assert!(!app.expire_notification(now + Duration::from_secs(4)));
        assert!(app.status.is_some());

        assert!(app.expire_notification(now + TOAST_TTL));
        assert!(app.status.is_none());
        assert!(app.error.is_none());
    }

    #[test]
    fn live_apply_updates_valid_hex_and_ignores_partial() {
        let mut app = App::new();
        app.set_focus(FocusId::FgHex);
        let original = app.foreground.to_hex();

        app.current_input = "#0f".to_string();
        assert!(!app.try_live_apply_color());
        assert_eq!(app.foreground.to_hex(), original);
        assert!(app.error.is_none());

        app.current_input = "#00ff00".to_string();
        assert!(app.try_live_apply_color());
        assert_eq!(app.foreground.to_hex(), "#00ff00");
    }

    #[test]
    fn delete_and_home_end_edit_at_caret() {
        let mut app = App::new();
        app.set_input_target(InputTarget::PreviewText);
        app.current_input = "abcd".to_string();
        app.cursor_char_idx = 1;
        app.delete_at_cursor();
        assert_eq!(app.current_input, "acd");
        app.move_cursor_end();
        assert_eq!(app.cursor_char_idx, 3);
        app.move_cursor_home();
        assert_eq!(app.cursor_char_idx, 0);
    }

    #[test]
    fn apply_fix_from_text_on_primary_does_not_rewrite_text_token() {
        let mut app = App::new();
        app.set_mode(Mode::Palette);
        app.palette.primary_input = "#808080".to_string();
        app.palette
            .set_matrix(MatrixAxis::Text.index(), MatrixAxis::Primary.index());
        app.open_fix_for_matrix();
        assert!(app.fix_open);
        assert!(matches!(
            app.fix_source,
            FixSource::Palette {
                text: MatrixAxis::Text,
                surface: MatrixAxis::Primary
            }
        ));
        assert_eq!(app.fix.axis, crate::fix::FixAxis::Bg);
        let secondary = app.palette.secondary_input.clone();
        let support = app.palette.support_input.clone();
        app.apply_fix();
        assert_eq!(app.palette.secondary_input, secondary);
        assert_eq!(app.palette.support_input, support);
        assert_ne!(app.palette.primary_input, "#808080");
    }

    #[test]
    fn generate_selects_an_off_diagonal_matrix_pair() {
        let mut app = App::new();
        assert!(app.generate_palette());
        assert_ne!(app.palette.matrix_row, app.palette.matrix_col);
    }
}
