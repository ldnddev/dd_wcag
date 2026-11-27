//! Window session. Every product action calls the shared [`dd_wcag::app::App`].

use std::time::{Duration, Instant};

use dd_wcag::app::{App, FixSource, FocusId, Mode, StylePreset};
use dd_wcag::cli::{self, LaunchSeed};
use dd_wcag::color::{self, Color};
use dd_wcag::contrast::contrast_body_text;
use dd_wcag::fix::FixAxis;
use dd_wcag::palette::{self, PaletteInput};
use dd_wcag::theme::{Theme, ThemeSource};

const DOUBLE_CLICK: Duration = Duration::from_millis(420);

#[derive(Clone, Debug, Default)]
pub struct Effect {
    pub sync_preview: bool,
    pub open_preview: bool,
    pub save_palette: bool,
    pub copy_text: Option<String>,
    pub copy_palette: bool,
    pub paste: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum Click {
    Mode(Mode),
    Focus(FocusId),
    Style(usize),
    Role(usize),
    Matrix {
        row: usize,
        col: usize,
    },
    Wcag,
    Apca,
    Swap,
    /// Copy whatever color field is focused.
    CopyFocused,
    /// Copy the contrast foreground. This is the Copy control.
    CopyPair,
    Fix,
    Preview,
    Generate,
    ApplyFix,
    NextFix,
    CloseFix,
    Nudge {
        axis: FixAxis,
        delta: f32,
    },
    Send {
        axis: FixAxis,
        role: PaletteInput,
    },
}

pub struct Product {
    pub app: App,
    last_matrix: Option<(usize, usize, Instant)>,
    last_role: Option<(usize, Instant)>,
}

impl Product {
    pub fn new(seed: Option<&LaunchSeed>) -> Self {
        let mut app = App::with_theme(Theme::default(), ThemeSource::Default);
        if let Some(seed) = seed {
            cli::apply_launch_seed(&mut app, seed);
        }
        Self {
            app,
            last_matrix: None,
            last_role: None,
        }
    }

    pub fn handle_key(&mut self, key: &str) -> Effect {
        let key = key.trim();
        let mut effect = Effect::default();
        if key.starts_with("ctrl-") {
            self.ctrl(key, &mut effect);
            return effect;
        }
        match key {
            "escape" => self.escape(),
            "tab" => self.tab(false, &mut effect),
            "shift-tab" | "backtab" => self.tab(true, &mut effect),
            "left" => self.horizontal(-1, &mut effect),
            "right" => self.horizontal(1, &mut effect),
            "home" => self.home(),
            "end" => self.end(),
            "delete" => self.delete(&mut effect),
            "backspace" => self.backspace(&mut effect),
            "enter" => self.enter(&mut effect),
            "up" => self.vertical(true, false, &mut effect),
            "down" => self.vertical(false, false, &mut effect),
            "shift-up" => self.vertical(true, true, &mut effect),
            "shift-down" => self.vertical(false, true, &mut effect),
            "pageup" => self.page(true, false),
            "pagedown" => self.page(false, false),
            "shift-pageup" => self.page(true, true),
            "shift-pagedown" => self.page(false, true),
            "space" => self.space(&mut effect),
            "[" => self.lightness(-0.02, &mut effect),
            "shift-[" => self.lightness(-0.10, &mut effect),
            "]" => self.lightness(0.02, &mut effect),
            "shift-]" => self.lightness(0.10, &mut effect),
            "{" => self.hue(-10.0, &mut effect),
            "shift-{" => self.hue(-30.0, &mut effect),
            "}" => self.hue(10.0, &mut effect),
            "shift-}" => self.hue(30.0, &mut effect),
            other if is_char(other) => self.char(other.chars().next().unwrap_or('?'), &mut effect),
            _ => {}
        }
        effect
    }

