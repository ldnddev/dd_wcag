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
        assert!(text.contains("footer:F1: Settings  Ctrl+Q: Quit"), "{text}");
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
        shell.handle_key("backspace");
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

#[gpui::test]
fn contrast_fix_preview_and_targets_match_the_terminal(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let shell = mount(cx, dir.path());
    shell.update(cx, |shell, _| {
        let text = shell.visible_text();
        assert!(text.contains("mode:Contrast"), "{text}");
        assert!(text.contains("WCAG AA"), "{text}");
        assert!(text.contains("APCA Lc75"), "{text}");
        assert!(text.contains("HEX #000000 #ffffff"), "{text}");
        assert!(text.contains("font:Roboto"), "{text}");
        assert!(text.contains("fix:closed"), "{text}");
        assert!(shell.product.apply_paste("  #336699  \n"));
        let text = shell.visible_text();
        assert!(text.contains("Foreground #336699"), "{text}");
        shell.handle_key("escape");
        shell.handle_key("space");
        let text = shell.visible_text();
        assert!(text.contains("Foreground #ffffff"), "{text}");
        assert!(text.contains("Background #336699"), "{text}");
        shell.handle_key("ctrl-s");
        let text = shell.visible_text();
        assert!(text.contains("Style Bld"), "{text}");
        assert!(text.contains("Weight 700"), "{text}");
        shell
            .product
            .app
            .set_focus(dd_wcag::app::FocusId::TargetWcag);
        shell.handle_key("enter");
        assert!(shell.visible_text().contains("WCAG AAA"));
        shell
            .product
            .app
            .set_focus(dd_wcag::app::FocusId::TargetApca);
        shell.handle_key("left");
        assert!(shell.visible_text().contains("APCA Lc60"));
        shell.handle_key("ctrl-f");
        let text = shell.visible_text();
        assert!(text.contains("fix:open"), "{text}");
        assert!(text.contains("fix-source:contrast"), "{text}");
        shell.handle_key("ctrl-n");
        shell.handle_key("]");
        shell.handle_key("ctrl-o");
        let text = shell.visible_text();
        assert!(text.contains("Preview written to"), "{text}");
        let html = std::fs::read_to_string("/tmp/dd_wcag_preview.html").unwrap();
        assert!(
            html.contains("336699") || html.contains("#ffffff"),
            "{html}"
        );
        shell.product.app.set_focus(dd_wcag::app::FocusId::FgHex);
        shell.product.app.current_input = "not-a-color".to_string();
        shell.product.app.sync_active_input();
        let before = format!("{:?}", shell.product.app.focus);
        shell.handle_key("tab");
        let text = shell.visible_text();
        assert!(text.contains(&format!("focus:{before}")), "{text}");
        assert!(text.contains("Invalid"), "{text}");
    });
}

#[gpui::test]
fn palette_generate_matrix_fix_and_save_match_the_terminal(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let shell = mount(cx, dir.path());
    shell.update(cx, |shell, _| {
        shell.handle_key("escape");
        shell.handle_key("ctrl-g");
        let text = shell.visible_text();
        assert!(text.contains("mode:Palette"), "{text}");
        assert!(text.contains("Primary #88D9F7"), "{text}");
        assert!(text.contains("blocking:0"), "{text}");
        assert!(text.contains("matrix-grid:"), "{text}");
        assert!(
            text.contains('✓') || text.contains('~') || text.contains('✗'),
            "{text}"
        );
        shell.handle_key("ctrl-s");
        let text = shell.visible_text();
        assert!(text.contains("Palette saved"), "{text}");
        let scss = std::fs::read_to_string(dir.path().join("_palette.scss")).unwrap();
        let tokens = std::fs::read_to_string(dir.path().join("_palette.tokens.json")).unwrap();
        assert!(scss.contains("$c_primary_default"));
        assert!(scss.contains("$c_text_primary"));
        assert!(tokens.contains("primary"));
        shell.handle_key("enter");
        let text = shell.visible_text();
        assert!(text.contains("fix:open"), "{text}");
        assert!(text.contains("fix-source:palette:"), "{text}");
        let primary = shell.product.app.palette.primary_input.clone();
        shell.handle_key("p");
        assert_ne!(shell.product.app.palette.primary_input, primary);
        shell.handle_key("escape");
        assert!(shell.visible_text().contains("fix:closed"));
        shell.activate(crate::product::Click::Matrix { row: 3, col: 0 });
        shell.activate(crate::product::Click::Matrix { row: 3, col: 0 });
        assert!(shell.visible_text().contains("fix:open"));
        shell.handle_key("escape");
        shell.handle_key("1");
        let text = shell.visible_text();
        assert!(text.contains("mode:Contrast"), "{text}");
        assert!(text.contains("Weight 400"), "{text}");
    });
}

#[gpui::test]
fn shift_nudge_page_and_toast_match_the_terminal(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let shell = mount(cx, dir.path());
    shell.update(cx, |shell, cx| {
        assert!(shell.product.apply_paste("#336699"));
        let before = shell.product.app.foreground.to_hex();
        shell.handle_key("shift-]");
        let lighter = shell.product.app.foreground.to_hex();
        assert_ne!(before, lighter, "shift+] nudges OKLab lightness by 0.10");
        shell.handle_key("shift-}");
        assert_ne!(
            lighter,
            shell.product.app.foreground.to_hex(),
            "shift+}} nudges hue by 30 degrees"
        );
        shell.handle_key("pagedown");
        assert!(shell.visible_text().contains("contrast-scroll:8"));
        shell.handle_key("shift-pagedown");
        assert!(shell.visible_text().contains("contrast-scroll:24"));
        shell.product.app.notify_success("Toast parity");
        shell.arm_toast(cx);
        assert!(shell.visible_text().contains("Toast parity"));
        shell.product.app.clear_notification();
        shell.clear_toast_arm();
        shell.product.app.notify_success("Toast parity");
        shell.arm_toast(cx);
    });
    cx.background_executor
        .advance_clock(std::time::Duration::from_secs(5));
    cx.background_executor.run_until_parked();
    shell.update(cx, |shell, _| {
        assert!(
            !shell.visible_text().contains("Toast parity"),
            "{}",
            shell.visible_text()
        );
    });
}
