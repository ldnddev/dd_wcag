//! # UI Module
//!
//! Renders the application's inputs, tabbed content, and help bar.

use crate::app::{ActiveTab, App, InputTarget};
use crate::palette::PALETTE_EXPORT_PATH;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let size = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(app.theme.base_background_color())),
        size,
    );

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

    if app.show_keybindings {
        render_keybindings_popup(frame, app, size);
    }

    if app.show_theme_debug {
        render_theme_debug_popup(frame, app, size);
    }

    if !app.show_keybindings && !app.show_theme_debug {
        if let Some((x, y)) = input_cursor_position(size, app) {
            frame.set_cursor_position((x, y));
        }
    }

    render_toast(frame, app, size);
}

fn render_inputs(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3)])
        .split(area);

    let top_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    let bottom_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    let fg_active = app.input_target == InputTarget::Foreground;
    let fg_title = if fg_active { "FG (active)" } else { "FG" };
    let fg_text = if fg_active {
        app.current_input.clone()
    } else {
        app.foreground_input.clone()
    };
    render_input_field(frame, app, top_cols[0], fg_title, fg_text, fg_active);

    let bg_active = app.input_target == InputTarget::Background;
    let bg_title = if bg_active { "BG (active)" } else { "BG" };
    let bg_text = if bg_active {
        app.current_input.clone()
    } else {
        app.background_input.clone()
    };
    render_input_field(frame, app, top_cols[1], bg_title, bg_text, bg_active);

    let preview_active = app.input_target == InputTarget::PreviewText;
    let preview_title = if preview_active {
        "PreviewText (active)"
    } else {
        "PreviewText"
    };
    let preview_text = if preview_active {
        app.current_input.clone()
    } else {
        app.preview_text.replace('\n', "\\n")
    };
    render_input_field(
        frame,
        app,
        bottom_cols[0],
        preview_title,
        preview_text,
        preview_active,
    );

    let font_active = app.input_target == InputTarget::FontFamily;
    let font_title = if font_active {
        "FontFamily (active)"
    } else {
        "FontFamily"
    };
    let font_text = if font_active {
        app.current_input.clone()
    } else {
        app.preview_font_family.clone()
    };
    render_input_field(
        frame,
        app,
        bottom_cols[1],
        font_title,
        font_text,
        font_active,
    );
}

fn render_input_field(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    title: &str,
    text: String,
    active: bool,
) {
    let title_style = if active {
        Style::default().fg(app.theme.text_active_focus_color())
    } else {
        Style::default().fg(app.theme.text_labels_color())
    };
    let border_style = if active {
        Style::default().fg(app.theme.input_border_focus_color())
    } else {
        Style::default().fg(app.theme.input_border_default_color())
    };
    let input_style = if active {
        Style::default()
            .fg(app.theme.input_text_focus_color())
            .bg(app.theme.body_background_color())
    } else {
        Style::default()
            .fg(app.theme.input_text_default_color())
            .bg(app.theme.body_background_color())
    };

    frame.render_widget(
        Paragraph::new(text).style(input_style).block(
            Block::default()
                .title(Line::styled(title.to_string(), title_style))
                .borders(Borders::ALL)
                .border_style(border_style)
                .style(Style::default().bg(app.theme.body_background_color())),
        ),
        area,
    );
}

fn render_middle(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    let titles = ["Input", "Conversions", "Contrast", "Preview", "Palette"];
    let selected = match app.active_tab {
        ActiveTab::Input => 0,
        ActiveTab::Conversions => 1,
        ActiveTab::Contrast => 2,
        ActiveTab::Preview => 3,
        ActiveTab::Palette => 4,
    };

    let tabs = Tabs::new(titles)
        .select(selected)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Line::styled(
                    "Tabs",
                    Style::default().fg(app.theme.text_labels_color()),
                ))
                .border_style(Style::default().fg(app.theme.border_default_color()))
                .style(Style::default().bg(app.theme.body_background_color())),
        )
        .highlight_style(
            Style::default()
                .fg(app.theme.text_active_focus_color())
                .bg(app.theme.selected_background_color())
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, chunks[0]);

    match app.active_tab {
        ActiveTab::Input => render_input_tab(frame, app, chunks[1]),
        ActiveTab::Conversions => render_conversions_tab(frame, app, chunks[1]),
        ActiveTab::Contrast => render_contrast_tab(frame, app, chunks[1]),
        ActiveTab::Preview => render_preview_tab(frame, app, chunks[1]),
        ActiveTab::Palette => render_palette_tab(frame, app, chunks[1]),
    }
}