    pub fn apply_paste(&mut self, raw: &str) -> bool {
        let text = raw.trim();
        if text.is_empty() {
            return false;
        }
        if self.app.mode == Mode::Palette && self.app.palette.editing {
            if Color::parse_input(text).is_ok() {
                self.app.palette.edit_input = text.to_string();
                self.app.palette.edit_cursor_char_idx = self.app.palette.edit_input.chars().count();
            } else {
                self.app.palette.insert_str_at_cursor(text);
            }
            return false;
        }
        if !self.app.focus.is_text_field() {
            return false;
        }
        let is_color = matches!(self.app.focus, FocusId::FgHex | FocusId::BgHex);
        if is_color && Color::parse_input(text).is_ok() {
            self.app.current_input = text.to_string();
            self.app.cursor_char_idx = self.app.current_input.chars().count();
            self.app.sync_active_input();
            return self.app.submit_input();
        }
        self.app.insert_str_at_cursor(text);
        self.app.sync_active_input();
        self.app.try_live_apply_color() || self.app.focus == FocusId::PreviewText
    }

    pub fn click(&mut self, click: Click) -> Effect {
        let mut effect = Effect::default();
        match click {
            Click::Mode(mode) => self.app.set_mode(mode),
            Click::Focus(focus) => self.app.set_focus(focus),
            Click::Style(index) => {
                self.app.set_focus(FocusId::Style);
                self.app.apply_style_preset(StylePreset::from_index(index));
                effect.sync_preview = true;
            }
            Click::Role(index) => self.click_role(index),
            Click::Matrix { row, col } => self.click_matrix(row, col, &mut effect),
            Click::Wcag => {
                self.app.set_focus(FocusId::TargetWcag);
                self.app.targets.wcag = self.app.targets.wcag.cycle();
            }
            Click::Apca => {
                self.app.set_focus(FocusId::TargetApca);
                self.app.targets.apca = self.app.targets.apca.cycle();
            }
            Click::Swap => {
                self.app.set_focus(FocusId::Swap);
                self.app.swap_colors();
                effect.sync_preview = true;
            }
            Click::CopyFocused => {
                effect.copy_text = self.app.copy_focused_hex();
            }
            Click::CopyPair => {
                self.app.set_focus(FocusId::CopyHex);
                effect.copy_text = Some(self.app.foreground.to_hex());
            }
            Click::Fix => {
                self.app.toggle_fix();
                effect.sync_preview = self.app.fix_open;
            }
            Click::Preview => effect.open_preview = true,
            Click::Generate => {
                self.app.set_mode(Mode::Palette);
                self.app.generate_palette();
                self.app.set_focus(FocusId::Matrix);
            }
            Click::ApplyFix => {
                self.app.apply_fix();
                effect.sync_preview = true;
            }
            Click::NextFix => self.app.next_fix_candidate(),
            Click::CloseFix => self.app.close_fix(),
            Click::Nudge { axis, delta } => self.app.nudge_fix(axis, delta),
            Click::Send { axis, role } => {
                self.app.set_focus(match axis {
                    FixAxis::Fg => FocusId::SendFg,
                    FixAxis::Bg => FocusId::SendBg,
                });
                self.app.send_fixed_to_role(axis, role);
            }
        }
        effect
    }

