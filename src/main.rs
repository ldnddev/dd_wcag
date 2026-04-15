//! # dd_wcag - Phase 1: Basic Project Setup
//!
//! This is the minimal entry point for the TUI application.
//! In Phase 1, we set up the terminal with Crossterm and Ratatui,
//! create a basic event loop that quits on 'q', and render an empty frame.

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

// Import modules
mod app;
mod color;
mod palette;
mod theme;
mod ui;
mod web_preview;

use app::{ActiveTab, App, InputTarget};
use palette::{PaletteApplyTarget, PALETTE_EXPORT_PATH};
use std::io::Write;
use std::process::{Command, Stdio};
use theme::Theme;

// Main function
fn main() -> Result<()> {
    // Setup terminal
    let mut terminal = setup_terminal()?;

    // Create app state
    let loaded_theme = Theme::load();
    let source = loaded_theme.source;
    let path = loaded_theme.path.clone();
    let mut app = App::with_theme(loaded_theme.theme, source);
    let path_label = path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "built-in defaults".to_string());
    app.notify_status(format!(
        "Theme health: {} theme v{} active ({path_label}).",
        source.label(),
        app.theme.version
    ));
    if let Some(warning) = loaded_theme.warning {
        app.notify_error(warning);
    }
    sync_web_preview(&mut app);

    // Run the main loop
    let res = run_loop(&mut terminal, &mut app);

    // Restore terminal
    restore_terminal(&mut terminal)?;

    res
}

fn sync_web_preview(app: &mut App) {
    if let Err(err) = web_preview::sync(app) {
        app.notify_error(format!("Failed to update web preview: {err}"));
    }
}

#[derive(Default)]
struct KeyEffects {
    quit: bool,
    sync_preview: bool,
    open_preview: bool,
    save_palette: bool,
    copy_palette: bool,
}

fn try_apply_active_input(app: &mut App) -> bool {
    app.sync_active_input();
    app.submit_input()
}

