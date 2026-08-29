use crate::app::{App, FocusId, Mode};
use crate::contrast::render_contrast;
use crate::layout::{
    LayoutMap, breakpoint, caret_line, centered, split_body_with_fix, split_header, split_shell,
};

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(app.theme.base_background_color())),
        size,
    );

    let mut map = LayoutMap {
        breakpoint: breakpoint(size),
        ..LayoutMap::default()
    };

    let shell = split_shell(size);
    map.footer = shell.footer;
    map.body = shell.body;

    let header = split_header(shell.header);
    map.tabs_contrast = header.tabs_contrast;
    map.tabs_palette = header.tabs_palette;
    map.target_wcag = header.target_wcag;
    map.target_apca = header.target_apca;
    render_header(frame, app, shell.header, &map);

    let (main, fix) = split_body_with_fix(shell.body, map.breakpoint, app.fix_open);
    if let Some(fix_area) = fix {
        map.fix_area = fix_area;
        render_fix_placeholder(frame, app, fix_area);
    }

    match app.mode {
        Mode::Contrast => {
            let rects = render_contrast(frame, app, main, map.breakpoint);
            map.fg_input = rects.fg_input;
            map.fg_swatch = rects.fg_swatch;
            map.bg_input = rects.bg_input;
            map.bg_swatch = rects.bg_swatch;
            map.size_input = rects.size_input;
            map.size_dec = rects.size_dec;
            map.size_inc = rects.size_inc;
            map.weight_input = rects.weight_input;
            map.weight_dec = rects.weight_dec;
            map.weight_inc = rects.weight_inc;
            map.style_btns = rects.style_btns;
            map.preview_text = rects.preview_text;
            map.font_family = rects.font_family;
            map.swap_btn = rects.swap_btn;
            map.copy_btn = rects.copy_btn;
            map.fix_btn = rects.fix_btn;
            map.web_btn = rects.web_btn;
            map.preview = rects.preview;
            map.scores_wcag = rects.scores_wcag;
            map.scores_apca = rects.scores_apca;
            map.contrast_panel = rects.panel;
            map.contrast_scrollbar = rects.scrollbar;
            if app.contrast_max_scroll > 0 && rects.scrollbar.width > 0 {
                render_edge_scrollbar(
                    frame,
                    app,
                    rects.scrollbar,
                    app.contrast_scroll as usize,
                    app.contrast_max_scroll as usize,
                );
            }
        }
        Mode::Palette => {
            let (detail, scrollbar, roles) = render_palette_tab(frame, app, main);
            map.detail = detail;
            map.detail_scrollbar = scrollbar;
            map.role_rows = roles;
        }
    }

    render_footer(frame, app, shell.footer);

    if app.show_keybindings {
        let popup = centered(size, 84, 84);
        map.popup_area = Some(popup);
        render_keybindings_popup(frame, app, popup);
    } else if app.show_theme_debug {
        let popup = centered(size, 72, 82);
        map.popup_area = Some(popup);
        render_theme_debug_popup(frame, app, popup);
    }

    if !app.show_keybindings && !app.show_theme_debug {
        if let Some((x, y)) = cursor_position(app, &map) {
            frame.set_cursor_position((x, y));
        }
    }

    if let Some(toast) = render_toast(frame, app, size) {
        map.toast_area = Some(toast);
    }

    app.layout = map;
}

fn render_header(frame: &mut Frame, app: &App, area: Rect, map: &LayoutMap) {
    frame.render_widget(
        Block::default()
            .title("dd_wcag")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.border_active_color()))
            .style(Style::default().bg(app.theme.base_background_color())),
        area,
    );

    paint_tab(
        frame,
        app,
        map.tabs_contrast,
        "Contrast",
        app.mode == Mode::Contrast,
    );
    paint_tab(
        frame,
        app,
        map.tabs_palette,
        "Palette",
        app.mode == Mode::Palette,
    );
    paint_tab(
        frame,
        app,
        map.target_wcag,
        &format!("WCAG {} ▾", app.targets.wcag.label()),
        app.focus == FocusId::TargetWcag,
    );
    paint_tab(
        frame,
        app,
        map.target_apca,
        &format!("APCA {} ▾", app.targets.apca.label()),
        app.focus == FocusId::TargetApca,
    );
}