    /// Lines the headless tests and the window both read.
    pub fn lines(&self) -> Vec<String> {
        let app = &self.app;
        let mut lines = vec![format!("mode:{}", mode_name(app.mode))];
        lines.push(format!("focus:{:?}", app.focus));
        lines.push(format!(
            "Foreground {}",
            shown_color(app, FocusId::FgHex, &app.foreground_input)
        ));
        lines.push(format!(
            "Background {}",
            shown_color(app, FocusId::BgHex, &app.background_input)
        ));
        lines.push(format!("Size {}", app.font_size_px));
        lines.push(format!("Weight {}", app.weight));
        let style = app
            .active_style_preset()
            .map(StylePreset::label)
            .unwrap_or("custom");
        lines.push(format!(
            "Style {style}{}",
            if app.italic { " italic" } else { "" }
        ));
        lines.push(format!("WCAG {}", app.targets.wcag.label()));
        lines.push(format!("APCA {}", app.targets.apca.label()));
        lines.push(contrast_body_text(
            &app.foreground,
            &app.background,
            app.font_size_px,
            app.weight,
        ));
        let lc = app.foreground.apca_lc(&app.background);
        let lookup = color::apca_lookup_lc(app.font_size_px, app.weight);
        let polarity = if lc >= 0.0 { "light text" } else { "dark text" };
        let apca_pass = if lc.abs() >= app.targets.apca.value() {
            "pass"
        } else {
            "fail"
        };
        lines.push(format!(
            "APCA Lc {lc:.2} {apca_pass} lookup Lc{lookup:.0} {polarity}"
        ));
        lines.push(format!(
            "HEX {} {}",
            app.foreground.to_hex(),
            app.background.to_hex()
        ));
        lines.push(format!(
            "RGB {} {}",
            app.foreground.to_rgb_str(),
            app.background.to_rgb_str()
        ));
        lines.push(format!(
            "HSL {} {}",
            app.foreground.to_hsl_str(),
            app.background.to_hsl_str()
        ));
        lines.push(format!("font:{}", app.preview_font_family));
        lines.push(format!("preview:{}", app.preview_text));
        lines.push(format!("contrast-scroll:{}", app.contrast_scroll));
        if app.fix_open {
            lines.push("fix:open".to_string());
            lines.push(format!(
                "fix-pair:{} on {}",
                app.fix.candidate_fg.to_hex(),
                app.fix.candidate_bg.to_hex()
            ));
            lines.push(format!(
                "fix-index:{}/{}",
                app.fix.candidate_index() + 1,
                app.fix.candidate_count().max(1)
            ));
            lines.push(format!("fix-l:{:.2}", app.fix.candidate_fg.oklab_l()));
            let source = match app.fix_source {
                FixSource::Contrast => "contrast".to_string(),
                FixSource::Palette { text, surface } => {
                    format!("palette:{} on {}", text.label(), surface.label())
                }
            };
            lines.push(format!("fix-source:{source}"));
        } else {
            lines.push("fix:closed".to_string());
        }
        for role in PaletteInput::ALL {
            let value = if app.palette.editing && app.palette.selected() == role {
                app.palette.edit_input.as_str()
            } else {
                app.palette.input_for(role)
            };
            lines.push(format!("{} {value}", role.label()));
        }
        let (wcag, apca) = app.contrast_thresholds();
        if let Some(cell) = palette::matrix_cell(
            &app.palette,
            app.palette.matrix_row,
            app.palette.matrix_col,
            wcag,
            apca,
        ) {
            lines.push(format!(
                "matrix:{} on {} {} {:.2} Lc {:.0}",
                cell.text.label(),
                cell.surface.label(),
                cell.glyph(),
                cell.ratio,
                cell.lc
            ));
        } else {
            lines.push("matrix:invalid".to_string());
        }
        let mut grid = String::from("matrix-grid:");
        for row in 0..5 {
            grid.push(' ');
            for col in 0..5 {
                if row == col {
                    grid.push('·');
                } else if let Some(cell) = palette::matrix_cell(&app.palette, row, col, wcag, apca)
                {
                    grid.push(cell.glyph());
                } else {
                    grid.push('?');
                }
            }
        }
        lines.push(grid);
        if let Some(generated) = &app.palette.generated {
            lines.push(format!(
                "generated:{} blocking:{} advisory:{}",
                generated.tokens.len(),
                generated.blocking_failures().len(),
                generated.advisory_failures().len()
            ));
            let start = app.palette.detail_scroll;
            for check in generated.checks.iter().skip(start).take(8) {
                let tag = if check.passes {
                    "PASS"
                } else if check.blocking {
                    "FAIL"
                } else {
                    "WARN"
                };
                lines.push(format!(
                    "{tag} {:.2}:1 {}{}",
                    check.ratio,
                    check.label,
                    if !check.blocking && !check.passes {
                        " advisory"
                    } else {
                        ""
                    }
                ));
            }
        } else {
            lines.push("generated:none".to_string());
        }
        lines.push(format!("detail-scroll:{}", app.palette.detail_scroll));
        if let Some(message) = app.error.as_ref().or(app.status.as_ref()) {
            lines.push(message.clone());
        }
        lines
    }