fn handle_key_event(app: &mut App, key: KeyEvent) -> KeyEffects {
    let mut effects = KeyEffects::default();
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    if ctrl {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => effects.quit = true,

            KeyCode::Char('b') | KeyCode::Char('B') => {
                app.is_bold = !app.is_bold;
                effects.sync_preview = true;
            }

            KeyCode::Char('f') | KeyCode::Char('F') => {
                app.cycle_font_family();
                effects.sync_preview = true;
            }

            KeyCode::Char('o') | KeyCode::Char('O') => {
                effects.open_preview = true;
            }

            KeyCode::Char('s') | KeyCode::Char('S') => {
                if app.active_tab == ActiveTab::Palette {
                    effects.save_palette = true;
                }
            }

            KeyCode::Char('c') | KeyCode::Char('C') => {
                if app.active_tab == ActiveTab::Palette {
                    effects.copy_palette = true;
                }
            }

            KeyCode::Up => {
                if app.active_tab == ActiveTab::Preview || app.active_tab == ActiveTab::Contrast {
                    app.adjust_font_size(1);
                    effects.sync_preview = true;
                }
            }

            KeyCode::Down => {
                if app.active_tab == ActiveTab::Preview || app.active_tab == ActiveTab::Contrast {
                    app.adjust_font_size(-1);
                    effects.sync_preview = true;
                }
            }

            _ => {}
        }

        return effects;
    }

    match key.code {
        KeyCode::Tab => match app.active_tab {
            ActiveTab::Input => match app.input_target {
                InputTarget::Foreground => {
                    if !try_apply_active_input(app) {
                        return effects;
                    }
                    effects.sync_preview = true;
                    app.set_input_target(InputTarget::Background);
                }
                InputTarget::Background => {
                    if !try_apply_active_input(app) {
                        return effects;
                    }
                    effects.sync_preview = true;
                    app.set_input_target(InputTarget::PreviewText);
                }
                InputTarget::PreviewText | InputTarget::None => {
                    if !try_apply_active_input(app) {
                        return effects;
                    }
                    effects.sync_preview = true;
                    app.set_input_target(InputTarget::FontFamily);
                }
                InputTarget::FontFamily => {
                    if !try_apply_active_input(app) {
                        return effects;
                    }
                    effects.sync_preview = true;
                    app.active_tab = ActiveTab::Conversions;
                    app.set_input_target(InputTarget::None);
                }
            },
            ActiveTab::Conversions => {
                app.active_tab = ActiveTab::Contrast;
            }
            ActiveTab::Contrast => {
                app.active_tab = ActiveTab::Preview;
            }
            ActiveTab::Preview => {
                app.active_tab = ActiveTab::Palette;
            }
            ActiveTab::Palette => {
                app.active_tab = ActiveTab::Input;
                app.set_input_target(InputTarget::Foreground);
            }
        },

        KeyCode::BackTab => match app.active_tab {
            ActiveTab::Input => match app.input_target {
                InputTarget::Background => {
                    if !try_apply_active_input(app) {
                        return effects;
                    }
                    effects.sync_preview = true;
                    app.set_input_target(InputTarget::Foreground);
                }
                InputTarget::PreviewText => {
                    if !try_apply_active_input(app) {
                        return effects;
                    }
                    effects.sync_preview = true;
                    app.set_input_target(InputTarget::Background);
                }
                InputTarget::FontFamily => {
                    if !try_apply_active_input(app) {
                        return effects;
                    }
                    effects.sync_preview = true;
                    app.set_input_target(InputTarget::PreviewText);
                }
                InputTarget::Foreground | InputTarget::None => {
                    if !try_apply_active_input(app) {
                        return effects;
                    }
                    effects.sync_preview = true;
                    app.active_tab = ActiveTab::Palette;
                    app.set_input_target(InputTarget::None);
                }
            },
            ActiveTab::Conversions => {
                app.active_tab = ActiveTab::Input;
                app.set_input_target(InputTarget::Background);
            }
            ActiveTab::Contrast => {
                app.active_tab = ActiveTab::Conversions;
            }
            ActiveTab::Preview => {
                app.active_tab = ActiveTab::Contrast;
            }
            ActiveTab::Palette => {
                app.active_tab = ActiveTab::Preview;
            }
        },

        KeyCode::Esc => {
            if app.show_keybindings {
                app.show_keybindings = false;
            } else if app.show_theme_debug {
                app.show_theme_debug = false;
            } else if app.active_tab == ActiveTab::Palette && app.palette.editing {
                app.palette.cancel_edit();
            } else if app.active_tab == ActiveTab::Palette && app.palette.pending_apply.is_some() {
                app.palette.pending_apply = None;
                app.notify_status("Palette apply cancelled.");
            } else {
                effects.quit = true;
            }
        }

        KeyCode::Up => {
            if app.active_tab == ActiveTab::Palette && !app.palette.editing {
                if app.palette.generated.is_some() {
                    app.palette.scroll_detail_up();
                } else {
                    app.palette.select_previous();
                }
            }
        }

        KeyCode::Down => {
            if app.active_tab == ActiveTab::Palette && !app.palette.editing {
                if app.palette.generated.is_some() {
                    app.palette.scroll_detail_down();
                } else {
                    app.palette.select_next();
                }
            }
        }

        KeyCode::F(1) => {
            app.show_keybindings = true;
            app.show_theme_debug = false;
        }

        KeyCode::F(2) => {
            app.show_theme_debug = true;
            app.show_keybindings = false;
        }

        KeyCode::Char(c) => {
            if app.active_tab == ActiveTab::Palette {
                if app.palette.editing {
                    app.palette.insert_char_at_cursor(c);
                } else if c == 'g' || c == 'G' {
                    if let Some(target) = app.palette.pending_apply {
                        if app.apply_selected_palette_color(target) {
                            effects.sync_preview = true;
                        }
                    } else {
                        app.generate_palette();
                    }
                } else if c == 'f' || c == 'F' {
                    app.palette.pending_apply = Some(PaletteApplyTarget::Foreground);
                    app.notify_status("Press G to apply selected palette color to FG.");
                } else if c == 'b' || c == 'B' {
                    app.palette.pending_apply = Some(PaletteApplyTarget::Background);
                    app.notify_status("Press G to apply selected palette color to BG.");
                }
            } else if app.active_tab == ActiveTab::Input && app.input_target != InputTarget::None {
                app.insert_char_at_cursor(c);
                app.sync_active_input();
                if app.input_target == InputTarget::PreviewText {
                    effects.sync_preview = true;
                }
            }
        }

        KeyCode::Backspace => {
            if app.active_tab == ActiveTab::Palette && app.palette.editing {
                app.palette.backspace_at_cursor();
            } else if app.active_tab == ActiveTab::Input && app.input_target != InputTarget::None {
                app.backspace_at_cursor();
                app.sync_active_input();
                if app.input_target == InputTarget::PreviewText {
                    effects.sync_preview = true;
                }
            }
        }

        KeyCode::Enter => {
            if app.active_tab == ActiveTab::Palette {
                if app.palette.editing {
                    app.palette.commit_edit();
                } else {
                    app.palette.begin_edit();
                }
            } else if app.active_tab == ActiveTab::Input
                && app.input_target == InputTarget::PreviewText
            {
                app.insert_newline_at_cursor();
                app.sync_active_input();
                effects.sync_preview = true;
            }
        }

        KeyCode::Left => {
            if app.active_tab == ActiveTab::Palette && app.palette.editing {
                app.palette.move_cursor_left();
            } else if app.active_tab == ActiveTab::Input && app.input_target != InputTarget::None {
                app.move_cursor_left();
            }
        }

        KeyCode::Right => {
            if app.active_tab == ActiveTab::Palette && app.palette.editing {
                app.palette.move_cursor_right();
            } else if app.active_tab == ActiveTab::Input && app.input_target != InputTarget::None {
                app.move_cursor_right();
            }
        }

        _ => {}
    }

    effects
}