fn paint_tab(frame: &mut Frame, app: &App, area: Rect, label: &str, active: bool) {
    let style = if active {
        Style::default()
            .fg(app.theme.text_active_focus_color())
            .bg(app.theme.selected_background_color())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(app.theme.text_secondary_color())
    };
    frame.render_widget(Paragraph::new(label).style(style), area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let keys = if area.width < 75 {
        "F1:Help  F2:Theme  Tab:Focus  Ctrl+G:Gen  Ctrl+F:Fix  Ctrl+O:Web  Ctrl+Q:Quit"
    } else if area.width < 110 {
        "F1: Help   F2: Theme   Tab: Focus   1/2: Tabs   Ctrl+G: Generate   Ctrl+F: Fix   Ctrl+O: Web   Ctrl+Q: Quit"
    } else {
        "F1: Help   F2: Theme   Tab: Focus   1/2: Contrast/Palette   Ctrl+G: Generate   Ctrl+F: Fix   Ctrl+O: Web   Ctrl+Q: Quit   (mouse: click/scroll/drag)"
    };
    frame.render_widget(
        Paragraph::new(keys).alignment(Alignment::Left).style(
            Style::default()
                .fg(app.theme.text_secondary_color())
                .bg(app.theme.base_background_color()),
        ),
        area,
    );
}

fn render_fix_placeholder(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(
        Paragraph::new("Fix pane — apply a nearby passing candidate (coming next). Esc closes.")
            .style(
                Style::default()
                    .fg(app.theme.text_secondary_color())
                    .bg(app.theme.body_background_color()),
            )
            .block(
                Block::default()
                    .title("Fix")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.border_active_color()))
                    .style(Style::default().bg(app.theme.body_background_color())),
            ),
        area,
    );
}

const PALETTE_VALUE_COL: u16 = 13;

fn cursor_position(app: &App, map: &LayoutMap) -> Option<(u16, u16)> {
    if app.mode == Mode::Palette && app.palette.editing {
        let idx = app.palette.selected_idx.min(3);
        let area = map.role_rows[idx];
        if area.width < 1 || area.height < 1 {
            return None;
        }
        let col = PALETTE_VALUE_COL.saturating_add(app.palette.cursor_col());
        let x = area
            .x
            .saturating_add(col)
            .min(area.x.saturating_add(area.width.saturating_sub(1)));
        return Some((x, area.y));
    }
    if !app.editing {
        return None;
    }
    let area = match app.focus {
        FocusId::FgHex => map.fg_input,
        FocusId::BgHex => map.bg_input,
        FocusId::PreviewText => map.preview_text,
        FocusId::FontFamily => map.font_family,
        _ => return None,
    };
    if area.width < 1 || area.height < 1 {
        return None;
    }
    if app.focus == FocusId::PreviewText {
        let (row, col) =
            crate::layout::visual_cursor(&app.current_input, app.cursor_char_idx, area.width);
        let scroll = crate::layout::view_scroll(row, area.height);
        let y = area.y.saturating_add(row.saturating_sub(scroll));
        let x = area
            .x
            .saturating_add(col)
            .min(area.x.saturating_add(area.width.saturating_sub(1)));
        return Some((
            x,
            y.min(area.y.saturating_add(area.height.saturating_sub(1))),
        ));
    }
    let col = app.cursor_char_idx.min(u16::MAX as usize) as u16;
    let x = area
        .x
        .saturating_add(col)
        .min(area.x.saturating_add(area.width.saturating_sub(1)));
    Some((x, area.y))
}