    fn ctrl(&mut self, key: &str, effect: &mut Effect) {
        let shift = key.contains("shift");
        match key {
            "ctrl-b" => {
                self.app.toggle_bold_preset();
                effect.sync_preview = true;
            }
            "ctrl-f" => {
                self.app.toggle_fix();
                effect.sync_preview = self.app.fix_open;
            }
            "ctrl-g" => {
                self.app.set_mode(Mode::Palette);
                self.app.generate_palette();
                self.app.set_focus(FocusId::Matrix);
            }
            "ctrl-t" => {
                self.app.cycle_font_family();
                effect.sync_preview = true;
            }
            "ctrl-o" => effect.open_preview = true,
            "ctrl-s" => {
                if self.app.mode == Mode::Palette {
                    effect.save_palette = true;
                } else {
                    self.app.cycle_style();
                    effect.sync_preview = true;
                }
            }
            "ctrl-c" => {
                if self.app.mode == Mode::Palette {
                    effect.copy_palette = true;
                } else if let Some(hex) = self.app.copy_focused_hex() {
                    effect.copy_text = Some(hex);
                }
            }
            "ctrl-v" => effect.paste = true,
            "ctrl-n" => {
                if self.app.fix_open {
                    self.app.next_fix_candidate();
                    self.app.set_focus(FocusId::NextFix);
                }
            }
            "ctrl-up" | "ctrl-shift-up" => self.step(true, shift, effect),
            "ctrl-down" | "ctrl-shift-down" => self.step(false, shift, effect),
            _ => {}
        }
    }

    fn escape(&mut self) {
        if self.app.fix_open {
            self.app.close_fix();
        } else if self.app.mode == Mode::Palette && self.app.palette.editing {
            self.app.palette.cancel_edit();
        } else if self.app.editing && self.app.focus.is_text_field() {
            self.app.editing = false;
        }
    }

    fn tab(&mut self, reverse: bool, effect: &mut Effect) {
        if self.try_apply() {
            effect.sync_preview = true;
            self.app.cycle_focus(reverse);
        }
    }

    fn try_apply(&mut self) -> bool {
        if !self.app.focus.is_text_field() {
            return true;
        }
        self.app.sync_active_input();
        self.app.submit_input()
    }

    fn horizontal(&mut self, dir: i16, effect: &mut Effect) {
        if self.app.focus == FocusId::Style {
            self.app.move_style_chip(dir);
            effect.sync_preview = true;
        } else if self.app.focus == FocusId::TargetWcag {
            self.app.targets.wcag = if dir < 0 {
                self.app.targets.wcag.cycle_back()
            } else {
                self.app.targets.wcag.cycle()
            };
        } else if self.app.focus == FocusId::TargetApca {
            self.app.targets.apca = if dir < 0 {
                self.app.targets.apca.cycle_back()
            } else {
                self.app.targets.apca.cycle()
            };
        } else if self.app.focus == FocusId::Matrix {
            self.app.palette.move_matrix(0, i32::from(dir));
        } else if matches!(self.app.focus, FocusId::SendFg | FocusId::SendBg) {
            self.app.move_fix_send_chip(i32::from(dir));
        } else if self.app.mode == Mode::Palette && self.app.palette.editing {
            if dir < 0 {
                self.app.palette.move_cursor_left();
            } else {
                self.app.palette.move_cursor_right();
            }
        } else if self.app.focus.is_text_field() {
            if dir < 0 {
                self.app.move_cursor_left();
            } else {
                self.app.move_cursor_right();
            }
        }
    }

    fn home(&mut self) {
        if self.app.mode == Mode::Palette && self.app.palette.editing {
            self.app.palette.move_cursor_home();
        } else if self.app.focus.is_text_field() {
            self.app.move_cursor_home();
        }
    }

    fn end(&mut self) {
        if self.app.mode == Mode::Palette && self.app.palette.editing {
            self.app.palette.move_cursor_end();
        } else if self.app.focus.is_text_field() {
            self.app.move_cursor_end();
        }
    }

    fn delete(&mut self, effect: &mut Effect) {
        if self.app.mode == Mode::Palette && self.app.palette.editing {
            self.app.palette.delete_at_cursor();
        } else if self.app.focus.is_text_field() {
            self.app.delete_at_cursor();
            self.app.sync_active_input();
            if self.app.try_live_apply_color() || self.app.focus == FocusId::PreviewText {
                effect.sync_preview = true;
            }
        }
    }

    fn backspace(&mut self, effect: &mut Effect) {
        if self.app.mode == Mode::Palette && self.app.palette.editing {
            self.app.palette.backspace_at_cursor();
        } else if self.app.focus.is_text_field() {
            self.app.backspace_at_cursor();
            self.app.sync_active_input();
            if self.app.try_live_apply_color() || self.app.focus == FocusId::PreviewText {
                effect.sync_preview = true;
            }
        }
    }