// Setup terminal (unchanged)
fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(Into::into)
}

// Restore terminal (unchanged)
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

// Main event loop with key handling (add tab switching)
fn run_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::render(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                let effects = handle_key_event(app, key);

                if effects.open_preview {
                    if let Err(err) = web_preview::open_in_browser() {
                        app.notify_error(format!(
                            "Failed to open browser preview ({}): {err}",
                            web_preview::preview_path().display()
                        ));
                    }
                }

                if effects.save_palette {
                    save_palette(app);
                }

                if effects.copy_palette {
                    copy_palette(app);
                }

                if effects.sync_preview {
                    sync_web_preview(app);
                }

                if effects.quit {
                    return Ok(());
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
            app.expire_notification(last_tick);
        }
    }
}

fn save_palette(app: &mut App) {
    match app.prepare_palette_export("saving") {
        Ok(scss) => match std::fs::write(PALETTE_EXPORT_PATH, scss) {
            Ok(()) => {
                app.notify_status(format!("Palette saved to ./{PALETTE_EXPORT_PATH}."));
            }
            Err(err) => {
                app.notify_error(format!("Failed to save {PALETTE_EXPORT_PATH}: {err}"));
            }
        },
        Err(err) => {
            app.notify_error(err);
        }
    }
}

fn copy_palette(app: &mut App) {
    match app.prepare_palette_export("copying") {
        Ok(scss) => match copy_to_clipboard(&scss) {
            Ok(()) => {
                app.copied_palette = Some(scss);
                app.notify_status("Palette copied to clipboard.");
            }
            Err(err) => {
                app.copied_palette = Some(scss);
                app.notify_error(format!(
                    "Could not access a system clipboard command: {err}. Palette is available in the app copy buffer."
                ));
            }
        },
        Err(err) => {
            app.notify_error(err);
        }
    }
}

fn copy_to_clipboard(content: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        return write_to_command_stdin("pbcopy", &[], content);
    }

    #[cfg(target_os = "windows")]
    {
        return write_to_command_stdin("clip", &[], content);
    }

    #[cfg(target_os = "linux")]
    {
        let attempts: [(&str, &[&str]); 4] = [
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
            ("termux-clipboard-set", &[]),
        ];
        let mut last_err = None;
        for (program, args) in attempts {
            match write_to_command_stdin(program, args, content) {
                Ok(()) => return Ok(()),
                Err(err) => last_err = Some(err),
            }
        }
        return Err(last_err.unwrap_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no clipboard command configured",
            )
        }));
    }

    #[allow(unreachable_code)]
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "clipboard copy is not supported on this platform",
    ))
}