fn render_input_tab(frame: &mut Frame, app: &App, area: Rect) {
    let target = match app.input_target {
        InputTarget::Foreground => "Foreground",
        InputTarget::Background => "Background",
        InputTarget::PreviewText => "Preview Text",
        InputTarget::FontFamily => "Font Family",
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
        "Preview font family: {}",
        app.preview_font_family
    )));
    body.push(Line::from(""));
    body.push(Line::from(format!(
        "Last parsed format: {}",
        app.last_parsed_format.as_deref().unwrap_or("-")
    )));

    frame.render_widget(
        Paragraph::new(body)
            .style(
                Style::default()
                    .fg(app.theme.text_primary_color())
                    .bg(app.theme.body_background_color()),
            )
            .block(
                Block::default()
                    .title(Line::styled(
                        "Input",
                        Style::default().fg(app.theme.text_labels_color()),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.border_default_color()))
                    .style(Style::default().bg(app.theme.body_background_color())),
            ),
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
        Paragraph::new(lines)
            .style(
                Style::default()
                    .fg(app.theme.text_primary_color())
                    .bg(app.theme.body_background_color()),
            )
            .block(
                Block::default()
                    .title(Line::styled(
                        "Conversions",
                        Style::default().fg(app.theme.text_labels_color()),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.border_default_color()))
                    .style(Style::default().bg(app.theme.body_background_color())),
            ),
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

    let lc = app.foreground.apca_lc(&app.background);
    let apca_pass =
        app.foreground
            .apca_passes(&app.background, app.font_size_px.into(), app.is_bold);
    let apca_threshold = if size <= 12.0 {
        if app.is_bold {
            75.0
        } else {
            90.0
        }
    } else if size <= 18.0 {
        if app.is_bold {
            60.0
        } else {
            75.0
        }
    } else if size <= 24.0 {
        if app.is_bold {
            45.0
        } else {
            60.0
        }
    } else {
        if app.is_bold {
            30.0
        } else {
            45.0
        }
    };

    let mut lines = vec![
        Line::from("WCAG Current Result"),
        Line::from(vec![
            Span::raw(format!(
                "{size:.0}px {weight} | ratio {ratio:.2} | needs >= {current_threshold} | "
            )),
            Span::styled(
                if current_pass { "PASS" } else { "FAIL" },
                Style::default().fg(if current_pass {
                    app.theme.success_color()
                } else {
                    app.theme.error_color()
                }),
            ),
        ]),
        Line::from(""),
        Line::from("APCA Current Result"),
        Line::from(vec![
            Span::raw(format!(
                "{size:.0}px {weight} | Lc {lc:.2} | needs >= {apca_threshold:.0} | "
            )),
            Span::styled(
                if apca_pass { "PASS" } else { "FAIL" },
                Style::default().fg(if apca_pass {
                    app.theme.success_color()
                } else {
                    app.theme.error_color()
                }),
            ),
        ]),
        Line::from(""),
        Line::from(format!("WCAG Quick Reference ({weight})")),
        Line::from("Size | Ratio | Result"),
    ];

    for quick_size in [12.0_f32, 14.0, 16.0, 18.0] {
        let quick_pass = app.passes_aa(quick_size, app.is_bold, ratio);
        lines.push(Line::from(vec![
            Span::raw(format!("{quick_size:.0}px | {ratio:.2} | ")),
            Span::styled(
                if quick_pass { "PASS" } else { "FAIL" },
                Style::default().fg(if quick_pass {
                    app.theme.success_color()
                } else {
                    app.theme.error_color()
                }),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(format!("APCA Quick Reference ({weight})")));
    lines.push(Line::from("Size | Lc | Result"));

    for quick_size in [12, 14, 16, 18, 24] {
        let quick_apca_pass = app
            .foreground
            .apca_passes(&app.background, quick_size, app.is_bold);
        lines.push(Line::from(vec![
            Span::raw(format!("{quick_size}px | {lc:.2} | ")),
            Span::styled(
                if quick_apca_pass { "PASS" } else { "FAIL" },
                Style::default().fg(if quick_apca_pass {
                    app.theme.success_color()
                } else {
                    app.theme.error_color()
                }),
            ),
        ]));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .style(
                Style::default()
                    .fg(app.theme.text_primary_color())
                    .bg(app.theme.body_background_color()),
            )
            .block(
                Block::default()
                    .title(Line::styled(
                        "Contrast",
                        Style::default().fg(app.theme.text_labels_color()),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.border_default_color()))
                    .style(Style::default().bg(app.theme.body_background_color())),
            ),
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
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.border_default_color())),
            ),
        area,
    );
}

fn render_palette_tab(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);

    render_palette_inputs(frame, app, chunks[0]);
    render_palette_detail(frame, app, chunks[1]);
}

