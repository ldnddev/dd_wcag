//! # UI Module
//!
//! Renders the application's inputs, tabbed content, and help bar.

use crate::app::{ActiveTab, App, InputTarget};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let size = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(size);

    render_inputs(frame, app, chunks[0]);
    render_middle(frame, app, chunks[1]);
    render_help(frame, chunks[2]);

    if let Some(error) = &app.error {
        let popup = centered_rect(size, 50, 20);
        let error_widget = Paragraph::new(error.as_str())
            .block(
                Block::default()
                    .title("Error")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red)),
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        frame.render_widget(error_widget, popup);
    }
}

fn render_inputs(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3)])
        .split(area);

    let fg_active = app.input_target == InputTarget::Foreground;
    let fg_title = if fg_active { "FG (active)" } else { "FG" };
    let fg_text = if fg_active {
        app.current_input.clone()
    } else {
        app.foreground_input.clone()
    };
    frame.render_widget(
        Paragraph::new(fg_text).block(
            Block::default()
                .title(fg_title)
                .borders(Borders::ALL)
                .border_style(if fg_active {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }),
        ),
        rows[0],
    );

    let bg_active = app.input_target == InputTarget::Background;
    let bg_title = if bg_active { "BG (active)" } else { "BG" };
    let bg_text = if bg_active {
        app.current_input.clone()
    } else {
        app.background_input.clone()
    };
    frame.render_widget(
        Paragraph::new(bg_text).block(
            Block::default()
                .title(bg_title)
                .borders(Borders::ALL)
                .border_style(if bg_active {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }),
        ),
        rows[1],
    );
}

fn render_middle(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    let titles = ["Input", "Conversions", "Contrast", "Preview"];
    let selected = match app.active_tab {
        ActiveTab::Input => 0,
        ActiveTab::Conversions => 1,
        ActiveTab::Contrast => 2,
        ActiveTab::Preview => 3,
    };

    let tabs = Tabs::new(titles)
        .select(selected)
        .block(Block::default().borders(Borders::ALL).title("Tabs"))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    frame.render_widget(tabs, chunks[0]);

    match app.active_tab {
        ActiveTab::Input => render_input_tab(frame, app, chunks[1]),
        ActiveTab::Conversions => render_conversions_tab(frame, app, chunks[1]),
        ActiveTab::Contrast => render_contrast_tab(frame, app, chunks[1]),
        ActiveTab::Preview => render_preview_tab(frame, app, chunks[1]),
    }
}

fn render_input_tab(frame: &mut Frame, app: &App, area: Rect) {
    let target = match app.input_target {
        InputTarget::Foreground => "Foreground",
        InputTarget::Background => "Background",
        InputTarget::None => "None",
    };

    let body = vec![
        Line::from(format!("Current target: {target}")),
        Line::from(format!("Current input: {}", app.current_input)),
    ];

    frame.render_widget(
        Paragraph::new(body).block(Block::default().title("Input").borders(Borders::ALL)),
        area,
    );
}

fn render_conversions_tab(frame: &mut Frame, app: &App, area: Rect) {
    let lines = vec![
        Line::from("Format | Foreground | Background"),
        Line::from(format!(
            "Hex | {} | {}",
            app.foreground.to_hex(),
            app.background.to_hex()
        )),
        Line::from(format!(
            "RGB | {} | {}",
            app.foreground.to_rgb_str(),
            app.background.to_rgb_str()
        )),
        Line::from(format!(
            "HSL | {} | {}",
            app.foreground.to_hsl_str(),
            app.background.to_hsl_str()
        )),
    ];

    frame.render_widget(
        Paragraph::new(lines).block(Block::default().title("Conversions").borders(Borders::ALL)),
        area,
    );
}

fn render_contrast_tab(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![Line::from(
        "Size | Normal Ratio | Normal Pass | Bold Ratio | Bold Pass",
    )];

    let size = app.font_size_px as f32;
    let ratio = app.contrast_ratio;
    let normal_pass = app.passes_aa(size, false, ratio);
    let bold_pass = app.passes_aa(size, true, ratio);

    lines.push(Line::from(vec![
        Span::raw(format!("{size:.0}px | ")),
        Span::styled(
            format!("{ratio:.2}"),
            Style::default().fg(if normal_pass { Color::Green } else { Color::Red }),
        ),
        Span::raw(" | "),
        Span::styled(
            if normal_pass { "PASS" } else { "FAIL" },
            Style::default().fg(if normal_pass { Color::Green } else { Color::Red }),
        ),
        Span::raw(" | "),
        Span::styled(
            format!("{ratio:.2}"),
            Style::default().fg(if bold_pass { Color::Green } else { Color::Red }),
        ),
        Span::raw(" | "),
        Span::styled(
            if bold_pass { "PASS" } else { "FAIL" },
            Style::default().fg(if bold_pass { Color::Green } else { Color::Red }),
        ),
    ]));

    frame.render_widget(
        Paragraph::new(lines).block(Block::default().title("Contrast").borders(Borders::ALL)),
        area,
    );
}

fn render_preview_tab(frame: &mut Frame, app: &App, area: Rect) {
    let mut style = app.foreground.to_style().bg(app.background.to_tui_color());
    let font_size = app.font_size_px;
    let weight = if app.is_bold { "bold" } else { "normal" };

    if app.is_bold {
        style = style.add_modifier(Modifier::BOLD);
    }

    frame.render_widget(
        Paragraph::new(app.preview_text.as_str())
            .style(style)
            .block(
                Block::default()
                    .title(format!("Preview ({font_size}px, {weight})"))
                    .borders(Borders::ALL),
            ),
        area,
    );
}

fn render_help(frame: &mut Frame, area: Rect) {
    let help = Paragraph::new(
        "Tab/Shift+Tab: cycle+apply FG/BG | Ctrl+Up/Down: size (+/-1px) | Ctrl+B: bold | Ctrl+O: open web preview | Ctrl+Q/Esc: quit",
    )
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::TOP));
    frame.render_widget(help, area);
}

fn centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
