//! Contrast window. One shell owns the checker, the theme, and the Settings sheet.

#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

use dd_wcag::cli::LaunchSeed;
use dd_wcag::color::Color;
use gpui::{
    App, ClickEvent, Context, FocusHandle, Focusable, FontWeight, IntoElement, KeyDownEvent,
    ParentElement, Render, ScrollHandle, StatefulInteractiveElement, Styled, Window, div,
    prelude::*, px, relative,
};
use ldnddev_theme::{
    ColorField, EditorKey, EditorOutcome, PickerOutcome, PickerRow, ThemeEditor, ThemePicker,
    ThemeSaveTarget, COLOR_FIELDS,
};

use crate::commands::{is_quit, is_settings};
use crate::paint::{self, Packed};
use crate::settings::{self, Density, GuiSettings};
use crate::theme_load::{self, LoadedTheme};

const PAGES: [&str; 3] = ["Themes", "Window", "Help"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Fg,
    Bg,
    Size,
    Weight,
}

struct Appearance {
    picker: ThemePicker,
    editor: Option<ThemeEditor>,
    direct_editor: bool,
    hex_error: Option<String>,
}

pub struct ShellOptions {
    pub settings_file: PathBuf,
    pub omarchy_paths: ldnddev_theme::OmarchyPaths,
    pub project_root: PathBuf,
    /// `None` uses the real XDG config home. Tests pass a temp directory.
    pub config_home: Option<PathBuf>,
    pub quit_runtime: bool,
    pub seed: Option<LaunchSeed>,
}

impl ShellOptions {
    pub fn production(seed: LaunchSeed) -> Self {
        Self {
            settings_file: settings::settings_path(),
            omarchy_paths: ldnddev_theme::OmarchyPaths::system(),
            project_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            config_home: None,
            quit_runtime: true,
            seed: Some(seed),
        }
    }

    #[cfg(test)]
    pub fn test(dir: &Path) -> Self {
        Self {
            settings_file: dir.join("gui.json"),
            omarchy_paths: ldnddev_theme::OmarchyPaths::from_root(&dir.join("omarchy")),
            project_root: dir.to_path_buf(),
            config_home: Some(dir.to_path_buf()),
            quit_runtime: false,
            seed: None,
        }
    }
}

pub struct Shell {
    focus: FocusHandle,
    pub quit: bool,
    quit_runtime: bool,
    settings_file: PathBuf,
    project_root: PathBuf,
    config_home: Option<PathBuf>,
    omarchy_paths: ldnddev_theme::OmarchyPaths,
    gui: GuiSettings,
    palette: ldnddev_theme::Palette,
    theme_label: String,
    theme_warning: Option<String>,
    /// Palette from the moment Settings opened. Close and Esc restore this.
    sheet_snapshot: Option<ldnddev_theme::Palette>,
    appearance: Option<Appearance>,
    nav: u8,
    row: usize,
    query: String,
    query_on: bool,
    settings_scroll: ScrollHandle,
    field: Field,
    fg: Color,
    bg: Color,
    fg_text: String,
    bg_text: String,
    size: u16,
    weight: u16,
    size_text: String,
    weight_text: String,
    status: String,
}

impl Shell {
    pub fn open(options: ShellOptions, cx: &mut Context<Self>) -> Self {
        let loaded = theme_load::load(
            &options.project_root,
            &options.omarchy_paths,
            options.config_home.as_deref(),
        );
        let gui = settings::load_from(&options.settings_file);
        let fg = Color::parse_hex("#000000").expect("black");
        let bg = Color::parse_hex("#ffffff").expect("white");
        let mut shell = Self {
            focus: cx.focus_handle(),
            quit: false,
            quit_runtime: options.quit_runtime,
            settings_file: options.settings_file,
            project_root: options.project_root,
            config_home: options.config_home,
            omarchy_paths: options.omarchy_paths,
            gui,
            palette: loaded.palette,
            theme_label: loaded.label,
            theme_warning: loaded.warning,
            sheet_snapshot: None,
            appearance: None,
            nav: 0,
            row: 0,
            query: String::new(),
            query_on: false,
            settings_scroll: ScrollHandle::new(),
            field: Field::Fg,
            fg,
            bg,
            fg_text: fg.to_hex(),
            bg_text: bg.to_hex(),
            size: 16,
            weight: 400,
            size_text: "16".to_string(),
            weight_text: "400".to_string(),
            status: String::new(),
        };
        if let Some(seed) = options.seed {
            shell.apply_seed(&seed);
        }
        shell
    }