fn render_palette_inputs(frame: &mut Frame, app: &App, area: Rect) {
    let selected = app.palette.selected();
    let mut lines = Vec::new();
    for input in crate::palette::PaletteInput::ALL {
        let marker = if input == selected { ">" } else { " " };
        let required = if input.required() { "*" } else { " " };
        let value = if app.palette.editing && input == selected {
            app.palette.edit_input.as_str()
        } else {
            app.palette.input_for(input)
        };
        let style = if input == selected {
            Style::default()
                .fg(app.theme.text_active_focus_color())
                .bg(app.theme.selected_background_color())
        } else {
            Style::default().fg(app.theme.text_primary_color())
        };
        lines.push(Line::styled(
            format!("{marker} {:<9}{required} {value}", input.label()),
            style,
        ));
    }

    lines.extend([
        Line::from(""),
        Line::from("* required"),
        Line::from(""),
        Line::from("Enter: edit"),
        Line::from("G: generate"),
        Line::from("Up/Down: select or scroll"),
        Line::from("F then G: apply to FG"),
        Line::from("B then G: apply to BG"),
        Line::from(format!("Ctrl+S: save {PALETTE_EXPORT_PATH}")),
        Line::from("Ctrl+C: copy values"),
    ]);

    frame.render_widget(
        Paragraph::new(lines)
            .style(
                Style::default()
                    .fg(app.theme.text_primary_color())
                    .bg(app.theme.body_background_color()),
            )
            .block(
                Block::default()
                    .title(Line::styled(
                        "Palette Inputs",
                        Style::default().fg(app.theme.text_labels_color()),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.border_default_color()))
                    .style(Style::default().bg(app.theme.body_background_color())),
            ),
        area,
    );
}