    fn enter(&mut self, effect: &mut Effect) {
        if matches!(self.app.focus, FocusId::SendFg | FocusId::SendBg) {
            self.app.send_fixed_selected_chip();
            return;
        }
        if self.app.mode == Mode::Palette {
            if self.app.palette.editing {
                self.app.palette.commit_edit();
                return;
            }
            if self.app.focus == FocusId::Generate {
                self.app.generate_palette();
                self.app.set_focus(FocusId::Matrix);
                return;
            }
            if self.app.focus == FocusId::Matrix {
                self.app.open_fix_for_matrix();
                effect.sync_preview = self.app.fix_open;
                return;
            }
            if matches!(self.app.focus, FocusId::Role(_)) {
                self.app.palette.begin_edit();
                return;
            }
        }
        if self.app.focus == FocusId::PreviewText {
            self.app.insert_newline_at_cursor();
            self.app.sync_active_input();
            effect.sync_preview = true;
        } else if self.app.focus == FocusId::Swap {
            self.app.swap_colors();
            effect.sync_preview = true;
        } else if self.app.focus == FocusId::CopyHex {
            effect.copy_text = self.app.copy_focused_hex();
        } else if self.app.focus == FocusId::FixBtn {
            self.app.toggle_fix();
        } else if self.app.focus == FocusId::ApplyFix {
            self.app.apply_fix();
            effect.sync_preview = true;
        } else if self.app.focus == FocusId::NextFix {
            self.app.next_fix_candidate();
        } else if self.app.focus == FocusId::CloseFix {
            self.app.close_fix();
        } else if self.app.focus == FocusId::OpenPreview {
            effect.open_preview = true;
        } else if self.app.focus == FocusId::TargetWcag {
            self.app.targets.wcag = self.app.targets.wcag.cycle();
        } else if self.app.focus == FocusId::TargetApca {
            self.app.targets.apca = self.app.targets.apca.cycle();
        } else if self.app.focus == FocusId::Style {
            self.app
                .apply_style_preset(StylePreset::from_index(self.app.style_chip));
            effect.sync_preview = true;
        } else if self.app.focus.is_text_field() {
            let _ = self.try_apply();
            effect.sync_preview = true;
        }
    }

    fn vertical(&mut self, up: bool, shift: bool, effect: &mut Effect) {
        if matches!(
            self.app.focus,
            FocusId::Size | FocusId::Weight | FocusId::Style
        ) {
            self.step(up, shift, effect);
            return;
        }
        if self.app.focus == FocusId::SendBg && up {
            self.app.set_focus(FocusId::SendFg);
        } else if self.app.focus == FocusId::SendFg && !up {
            self.app.set_focus(FocusId::SendBg);
        } else if self.app.focus == FocusId::Detail {
            let step = if shift { 8 } else { 1 };
            self.app
                .palette
                .scroll_detail_by(if up { -step } else { step });
        } else if self.app.focus == FocusId::Matrix {
            self.app.palette.move_matrix(if up { -1 } else { 1 }, 0);
        } else if self.app.mode == Mode::Palette && !self.app.palette.editing {
            if up {
                self.app.palette.select_previous();
            } else {
                self.app.palette.select_next();
            }
            if let FocusId::Role(_) = self.app.focus {
                self.app
                    .set_focus(FocusId::Role(self.app.palette.selected_idx));
            }
        }
    }

    fn step(&mut self, up: bool, shift: bool, effect: &mut Effect) {
        let sign: i16 = if up { 1 } else { -1 };
        match self.app.focus {
            FocusId::Size => {
                let delta = if shift { 4 } else { 1 };
                self.app.adjust_font_size(sign * delta);
                effect.sync_preview = true;
            }
            FocusId::Weight => {
                let delta = if shift { 200 } else { 100 };
                self.app.adjust_weight(sign * delta);
                effect.sync_preview = true;
            }
            FocusId::Style => {
                self.app.move_style_chip(sign);
                effect.sync_preview = true;
            }
            FocusId::Detail => {
                let step = if shift { 8 } else { 3 };
                self.app.palette.scroll_detail_by(i32::from(sign) * step);
            }
            FocusId::NudgeFg => {
                let delta = if shift { 0.10 } else { 0.02 };
                self.app.nudge_fix(FixAxis::Fg, f32::from(sign) * delta);
            }
            FocusId::NudgeBg => {
                let delta = if shift { 0.10 } else { 0.02 };
                self.app.nudge_fix(FixAxis::Bg, f32::from(sign) * delta);
            }
            _ => {}
        }
    }