    #[cfg(test)]
    pub fn palette_stamp(&self) -> String {
        self.palette
            .colors()
            .iter()
            .map(|(key, color)| format!("{key}={}", color.to_hex()))
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn handle_key(&mut self, key: &str) {
        let key = key.to_ascii_lowercase();
        if is_quit(&key) {
            self.quit = true;
            return;
        }
        if self.appearance.is_some() {
            self.settings_key(&key);
            return;
        }
        if is_settings(&key) {
            self.open_settings();
            return;
        }
        self.body_key(&key);
    }

    pub fn set_density(&mut self, density: Density) {
        self.gui.density = density;
        let _ = settings::save_to(&self.settings_file, &self.gui);
    }

    pub fn set_vim_keys(&mut self, on: bool) {
        self.gui.vim_keys = on;
        let _ = settings::save_to(&self.settings_file, &self.gui);
    }

    #[cfg(test)]
    pub fn visible_text(&self) -> String {
        let mut lines = Vec::new();
        lines.push("dd_wcag".to_string());
        lines.extend(self.body_lines());
        lines.push(format!("footer:{}", self.footer_text()));
        if self.appearance.is_some() {
            lines.push("settings-panel:right:70".to_string());
            lines.push("settings-nav:Themes,Window,Help".to_string());
            let page = if !self.query.trim().is_empty() {
                "Results"
            } else {
                PAGES.get(self.nav as usize).copied().unwrap_or("Themes")
            };
            lines.push(format!("settings-page:{page}"));
            lines.push("Settings".to_string());
            for (index, label) in PAGES.iter().enumerate() {
                let mark = if self.query.trim().is_empty() && self.nav as usize == index {
                    ">"
                } else {
                    " "
                };
                lines.push(format!("{mark} {label}"));
            }
            if !self.query.trim().is_empty() {
                lines.push("Results".to_string());
                lines.extend(self.result_labels());
            } else if self.nav == 1 {
                lines.push(format!("Density {}", self.gui.density.label()));
                lines.push(format!(
                    "Vim keys {}",
                    if self.gui.vim_keys { "on" } else { "off" }
                ));
                lines.push("Comfortable".to_string());
                lines.push("Compact".to_string());
            } else if self.nav == 2 {
                lines.extend(crate::shortcuts::lines());
            } else if let Some(appearance) = &self.appearance {
                lines.push(format!("Source: {}", self.theme_label));
                let status = self
                    .theme_warning
                    .clone()
                    .unwrap_or_else(|| "OK (loaded cleanly)".to_string());
                lines.push(format!("Status: {status}"));
                lines.push(format!(
                    "Save to {}",
                    target_word(appearance.picker.save_target)
                ));
                if let Some(editor) = &appearance.editor {
                    lines.push("Theme color editor".to_string());
                    for (index, field) in editor.fields.iter().enumerate() {
                        let hex = editor
                            .palette
                            .get(field.key)
                            .map(|color| color.to_hex())
                            .unwrap_or_else(|| "#000000".to_string());
                        let mark = if index == editor.selected { ">" } else { " " };
                        lines.push(format!("{mark} {} {hex}", field.key));
                    }
                    if let Some(err) = &appearance.hex_error {
                        lines.push(err.clone());
                    }
                } else {
                    lines.push("theme-list:flow".to_string());
                    for (index, row) in appearance.picker.rows.iter().enumerate() {
                        let mark = if index == appearance.picker.selected {
                            ">"
                        } else {
                            " "
                        };
                        lines.push(format!("{mark} {}", theme_row_label(row)));
                    }
                    lines.push("Edit colors…".to_string());
                }
                lines.push("Save".to_string());
            }
        }
        if !self.status.is_empty() {
            lines.push(self.status.clone());
        }
        lines.join("\n")
    }

    #[cfg(test)]
    fn body_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("Foreground {}", self.fg_text),
            format!("Background {}", self.bg_text),
            format!("Size {}", self.size_text),
            format!("Weight {}", self.weight_text),
        ];
        lines.push(dd_wcag::contrast::contrast_body_text(
            &self.fg,
            &self.bg,
            self.size,
            self.weight,
        ));
        lines
    }

    fn footer_text(&self) -> String {
        if self.appearance.is_some() {
            "F1: Settings  Esc: Close  Ctrl+Q: Quit".to_string()
        } else {
            "F1: Settings  Ctrl+Q: Quit".to_string()
        }
    }

    fn row_h(&self) -> f32 {
        match self.gui.density {
            Density::Comfortable => 28.0,
            Density::Compact => 24.0,
        }
    }

    fn apply_seed(&mut self, seed: &LaunchSeed) {
        self.size = seed.size_px;
        self.weight = seed.weight;
        self.size_text = self.size.to_string();
        self.weight_text = self.weight.to_string();
        if let Some(color) = &seed.fg {
            self.fg = color.color;
            self.fg_text = color.text.clone();
        }
        if let Some(color) = &seed.bg {
            self.bg = color.color;
            self.bg_text = color.text.clone();
        }
    }

    fn body_key(&mut self, key: &str) {
        match key {
            "tab" => self.cycle_field(1),
            "shift-tab" | "backtab" => self.cycle_field(-1),
            "backspace" => self.backspace_field(),
            "up" => self.step_field(true),
            "down" => self.step_field(false),
            other if is_char(other) => self.type_field(other.chars().next().unwrap_or('?')),
            _ => {}
        }
    }

    fn cycle_field(&mut self, dir: i32) {
        let order = [Field::Fg, Field::Bg, Field::Size, Field::Weight];
        let index = order
            .iter()
            .position(|field| *field == self.field)
            .unwrap_or(0) as i32;
        let next = (index + dir).rem_euclid(order.len() as i32) as usize;
        self.field = order[next];
    }

    fn backspace_field(&mut self) {
        match self.field {
            Field::Fg => {
                self.fg_text.pop();
                self.apply_hex_draft();
            }
            Field::Bg => {
                self.bg_text.pop();
                self.apply_hex_draft();
            }
            Field::Size => {
                self.size_text.pop();
                self.apply_metric_draft();
            }
            Field::Weight => {
                self.weight_text.pop();
                self.apply_metric_draft();
            }
        }
    }

    fn step_field(&mut self, up: bool) {
        let sign: i16 = if up { 1 } else { -1 };
        match self.field {
            Field::Size => {
                self.size = (self.size as i16 + sign).clamp(6, 120) as u16;
                self.size_text = self.size.to_string();
            }
            Field::Weight => {
                self.weight = ((self.weight as i16 + sign * 100) / 100 * 100).clamp(100, 900) as u16;
                self.weight_text = self.weight.to_string();
            }
            _ => {}
        }
    }

    fn type_field(&mut self, ch: char) {
        match self.field {
            Field::Fg | Field::Bg => {
                if ch == '#' || ch.is_ascii_hexdigit() {
                    let text = match self.field {
                        Field::Fg => &mut self.fg_text,
                        _ => &mut self.bg_text,
                    };
                    if text.len() < 7 {
                        text.push(ch.to_ascii_lowercase());
                    }
                    self.apply_hex_draft();
                }
            }
            Field::Size | Field::Weight => {
                if ch.is_ascii_digit() {
                    let text = match self.field {
                        Field::Size => &mut self.size_text,
                        _ => &mut self.weight_text,
                    };
                    if text.len() < 3 {
                        text.push(ch);
                    }
                    self.apply_metric_draft();
                }
            }
        }
    }

    fn apply_hex_draft(&mut self) {
        let text = match self.field {
            Field::Fg => self.fg_text.clone(),
            Field::Bg => self.bg_text.clone(),
            _ => return,
        };
        let parsed = Color::parse_input(&text)
            .map(|(color, _)| color)
            .or_else(|_| Color::parse_hex(&text).map_err(|err| err.to_string()));
        if let Ok(color) = parsed {
            match self.field {
                Field::Fg => self.fg = color,
                Field::Bg => self.bg = color,
                _ => {}
            }
        }
    }

    fn apply_metric_draft(&mut self) {
        match self.field {
            Field::Size => {
                if let Ok(value) = self.size_text.parse::<u16>() {
                    self.size = value.clamp(6, 120);
                }
            }
            Field::Weight => {
                if let Ok(value) = self.weight_text.parse::<u16>() {
                    let stepped = (value / 100) * 100;
                    self.weight = stepped.clamp(100, 900);
                }
            }
            _ => {}
        }
    }

    fn open_settings(&mut self) {
        if self.appearance.is_some() {
            self.close_settings();
            return;
        }
        let snapshot = self.palette.clone();
        let fields = theme_fields();
        let picker = ThemePicker::new(
            self.palette.clone(),
            &fields,
            self.omarchy_paths.clone(),
        );
        let direct = !picker.lists_omarchy();
        let mut appearance = Appearance {
            picker,
            editor: None,
            direct_editor: direct,
            hex_error: None,
        };
        if direct {
            let palette = appearance.picker.palette.clone();
            let target = appearance.picker.save_target;
            let mut editor = ThemeEditor::new(palette, &fields);
            editor.save_target = target;
            appearance.editor = Some(editor);
        }
        self.sheet_snapshot = Some(snapshot);
        self.nav = 0;
        self.row = 0;
        self.query.clear();
        self.query_on = false;
        self.appearance = Some(appearance);
    }

    fn close_settings(&mut self) {
        if let Some(snapshot) = self.sheet_snapshot.take() {
            self.palette = snapshot;
        }
        self.appearance = None;
        self.query.clear();
        self.query_on = false;
        self.nav = 0;
        self.row = 0;
    }

    fn settings_key(&mut self, key: &str) {
        if is_settings(key) {
            self.close_settings();
            return;
        }
        if !self.query.trim().is_empty() && key == "escape" {
            self.query.clear();
            self.query_on = false;
            self.row = 0;
            return;
        }
        if self.query_on {
            self.query_key(key);
            return;
        }
        let editing = self.editing_hex();
        if key == "/" && !editing {
            self.query_on = true;
            self.query.clear();
            self.row = 0;
            return;
        }
        if let Some(dir) = section_delta(key, editing) {
            self.jump_page(dir);
            return;
        }
        if self.nav == 0 && key == "t" && !editing {
            self.toggle_target();
            return;
        }
        if self.nav == 0 && matches!(key, "y" | "s") && !editing {
            self.save_theme();
            return;
        }
        if self.nav == 0 && self.editor_open() {
            self.editor_key(key);
            return;
        }
        if self.nav == 0 {
            self.list_key(key);
            return;
        }
        if self.nav == 1 {
            self.window_key(key);
            return;
        }
        if key == "escape" {
            self.close_settings();
        }
    }

    fn query_key(&mut self, key: &str) {
        match key {
            "escape" => {
                self.query_on = false;
                self.query.clear();
                self.row = 0;
            }
            "backspace" => {
                self.query.pop();
                self.row = 0;
            }
            "enter" => self.activate_result(),
            "up" | "k" => self.row = self.row.saturating_sub(1),
            "down" | "j" => {
                let count = self.result_labels().len();
                if count > 0 {
                    self.row = (self.row + 1).min(count - 1);
                }
            }
            other if is_char(other) => {
                self.query.push_str(other);
                self.row = 0;
            }
            _ => {}
        }
    }

    fn list_key(&mut self, key: &str) {
        let vim = self.gui.vim_keys;
        match key {
            "escape" => self.close_settings(),
            "enter" => self.apply_selected(),
            "e" => self.open_color_editor(),
            "up" => self.move_theme(-1),
            "down" => self.move_theme(1),
            "pageup" => self.move_theme(-8),
            "pagedown" => self.move_theme(8),
            "k" if vim => self.move_theme(-1),
            "j" if vim => self.move_theme(1),
            "g" if vim => self.move_theme_edge(true),
            "shift-g" if vim => self.move_theme_edge(false),
            _ => {}
        }
    }

    fn window_key(&mut self, key: &str) {
        let vim = self.gui.vim_keys;
        let last = 1usize;
        match key {
            "escape" => self.close_settings(),
            "up" => self.row = self.row.saturating_sub(1),
            "down" => self.row = (self.row + 1).min(last),
            "k" if vim => self.row = self.row.saturating_sub(1),
            "j" if vim => self.row = (self.row + 1).min(last),
            "enter" => self.activate_window_row(),
            _ => {}
        }
    }

    fn jump_page(&mut self, dir: i32) {
        let next = (self.nav as i32 + dir).clamp(0, (PAGES.len() - 1) as i32) as u8;
        if next != self.nav {
            self.nav = next;
            self.row = 0;
        }
    }

    fn editing_hex(&self) -> bool {
        self.appearance
            .as_ref()
            .and_then(|appearance| appearance.editor.as_ref())
            .is_some_and(|editor| editor.editing_hex)
    }

    fn editor_open(&self) -> bool {
        self.appearance
            .as_ref()
            .is_some_and(|appearance| appearance.editor.is_some())
    }

    fn toggle_target(&mut self) {
        let Some(appearance) = &mut self.appearance else {
            return;
        };
        appearance.picker.save_target = appearance.picker.save_target.toggled();
        let target = appearance.picker.save_target;
        if let Some(editor) = &mut appearance.editor {
            editor.save_target = target;
        }
    }

    fn save_theme(&mut self) {
        let Some(appearance) = &self.appearance else {
            return;
        };
        let palette = appearance
            .editor
            .as_ref()
            .map(|editor| editor.palette.clone())
            .unwrap_or_else(|| appearance.picker.palette.clone());
        let target = appearance
            .editor
            .as_ref()
            .map(|editor| editor.save_target)
            .unwrap_or(appearance.picker.save_target);
        let fields = appearance
            .editor
            .as_ref()
            .map(|editor| editor.fields.clone())
            .unwrap_or_else(theme_fields);
        let default_home = ldnddev_theme::default_config_home();
        let home = self.config_home.as_deref().or(default_home.as_deref());
        match ldnddev_theme::save_theme(
            &palette,
            &self.project_root,
            theme_load::THEME_FILE,
            target,
            home,
            &fields,
        ) {
            Ok(path) => self.status = format!("Theme saved ({})", path.display()),
            Err(err) => self.status = format!("Theme save failed: {err}"),
        }
    }

    fn move_theme(&mut self, delta: i32) {
        let palette = {
            let Some(appearance) = &mut self.appearance else {
                return;
            };
            if appearance.editor.is_some() {
                return;
            }
            let len = appearance.picker.rows.len() as i32;
            if len == 0 {
                return;
            }
            let next = (appearance.picker.selected as i32 + delta).clamp(0, len - 1) as usize;
            appearance.picker.select(next);
            let _ = appearance.picker.preview_selected();
            appearance.picker.palette.clone()
        };
        self.palette = palette;
    }

    fn move_theme_edge(&mut self, first: bool) {
        let palette = {
            let Some(appearance) = &mut self.appearance else {
                return;
            };
            let last = appearance.picker.rows.len().saturating_sub(1);
            appearance.picker.select(if first { 0 } else { last });
            let _ = appearance.picker.preview_selected();
            appearance.picker.palette.clone()
        };
        self.palette = palette;
    }

    fn apply_selected(&mut self) {
        let outcome = {
            let Some(appearance) = &mut self.appearance else {
                return;
            };
            appearance.picker.handle(EditorKey::Enter)
        };
        if let PickerOutcome::Error(message) = outcome {
            self.status = message;
            return;
        }
        self.install_picker_palette();
    }

    fn install_picker_palette(&mut self) {
        if let Some(appearance) = &self.appearance {
            self.palette = appearance.picker.palette.clone();
        }
    }

    fn choose_theme(&mut self, index: usize) {
        if self
            .appearance
            .as_ref()
            .is_some_and(|appearance| appearance.editor.is_some())
        {
            return;
        }
        if let Some(appearance) = &mut self.appearance {
            appearance.picker.select(index);
            let _ = appearance.picker.preview_selected();
        }
        self.install_picker_palette();
    }

    fn open_color_editor(&mut self) {
        let fields = theme_fields();
        let Some(appearance) = &mut self.appearance else {
            return;
        };
        if appearance.editor.is_some() {
            return;
        }
        let palette = appearance.picker.palette.clone();
        let target = appearance.picker.save_target;
        let mut editor = ThemeEditor::new(palette, &fields);
        editor.save_target = target;
        appearance.editor = Some(editor);
        appearance.hex_error = None;
    }

    fn editor_key(&mut self, key: &str) {
        let vim = self.gui.vim_keys;
        let editing = self.editing_hex();
        let shift = key.starts_with("shift-");
        let Some(editor_key) = editor_key_from(key, vim, editing) else {
            return;
        };
        let outcome = {
            let Some(editor) = self
                .appearance
                .as_mut()
                .and_then(|appearance| appearance.editor.as_mut())
            else {
                return;
            };
            editor.handle(editor_key, shift)
        };
        match outcome {
            EditorOutcome::RequestSave => self.save_theme(),
            EditorOutcome::Closed { .. } => self.step_back_editor(),
            EditorOutcome::HexError(message) => {
                if let Some(appearance) = &mut self.appearance {
                    appearance.hex_error = Some(message);
                }
            }
            EditorOutcome::PaletteChanged => self.sync_editor_palette(),
            EditorOutcome::None => {
                if let Some(appearance) = &mut self.appearance
                    && appearance
                        .editor
                        .as_ref()
                        .is_some_and(|editor| editor.editing_hex)
                {
                    appearance.hex_error = None;
                }
            }
        }
    }

    fn sync_editor_palette(&mut self) {
        let palette = {
            let Some(appearance) = &mut self.appearance else {
                return;
            };
            let Some(editor) = &appearance.editor else {
                return;
            };
            let palette = editor.palette.clone();
            appearance.picker.palette = palette.clone();
            appearance.hex_error = None;
            palette
        };
        self.palette = palette;
    }

    fn step_back_editor(&mut self) {
        let direct = self
            .appearance
            .as_ref()
            .is_some_and(|appearance| appearance.direct_editor);
        let palette = {
            let Some(appearance) = &mut self.appearance else {
                return;
            };
            let palette = appearance.editor.as_ref().map(|editor| editor.palette.clone());
            appearance.editor = None;
            appearance.hex_error = None;
            if let Some(palette) = palette.clone() {
                appearance.picker.palette = palette;
            }
            palette
        };
        if direct {
            self.close_settings();
            return;
        }
        if let Some(palette) = palette {
            self.palette = palette;
        }
    }

    fn activate_window_row(&mut self) {
        if self.row == 0 {
            let next = match self.gui.density {
                Density::Comfortable => Density::Compact,
                Density::Compact => Density::Comfortable,
            };
            self.set_density(next);
        } else {
            self.set_vim_keys(!self.gui.vim_keys);
        }
    }

    fn result_labels(&self) -> Vec<String> {
        let query = self.query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        let mut hits = Vec::new();
        if let Some(appearance) = &self.appearance {
            for row in &appearance.picker.rows {
                let label = theme_row_label(row);
                if label.to_ascii_lowercase().contains(&query) {
                    hits.push(label);
                }
            }
        }
        for label in ["Density", "Vim keys", "Comfortable", "Compact"] {
            if label.to_ascii_lowercase().contains(&query) {
                hits.push(label.to_string());
            }
        }
        for row in crate::shortcuts::rows() {
            if row.action.to_ascii_lowercase().contains(&query) {
                hits.push(row.action.to_string());
            }
        }
        hits
    }

    fn activate_result(&mut self) {
        let Some(label) = self.result_labels().get(self.row).cloned() else {
            return;
        };
        self.query.clear();
        self.query_on = false;
        if label == "Density" || label == "Comfortable" || label == "Compact" {
            self.nav = 1;
            self.row = 0;
            if label == "Compact" {
                self.set_density(Density::Compact);
            } else if label == "Comfortable" {
                self.set_density(Density::Comfortable);
            }
            return;
        }
        if label == "Vim keys" {
            self.nav = 1;
            self.row = 1;
            self.activate_window_row();
            return;
        }
        if let Some(index) = self.appearance.as_ref().and_then(|appearance| {
            appearance
                .picker
                .rows
                .iter()
                .position(|row| theme_row_label(row) == label)
        }) {
            self.nav = 0;
            self.choose_theme(index);
            return;
        }
        self.nav = 2;
        self.row = 0;
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, _cx: &mut Context<Self>) {
        self.handle_key(&chord(event));
    }
}

