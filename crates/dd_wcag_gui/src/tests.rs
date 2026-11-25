//! Headless shell tests. GPUI's TestAppContext does not open a display.

use std::path::Path;

use gpui::{AppContext, TestAppContext};

use crate::settings::Density;
use crate::shell::{Shell, ShellOptions};

fn mount(cx: &mut TestAppContext, dir: &Path) -> gpui::Entity<Shell> {
    cx.update(gpui_component::init);
    cx.new(|cx| Shell::open(ShellOptions::test(dir), cx))
}

#[test]
fn contrast_body_for_black_on_white_contains_ratio() {
    let fg = dd_wcag::color::Color::parse_hex("#000000").unwrap();
    let bg = dd_wcag::color::Color::parse_hex("#ffffff").unwrap();
    let text = dd_wcag::contrast::contrast_body_text(&fg, &bg, 16, 400);
    assert!(text.contains("21.00"), "{text}");
    assert!(text.contains("AA pass"), "{text}");
    assert!(text.contains("AAA pass"), "{text}");
}

#[gpui::test]
fn settings_sheet_pages_and_palette_restore(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let shell = mount(cx, dir.path());
    shell.update(cx, |shell, _| {
        let text = shell.visible_text();
        assert!(text.contains("21.00"), "{text}");
        assert!(text.contains("AA pass"), "{text}");
        assert!(text.contains("AAA pass"), "{text}");
        assert!(
            text.contains("footer:F1: Settings  Ctrl+Q: Quit"),
            "{text}"
        );
        let stamp = shell.palette_stamp();
        shell.handle_key("f1");
        let text = shell.visible_text();
        assert!(text.contains("settings-panel:right:70"), "{text}");
        assert!(text.contains("settings-nav:Themes,Window,Help"), "{text}");
        assert!(text.contains("settings-page:Themes"), "{text}");
        assert!(
            text.contains("theme-list:flow") || text.contains("Theme color editor"),
            "{text}"
        );
        assert!(text.contains("Theme color editor"), "{text}");
        assert!(
            text.contains("footer:F1: Settings  Esc: Close  Ctrl+Q: Quit"),
            "{text}"
        );
        shell.handle_key("right");
        let nudged = shell.palette_stamp();
        assert_ne!(nudged, stamp, "editor nudge changes the live palette");
        shell.handle_key("tab");
        let text = shell.visible_text();
        assert!(text.contains("settings-page:Window"), "{text}");
        assert!(text.contains("Density"), "{text}");
        assert!(text.contains("Vim keys"), "{text}");
        shell.set_density(Density::Compact);
        shell.set_vim_keys(false);
        let saved = std::fs::read_to_string(dir.path().join("gui.json")).unwrap();
        assert!(saved.contains("compact"), "{saved}");
        assert!(saved.contains("vim_keys"), "{saved}");
        assert!(saved.contains("reduced_motion"), "{saved}");
        assert!(!saved.contains("base_background"), "{saved}");
        shell.handle_key("tab");
        let text = shell.visible_text();
        assert!(text.contains("settings-page:Help"), "{text}");
        assert!(text.contains("Settings · F1 · F1"), "{text}");
        shell.handle_key("f1");
        let text = shell.visible_text();
        assert!(!text.contains("settings-panel:right:70"), "{text}");
        assert_eq!(shell.palette_stamp(), stamp);
        shell.handle_key("q");
        assert!(!shell.quit);
        shell.handle_key("f2");
        assert!(shell.visible_text().contains("settings-panel:right:70"));
        shell.handle_key("f2");
        let text = shell.visible_text();
        assert!(!text.contains("settings-panel:right:70"), "{text}");
        assert_eq!(shell.palette_stamp(), stamp);
        shell.handle_key("ctrl-/");
        assert!(shell.visible_text().contains("settings-page:Themes"));
        shell.handle_key("ctrl-,");
        assert!(!shell.visible_text().contains("settings-panel:right:70"));
        assert_eq!(shell.palette_stamp(), stamp);
        shell.handle_key("tab");
        shell.handle_key("tab");
        shell.handle_key("up");
        let text = shell.visible_text();
        assert!(text.contains("17px"), "{text}");
        assert!(text.contains("21.00"), "{text}");
    });
    let theme = dir.path().join("dd_wcag_theme.yml");
    assert!(
        !theme.exists(),
        "window preferences must not be written into the theme YAML"
    );
}
