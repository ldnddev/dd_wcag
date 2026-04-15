use ratatui::style::Color;
use std::fs;
use std::path::{Path, PathBuf};

pub const PROJECT_THEME_FILE: &str = "dd_wcag_theme.yml";
pub const SUPPORTED_THEME_VERSION: u16 = 1;

const THEME_KEYS: [&str; 26] = [
    "base_background",
    "body_background",
    "modal_background",
    "text_primary",
    "text_secondary",
    "text_labels",
    "text_active_focus",
    "modal_labels",
    "modal_text",
    "selected_background",
    "border_default",
    "border_active",
    "scrollbar",
    "scrollbar_hover",
    "input_border_default",
    "input_border_focus",
    "input_text_default",
    "input_text_focus",
    "cursor",
    "success",
    "warning",
    "error",
    "info",
    "folders",
    "files",
    "links",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeSource {
    Local,
    Global,
    Default,
}

impl ThemeSource {
    pub fn label(self) -> &'static str {
        match self {
            ThemeSource::Local => "local",
            ThemeSource::Global => "global",
            ThemeSource::Default => "default",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoadedTheme {
    pub theme: Theme,
    pub source: ThemeSource,
    pub path: Option<PathBuf>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub version: u16,
    pub base_background: String,
    pub body_background: String,
    pub modal_background: String,
    pub text_primary: String,
    pub text_secondary: String,
    pub text_labels: String,
    pub text_active_focus: String,
    pub modal_labels: String,
    pub modal_text: String,
    pub selected_background: String,
    pub border_default: String,
    pub border_active: String,
    pub scrollbar: String,
    pub scrollbar_hover: String,
    pub input_border_default: String,
    pub input_border_focus: String,
    pub input_text_default: String,
    pub input_text_focus: String,
    pub cursor: String,
    pub success: String,
    pub warning: String,
    pub error: String,
    pub info: String,
    pub folders: String,
    pub files: String,
    pub links: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            version: SUPPORTED_THEME_VERSION,
            base_background: "#0F1114".to_string(),
            body_background: "#2A2D31".to_string(),
            modal_background: "#1C1E21".to_string(),
            text_primary: "#F5F6F7".to_string(),
            text_secondary: "#9EA3AA".to_string(),
            text_labels: "#FFAF46".to_string(),
            text_active_focus: "#64B4F5".to_string(),
            modal_labels: "#64B4F5".to_string(),
            modal_text: "#F5F6F7".to_string(),
            selected_background: "#0F1114".to_string(),
            border_default: "#F5F6F7".to_string(),
            border_active: "#64B4F5".to_string(),
            scrollbar: "#FFA087".to_string(),
            scrollbar_hover: "#64B4F5".to_string(),
            input_border_default: "#F5F6F7".to_string(),
            input_border_focus: "#64B4F5".to_string(),
            input_text_default: "#F5F6F7".to_string(),
            input_text_focus: "#64B4F5".to_string(),
            cursor: "#64B4F5".to_string(),
            success: "#82e0aa".to_string(),
            warning: "#f5c469".to_string(),
            error: "#e57373".to_string(),
            info: "#5dade2".to_string(),
            folders: "#64B4F5".to_string(),
            files: "#FFAF46".to_string(),
            links: "#FFA087".to_string(),
        }
    }
}

impl Theme {
    pub fn load() -> LoadedTheme {
        let local_path = PathBuf::from(PROJECT_THEME_FILE);
        let global_path = global_theme_path();

        for (source, path) in [
            (ThemeSource::Local, local_path),
            (ThemeSource::Global, global_path),
        ] {
            if !path.exists() {
                continue;
            }

            return match Self::load_from_file(&path) {
                Ok(theme) => LoadedTheme {
                    theme,
                    source,
                    path: Some(path),
                    warning: None,
                },
                Err(err) => LoadedTheme {
                    theme: Self::default(),
                    source: ThemeSource::Default,
                    path: None,
                    warning: Some(format!(
                        "Theme load warning (using defaults): {}: {err}",
                        path.display()
                    )),
                },
            };
        }

        LoadedTheme {
            theme: Self::default(),
            source: ThemeSource::Default,
            path: None,
            warning: None,
        }
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|err| format!("failed to read theme file: {err}"))?;

        Self::from_yaml_str(&content)
    }

    pub fn tokens(&self) -> Vec<(&'static str, &str)> {
        vec![
            ("base_background", &self.base_background),
            ("body_background", &self.body_background),
            ("modal_background", &self.modal_background),
            ("text_primary", &self.text_primary),
            ("text_secondary", &self.text_secondary),
            ("text_labels", &self.text_labels),
            ("text_active_focus", &self.text_active_focus),
            ("modal_labels", &self.modal_labels),
            ("modal_text", &self.modal_text),
            ("selected_background", &self.selected_background),
            ("border_default", &self.border_default),
            ("border_active", &self.border_active),
            ("scrollbar", &self.scrollbar),
            ("scrollbar_hover", &self.scrollbar_hover),
            ("input_border_default", &self.input_border_default),
            ("input_border_focus", &self.input_border_focus),
            ("input_text_default", &self.input_text_default),
            ("input_text_focus", &self.input_text_focus),
            ("cursor", &self.cursor),
            ("success", &self.success),
            ("warning", &self.warning),
            ("error", &self.error),
            ("info", &self.info),
            ("folders", &self.folders),
            ("files", &self.files),
            ("links", &self.links),
        ]
    }

    fn from_yaml_str(content: &str) -> Result<Self, String> {
        let mut theme = Self::default();
        let mut in_colors = false;
        let mut saw_colors = false;
        let mut saw_version = false;

        for (line_idx, raw_line) in content.lines().enumerate() {
            let without_comment = strip_yaml_comment(raw_line);
            let line = without_comment.trim_end();
            if line.trim().is_empty() {
                continue;
            }

            let indent = line.len() - line.trim_start().len();
            let trimmed = line.trim();
            if indent == 0 {
                if trimmed == "colors:" {
                    in_colors = true;
                    saw_colors = true;
                    continue;
                }

                in_colors = false;
                let Some((key, value)) = trimmed.split_once(':') else {
                    continue;
                };
                if key.trim() != "version" {
                    continue;
                }

                saw_version = true;
                let value = parse_yaml_scalar(value);
                let version = value
                    .parse::<u16>()
                    .map_err(|_| format!("line {}: version must be an integer", line_idx + 1))?;
                if version != SUPPORTED_THEME_VERSION {
                    return Err(format!(
                        "unsupported theme version {version}; expected {SUPPORTED_THEME_VERSION}"
                    ));
                }
                theme.version = version;
                continue;
            }

            if !in_colors {
                continue;
            }

            let Some((key, value)) = trimmed.split_once(':') else {
                continue;
            };
            let key = key.trim();
            if !THEME_KEYS.contains(&key) {
                continue;
            }

            let value = parse_yaml_scalar(value);
            if value.is_empty() {
                return Err(format!("line {}: {key} is empty", line_idx + 1));
            }
            if parse_hex_color(value).is_none() {
                return Err(format!(
                    "line {}: {key} is not a 6-digit hex color",
                    line_idx + 1
                ));
            }

            theme.set_token(key, value);
        }

        if !saw_colors {
            return Err("missing required top-level colors: mapping".to_string());
        }
        if !saw_version {
            return Err(format!(
                "missing required top-level version: {SUPPORTED_THEME_VERSION}"
            ));
        }

        Ok(theme)
    }

    fn set_token(&mut self, key: &str, value: &str) {
        let value = value.to_string();
        match key {
            "base_background" => self.base_background = value,
            "body_background" => self.body_background = value,
            "modal_background" => self.modal_background = value,
            "text_primary" => self.text_primary = value,
            "text_secondary" => self.text_secondary = value,
            "text_labels" => self.text_labels = value,
            "text_active_focus" => self.text_active_focus = value,
            "modal_labels" => self.modal_labels = value,
            "modal_text" => self.modal_text = value,
            "selected_background" => self.selected_background = value,
            "border_default" => self.border_default = value,
            "border_active" => self.border_active = value,
            "scrollbar" => self.scrollbar = value,
            "scrollbar_hover" => self.scrollbar_hover = value,
            "input_border_default" => self.input_border_default = value,
            "input_border_focus" => self.input_border_focus = value,
            "input_text_default" => self.input_text_default = value,
            "input_text_focus" => self.input_text_focus = value,
            "cursor" => self.cursor = value,
            "success" => self.success = value,
            "warning" => self.warning = value,
            "error" => self.error = value,
            "info" => self.info = value,
            "folders" => self.folders = value,
            "files" => self.files = value,
            "links" => self.links = value,
            _ => {}
        }
    }

    pub fn base_background_color(&self) -> Color {
        color_or_reset(&self.base_background)
    }

    pub fn body_background_color(&self) -> Color {
        color_or_reset(&self.body_background)
    }

    pub fn modal_background_color(&self) -> Color {
        color_or_reset(&self.modal_background)
    }

    pub fn text_primary_color(&self) -> Color {
        color_or_reset(&self.text_primary)
    }

    pub fn text_secondary_color(&self) -> Color {
        color_or_reset(&self.text_secondary)
    }

    pub fn text_labels_color(&self) -> Color {
        color_or_reset(&self.text_labels)
    }

    pub fn text_active_focus_color(&self) -> Color {
        color_or_reset(&self.text_active_focus)
    }

    pub fn modal_labels_color(&self) -> Color {
        color_or_reset(&self.modal_labels)
    }

    pub fn modal_text_color(&self) -> Color {
        color_or_reset(&self.modal_text)
    }

    pub fn selected_background_color(&self) -> Color {
        color_or_reset(&self.selected_background)
    }

    pub fn border_default_color(&self) -> Color {
        color_or_reset(&self.border_default)
    }

    pub fn border_active_color(&self) -> Color {
        color_or_reset(&self.border_active)
    }

    pub fn input_border_default_color(&self) -> Color {
        color_or_reset(&self.input_border_default)
    }

    pub fn input_border_focus_color(&self) -> Color {
        color_or_reset(&self.input_border_focus)
    }

    pub fn input_text_default_color(&self) -> Color {
        color_or_reset(&self.input_text_default)
    }

    pub fn input_text_focus_color(&self) -> Color {
        color_or_reset(&self.input_text_focus)
    }

    pub fn success_color(&self) -> Color {
        color_or_reset(&self.success)
    }

    pub fn error_color(&self) -> Color {
        color_or_reset(&self.error)
    }

    pub fn info_color(&self) -> Color {
        color_or_reset(&self.info)
    }
}