impl Focusable for Shell {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.quit && self.quit_runtime {
            cx.quit();
        }
        if !self.focus.is_focused(window) {
            self.focus.focus(window, cx);
        }
        cx.set_reduce_motion(self.gui.reduced_motion);
        let pack = paint::pack(&self.palette);
        let bar = self.row_h();
        let footer = self.footer_text();
        let summary = dd_wcag::contrast::contrast_body_text(&self.fg, &self.bg, self.size, self.weight);
        let fg_text = self.fg_text.clone();
        let bg_text = self.bg_text.clone();
        let size_text = self.size_text.clone();
        let weight_text = self.weight_text.clone();
        let fields = CheckerFields {
            focus: self.field,
            fg_text: &fg_text,
            bg_text: &bg_text,
            size_text: &size_text,
            weight_text: &weight_text,
            fg_swatch: paint::hsla_bytes(self.fg.rgb_u8()),
            bg_swatch: paint::hsla_bytes(self.bg.rgb_u8()),
        };
        let open = self.appearance.is_some();
        let warning = self.theme_warning.clone();
        let body = body_column(cx, bar, pack, &fields, &summary);
        let panel = if open {
            Some(settings_panel(cx, self, pack, bar))
        } else {
            None
        };

        div()
            .id("shell")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|shell, event: &KeyDownEvent, _, cx| {
                shell.on_key_down(event, cx);
                cx.notify();
            }))
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(pack.base)
            .text_color(pack.text)
            .text_size(px(13.0))
            .child(header_bar(bar, pack))
            .child(body)
            .child(footer_bar(bar, pack, footer))
            .when_some(warning, |row, warning| {
                row.child(
                    div()
                        .px(px(12.0))
                        .text_color(pack.secondary)
                        .child(warning),
                )
            })
            .when_some(panel, |row, panel| row.child(panel))
    }
}

