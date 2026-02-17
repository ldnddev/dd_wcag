//! # dd_wcag - Phase 1: Basic Project Setup
//!
//! This is the minimal entry point for the TUI application.
//! In Phase 1, we set up the terminal with Crossterm and Ratatui,
//! create a basic event loop that quits on 'q', and render an empty frame.

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

// Import modules
mod app;
mod color;
mod ui;

use app::{App, InputTarget, ActiveTab};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// Main function
fn main() -> Result<()> {
    // Setup terminal
    let mut terminal = setup_terminal()?;

    // Create app state
    let mut app = App::new();

    // Run the main loop
    let res = run_loop(&mut terminal, &mut app);

    // Restore terminal
    restore_terminal(&mut terminal)?;

    res
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

// Main event loop with key handling
fn run_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::render(f, app))?;

        let timeout = tick_rate.checked_sub(last_tick.elapsed()).unwrap_or(Duration::ZERO);
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    // Quit
                    KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::NONE) => return Ok(()),
                    KeyCode::Esc => return Ok(()),

                    // Tab to switch input target and clear current input
                    KeyCode::Tab => {
                        app.input_target = match app.input_target {
                            InputTarget::Foreground => InputTarget::Background,
                            InputTarget::Background => InputTarget::Foreground,
                            InputTarget::None => InputTarget::Foreground,
                        };
                        app.current_input.clear();
                    }

                    // Enter to submit input
                    KeyCode::Enter => {
een app.submit_input();
                    }

                    // Char input for current field
                    KeyCode::Char(c) => {
                        if app.input_target != InputTarget::None {
                            app.current_input.push(c);
                        }
                    }

                    //green Backspace to delete char
                    KeyCode::Backspace => {
                        if app.input_target != InputTarget::None {
                            app.current_input.pop();
                        }
                    }

                    // B to toggle bold
                    KeyCode::Char('b') | KeyCode::Char('B') => {
                        app.is_bold = !app.is_bold;
                    }

                    // Arrows to cycle font size
                    KeyCode::Up => {
                        if app.font_size_idx > 0 {
                            app.font_size_idx -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if app.font_size_idx < app::FONT_SIZES.len() - 1 {
                            app.font_size_idx += 1;
                        }
                    }

                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

// Sets up the terminal in raw mode with alternate screen
fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(Into::into)
}

// Restores the terminal to normal mode
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

// Main event loop: Renders UI and handles 'q' to quit
fn run_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(ui::render)?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if let KeyCode::Char('q') = key.code {
                    return Ok(());
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}