fn write_to_command_stdin(program: &str, args: &[&str], content: &str) -> std::io::Result<()> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(content.as_bytes())?;
    }
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "{program} exited with {status}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn tab_auto_applies_foreground_and_moves_focus() {
        let mut app = App::new();
        app.active_tab = ActiveTab::Input;
        app.set_input_target(InputTarget::Foreground);
        app.current_input = "#00ff00".to_string();

        let effects = handle_key_event(&mut app, key(KeyCode::Tab, KeyModifiers::NONE));

        assert_eq!(app.foreground.to_hex(), "#00ff00");
        assert_eq!(app.input_target, InputTarget::Background);
        assert!(effects.sync_preview);
        assert!(!effects.quit);
    }

    #[test]
    fn tab_with_invalid_input_keeps_focus_and_sets_error() {
        let mut app = App::new();
        app.active_tab = ActiveTab::Input;
        app.set_input_target(InputTarget::Foreground);
        app.current_input = "#zzzzzz".to_string();

        let effects = handle_key_event(&mut app, key(KeyCode::Tab, KeyModifiers::NONE));

        assert_eq!(app.input_target, InputTarget::Foreground);
        assert!(app.error.is_some());
        assert!(!effects.sync_preview);
    }

    #[test]
    fn ctrl_up_down_adjusts_size_with_expected_direction_and_bounds() {
        let mut app = App::new();
        app.active_tab = ActiveTab::Preview;
        app.font_size_px = 12;

        handle_key_event(&mut app, key(KeyCode::Up, KeyModifiers::CONTROL));
        assert_eq!(app.font_size_px, 13);

        handle_key_event(&mut app, key(KeyCode::Down, KeyModifiers::CONTROL));
        assert_eq!(app.font_size_px, 12);

        app.font_size_px = 120;
        handle_key_event(&mut app, key(KeyCode::Up, KeyModifiers::CONTROL));
        assert_eq!(app.font_size_px, 120);

        app.font_size_px = 6;
        handle_key_event(&mut app, key(KeyCode::Down, KeyModifiers::CONTROL));
        assert_eq!(app.font_size_px, 6);
    }

    #[test]
    fn esc_quits_when_error_toast_is_visible() {
        let mut app = App::new();
        app.notify_error("error");

        let quit = handle_key_event(&mut app, key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.error.is_some());
        assert!(quit.quit);
    }

    #[test]
    fn esc_quits_when_status_toast_is_visible() {
        let mut app = App::new();
        app.notify_status("status");

        let quit = handle_key_event(&mut app, key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.status.is_some());
        assert!(quit.quit);
    }

    #[test]
    fn f1_opens_keybindings_popup() {
        let mut app = App::new();
        handle_key_event(&mut app, key(KeyCode::F(1), KeyModifiers::NONE));
        assert!(app.show_keybindings);
    }

    #[test]
    fn f2_opens_theme_debug_popup() {
        let mut app = App::new();
        handle_key_event(&mut app, key(KeyCode::F(2), KeyModifiers::NONE));
        assert!(app.show_theme_debug);
    }

    #[test]
    fn ctrl_f_cycles_font_family() {
        let mut app = App::new();
        let start = app.preview_font_family.clone();
        let effects = handle_key_event(&mut app, key(KeyCode::Char('f'), KeyModifiers::CONTROL));
        assert_ne!(app.preview_font_family, start);
        assert!(effects.sync_preview);
    }

    #[test]
    fn enter_in_preview_text_adds_newline_and_syncs_preview() {
        let mut app = App::new();
        app.active_tab = ActiveTab::Input;
        app.set_input_target(InputTarget::PreviewText);
        app.current_input = "Line 1".to_string();
        app.cursor_char_idx = app.current_input.chars().count();

        let effects = handle_key_event(&mut app, key(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(app.current_input, "Line 1\n");
        assert_eq!(app.preview_text, "Line 1\n");
        assert!(effects.sync_preview);
    }

    #[test]
    fn left_right_move_cursor_and_char_inserts_at_cursor() {
        let mut app = App::new();
        app.active_tab = ActiveTab::Input;
        app.set_input_target(InputTarget::PreviewText);
        app.current_input = "ab".to_string();
        app.cursor_char_idx = 2;

        handle_key_event(&mut app, key(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.cursor_char_idx, 1);

        handle_key_event(&mut app, key(KeyCode::Char('X'), KeyModifiers::NONE));
        assert_eq!(app.current_input, "aXb");
        assert_eq!(app.cursor_char_idx, 2);

        handle_key_event(&mut app, key(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(app.cursor_char_idx, 3);
    }

    #[test]
    fn tab_cycle_includes_palette_tab() {
        let mut app = App::new();
        app.active_tab = ActiveTab::Preview;

        handle_key_event(&mut app, key(KeyCode::Tab, KeyModifiers::NONE));

        assert_eq!(app.active_tab, ActiveTab::Palette);
    }

    #[test]
    fn backtab_from_foreground_moves_to_palette_tab() {
        let mut app = App::new();
        app.active_tab = ActiveTab::Input;
        app.set_input_target(InputTarget::Foreground);

        handle_key_event(&mut app, key(KeyCode::BackTab, KeyModifiers::SHIFT));

        assert_eq!(app.active_tab, ActiveTab::Palette);
        assert_eq!(app.input_target, InputTarget::None);
    }

    #[test]
    fn palette_enter_edits_and_commits_selected_color() {
        let mut app = App::new();
        app.active_tab = ActiveTab::Palette;

        handle_key_event(&mut app, key(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.palette.editing);

        app.palette.edit_input = "#000000".to_string();
        app.palette.edit_cursor_char_idx = app.palette.edit_input.chars().count();
        handle_key_event(&mut app, key(KeyCode::Enter, KeyModifiers::NONE));

        assert!(!app.palette.editing);
        assert_eq!(app.palette.primary_input, "#000000");
    }

    #[test]
    fn palette_g_generates_scss() {
        let mut app = App::new();
        app.active_tab = ActiveTab::Palette;

        handle_key_event(&mut app, key(KeyCode::Char('g'), KeyModifiers::NONE));

        assert!(app.palette.generated.is_some());
        assert!(app.error.is_none());
    }

    #[test]
    fn palette_fg_sequence_applies_selected_color_to_foreground() {
        let mut app = App::new();
        app.active_tab = ActiveTab::Palette;
        app.palette.primary_input = "#123456".to_string();

        handle_key_event(&mut app, key(KeyCode::Char('f'), KeyModifiers::NONE));
        let effects = handle_key_event(&mut app, key(KeyCode::Char('g'), KeyModifiers::NONE));

        assert_eq!(app.foreground.to_hex(), "#123456");
        assert_eq!(app.foreground_input, "#123456");
        assert!(effects.sync_preview);
    }

    #[test]
    fn palette_bg_sequence_applies_selected_color_to_background() {
        let mut app = App::new();
        app.active_tab = ActiveTab::Palette;
        app.palette.primary_input = "#abcdef".to_string();

        handle_key_event(&mut app, key(KeyCode::Char('b'), KeyModifiers::NONE));
        let effects = handle_key_event(&mut app, key(KeyCode::Char('g'), KeyModifiers::NONE));

        assert_eq!(app.background.to_hex(), "#abcdef");
        assert_eq!(app.background_input, "#abcdef");
        assert!(effects.sync_preview);
    }

    #[test]
    fn palette_up_down_scroll_generated_detail() {
        let mut app = App::new();
        app.active_tab = ActiveTab::Palette;
        app.generate_palette();

        handle_key_event(&mut app, key(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.palette.detail_scroll, 1);

        handle_key_event(&mut app, key(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.palette.detail_scroll, 0);
    }

    #[test]
    fn ctrl_s_and_ctrl_c_set_palette_effects() {
        let mut app = App::new();
        app.active_tab = ActiveTab::Palette;

        let save = handle_key_event(&mut app, key(KeyCode::Char('s'), KeyModifiers::CONTROL));
        let copy = handle_key_event(&mut app, key(KeyCode::Char('c'), KeyModifiers::CONTROL));

        assert!(save.save_palette);
        assert!(copy.copy_palette);
    }
}