fn header_bar(bar: f32, pack: Packed) -> gpui::Div {
    div()
        .h(px(bar))
        .px(px(12.0))
        .flex()
        .items_center()
        .bg(pack.raised)
        .text_color(pack.text)
        .font_weight(FontWeight::BOLD)
        .border_b_1()
        .border_color(pack.border)
        .child("dd_wcag")
}

fn footer_bar(bar: f32, pack: Packed, footer: String) -> gpui::Div {
    div()
        .h(px(bar))
        .px(px(12.0))
        .flex()
        .items_center()
        .bg(pack.raised)
        .text_color(pack.secondary)
        .border_t_1()
        .border_color(pack.border)
        .child(footer)
}

fn summary_color(line: &str, pack: Packed) -> gpui::Hsla {
    if line.contains('✗') || line.contains("fail") {
        pack.error
    } else if line.contains('✓') || line.contains("pass") {
        pack.success
    } else {
        pack.text
    }
}

struct CheckerFields<'a> {
    focus: Field,
    fg_text: &'a str,
    bg_text: &'a str,
    size_text: &'a str,
    weight_text: &'a str,
    fg_swatch: gpui::Hsla,
    bg_swatch: gpui::Hsla,
}

struct HexRow<'a> {
    id: &'static str,
    label: &'a str,
    value: &'a str,
    swatch: gpui::Hsla,
    on: bool,
    field: Field,
}

