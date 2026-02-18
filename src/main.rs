//! # dd_wcag - Phase 1: Basic Project Setup
//!
//! This is the minimal entry point for the TUI application.
//! In Phase 1, we set up the terminal with Crossterm and Ratatui,
//! create a basic event loop that quits on 'q', and render an empty frame.

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
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
mod web_preview;

use app::{ActiveTab, App, InputTarget};

// Main function
fn main() -> Result<()> {
    // Setup terminal
    let mut terminal = setup_terminal()?;

    // Create app state
    let mut app = App::new();
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
                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

                if ctrl {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(()),

                        KeyCode::Char('b') | KeyCode::Char('B') => {
                            app.is_bold = !app.is_bold;
                            sync_web_preview(app);
                        },

                        KeyCode::Char('o') | KeyCode::Char('O') => {
                            if let Err(err) = web_preview::open_in_browser() {
                                app.error = Some(format!(
                                    "Failed to open browser preview ({}): {err}",
                                    web_preview::preview_path().display()
                                ));
                            }
                        },

                        KeyCode::Up => {
                            if app.active_tab == ActiveTab::Preview || app.active_tab == ActiveTab::Contrast {
                                app.adjust_font_size(1);
                                sync_web_preview(app);
                            }
                        },

                        KeyCode::Down => {
                            if app.active_tab == ActiveTab::Preview || app.active_tab == ActiveTab::Contrast {
                                app.adjust_font_size(-1);
                                sync_web_preview(app);
                            }
                        },

                        _ => {},
                    }

                    continue;
                }

                match key.code {
                    KeyCode::Tab => {
                        match app.active_tab {
                            ActiveTab::Input => match app.input_target {
                                InputTarget::Foreground => {
                                    app.sync_active_input();
                                    if !app.submit_input() {
                                        continue;
                                    }
                                    sync_web_preview(app);
                                    app.set_input_target(InputTarget::Background);
                                }
                                InputTarget::Background | InputTarget::None => {
                                    app.sync_active_input();
                                    if !app.submit_input() {
                                        continue;
                                    }
                                    sync_web_preview(app);
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
                        }
                    },

                    KeyCode::BackTab => {
                        match app.active_tab {
                            ActiveTab::Input => match app.input_target {
                                InputTarget::Background => {
                                    app.sync_active_input();
                                    if !app.submit_input() {
                                        continue;
                                    }
                                    sync_web_preview(app);
                                    app.set_input_target(InputTarget::Foreground);
                                }
                                InputTarget::Foreground | InputTarget::None => {
                                    app.sync_active_input();
                                    if !app.submit_input() {
                                        continue;
                                    }
                                    sync_web_preview(app);
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
                        }
                    },

                    KeyCode::Esc => {
                        if app.error.is_some() {
                            app.error = None;
                        } else {
                            return Ok(());
                        }
                    },

                    KeyCode::Char(c) => {
                        if app.active_tab == ActiveTab::Input && app.input_target != InputTarget::None {
                            app.current_input.push(c);
                            app.sync_active_input();
                        }
                    },

                    KeyCode::Backspace => {
                        if app.active_tab == ActiveTab::Input && app.input_target != InputTarget::None {
                            app.current_input.pop();
                            app.sync_active_input();
                        }
                    },

                    _ => {},
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}