fn render_palette_detail(frame: &mut Frame, app: &App, area: Rect) {
    let selected = app.palette.selected();
    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled(
                selected.label(),
                Style::default()
                    .fg(app.theme.text_active_focus_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" theme builder"),
        ]),
        Line::from(format!("Base: {}", app.palette.selected_input())),
    ];

    match crate::palette::parse_palette_color(app.palette.selected_input()) {
        Ok(color) => {
            lines.push(Line::from(format!("Hex:  {}", color.to_hex())));
            lines.push(Line::from(format!("RGB:  {}", color.to_rgb_str())));
            lines.push(Line::from(format!("HSL:  {}", color.to_hsl_str())));
        }
        Err(err) => {
            lines.push(Line::styled(
                err,
                Style::default().fg(app.theme.error_color()),
            ));
        }
    }

    lines.push(Line::from(""));

    if let Some(generated) = &app.palette.generated {
        let blocking = generated.blocking_failures();
        let advisory = generated.advisory_failures();
        lines.push(Line::from(format!(
            "Generated: {} tokens | {} blocking failure(s) | {} advisory warning(s)",
            generated.tokens.len(),
            blocking.len(),
            advisory.len()
        )));

        lines.push(Line::from(""));
        lines.push(Line::from("Generated tokens:"));
        let prefix = format!("$c_{}", selected.label().to_lowercase());
        for token in generated
            .tokens
            .iter()
            .filter(|token| token.name.starts_with(&prefix))
        {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<38}", token.name),
                    Style::default().fg(app.theme.text_labels_color()),
                ),
                Span::styled(
                    token.color.to_hex(),
                    Style::default().fg(theme_color(&token.color.to_hex())),
                ),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from("All generated variables:"));
        for token in &generated.tokens {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:<42}", token.name),
                    Style::default().fg(app.theme.text_labels_color()),
                ),
                Span::styled(
                    token.color.to_hex(),
                    Style::default().fg(theme_color(&token.color.to_hex())),
                ),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from("Compliance checks:"));
        for check in &generated.checks {
            lines.push(Line::from(vec![
                Span::styled(
                    if check.passes { "PASS " } else { "FAIL " },
                    Style::default().fg(if check.passes {
                        app.theme.success_color()
                    } else {
                        app.theme.error_color()
                    }),
                ),
                Span::raw(format!(
                    "{:.2}:1 >= {:.1}:1 {}",
                    check.ratio, check.threshold, check.label
                )),
            ]));
        }
    } else {
        lines.extend([
            Line::from("Generated: none"),
            Line::from(""),
            Line::from("Press G to generate a compliant _palette.scss draft."),
            Line::from("Text roles are fixed and will not be changed."),
        ]);
    }

    if let Some(target) = app.palette.pending_apply {
        lines.push(Line::from(""));
        lines.push(Line::styled(
            format!(
                "Pending: press G to apply selected color to {}",
                target.label()
            ),
            Style::default().fg(app.theme.info_color()),
        ));
    }

    let visible_height = area.height.saturating_sub(2).max(1) as usize;
    let max_scroll = lines.len().saturating_sub(visible_height);
    let scroll = app.palette.detail_scroll.min(max_scroll);
    let mut visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .collect();
    if max_scroll > 0 && !visible_lines.is_empty() {
        visible_lines[0] = Line::from(vec![
            Span::styled(
                format!("Scroll {}/{}  ", scroll + 1, max_scroll + 1),
                Style::default().fg(app.theme.text_secondary_color()),
            ),
            Span::raw("Up/Down"),
        ]);
    }

    frame.render_widget(
        Paragraph::new(visible_lines)
            .style(
                Style::default()
                    .fg(app.theme.text_primary_color())
                    .bg(app.theme.body_background_color()),
            )
            .block(
                Block::default()
                    .title(Line::styled(
                        "Selected / Generated Detail",
                        Style::default().fg(app.theme.text_labels_color()),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.border_default_color()))
                    .style(Style::default().bg(app.theme.body_background_color())),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_help(frame: &mut Frame, app: &App, area: Rect) {
    let focus = match app.active_tab {
        ActiveTab::Input => match app.input_target {
            InputTarget::Foreground => "Input > FG",
            InputTarget::Background => "Input > BG",
            InputTarget::PreviewText => "Input > PreviewText",
            InputTarget::FontFamily => "Input > FontFamily",
            InputTarget::None => "Input",
        },
        ActiveTab::Conversions => "Conversions",
        ActiveTab::Contrast => "Contrast",
        ActiveTab::Preview => "Preview",
        ActiveTab::Palette => {
            if app.palette.editing {
                "Palette > Edit"
            } else {
                "Palette"
            }
        }
    };

    let help = Paragraph::new(format!(
        "Focus: {focus} | Theme: {} | F1: keybindings | F2: theme",
        app.theme_source.label()
    ))
    .alignment(Alignment::Center)
    .style(
        Style::default()
            .fg(app.theme.text_secondary_color())
            .bg(app.theme.base_background_color()),
    )
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(app.theme.border_default_color()))
            .style(Style::default().bg(app.theme.base_background_color())),
    );
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

fn bottom_right_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width.saturating_sub(2)).max(20);
    let height = height.min(area.height.saturating_sub(2)).max(3);
    Rect {
        x: area.x.saturating_add(area.width.saturating_sub(width + 1)),
        y: area
            .y
            .saturating_add(area.height.saturating_sub(height + 1)),
        width,
        height,
    }
}

