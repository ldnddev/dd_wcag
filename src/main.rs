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
use std::env;
use std::io::{self, Stdout};
use std::path::PathBuf;
use std::time::{Duration, Instant};

// Import modules
mod app;
mod color;
mod theme;
mod ui;
mod web_preview;

use app::{ActiveTab, App, InputTarget};
use theme::Theme;

fn theme_path() -> Option<PathBuf> {
    let home = env::var("HOME").ok()?;
    Some(
        PathBuf::from(home)
            .join(".config")
            .join("ldnddev")
            .join("dd_wcag")
            .join("theme.yml"),
    )
}

// Main function
fn main() -> Result<()> {
    // Setup terminal
    let mut terminal = setup_terminal()?;

    // Create app state
    let loaded_theme = theme_path()
        .ok_or_else(|| "HOME is not set".to_string())
        .and_then(Theme::load_from_file);
    let mut app = match loaded_theme {
        Ok(theme) => App::with_theme(theme),
        Err(err) => {
            let mut app = App::new();
            app.error = Some(format!("Theme load warning (using defaults): {err}"));
            app
        }
    };
    sync_web_preview(&mut app);

    // Run the main loop
    let res = run_loop(&mut terminal, &mut app);

    // Restore terminal
    restore_terminal(&mut terminal)?;

    res
}

fn sync_web_preview(app: &mut App) {
    if let Err(err) = web_preview::sync(app) {
        app.error = Some(format!("Failed to update web preview: {err}"));
    }
}

#[derive(Default)]
struct KeyEffects {
    quit: bool,
    sync_preview: bool,
    open_preview: bool,
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
                    app.active_tab = ActiveTab::Preview;
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
        },

        KeyCode::Esc => {
            if app.show_keybindings {
                app.show_keybindings = false;
            } else if app.error.is_some() {
                app.error = None;
            } else {
                effects.quit = true;
            }
        }

        KeyCode::F(1) => {
            app.show_keybindings = true;
        }

        KeyCode::Char(c) => {
            if app.active_tab == ActiveTab::Input && app.input_target != InputTarget::None {
                app.insert_char_at_cursor(c);
                app.sync_active_input();
                if app.input_target == InputTarget::PreviewText {
                    effects.sync_preview = true;
                }
            }
        }

        KeyCode::Backspace => {
            if app.active_tab == ActiveTab::Input && app.input_target != InputTarget::None {
                app.backspace_at_cursor();
                app.sync_active_input();
                if app.input_target == InputTarget::PreviewText {
                    effects.sync_preview = true;
                }
            }
        }

        KeyCode::Enter => {
            if app.active_tab == ActiveTab::Input && app.input_target == InputTarget::PreviewText {
                app.insert_newline_at_cursor();
                app.sync_active_input();
                effects.sync_preview = true;
            }
        }

        KeyCode::Left => {
            if app.active_tab == ActiveTab::Input && app.input_target != InputTarget::None {
                app.move_cursor_left();
            }
        }

        KeyCode::Right => {
            if app.active_tab == ActiveTab::Input && app.input_target != InputTarget::None {
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

        let timeout = tick_rate.checked_sub(last_tick.elapsed()).unwrap_or(Duration::ZERO);
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                let effects = handle_key_event(app, key);

                if effects.open_preview {
                    if let Err(err) = web_preview::open_in_browser() {
                        app.error = Some(format!(
                            "Failed to open browser preview ({}): {err}",
                            web_preview::preview_path().display()
                        ));
                    }
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
        }
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
    fn esc_dismisses_error_before_quit() {
        let mut app = App::new();
        app.error = Some("error".to_string());

        let dismiss = handle_key_event(&mut app, key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.error.is_none());
        assert!(!dismiss.quit);

        let quit = handle_key_event(&mut app, key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(quit.quit);
    }

    #[test]
    fn f1_opens_keybindings_popup() {
        let mut app = App::new();
        handle_key_event(&mut app, key(KeyCode::F(1), KeyModifiers::NONE));
        assert!(app.show_keybindings);
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
}