fn render_palette_tab(frame: &mut Frame, app: &mut App, area: Rect) -> (Rect, Rect, [Rect; 4]) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);

    let roles = render_palette_inputs(frame, app, chunks[0]);
    let scrollbar = render_palette_detail(frame, app, chunks[1]);
    (chunks[1], scrollbar, roles)
}

fn palette_caret_style(app: &App) -> Style {
    Style::default()
        .fg(app.theme.selected_background_color())
        .bg(app.theme.cursor_color())
        .add_modifier(Modifier::BOLD)
}

fn render_palette_inputs(frame: &mut Frame, app: &App, area: Rect) -> [Rect; 4] {
    frame.render_widget(
        Block::default()
            .title(Line::styled(
                "Palette Inputs",
                Style::default().fg(app.theme.text_labels_color()),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.border_default_color()))
            .style(
                Style::default()
                    .fg(app.theme.text_primary_color())
                    .bg(app.theme.body_background_color()),
            ),
        area,
    );

    let inner = area.inner(ratatui::layout::Margin::new(1, 1));
    let selected = app.palette.selected();
    let caret = palette_caret_style(app);
    let mut role_rows = [Rect::default(); 4];
    for (i, input) in crate::palette::PaletteInput::ALL.iter().enumerate() {
        let row = Rect {
            x: inner.x,
            y: inner.y.saturating_add(i as u16),
            width: inner.width,
            height: 1,
        };
        role_rows[i] = row;
        let marker = if *input == selected { ">" } else { " " };
        let required = if input.required() { "*" } else { " " };
        let prefix = format!("{marker} {:<9}{required} ", input.label());
        let value = if app.palette.editing && *input == selected {
            app.palette.edit_input.as_str()
        } else {
            app.palette.input_for(*input)
        };
        let style = if *input == selected {
            Style::default()
                .fg(app.theme.text_active_focus_color())
                .bg(app.theme.selected_background_color())
        } else {
            Style::default()
                .fg(app.theme.text_primary_color())
                .bg(app.theme.body_background_color())
        };
        let line = if app.palette.editing && *input == selected {
            let mut spans = vec![Span::styled(prefix, style)];
            spans.extend(caret_line(value, app.palette.edit_cursor_char_idx, style, caret).spans);
            Line::from(spans)
        } else {
            Line::styled(format!("{prefix}{value}"), style)
        };
        frame.render_widget(Paragraph::new(line).style(style), row);
    }

    let help_y = inner.y.saturating_add(5);
    if help_y < inner.y.saturating_add(inner.height) {
        let help = Rect {
            x: inner.x,
            y: help_y,
            width: inner.width,
            height: inner
                .height
                .saturating_sub(help_y.saturating_sub(inner.y)),
        };
        frame.render_widget(
            Paragraph::new(vec![
                Line::from("* required   Text roles are fixed"),
                Line::from("Enter: edit   Ctrl+G: generate"),
                Line::from("Ctrl+S: save (choose file)   Ctrl+C: copy"),
            ])
            .style(
                Style::default()
                    .fg(app.theme.text_secondary_color())
                    .bg(app.theme.body_background_color()),
            ),
            help,
        );
    }
    role_rows
}

fn render_palette_detail(frame: &mut Frame, app: &mut App, area: Rect) -> Rect {
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
            Line::from("Press Ctrl+G to generate a compliant _palette.scss draft."),
            Line::from("Text roles are fixed and will not be changed."),
        ]);
    }

    let visible_height = area.height.saturating_sub(2).max(1) as usize;
    let max_scroll = lines.len().saturating_sub(visible_height);
    app.palette.detail_max_scroll = max_scroll;
    app.palette.detail_scroll = app.palette.detail_scroll.min(max_scroll);
    let scroll = app.palette.detail_scroll;
    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .collect();
    let show_scrollbar = max_scroll > 0;
    let title = if max_scroll > 0 {
        format!("Generated Detail  {}/{}", scroll + 1, max_scroll + 1)
    } else {
        "Selected / Generated Detail".to_string()
    };

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
                        title,
                        Style::default().fg(if app.focus == FocusId::Detail {
                            app.theme.text_active_focus_color()
                        } else {
                            app.theme.text_labels_color()
                        }),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if app.focus == FocusId::Detail {
                        app.theme.border_active_color()
                    } else {
                        app.theme.border_default_color()
                    }))
                    .style(Style::default().bg(app.theme.body_background_color())),
            )
            .wrap(Wrap { trim: false }),
        area,
    );

    let mut scrollbar = Rect::default();
    if show_scrollbar {
        render_vertical_scrollbar(frame, app, area, scroll, max_scroll);
        scrollbar = Rect {
            x: area.x.saturating_add(area.width.saturating_sub(2)),
            y: area.y.saturating_add(1),
            width: 1,
            height: area.height.saturating_sub(2),
        };
    }
    scrollbar
}

