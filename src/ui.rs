//! # UI Module (Phase 1)
//!
//! This module handles rendering the TUI using Ratatui.
//! In Phase 1, it renders an empty frame with a title block
//! and a basic help bar at the bottom.

use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

// Main render function (updated for Phase 2 basic layout)
pub fn render(frame: &mut Frame, app: &App) {
    let size = frame.area();

    // Vertical split: inputs (top), preview/contrast (middle), help (bottom)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(size);

    // Top: Input area
    render_inputs(frame, app, chunks[0]);

    // Middle: Preview and simple contrast table
    render_preview_and_contrast(frame, app, chunks[1]);

    // Bottom: Help bar (updated with more keys)
    render_help(frame, chunks[2]);
}

// Renders FG/BG input fields with current values and active indicator
fn render_inputs(frame: &mut Frame, app: &App, area: Rect) {
    let input_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2)])
        .split(area);

    // Foreground
    let fg_title = if app.input_target == InputTarget::Foreground { "FG (active)" } else { "FG" };
    let fg_block = Block::default().title(fg_title).borders(Borders::ALL);
    let fg_text = Paragraph::new(app.foreground.to_hex());
    frame.render_widget(fg_text.block(fg_block), input_chunks[0]);

    // Background
    let bg_title = if app.input_target == InputTarget::Background { "BG (active)" } else { "BG" };
    let bg_block = Block::default().title(bg_title).borders(Borders::ALL);
    let bg_text = Paragraph::new(app.background.to_hex());
    frame.render_widget(bg_text.block(bg_block), input_chunks[1]);
}

// Renders preview with FG/BG styles and simple contrast info
fn render_preview_and_contrast(frame: &mut Frame, app: &App, area: Rect) {
    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    // Preview paragraph with styles
    let mut preview_style = Style::default();
    if let Some(fg) = app.parsed_fg {
        preview_style = preview_style.fg(fg.to_style().fg.unwrap_or(Color::White));
    }
    if let Some(bg) = app.parsed_bg {
        preview_style = preview_style.bg(bg.to_style().fg.unwrap_or(Color::Black)); // Use fg as temp bg
    }
    if app.is_bold {
        preview_style = preview_style.add_modifier(Modifier::BOLD);
    }
    let preview = Paragraph::new(app.preview_text.as_str())
        .style(preview_style)
        .block(Block::default().title("Preview").borders(Borders::ALL));
    frame.render_widget(preview, middle_chunks[0]);

    // Simple contrast table
    let mut table_lines = vec![Line::from("Size | Pass")];
    for (i, size) in FONT_SIZES.iter().enumerate() {
        let pass = app.passes_aa(*size, app.is_bold, app.contrast_ratio);
        table_lines.push(Line::from(format!("{}px | {}", size, if pass { "PASS" } else { "FAIL" })));
    }
    let table = Paragraph::new(table_lines)
        .block(Block::default().title(format!("Contrast: {:.2}", app.contrast_ratio)).borders(Borders::ALL));
    frame.render_widget(table, middle_chunks[1]);
}

// Help bar (updated with architecture spec text)
fn render_help(frame: &mut Frame, area: Rect) {
    let help_text = Line::from("Tab: switch FG/BG | Enter: edit | Arrow: size | B: toggle bold | q/Esc: quit");
    let help = Paragraph::new(help_text)
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::TOP));
    frame.render_widget(help, area);
}

// Error display if any
if let Some(error) = &app.error {
    let error_area = Rect::new(size.width / 4, size.height / 4, size.width / 2, 3);
    let error_block = Block::default().title("Error").borders(Borders::ALL).style(Style::default().fg(Color::Red));
    let error_p = Paragraph::new(error.as_str()).block(error_block);
    frame.render_widget(Clear, error_area); // Clear background
    frame.render_widget(error_p, error_area);
}

// Renders preview paragraph and simple contrast table
fn render_preview_and_contrast(frame: &mut Frame, app: &App, area: Rect) {
    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Preview
    let preview_style = if let (Some(fg), Some(bg)) = (app.parsed_fg, app.parsed_bg) {
        fg.to_style().bg(bg.to_style().fg) // Simplified for basic preview
    } else {
        Style::default()
    };
    if app.is_bold {
        preview_style = preview_style.add_modifier(Modifier::BOLD);
    }
    let preview = Paragraph::new(app.preview_text.as_str())
        .style(preview_style)
        .block(Block::default().title("Preview").borders(Borders::ALL));
    frame.render_widget(preview, middle_chunks[0]);

    // Simple contrast table
    let table_text = format!(
        "Ratio: {:.2}\nPass AA: {}",
        app.contrast_ratio,
        if app.passes_aa(
            app::FONT_SIZES[app.font_size_idx],
            app.is_bold,
            app.contrast_ratio
        ) {
            "Yes"
        } else {
            "No"
        }
    );
    let table =
        Paragraph::new(table_text).block(Block::default().title("Contrast").borders(Borders::ALL));
    frame.render_widget(table, middle_chunks[1]);
}

// Updated help bar with Phase 2 keys
fn render_help(frame: &mut Frame, area: Rect) {
    let help_text =
        Line::from("Tab: switch FG/BG | Enter: submit | Arrows: size | B: bold | q/Esc: quit");
    let help = Paragraph::new(help_text)
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::TOP));
    frame.render_widget(help, area);
}
