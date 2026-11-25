//! App id and window options.
//!
//! On Wayland `app_id` is the xdg app id. On X11 it is the window class.

use gpui::{Bounds, Pixels, SharedString, TitlebarOptions, WindowBounds, WindowOptions};

pub const MAIN_APP_ID: &str = "dev.ldnddev.dd_wcag";

pub fn main_window_options(bounds: Bounds<Pixels>) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        app_id: Some(MAIN_APP_ID.to_string()),
        titlebar: Some(TitlebarOptions {
            title: Some(SharedString::from("dd_wcag".to_string())),
            ..Default::default()
        }),
        focus: true,
        show: true,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_id_matches_the_desktop_class() {
        assert_eq!(MAIN_APP_ID, "dev.ldnddev.dd_wcag");
    }
}