fn render_toast(frame: &mut Frame, app: &App, area: Rect) {
    let (title, message, border_color) = if let Some(error) = &app.error {
        ("Error", error.as_str(), app.theme.error_color())
    } else if let Some(status) = &app.status {
        ("Status", status.as_str(), app.theme.info_color())
    } else {
        return;
    };

    let line_count = message.lines().count().max(1) as u16;
    let toast = bottom_right_rect(area, 54, line_count.saturating_add(2).clamp(3, 7));
    frame.render_widget(Clear, toast);
    let widget = Paragraph::new(message)
        .style(
            Style::default()
                .fg(app.theme.modal_text_color())
                .bg(app.theme.modal_background_color()),
        )
        .block(
            Block::default()
                .title(Line::styled(
                    title,
                    Style::default().fg(app.theme.modal_labels_color()),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(app.theme.modal_background_color())),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, toast);
}

fn render_keybindings_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(area, 80, 70);
    frame.render_widget(Clear, popup);
    let lines = vec![
        Line::from("Navigation"),
        Line::from("Tab / Shift+Tab: cycle focus and auto-apply FG/BG/PreviewText/FontFamily"),
        Line::from("Left / Right: move cursor in active input field"),
        Line::from("Enter: insert newline when focus is PreviewText"),
        Line::from("Backspace: delete before cursor in active input field"),
        Line::from(""),
        Line::from("Actions"),
        Line::from("Ctrl+Up / Ctrl+Down: increase/decrease font size (6..=120)"),
        Line::from("Ctrl+B: toggle bold"),
        Line::from("Ctrl+F: toggle preset Google font family"),
        Line::from(format!("Ctrl+S: save palette to {PALETTE_EXPORT_PATH}")),
        Line::from("Ctrl+C: copy generated palette values"),
        Line::from("Palette: F then G applies selected color to FG"),
        Line::from("Palette: B then G applies selected color to BG"),
        Line::from("F1: open keybindings popup"),
        Line::from("F2: open theme debug popup"),
        Line::from("Ctrl+O: open web preview (/tmp/dd_wcag_preview.html)"),
        Line::from("Ctrl+Q: quit"),
        Line::from("Esc: close this popup (or close error / quit when popup is not open)"),
    ];

    let widget = Paragraph::new(lines)
        .style(
            Style::default()
                .fg(app.theme.modal_text_color())
                .bg(app.theme.modal_background_color()),
        )
        .block(
            Block::default()
                .title(Line::styled(
                    "Keybindings",
                    Style::default().fg(app.theme.modal_labels_color()),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.border_active_color()))
                .style(Style::default().bg(app.theme.modal_background_color())),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, popup);
}

fn render_theme_debug_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(area, 72, 82);
    frame.render_widget(Clear, popup);

    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                "Source:  ",
                Style::default().fg(app.theme.modal_labels_color()),
            ),
            Span::styled(
                app.theme_source.label(),
                Style::default().fg(app.theme.modal_text_color()),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Version: ",
                Style::default().fg(app.theme.modal_labels_color()),
            ),
            Span::styled(
                app.theme.version.to_string(),
                Style::default().fg(app.theme.modal_text_color()),
            ),
        ]),
        Line::from(""),
    ];

    for (key, value) in app.theme.tokens() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{key:<22}"),
                Style::default().fg(app.theme.modal_labels_color()),
            ),
            Span::styled(value.to_string(), Style::default().fg(theme_color(value))),
        ]));
    }

    let widget = Paragraph::new(lines)
        .style(
            Style::default()
                .fg(app.theme.modal_text_color())
                .bg(app.theme.modal_background_color()),
        )
        .block(
            Block::default()
                .title(Line::styled(
                    "Theme",
                    Style::default().fg(app.theme.modal_labels_color()),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.border_active_color()))
                .style(Style::default().bg(app.theme.modal_background_color())),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, popup);
}