fn render_toast(frame: &mut Frame, app: &App, area: Rect) -> Option<Rect> {
    let (title, message, border_color) = if let Some(error) = &app.error {
        ("Error", error.as_str(), app.theme.error_color())
    } else if let Some(status) = &app.status {
        ("Status", status.as_str(), app.theme.info_color())
    } else {
        return None;
    };

    let line_count = message.lines().count().max(1) as u16;
    let toast =
        crate::layout::bottom_right_rect(area, 32, line_count.saturating_add(2).clamp(3, 4));
    frame.render_widget(Clear, toast);
    frame.render_widget(
        Paragraph::new(message)
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
            .wrap(Wrap { trim: true }),
        toast,
    );
    Some(toast)
}

fn render_edge_scrollbar(
    frame: &mut Frame,
    app: &App,
    track: Rect,
    scroll: usize,
    max_scroll: usize,
) {
    if track.height == 0 || track.width == 0 {
        return;
    }
    for offset in 0..track.height {
        frame.render_widget(
            Paragraph::new("│").style(
                Style::default()
                    .fg(app.theme.text_secondary_color())
                    .bg(app.theme.body_background_color()),
            ),
            Rect {
                x: track.x,
                y: track.y.saturating_add(offset),
                width: 1,
                height: 1,
            },
        );
    }
    let thumb_y_offset = if max_scroll == 0 || track.height == 0 {
        0
    } else {
        ((scroll as u32 * track.height.saturating_sub(1) as u32) / max_scroll as u32) as u16
    };
    let hovered = app.mouse_pos.is_some_and(|(col, row)| {
        col == track.x && row >= track.y && row < track.y.saturating_add(track.height)
    });
    let thumb_color = if hovered || app.scrollbar_dragging {
        app.theme.scrollbar_hover_color()
    } else {
        app.theme.scrollbar_color()
    };
    frame.render_widget(
        Paragraph::new("█").style(
            Style::default()
                .fg(thumb_color)
                .bg(app.theme.body_background_color()),
        ),
        Rect {
            x: track.x,
            y: track.y.saturating_add(thumb_y_offset),
            width: 1,
            height: 1,
        },
    );
}

fn render_vertical_scrollbar(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    scroll: usize,
    max_scroll: usize,
) {
    if area.height <= 2 || area.width <= 2 {
        return;
    }

    let track_height = area.height.saturating_sub(2);
    let x = area.x.saturating_add(area.width.saturating_sub(2));
    for offset in 0..track_height {
        frame.render_widget(
            Paragraph::new("│").style(
                Style::default()
                    .fg(app.theme.text_secondary_color())
                    .bg(app.theme.body_background_color()),
            ),
            Rect {
                x,
                y: area.y.saturating_add(1).saturating_add(offset),
                width: 1,
                height: 1,
            },
        );
    }

    let thumb_y_offset = if max_scroll == 0 {
        0
    } else {
        ((scroll as u32 * track_height.saturating_sub(1) as u32) / max_scroll as u32) as u16
    };
    let thumb_color = if app
        .mouse_pos
        .is_some_and(|(col, row)| col == x && row >= area.y && row < area.y + area.height)
        || app.scrollbar_dragging
    {
        app.theme.scrollbar_hover_color()
    } else {
        app.theme.scrollbar_color()
    };
    frame.render_widget(
        Paragraph::new("█").style(
            Style::default()
                .fg(thumb_color)
                .bg(app.theme.body_background_color()),
        ),
        Rect {
            x,
            y: area.y.saturating_add(1).saturating_add(thumb_y_offset),
            width: 1,
            height: 1,
        },
    );
}

