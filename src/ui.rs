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
    render_help(frame, app, chunks[2]);

    if let Some(error) = &app.error {
        let popup = centered_rect(size, 50, 20);
        let error_widget = Paragraph::new(vec![
            Line::from(error.as_str()),
            Line::from(""),
            Line::from("Press Esc to dismiss."),
        ])
            .block(
                Block::default()
                    .title("Error (Esc closes)")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red)),
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        frame.render_widget(error_widget, popup);
    }

    if app.error.is_none() {
        if let Some((x, y)) = input_cursor_position(size, app) {
            frame.set_cursor_position((x, y));
        }
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
        InputTarget::PreviewText => "Preview Text",
        InputTarget::None => "None",
    };

    let mut body = vec![
        Line::from(format!("Current target: {target}")),
        Line::from("Current input:"),
    ];

    body.extend(app.current_input.lines().map(Line::from));
    if app.current_input.is_empty() {
        body.push(Line::from(""));
    }

    body.push(Line::from(""));
    body.push(Line::from("Preview text:"));
    body.extend(app.preview_text.lines().map(Line::from));
    if app.preview_text.is_empty() {
        body.push(Line::from(""));
    }

    body.push(Line::from(""));
    body.push(Line::from(format!(
        "Last parsed format: {}",
        app.last_parsed_format.as_deref().unwrap_or("-")
    )));

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
    let size = app.font_size_px as f32;
    let ratio = app.contrast_ratio;
    let weight = if app.is_bold { "bold" } else { "normal" };
    let current_pass = app.passes_aa(size, app.is_bold, ratio);
    let current_threshold = if (app.is_bold && size >= 14.0) || (!app.is_bold && size >= 18.0) {
        "3.0"
    } else {
        "4.5"
    };

    let mut lines = vec![
        Line::from("Current Result"),
        Line::from(vec![
            Span::raw(format!(
                "{size:.0}px {weight} | ratio {ratio:.2} | needs >= {current_threshold} | "
            )),
            Span::styled(
                if current_pass { "PASS" } else { "FAIL" },
                Style::default().fg(if current_pass { Color::Green } else { Color::Red }),
            ),
        ]),
        Line::from(""),
        Line::from(format!("Quick Reference ({weight})")),
        Line::from("Size | Ratio | Result"),
    ];

    for quick_size in [12.0_f32, 14.0, 16.0, 18.0] {
        let quick_pass = app.passes_aa(quick_size, app.is_bold, ratio);
        lines.push(Line::from(vec![
            Span::raw(format!("{quick_size:.0}px | {ratio:.2} | ")),
            Span::styled(
                if quick_pass { "PASS" } else { "FAIL" },
                Style::default().fg(if quick_pass { Color::Green } else { Color::Red }),
            ),
        ]));
    }

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

fn render_help(frame: &mut Frame, app: &App, area: Rect) {
    let focus = match app.active_tab {
        ActiveTab::Input => match app.input_target {
            InputTarget::Foreground => "Input > FG",
            InputTarget::Background => "Input > BG",
            InputTarget::PreviewText => "Input > PreviewText",
            InputTarget::None => "Input",
        },
        ActiveTab::Conversions => "Conversions",
        ActiveTab::Contrast => "Contrast",
        ActiveTab::Preview => "Preview",
    };

    let help = Paragraph::new(format!(
        "Focus: {focus} | Tab/Shift+Tab: cycle+apply FG/BG/PreviewText | Enter: newline (PreviewText) | Left/Right: move cursor | Ctrl+Up/Down: size (+/-1px) | Ctrl+B: bold | Ctrl+O: open web preview (/tmp/dd_wcag_preview.html) | Ctrl+Q/Esc: quit"
    ))
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

fn input_cursor_position(size: Rect, app: &App) -> Option<(u16, u16)> {
    if app.active_tab != ActiveTab::Input || app.input_target == InputTarget::None {
        return None;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(size);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3)])
        .split(chunks[0]);

    let middle = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(chunks[1]);
    let input_tab_area = middle[1];

    let (cursor_row, cursor_col) = app.cursor_line_col();

    let (base_x, base_y, max_x, max_y) = match app.input_target {
        InputTarget::Foreground => (
            rows[0].x.saturating_add(1),
            rows[0].y.saturating_add(1),
            rows[0]
                .x
                .saturating_add(rows[0].width.saturating_sub(2)),
            rows[0]
                .y
                .saturating_add(rows[0].height.saturating_sub(2)),
        ),
        InputTarget::Background => (
            rows[1].x.saturating_add(1),
            rows[1].y.saturating_add(1),
            rows[1]
                .x
                .saturating_add(rows[1].width.saturating_sub(2)),
            rows[1]
                .y
                .saturating_add(rows[1].height.saturating_sub(2)),
        ),
        InputTarget::PreviewText => (
            input_tab_area
                .x
                .saturating_add(1)
                .saturating_add(0),
            input_tab_area.y.saturating_add(3),
            input_tab_area
                .x
                .saturating_add(input_tab_area.width.saturating_sub(2)),
            input_tab_area
                .y
                .saturating_add(input_tab_area.height.saturating_sub(2)),
        ),
        InputTarget::None => return None,
    };

    let (x, y) = if app.input_target == InputTarget::PreviewText {
        let width = input_tab_area.width.saturating_sub(2).max(1);
        let wrapped_rows = cursor_col / width;
        let wrapped_col = cursor_col % width;
        let y = base_y
            .saturating_add(cursor_row)
            .saturating_add(wrapped_rows)
            .min(max_y);
        let x = base_x.saturating_add(wrapped_col).min(max_x);
        (x, y)
    } else {
        let x = base_x.saturating_add(cursor_col).min(max_x);
        (x, base_y)
    };

    Some((x, y))
}