fn body_column(
    cx: &mut Context<Shell>,
    bar: f32,
    pack: Packed,
    fields: &CheckerFields<'_>,
    summary: &str,
) -> gpui::Div {
    div()
        .flex_1()
        .min_h(px(0.0))
        .flex()
        .flex_col()
        .p(px(12.0))
        .gap(px(8.0))
        .child(hex_row(
            cx,
            pack,
            bar,
            &HexRow {
                id: "fg-hex",
                label: "Foreground",
                value: fields.fg_text,
                swatch: fields.fg_swatch,
                on: fields.focus == Field::Fg,
                field: Field::Fg,
            },
        ))
        .child(hex_row(
            cx,
            pack,
            bar,
            &HexRow {
                id: "bg-hex",
                label: "Background",
                value: fields.bg_text,
                swatch: fields.bg_swatch,
                on: fields.focus == Field::Bg,
                field: Field::Bg,
            },
        ))
        .child(metric_row(
            cx,
            "size-field",
            "Size",
            fields.size_text,
            fields.focus == Field::Size,
            pack,
            Field::Size,
        ))
        .child(metric_row(
            cx,
            "weight-field",
            "Weight",
            fields.weight_text,
            fields.focus == Field::Weight,
            pack,
            Field::Weight,
        ))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .children(summary.lines().map(|line| {
                    div()
                        .text_color(summary_color(line, pack))
                        .child(line.to_string())
                })),
        )
}