    fn page(&mut self, up: bool, shift: bool) {
        if self.app.mode == Mode::Palette {
            if let Some(generated) = &self.app.palette.generated {
                self.app.palette.detail_max_scroll = generated.checks.len().saturating_sub(12);
            }
            self.app.set_focus(FocusId::Detail);
            self.app.palette.scroll_detail_by(if up { -10 } else { 10 });
        } else {
            if self.app.contrast_max_scroll == 0 {
                self.app.contrast_max_scroll = 64;
            }
            let delta = if shift { 16 } else { 8 };
            self.app.scroll_contrast_by(if up { -delta } else { delta });
        }
    }

    fn space(&mut self, effect: &mut Effect) {
        if self.typing() {
            self.char(' ', effect);
            return;
        }
        if self.app.mode == Mode::Palette && self.app.focus == FocusId::Matrix {
            self.app.palette.swap_matrix_axes();
        } else if self.app.mode == Mode::Contrast {
            if self.app.focus == FocusId::Style {
                self.app
                    .apply_style_preset(StylePreset::from_index(self.app.style_chip));
                effect.sync_preview = true;
            } else if self.app.focus == FocusId::TargetWcag {
                self.app.targets.wcag = self.app.targets.wcag.cycle();
            } else if self.app.focus == FocusId::TargetApca {
                self.app.targets.apca = self.app.targets.apca.cycle();
            } else {
                self.app.swap_colors();
                effect.sync_preview = true;
            }
        }
    }

    fn lightness(&mut self, delta: f32, effect: &mut Effect) {
        if !self.color_nudge_focus() {
            return;
        }
        let axis = self.nudge_axis();
        if self.fix_nudge_focus()
            || (self.app.fix_open && !matches!(self.app.focus, FocusId::FgHex | FocusId::BgHex))
        {
            self.app.nudge_fix(axis, delta);
        } else {
            self.nudge_live(axis, delta);
            effect.sync_preview = true;
        }
    }

    fn hue(&mut self, degrees: f32, effect: &mut Effect) {
        if !self.color_nudge_focus() {
            return;
        }
        let axis = self.nudge_axis();
        if self.fix_nudge_focus()
            || (self.app.fix_open && !matches!(self.app.focus, FocusId::FgHex | FocusId::BgHex))
        {
            self.app.nudge_fix_hue(axis, degrees);
        } else {
            match axis {
                FixAxis::Fg => {
                    self.app.foreground = self.app.foreground.nudge_hue(degrees);
                    self.app.foreground_input = self.app.foreground.to_hex();
                    if self.app.focus == FocusId::FgHex {
                        self.app.current_input = self.app.foreground_input.clone();
                        self.app.cursor_char_idx = self.app.current_input.chars().count();
                    }
                }
                FixAxis::Bg => {
                    self.app.background = self.app.background.nudge_hue(degrees);
                    self.app.background_input = self.app.background.to_hex();
                    if self.app.focus == FocusId::BgHex {
                        self.app.current_input = self.app.background_input.clone();
                        self.app.cursor_char_idx = self.app.current_input.chars().count();
                    }
                }
            }
            self.app.update_contrast();
            effect.sync_preview = true;
        }
    }

    fn char(&mut self, c: char, effect: &mut Effect) {
        if self.app.fix_open && !self.typing() {
            if let Some(role) = role_from_key(c) {
                if self.fix_focus() {
                    let axis = if matches!(self.app.focus, FocusId::NudgeBg | FocusId::SendBg) {
                        FixAxis::Bg
                    } else {
                        FixAxis::Fg
                    };
                    self.app.send_fixed_to_role(axis, role);
                    return;
                }
            }
        }
        if !self.typing() {
            match c {
                '1' => {
                    self.app.set_mode(Mode::Contrast);
                    return;
                }
                '2' => {
                    self.app.set_mode(Mode::Palette);
                    return;
                }
                _ => {}
            }
        }
        if self.app.mode == Mode::Palette && self.app.palette.editing {
            self.app.palette.insert_char_at_cursor(c);
            return;
        }
        if self.app.editing && self.app.focus.is_text_field() {
            self.app.insert_char_at_cursor(c);
            self.app.sync_active_input();
            if self.app.try_live_apply_color() || self.app.focus == FocusId::PreviewText {
                effect.sync_preview = true;
            }
        }
    }