pub fn global_theme_path() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("ldnddev")
        .join(PROJECT_THEME_FILE)
}

fn color_or_reset(input: &str) -> Color {
    parse_hex_color(input).unwrap_or(Color::Reset)
}

fn parse_hex_color(input: &str) -> Option<Color> {
    let s = input.trim().strip_prefix('#').unwrap_or(input.trim());
    if s.len() != 6 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

fn parse_yaml_scalar(raw: &str) -> &str {
    let v = raw.trim();
    if let Some(rest) = v.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            return &rest[..end];
        }
    }
    if let Some(rest) = v.strip_prefix('\'') {
        if let Some(end) = rest.find('\'') {
            return &rest[..end];
        }
    }
    v.split_whitespace().next().unwrap_or("")
}

fn strip_yaml_comment(raw: &str) -> &str {
    let mut in_single = false;
    let mut in_double = false;
    for (idx, ch) in raw.char_indices() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '#' if !in_single && !in_double => return &raw[..idx],
            _ => {}
        }
    }
    raw
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_colors_mapping() {
        let theme = Theme::from_yaml_str(
            r##"
version: 1
colors:
  base_background: "#010203"
  text_primary: "#F5F6F7"
"##,
        )
        .expect("theme parses");

        assert_eq!(theme.base_background, "#010203");
        assert_eq!(theme.text_primary, "#F5F6F7");
        assert_eq!(theme.version, SUPPORTED_THEME_VERSION);
        assert_eq!(theme.error, Theme::default().error);
    }

    #[test]
    fn rejects_invalid_hex_values() {
        let err = Theme::from_yaml_str(
            r##"
version: 1
colors:
  base_background: "#xyz"
"##,
        )
        .expect_err("invalid color should fail");

        assert!(err.contains("base_background"));
    }

    #[test]
    fn rejects_missing_theme_version() {
        let err = Theme::from_yaml_str(
            r##"
colors:
  base_background: "#010203"
"##,
        )
        .expect_err("missing version should fail");

        assert!(err.contains("missing required top-level version"));
    }

    #[test]
    fn rejects_unsupported_theme_version() {
        let err = Theme::from_yaml_str(
            r##"
version: 2
colors:
  base_background: "#010203"
"##,
        )
        .expect_err("unsupported version should fail");

        assert!(err.contains("unsupported theme version 2"));
    }

    #[test]
    fn source_labels_match_standard_names() {
        assert_eq!(ThemeSource::Local.label(), "local");
        assert_eq!(ThemeSource::Global.label(), "global");
        assert_eq!(ThemeSource::Default.label(), "default");
    }
}