fn hex_row(
    cx: &mut Context<Shell>,
    pack: Packed,
    bar: f32,
    row: &HexRow<'_>,
) -> gpui::Stateful<gpui::Div> {
    let field = row.field;
    div()
        .id(row.id)
        .h(px(36.0))
        .px(px(8.0))
        .flex()
        .items_center()
        .gap(px(8.0))
        .border_1()
        .border_color(if row.on { pack.accent } else { pack.border })
        .bg(pack.panel)
        .cursor_pointer()
        .on_click(cx.listener(move |shell, _: &ClickEvent, _, cx| {
            shell.field = field;
            cx.stop_propagation();
            cx.notify();
        }))
        .child(
            div()
                .w(px(110.0))
                .text_color(pack.secondary)
                .child(row.label.to_string()),
        )
        .child(div().flex_1().child(row.value.to_string()))
        .child(div().w(px(bar)).h(px(bar)).bg(row.swatch))
}

fn metric_row(
    cx: &mut Context<Shell>,
    id: &'static str,
    label: &str,
    value: &str,
    on: bool,
    pack: Packed,
    field: Field,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .h(px(36.0))
        .px(px(8.0))
        .flex()
        .items_center()
        .gap(px(8.0))
        .border_1()
        .border_color(if on { pack.accent } else { pack.border })
        .bg(pack.panel)
        .cursor_pointer()
        .on_click(cx.listener(move |shell, _: &ClickEvent, _, cx| {
            shell.field = field;
            cx.stop_propagation();
            cx.notify();
        }))
        .child(div().w(px(110.0)).text_color(pack.secondary).child(label.to_string()))
        .child(value.to_string())
}

fn settings_panel(
    cx: &mut Context<Shell>,
    shell: &Shell,
    pack: Packed,
    bar: f32,
) -> gpui::Stateful<gpui::Div> {
    let page = if !shell.query.trim().is_empty() {
        "Results".to_string()
    } else {
        PAGES
            .get(shell.nav as usize)
            .copied()
            .unwrap_or("Themes")
            .to_string()
    };
    div()
        .id("settings-panel")
        .occlude()
        .absolute()
        .top(px(bar))
        .right(px(0.0))
        .bottom(px(bar))
        .w(relative(0.7))
        .flex()
        .flex_col()
        .overflow_hidden()
        .bg(pack.panel)
        .text_color(pack.text)
        .border_l_1()
        .border_color(pack.border)
        .on_click(cx.listener(|_shell, _: &ClickEvent, _, cx| {
            cx.stop_propagation();
        }))
        .child(settings_header(cx, pack))
        .child(
            div()
                .flex_1()
                .min_h(px(0.0))
                .flex()
                .flex_row()
                .overflow_hidden()
                .child(settings_nav(cx, shell, pack))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .h_full()
                        .flex()
                        .flex_col()
                        .overflow_hidden()
                        .child(
                            div()
                                .h(px(28.0))
                                .px(px(8.0))
                                .flex()
                                .items_center()
                                .font_weight(FontWeight::BOLD)
                                .bg(pack.raised)
                                .border_b_1()
                                .border_color(pack.border)
                                .child(page),
                        )
                        .child(settings_scroll(cx, shell, pack)),
                ),
        )
}

