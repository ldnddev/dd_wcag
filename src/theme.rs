use ratatui::style::Color;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Theme {
    pub border: String,
    pub highlight: String,
    pub success: String,
    pub error: String,
    pub text: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            border: "#45475a".to_string(),
            highlight: "#cba6f7".to_string(),
            success: "#a6e3a1".to_string(),
            error: "#f38ba8".to_string(),
            text: "#cdd6f4".to_string(),
        }
    }
}

impl Theme {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|err| format!("failed to read theme file: {err}"))?;

        let mut theme = Self::default();
        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let Some((key, value)) = line.split_once(':') else {
                continue;
            };

            let key = key.trim();
            let value = parse_yaml_scalar(value);

            match key {
                "border" if !value.is_empty() => theme.border = value.to_string(),
                "highlight" if !value.is_empty() => theme.highlight = value.to_string(),
                "success" if !value.is_empty() => theme.success = value.to_string(),
                "error" if !value.is_empty() => theme.error = value.to_string(),
                "text" if !value.is_empty() => theme.text = value.to_string(),
                _ => {}
            }
        }

        Ok(theme)
    }

    pub fn border_color(&self) -> Color {
        parse_hex_color(&self.border).unwrap_or(Color::Gray)
    }

    pub fn highlight_color(&self) -> Color {
        parse_hex_color(&self.highlight).unwrap_or(Color::Yellow)
    }

    pub fn success_color(&self) -> Color {
        parse_hex_color(&self.success).unwrap_or(Color::Green)
    }

    pub fn error_color(&self) -> Color {
        parse_hex_color(&self.error).unwrap_or(Color::Red)
    }

    pub fn text_color(&self) -> Color {
        parse_hex_color(&self.text).unwrap_or(Color::White)
    }
}

fn parse_hex_color(input: &str) -> Option<Color> {
    let s = input.trim().strip_prefix('#').unwrap_or(input.trim());
    if s.len() != 6 {
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
    // For unquoted values, keep token up to first whitespace.
    v.split_whitespace().next().unwrap_or("")
}