fn theme_color(input: &str) -> ratatui::style::Color {
    let s = input.trim().strip_prefix('#').unwrap_or(input.trim());
    if s.len() != 6 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return ratatui::style::Color::Reset;
    }
    let Some(r) = u8::from_str_radix(&s[0..2], 16).ok() else {
        return ratatui::style::Color::Reset;
    };
    let Some(g) = u8::from_str_radix(&s[2..4], 16).ok() else {
        return ratatui::style::Color::Reset;
    };
    let Some(b) = u8::from_str_radix(&s[4..6], 16).ok() else {
        return ratatui::style::Color::Reset;
    };
    ratatui::style::Color::Rgb(r, g, b)
}

fn input_cursor_position(size: Rect, app: &App) -> Option<(u16, u16)> {
    if app.active_tab == ActiveTab::Palette && app.palette.editing {
        return palette_cursor_position(size, app);
    }

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

    let top_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    let bottom_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    let (cursor_row, cursor_col) = app.cursor_line_col();

    let (base_x, base_y, max_x, max_y) = match app.input_target {
        InputTarget::Foreground => (
            top_cols[0].x.saturating_add(1),
            top_cols[0].y.saturating_add(1),
            top_cols[0]
                .x
                .saturating_add(top_cols[0].width.saturating_sub(2)),
            top_cols[0]
                .y
                .saturating_add(top_cols[0].height.saturating_sub(2)),
        ),
        InputTarget::Background => (
            top_cols[1].x.saturating_add(1),
            top_cols[1].y.saturating_add(1),
            top_cols[1]
                .x
                .saturating_add(top_cols[1].width.saturating_sub(2)),
            top_cols[1]
                .y
                .saturating_add(top_cols[1].height.saturating_sub(2)),
        ),
        InputTarget::PreviewText => (
            bottom_cols[0].x.saturating_add(1),
            bottom_cols[0].y.saturating_add(1),
            bottom_cols[0]
                .x
                .saturating_add(bottom_cols[0].width.saturating_sub(2)),
            bottom_cols[0]
                .y
                .saturating_add(bottom_cols[0].height.saturating_sub(2)),
        ),
        InputTarget::FontFamily => (
            bottom_cols[1].x.saturating_add(1),
            bottom_cols[1].y.saturating_add(1),
            bottom_cols[1]
                .x
                .saturating_add(bottom_cols[1].width.saturating_sub(2)),
            bottom_cols[1]
                .y
                .saturating_add(bottom_cols[1].height.saturating_sub(2)),
        ),
        InputTarget::None => return None,
    };

    let (x, y) = if app.input_target == InputTarget::PreviewText {
        let width = bottom_cols[0].width.saturating_sub(2).max(1);
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

fn palette_cursor_position(size: Rect, app: &App) -> Option<(u16, u16)> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(size);

    let middle_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(chunks[1]);

    let palette_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(middle_chunks[1]);

    let row = app.palette.selected_idx.min(3) as u16;
    let input_start_col = 13;
    let x = palette_chunks[0]
        .x
        .saturating_add(1 + input_start_col)
        .saturating_add(app.palette.cursor_col())
        .min(
            palette_chunks[0]
                .x
                .saturating_add(palette_chunks[0].width.saturating_sub(2)),
        );
    let y = palette_chunks[0].y.saturating_add(1).saturating_add(row);
    Some((x, y))
}