fn render_keybindings_popup(frame: &mut Frame, app: &App, popup: Rect) {
    frame.render_widget(Clear, popup);
    let lines = vec![
        Line::from("Navigation"),
        Line::from("1 / 2: Contrast / Palette"),
        Line::from(
            "Tab / Shift+Tab: next/prev control (auto-apply; invalid color blocks the move)",
        ),
        Line::from("Left / Right: caret in a text field; Style chips: previous/next preset"),
        Line::from("Up / Down: Size/Weight step; Style chips: previous/next; Palette list scroll"),
        Line::from("Ctrl+Up / Ctrl+Down: step the focused Size, Weight, Style, or Fix gauge"),
        Line::from("Shift+Ctrl+Up / Shift+Ctrl+Down: larger step (size ±4, weight ±200)"),
        Line::from(
            "Enter: commit field, activate button, edit palette role, newline in PreviewText",
        ),
        Line::from("Backspace: delete before caret"),
        Line::from("Esc: blur edit, close Fix, close this popup or F2 (never quits)"),
        Line::from("Ctrl+Q: quit"),
        Line::from(""),
        Line::from("Contrast"),
        Line::from("Ctrl+S: cycle Regular / Bold / Italic / Bold+Italic"),
        Line::from("Left/Right or Up/Down on Style: select a chip (underlined = keyboard focus)"),
        Line::from("Ctrl+B: toggle bold (400↔700)   Ctrl+T: cycle font family presets"),
        Line::from("Space: swap FG/BG (on Style: apply the focused chip)"),
        Line::from("Ctrl+C: copy focused hex   Ctrl+F: toggle Fix pane   Ctrl+O: web preview"),
        Line::from("PageUp / PageDown: scroll the left column when it does not fit"),
        Line::from("Mouse wheel over the left column: scroll (size/weight still step)"),
        Line::from(""),
        Line::from("Palette"),
        Line::from("Ctrl+G: generate full _palette.scss (focuses the detail list to scroll)"),
        Line::from("Enter: begin/commit role edit   Up/Down: select role; Detail: scroll"),
        Line::from("PageUp / PageDown: scroll generated output (Contrast: left column)"),
        Line::from("Ctrl+S: save via file picker   Ctrl+C: copy generated SCSS"),
        Line::from(""),
        Line::from("F1: this help   F2: theme source and tokens"),
        Line::from(""),
        Line::from("Mouse"),
        Line::from("Click a field: focus + place caret   Click tab / WCAG / APCA: switch or cycle"),
        Line::from("Click ↓/↑: step size or weight   Wheel over size/weight: same as Ctrl+Up/Down"),
        Line::from("Click a style chip: apply it   Click Swap / Copy / Fix / Web: that action"),
        Line::from("Click toast: dismiss   Click outside F1/F2: close"),
        Line::from("Shift+click a swatch: copy that hex"),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .style(
                Style::default()
                    .fg(app.theme.modal_text_color())
                    .bg(app.theme.modal_background_color()),
            )
            .block(
                Block::default()
                    .title(Line::styled(
                        "Keys & Mouse",
                        Style::default().fg(app.theme.modal_labels_color()),
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.border_active_color()))
                    .style(Style::default().bg(app.theme.modal_background_color())),
            )
            .wrap(Wrap { trim: true }),
        popup,
    );
}

fn render_theme_debug_popup(frame: &mut Frame, app: &App, popup: Rect) {
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
    frame.render_widget(
        Paragraph::new(lines)
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
            .wrap(Wrap { trim: true }),
        popup,
    );
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