fn settings_header(cx: &mut Context<Shell>, pack: Packed) -> gpui::Div {
    div()
        .flex_shrink_0()
        .h(px(48.0))
        .px(px(16.0))
        .flex()
        .items_center()
        .gap(px(12.0))
        .border_b_1()
        .border_color(pack.border)
        .child("Settings")
        .child(div().flex_1())
        .child(div().text_color(pack.secondary).child("F1"))
        .child(
            div()
                .id("settings-close")
                .px(px(8.0))
                .cursor_pointer()
                .child("Close")
                .on_click(cx.listener(|shell, _: &ClickEvent, _, cx| {
                    shell.close_settings();
                    cx.stop_propagation();
                    cx.notify();
                })),
        )
}

fn settings_nav(cx: &mut Context<Shell>, shell: &Shell, pack: Packed) -> gpui::Div {
    let query = if shell.query.is_empty() {
        "Search".to_string()
    } else {
        shell.query.clone()
    };
    let mut column = div()
        .w(px(200.0))
        .h_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .border_r_1()
        .border_color(pack.border)
        .child(
            div()
                .id("settings-search")
                .h(px(36.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .cursor_pointer()
                .text_color(if shell.query_on {
                    pack.text
                } else {
                    pack.secondary
                })
                .on_click(cx.listener(|shell, _: &ClickEvent, _, cx| {
                    shell.query_on = true;
                    cx.stop_propagation();
                    cx.notify();
                }))
                .child(query),
        );
    for (index, label) in PAGES.iter().enumerate() {
        let on = shell.query.trim().is_empty() && shell.nav as usize == index;
        let mark = if on { ">" } else { " " };
        column = column.child(
            div()
                .id(("settings-nav", index))
                .h(px(32.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .overflow_hidden()
                .whitespace_nowrap()
                .cursor_pointer()
                .when(on, |row| row.bg(pack.selection).text_color(pack.text))
                .on_click(cx.listener(move |shell, _: &ClickEvent, _, cx| {
                    shell.nav = index as u8;
                    shell.row = 0;
                    shell.query.clear();
                    shell.query_on = false;
                    cx.stop_propagation();
                    cx.notify();
                }))
                .child(format!("{mark} {label}")),
        );
    }
    column
}

fn settings_scroll(
    cx: &mut Context<Shell>,
    shell: &Shell,
    pack: Packed,
) -> gpui::Stateful<gpui::Div> {
    let mut body = div()
        .id("settings-scroll")
        .track_scroll(&shell.settings_scroll)
        .flex_1()
        .min_h(px(0.0))
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .p(px(8.0));
    if !shell.query.trim().is_empty() {
        for (index, label) in shell.result_labels().into_iter().enumerate() {
            let on = index == shell.row;
            body = body.child(
                div()
                    .id(("settings-result", index))
                    .h(px(28.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .overflow_hidden()
                    .when(on, |row| row.bg(pack.selection))
                    .child(label),
            );
        }
        return body;
    }
    if shell.nav == 1 {
        return body
            .child(outlined(
                "Density",
                pack,
                div()
                    .flex()
                    .flex_col()
                    .child(window_choice(cx, "density-comfortable", "Comfortable", shell.row == 0 && shell.gui.density == Density::Comfortable, pack, 0))
                    .child(window_choice(cx, "density-compact", "Compact", shell.row == 0 && shell.gui.density == Density::Compact, pack, 0)),
            ))
            .child(div().h(px(12.0)))
            .child(outlined(
                "Keys",
                pack,
                div().flex().flex_col().child(window_choice(
                    cx,
                    "vim-keys",
                    if shell.gui.vim_keys {
                        "Vim keys on"
                    } else {
                        "Vim keys off"
                    },
                    shell.row == 1,
                    pack,
                    1,
                )),
            ));
    }
    if shell.nav == 2 {
        return body.child(outlined(
            "Keyboard shortcuts",
            pack,
            div().flex().flex_col().children(
                crate::shortcuts::lines()
                    .into_iter()
                    .enumerate()
                    .map(|(index, line)| {
                        div()
                            .id(("shortcut", index))
                            .h(px(24.0))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(line)
                    }),
            ),
        ));
    }
    let Some(appearance) = &shell.appearance else {
        return body;
    };
    if let Some(editor) = &appearance.editor {
        body = body.child(div().child("Theme color editor"));
        for (index, field) in editor.fields.iter().enumerate() {
            let hex = editor
                .palette
                .get(field.key)
                .map(|color| color.to_hex())
                .unwrap_or_else(|| "#000000".to_string());
            let on = index == editor.selected;
            let mark = if on { ">" } else { " " };
            body = body.child(
                div()
                    .id(("theme-color", index))
                    .h(px(28.0))
                    .px(px(8.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .when(on, |row| row.bg(pack.selection))
                    .child(format!("{mark} {} {hex}", field.key)),
            );
        }
    } else {
        for (index, row) in appearance.picker.rows.iter().enumerate() {
            let on = index == appearance.picker.selected;
            let mark = if on { ">" } else { " " };
            let label = theme_row_label(row);
            body = body.child(
                div()
                    .id(("theme-row", index))
                    .h(px(28.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .cursor_pointer()
                    .when(on, |row| row.bg(pack.selection))
                    .on_click(cx.listener(move |shell, _: &ClickEvent, _, cx| {
                        shell.choose_theme(index);
                        cx.stop_propagation();
                        cx.notify();
                    }))
                    .child(format!("{mark} {label}")),
            );
        }
        body = body.child(
            div()
                .id("edit-colors")
                .h(px(28.0))
                .cursor_pointer()
                .child("Edit colors…")
                .on_click(cx.listener(|shell, _: &ClickEvent, _, cx| {
                    shell.open_color_editor();
                    cx.stop_propagation();
                    cx.notify();
                })),
        );
    }
    let save_to = target_word(appearance.picker.save_target);
    body.child(div().text_color(pack.secondary).child(format!("Source: {}", shell.theme_label)))
        .child(div().child(format!("Save to {save_to}")))
        .child(
            div()
                .id("theme-save")
                .h(px(28.0))
                .cursor_pointer()
                .child("Save")
                .on_click(cx.listener(|shell, _: &ClickEvent, _, cx| {
                    shell.save_theme();
                    cx.stop_propagation();
                    cx.notify();
                })),
        )
}

fn window_choice(
    cx: &mut Context<Shell>,
    id: &'static str,
    label: &str,
    on: bool,
    pack: Packed,
    row: usize,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .h(px(28.0))
        .px(px(8.0))
        .flex()
        .items_center()
        .cursor_pointer()
        .when(on, |row| row.bg(pack.selection))
        .on_click(cx.listener(move |shell, _: &ClickEvent, _, cx| {
            shell.row = row;
            shell.activate_window_row();
            cx.stop_propagation();
            cx.notify();
        }))
        .child(label.to_string())
}

fn outlined(title: &str, pack: Packed, body: gpui::Div) -> gpui::Div {
    div()
        .w_full()
        .border_1()
        .border_color(pack.border)
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(28.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .font_weight(FontWeight::BOLD)
                .bg(pack.raised)
                .text_color(pack.text)
                .border_b_1()
                .border_color(pack.border)
                .child(title.to_string()),
        )
        .child(div().p(px(8.0)).child(body))
}

fn theme_fields() -> Vec<ColorField> {
    let mut fields = COLOR_FIELDS.to_vec();
    for extra in dd_wcag::theme::extra_theme_fields() {
        if !fields.iter().any(|field| field.key == extra.key) {
            fields.push(*extra);
        }
    }
    fields
}

fn theme_row_label(row: &PickerRow) -> String {
    match row {
        PickerRow::Follow { name } => format!("Follow {name}"),
        PickerRow::Theme(theme) => theme.name.clone(),
        PickerRow::Custom => "Custom".to_string(),
    }
}

fn target_word(target: ThemeSaveTarget) -> &'static str {
    match target {
        ThemeSaveTarget::Global => "Global",
        ThemeSaveTarget::Local => "Local",
    }
}

fn section_delta(key: &str, editing_hex: bool) -> Option<i32> {
    match key {
        "tab" => Some(1),
        "shift-tab" | "backtab" => Some(-1),
        "]" if !editing_hex => Some(1),
        "[" if !editing_hex => Some(-1),
        _ => None,
    }
}

fn editor_key_from(key: &str, vim: bool, editing_hex: bool) -> Option<EditorKey> {
    let shift = key.starts_with("shift-");
    let bare = key.strip_prefix("shift-").unwrap_or(key);
    match bare {
        "escape" => Some(EditorKey::Esc),
        "enter" => Some(EditorKey::Enter),
        "backspace" => Some(EditorKey::Backspace),
        "up" => Some(EditorKey::Up),
        "down" => Some(EditorKey::Down),
        "left" => Some(EditorKey::Left),
        "right" => Some(EditorKey::Right),
        // h/l/j/k/g are hex digits. A failing match guard does not fall through.
        "h" if editing_hex => Some(EditorKey::Char('h')),
        "h" => Some(EditorKey::Left),
        "l" if editing_hex => Some(EditorKey::Char('l')),
        "l" => Some(EditorKey::Right),
        "j" if editing_hex || vim => Some(EditorKey::Char('j')),
        "j" => None,
        "k" if editing_hex || vim => Some(EditorKey::Char('k')),
        "k" => None,
        "s" | "y" if !editing_hex => Some(EditorKey::Char('s')),
        "-" | "_" => Some(EditorKey::Char('-')),
        "=" | "+" => Some(EditorKey::Char('+')),
        "[" => Some(EditorKey::Char('[')),
        "]" => Some(EditorKey::Char(']')),
        "g" if editing_hex => Some(EditorKey::Char('g')),
        "g" if shift && vim => Some(EditorKey::Char('G')),
        "g" => None,
        other if is_char(other) => other.chars().next().map(EditorKey::Char),
        _ => None,
    }
}

fn is_char(key: &str) -> bool {
    key.chars().count() == 1 && !key.chars().any(|ch| ch.is_control())
}

fn chord(event: &KeyDownEvent) -> String {
    let mut parts = Vec::new();
    let mods = event.keystroke.modifiers;
    if mods.control || mods.platform {
        parts.push("ctrl");
    }
    if mods.shift {
        parts.push("shift");
    }
    if mods.alt {
        parts.push("alt");
    }
    let key = event.keystroke.key.to_ascii_lowercase();
    parts.push(key.as_str());
    parts.join("-")
}

#[allow(dead_code)]
fn _loaded_marker(theme: &LoadedTheme) -> &str {
    theme.label.as_str()
}
