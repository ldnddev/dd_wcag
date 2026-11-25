//! Same YAML lookup as the terminal: `load_lookup`, version 1, then `apply_omarchy`.

use std::path::Path;

use ldnddev_theme::{
    OmarchyPaths, Palette, ParseMode, ThemeSource, apply_omarchy, load_lookup,
};

pub const THEME_FILE: &str = "dd_wcag_theme.yml";

#[derive(Clone, Debug)]
pub struct LoadedTheme {
    pub palette: Palette,
    pub label: String,
    pub warning: Option<String>,
}

pub fn load(
    project_root: &Path,
    paths: &OmarchyPaths,
    config_home: Option<&Path>,
) -> LoadedTheme {
    let default_home = ldnddev_theme::default_config_home();
    let home = config_home.or(default_home.as_deref());
    match load_lookup(project_root, THEME_FILE, home, ParseMode::Strict) {
        Ok(palette) => resolve(palette, paths),
        Err(err) => LoadedTheme {
            palette: Palette::builtin(),
            label: "built-in defaults".to_string(),
            warning: Some(format!("Theme load warning (using defaults): {err}")),
        },
    }
}

fn resolve(palette: Palette, paths: &OmarchyPaths) -> LoadedTheme {
    let label = match palette.source {
        ThemeSource::Local => "local",
        ThemeSource::Global => "global",
        ThemeSource::Default => "built-in defaults",
    }
    .to_string();
    if palette.omarchy.is_none() {
        return LoadedTheme {
            palette,
            label,
            warning: None,
        };
    }
    match apply_omarchy(&palette, paths) {
        Ok(resolved) => LoadedTheme {
            palette: resolved,
            label,
            warning: None,
        },
        Err(err) => LoadedTheme {
            palette: Palette::builtin(),
            label: "built-in defaults".to_string(),
            warning: Some(format!(
                "omarchy theme failed ({err}); using built-in defaults"
            )),
        },
    }
}
