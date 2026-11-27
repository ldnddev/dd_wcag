//! Window preferences. They never go in the theme YAML.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    #[default]
    Comfortable,
    Compact,
}

impl Density {
    #[cfg(test)]
    pub fn label(self) -> &'static str {
        match self {
            Self::Comfortable => "Comfortable",
            Self::Compact => "Compact",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuiSettings {
    #[serde(default)]
    pub density: Density,
    /// On by default, matching the terminal.
    #[serde(default = "default_true")]
    pub vim_keys: bool,
    #[serde(default)]
    pub reduced_motion: bool,
}

fn default_true() -> bool {
    true
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            density: Density::Comfortable,
            vim_keys: true,
            reduced_motion: false,
        }
    }
}

pub fn config_home() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg);
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".config");
    }
    PathBuf::from(".config")
}

/// `$XDG_CONFIG_HOME/ldnddev/dd_wcag/gui.json`, or `~/.config/ldnddev/dd_wcag/gui.json`.
pub fn settings_path() -> PathBuf {
    config_home()
        .join("ldnddev")
        .join("dd_wcag")
        .join("gui.json")
}

pub fn load_from(path: &Path) -> GuiSettings {
    let Ok(bytes) = fs::read(path) else {
        return GuiSettings::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

pub fn save_to(path: &Path, settings: &GuiSettings) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(settings).unwrap_or_else(|_| b"{}".to_vec());
    fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_json_round_trip_does_not_touch_theme_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gui.json");
        let theme = dir.path().join("dd_wcag_theme.yml");
        let theme_text = "version: 1\nomarchy: retro-82\n";
        std::fs::write(&theme, theme_text).unwrap();
        let settings = GuiSettings {
            density: Density::Compact,
            vim_keys: false,
            reduced_motion: true,
        };
        save_to(&path, &settings).unwrap();
        let loaded = load_from(&path);
        assert_eq!(loaded, settings);
        assert!(!loaded.vim_keys);
        assert!(loaded.reduced_motion);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("compact"));
        assert!(text.contains("vim_keys"));
        assert!(text.contains("reduced_motion"));
        assert_eq!(std::fs::read_to_string(&theme).unwrap(), theme_text);
        assert!(!text.contains("base_background"));
    }
}