    fn click_role(&mut self, index: usize) {
        let now = Instant::now();
        let double = self
            .last_role
            .is_some_and(|(prev, at)| prev == index && now.duration_since(at) <= DOUBLE_CLICK);
        self.last_role = Some((index, now));
        self.app.palette.selected_idx = index;
        self.app.set_focus(FocusId::Role(index));
        if double {
            self.app.palette.begin_edit();
        }
    }

    fn click_matrix(&mut self, row: usize, col: usize, effect: &mut Effect) {
        if row == col || row > 4 || col > 4 {
            return;
        }
        let now = Instant::now();
        let double = self.last_matrix.is_some_and(|(prev_row, prev_col, at)| {
            prev_row == row && prev_col == col && now.duration_since(at) <= DOUBLE_CLICK
        });
        self.last_matrix = Some((row, col, now));
        self.app.palette.set_matrix(row, col);
        self.app.set_focus(FocusId::Matrix);
        if double {
            self.app.open_fix_for_matrix();
            effect.sync_preview = self.app.fix_open;
        }
    }

    fn nudge_live(&mut self, axis: FixAxis, delta: f32) {
        match axis {
            FixAxis::Fg => {
                self.app.foreground = self.app.foreground.nudge_oklab_l(delta);
                self.app.foreground_input = self.app.foreground.to_hex();
                if self.app.focus == FocusId::FgHex {
                    self.app.current_input = self.app.foreground_input.clone();
                    self.app.cursor_char_idx = self.app.current_input.chars().count();
                }
            }
            FixAxis::Bg => {
                self.app.background = self.app.background.nudge_oklab_l(delta);
                self.app.background_input = self.app.background.to_hex();
                if self.app.focus == FocusId::BgHex {
                    self.app.current_input = self.app.background_input.clone();
                    self.app.cursor_char_idx = self.app.current_input.chars().count();
                }
            }
        }
        self.app.update_contrast();
    }

    fn nudge_axis(&self) -> FixAxis {
        match self.app.focus {
            FocusId::NudgeBg | FocusId::BgHex => FixAxis::Bg,
            _ => FixAxis::Fg,
        }
    }

    fn typing(&self) -> bool {
        self.app.editing || self.app.palette.editing
    }

    fn color_nudge_focus(&self) -> bool {
        matches!(
            self.app.focus,
            FocusId::FgHex | FocusId::BgHex | FocusId::NudgeFg | FocusId::NudgeBg
        )
    }

    fn fix_nudge_focus(&self) -> bool {
        matches!(self.app.focus, FocusId::NudgeFg | FocusId::NudgeBg)
    }

    fn fix_focus(&self) -> bool {
        matches!(
            self.app.focus,
            FocusId::NudgeFg
                | FocusId::NudgeBg
                | FocusId::SendFg
                | FocusId::SendBg
                | FocusId::ApplyFix
                | FocusId::NextFix
                | FocusId::CloseFix
        )
    }
}

fn shown_color(app: &App, focus: FocusId, stored: &str) -> String {
    if app.focus == focus {
        app.current_input.clone()
    } else {
        stored.to_string()
    }
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Contrast => "Contrast",
        Mode::Palette => "Palette",
    }
}

fn role_from_key(c: char) -> Option<PaletteInput> {
    match c {
        'p' | 'P' => Some(PaletteInput::Primary),
        's' | 'S' => Some(PaletteInput::Secondary),
        't' | 'T' => Some(PaletteInput::Tertiary),
        'u' | 'U' => Some(PaletteInput::Support),
        _ => None,
    }
}

fn is_char(key: &str) -> bool {
    key.chars().count() == 1 && !key.chars().any(|ch| ch.is_control())
}
