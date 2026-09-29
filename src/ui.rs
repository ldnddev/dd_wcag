use crate::app::{App, FocusId, Mode};
use crate::contrast::render_contrast;
use crate::fix::PairVerdict;
use crate::layout::{
    Hit, LayoutMap, breakpoint, caret_line, centered, split_body_with_fix, split_header,
    split_shell,
};
use crate::palette::{
    MatrixAxis, matrix_cell, matrix_text_swatch, off_diagonal_pairs, pair_list_index,
};

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
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

    let (main, fix_strip) = split_body_with_fix(shell.body, map.breakpoint, app.fix_open);

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
            render_palette_tab(frame, app, main, &mut map);
        }
    }

    if let Some(fix_area) = fix_strip {
        render_fix(frame, app, fix_area, &mut map, false);
    } else if app.fix_open {
        let overlay = centered(shell.body, 80, 55);
        frame.render_widget(Clear, overlay);
        render_fix(frame, app, overlay, &mut map, true);
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

fn render_fix(frame: &mut Frame, app: &App, area: Rect, map: &mut LayoutMap, overlay: bool) {
    map.fix_area = area;
    let border = if overlay {
        app.theme.border_active_color()
    } else {
        app.theme.border_default_color()
    };
    frame.render_widget(
        Block::default()
            .title(Line::styled(
                "Fix",
                Style::default().fg(app.theme.text_labels_color()),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border))
            .style(Style::default().bg(app.theme.body_background_color())),
        area,
    );
    let inner = area.inner(ratatui::layout::Margin::new(1, 1));
    let (now, fixed, nudge) = if overlay || inner.height >= 12 {
        let [now, fixed, nudge] = Layout::vertical([
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Min(6),
        ])
        .areas(inner);
        (now, fixed, nudge)
    } else {
        let [now, fixed, nudge] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .areas(inner);
        (now, fixed, nudge)
    };
    map.now_area = now;
    map.fixed_area = fixed;

    let (wcag_th, apca_bar) = app.contrast_thresholds();
    paint_fix_pair(
        frame,
        app,
        now,
        "NOW",
        app.fix.original_fg,
        app.fix.original_bg,
        PairVerdict::of(app.fix.original_fg, app.fix.original_bg, wcag_th, apca_bar),
    );
    paint_fix_pair(
        frame,
        app,
        fixed,
        "FIXED",
        app.fix.candidate_fg,
        app.fix.candidate_bg,
        PairVerdict::of(
            app.fix.candidate_fg,
            app.fix.candidate_bg,
            wcag_th,
            apca_bar,
        ),
    );

    let [fg_row, bg_row, btn_row, send_fg_row, send_bg_row] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(nudge);
    map.nudge_fg = paint_l_gauge(
        frame,
        app,
        fg_row,
        "FG",
        app.fix.candidate_fg.oklab_l(),
        app.focus == FocusId::NudgeFg,
    );
    map.nudge_bg = paint_l_gauge(
        frame,
        app,
        bg_row,
        "BG",
        app.fix.candidate_bg.oklab_l(),
        app.focus == FocusId::NudgeBg,
    );

    let [apply, next, close] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .areas(btn_row);
    map.apply_btn = apply;
    map.next_btn = next;
    map.close_fix = close;
    paint_fix_button(frame, app, apply, "Apply", app.focus == FocusId::ApplyFix);
    paint_fix_button(frame, app, next, "Next", app.focus == FocusId::NextFix);
    paint_fix_button(frame, app, close, "Close", app.focus == FocusId::CloseFix);

    map.send_fg = paint_send_row(frame, app, send_fg_row, "FG→", app.focus == FocusId::SendFg);
    map.send_bg = paint_send_row(frame, app, send_bg_row, "BG→", app.focus == FocusId::SendBg);
}

fn paint_fix_pair(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    title: &str,
    fg: crate::color::Color,
    bg: crate::color::Color,
    verdict: PairVerdict,
) {
    frame.render_widget(
        Block::default()
            .title(Line::styled(
                title.to_string(),
                Style::default().fg(app.theme.text_labels_color()),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.input_border_default_color()))
            .style(Style::default().bg(app.theme.body_background_color())),
        area,
    );
    let inner = area.inner(ratatui::layout::Margin::new(1, 1));
    let sample_style = fg.to_style().bg(bg.to_tui_color());
    let (wcag_th, apca_bar) = app.contrast_thresholds();
    let polarity = if verdict.lc >= 0.0 {
        "light text"
    } else {
        "dark text"
    };
    let lines = vec![
        Line::styled(format!("{} on {}", fg.to_hex(), bg.to_hex()), sample_style),
        Line::from(vec![
            Span::raw(format!(
                "WCAG {:.2}:1 {} ≥{wcag_th} ",
                verdict.ratio,
                app.targets.wcag.label()
            )),
            metric_mark(app, verdict.wcag),
        ]),
        Line::from(vec![
            Span::raw(format!(
                "APCA Lc {:+.0} {} ≥{apca_bar:.0} · {polarity} ",
                verdict.lc,
                app.targets.apca.label()
            )),
            metric_mark(app, verdict.apca),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines).style(
            Style::default()
                .fg(app.theme.text_primary_color())
                .bg(app.theme.body_background_color()),
        ),
        inner,
    );
}

fn metric_mark(app: &App, pass: bool) -> Span<'static> {
    if pass {
        Span::styled("PASS", Style::default().fg(app.theme.success_color()))
    } else {
        Span::styled("FAIL", Style::default().fg(app.theme.error_color()))
    }
}

fn paint_l_gauge(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    label: &str,
    l: f32,
    focused: bool,
) -> Rect {
    let l = l.clamp(0.0, 1.0);
    let [lab, bar] = Layout::horizontal([Constraint::Length(6), Constraint::Fill(1)]).areas(area);
    let label_style = if focused {
        Style::default().fg(app.theme.text_active_focus_color())
    } else {
        Style::default().fg(app.theme.text_labels_color())
    };
    frame.render_widget(Paragraph::new(format!("{label} L")).style(label_style), lab);
    let width = bar.width.max(1) as usize;
    let filled = ((l * width as f32).round() as usize).min(width);
    let mut track = "█".repeat(filled);
    track.push_str(&"░".repeat(width.saturating_sub(filled)));
    let bar_style = if focused {
        Style::default()
            .fg(app.theme.text_active_focus_color())
            .bg(app.theme.selected_background_color())
    } else {
        Style::default()
            .fg(app.theme.text_secondary_color())
            .bg(app.theme.body_background_color())
    };
    frame.render_widget(
        Paragraph::new(format!("{track} {l:.2}")).style(bar_style),
        bar,
    );
    bar
}

fn paint_send_row(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    label: &str,
    row_focused: bool,
) -> [Rect; 4] {
    let [lab, chips_area] =
        Layout::horizontal([Constraint::Length(4), Constraint::Fill(1)]).areas(area);
    let label_style = if row_focused {
        Style::default().fg(app.theme.text_active_focus_color())
    } else {
        Style::default().fg(app.theme.text_labels_color())
    };
    frame.render_widget(Paragraph::new(label).style(label_style), lab);
    let chips: [Rect; 4] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .areas(chips_area);
    for (i, role) in crate::palette::PaletteInput::ALL.iter().enumerate() {
        let selected = row_focused && app.fix_send_chip == i;
        let mut style = if selected {
            Style::default()
                .fg(app.theme.text_active_focus_color())
                .bg(app.theme.selected_background_color())
                .add_modifier(Modifier::BOLD)
        } else if row_focused {
            Style::default().fg(app.theme.text_secondary_color())
        } else {
            Style::default().fg(app.theme.text_secondary_color())
        };
        if selected {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        frame.render_widget(Paragraph::new(role.short_label()).style(style), chips[i]);
    }
    chips
}

fn paint_fix_button(frame: &mut Frame, app: &App, area: Rect, label: &str, focused: bool) {
    let hovered = app.hovered.is_some_and(|hit| match label {
        "Apply" => matches!(hit, crate::layout::Hit::ApplyFix),
        "Next" => matches!(hit, crate::layout::Hit::NextFix),
        "Close" => matches!(hit, crate::layout::Hit::CloseFix),
        _ => false,
    });
    let mut style = if focused {
        Style::default()
            .fg(app.theme.text_active_focus_color())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(app.theme.text_secondary_color())
    };
    if hovered {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    frame.render_widget(Paragraph::new(format!("[{label}]")).style(style), area);
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

fn render_palette_tab(frame: &mut Frame, app: &mut App, area: Rect, map: &mut LayoutMap) {
    if map.breakpoint.contrast_side_by_side() {
        let roles_w = match map.breakpoint {
            crate::layout::Breakpoint::Wide => 28,
            _ => 24,
        };
        let [roles, right] =
            Layout::horizontal([Constraint::Length(roles_w), Constraint::Fill(1)]).areas(area);
        render_palette_inputs(frame, app, roles, map);
        let [matrix, detail] =
            Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)]).areas(right);
        render_palette_matrix(frame, app, matrix, map);
        map.detail_scrollbar = render_palette_detail(frame, app, detail);
        map.detail = detail;
    } else {
        let [roles, pairs, detail] = Layout::vertical([
            Constraint::Length(8),
            Constraint::Fill(1),
            Constraint::Length(6),
        ])
        .areas(area);
        render_palette_inputs(frame, app, roles, map);
        render_palette_pair_list(frame, app, pairs, map);
        map.detail_scrollbar = render_palette_detail(frame, app, detail);
        map.detail = detail;
    }
}

fn palette_caret_style(app: &App) -> Style {
    Style::default()
        .fg(app.theme.selected_background_color())
        .bg(app.theme.cursor_color())
        .add_modifier(Modifier::BOLD)
}

fn render_palette_inputs(frame: &mut Frame, app: &App, area: Rect, map: &mut LayoutMap) {
    let roles_focused = matches!(app.focus, FocusId::Role(_));
    frame.render_widget(
        Block::default()
            .title(Line::styled(
                "Roles",
                Style::default().fg(if roles_focused {
                    app.theme.text_active_focus_color()
                } else {
                    app.theme.text_labels_color()
                }),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if roles_focused {
                app.theme.border_active_color()
            } else {
                app.theme.border_default_color()
            }))
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
        if row.y >= inner.y.saturating_add(inner.height) {
            break;
        }
        role_rows[i] = row;
        let [text, swatch] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(2)]).areas(row);
        let marker = if *input == selected { ">" } else { " " };
        let required = if input.required() { "*" } else { " " };
        let prefix = format!("{marker} {:<9}{required} ", input.label());
        let value = if app.palette.editing && *input == selected {
            app.palette.edit_input.as_str()
        } else {
            app.palette.input_for(*input)
        };
        let focused = app.focus == FocusId::Role(i);
        let style = if focused || *input == selected {
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
        frame.render_widget(Paragraph::new(line).style(style), text);
        let swatch_color = crate::palette::parse_palette_color(value)
            .ok()
            .map(|c| c.to_tui_color())
            .unwrap_or_else(|| app.theme.body_background_color());
        frame.render_widget(
            Paragraph::new("  ").style(Style::default().bg(swatch_color)),
            swatch,
        );
    }
    map.role_rows = role_rows;

    let text_y = inner.y.saturating_add(4);
    if text_y < inner.y.saturating_add(inner.height) {
        let row = Rect {
            x: inner.x,
            y: text_y,
            width: inner.width,
            height: 1,
        };
        map.text_row = row;
        let [text, swatch] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(2)]).areas(row);
        let text_selected = app.palette.matrix_text() == MatrixAxis::Text
            || app.palette.matrix_surface() == MatrixAxis::Text;
        let style = if text_selected {
            Style::default()
                .fg(app.theme.text_active_focus_color())
                .bg(app.theme.selected_background_color())
        } else {
            Style::default()
                .fg(app.theme.text_secondary_color())
                .bg(app.theme.body_background_color())
        };
        let hex = matrix_text_swatch().to_hex();
        frame.render_widget(
            Paragraph::new(format!("  {:<9}  {hex}", "Text")).style(style),
            text,
        );
        frame.render_widget(
            Paragraph::new("  ").style(Style::default().bg(matrix_text_swatch().to_tui_color())),
            swatch,
        );
    }

    let controls_y = inner.y.saturating_add(5);
    if controls_y < inner.y.saturating_add(inner.height) {
        let row = Rect {
            x: inner.x,
            y: controls_y,
            width: inner.width,
            height: 1,
        };
        render_palette_size_weight(frame, app, row, map);
    }

    let actions_y = inner.y.saturating_add(6);
    if actions_y < inner.y.saturating_add(inner.height) {
        let row = Rect {
            x: inner.x,
            y: actions_y,
            width: inner.width,
            height: 1,
        };
        let [generate, web] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(row);
        map.generate_btn = generate;
        map.web_btn = web;
        paint_palette_button(
            frame,
            app,
            generate,
            "Generate",
            app.focus == FocusId::Generate,
            matches!(app.hovered, Some(Hit::Generate)),
        );
        paint_palette_button(
            frame,
            app,
            web,
            "Web",
            app.focus == FocusId::OpenPreview,
            matches!(app.hovered, Some(Hit::WebBtn)),
        );
    }
}

fn render_palette_size_weight(frame: &mut Frame, app: &App, area: Rect, map: &mut LayoutMap) {
    let [size_side, wt_side] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(area);
    let [size_lab, size_in, size_dec, size_inc] = Layout::horizontal([
        Constraint::Length(5),
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(size_side);
    let [wt_lab, wt_in, wt_dec, wt_inc] = Layout::horizontal([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(wt_side);

    let size_focused = app.focus == FocusId::Size;
    let wt_focused = app.focus == FocusId::Weight;
    let label_style = Style::default()
        .fg(app.theme.text_labels_color())
        .bg(app.theme.body_background_color());
    frame.render_widget(Paragraph::new("Size ").style(label_style), size_lab);
    frame.render_widget(Paragraph::new("Wt ").style(label_style), wt_lab);
    paint_palette_value(
        frame,
        app,
        size_in,
        &app.font_size_px.to_string(),
        size_focused,
    );
    paint_palette_value(frame, app, wt_in, &app.weight.to_string(), wt_focused);
    paint_palette_stepper(frame, app, size_dec, "↓", size_focused);
    paint_palette_stepper(frame, app, size_inc, "↑", size_focused);
    paint_palette_stepper(frame, app, wt_dec, "↓", wt_focused);
    paint_palette_stepper(frame, app, wt_inc, "↑", wt_focused);
    map.size_input = size_in;
    map.size_dec = size_dec;
    map.size_inc = size_inc;
    map.weight_input = wt_in;
    map.weight_dec = wt_dec;
    map.weight_inc = wt_inc;
}

fn paint_palette_value(frame: &mut Frame, app: &App, area: Rect, text: &str, focused: bool) {
    let style = if focused {
        Style::default()
            .fg(app.theme.input_text_focus_color())
            .bg(app.theme.selected_background_color())
    } else {
        Style::default()
            .fg(app.theme.input_text_default_color())
            .bg(app.theme.body_background_color())
    };
    frame.render_widget(Paragraph::new(text).style(style), area);
}

fn paint_palette_stepper(frame: &mut Frame, app: &App, area: Rect, glyph: &str, focused: bool) {
    let style = if focused {
        Style::default().fg(app.theme.text_active_focus_color())
    } else {
        Style::default().fg(app.theme.text_secondary_color())
    };
    frame.render_widget(Paragraph::new(glyph).style(style), area);
}

fn paint_palette_button(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    label: &str,
    focused: bool,
    hovered: bool,
) {
    let mut style = if focused {
        Style::default()
            .fg(app.theme.text_active_focus_color())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(app.theme.text_secondary_color())
    };
    if hovered {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    frame.render_widget(Paragraph::new(format!("[{label}]")).style(style), area);
}

fn render_palette_matrix(frame: &mut Frame, app: &App, area: Rect, map: &mut LayoutMap) {
    let focused = app.focus == FocusId::Matrix;
    frame.render_widget(
        Block::default()
            .title(Line::styled(
                "Pairs  text \\ surface",
                Style::default().fg(if focused {
                    app.theme.text_active_focus_color()
                } else {
                    app.theme.text_labels_color()
                }),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if focused {
                app.theme.border_active_color()
            } else {
                app.theme.border_default_color()
            }))
            .style(Style::default().bg(app.theme.body_background_color())),
        area,
    );
    let inner = area.inner(ratatui::layout::Margin::new(1, 1));
    map.matrix_area = inner;
    if inner.width < 12 || inner.height < 6 {
        return;
    }
    let header_h = 1u16;
    let body_h = inner.height.saturating_sub(header_h);
    let row_h = if body_h >= 10 { 2 } else { 1 };
    let [gutter, rest] =
        Layout::horizontal([Constraint::Length(5), Constraint::Fill(1)]).areas(inner);
    let cols: [Rect; 5] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .areas(rest);

    let header_style = Style::default()
        .fg(app.theme.text_labels_color())
        .bg(app.theme.body_background_color());
    for (i, axis) in MatrixAxis::ALL.iter().enumerate() {
        frame.render_widget(
            Paragraph::new(axis.short_label()).style(header_style),
            Rect {
                x: cols[i].x,
                y: inner.y,
                width: cols[i].width,
                height: 1,
            },
        );
    }

    let (wcag_th, apca_bar) = app.contrast_thresholds();
    for r in 0..5 {
        let y = inner
            .y
            .saturating_add(header_h)
            .saturating_add((r as u16) * row_h);
        if y >= inner.y.saturating_add(inner.height) {
            break;
        }
        let h = row_h.min(inner.y.saturating_add(inner.height).saturating_sub(y));
        frame.render_widget(
            Paragraph::new(MatrixAxis::from_index(r).short_label()).style(header_style),
            Rect {
                x: gutter.x,
                y,
                width: gutter.width,
                height: h,
            },
        );
        for c in 0..5 {
            let cell_rect = Rect {
                x: cols[c].x,
                y,
                width: cols[c].width,
                height: h,
            };
            map.matrix_cells[r][c] = cell_rect;
            let selected = app.palette.matrix_row == r && app.palette.matrix_col == c;
            let hovered =
                matches!(app.hovered, Some(Hit::MatrixCell(hr, hc)) if hr == r && hc == c);
            if r == c {
                frame.render_widget(
                    Paragraph::new(" · ").style(
                        Style::default()
                            .fg(app.theme.text_secondary_color())
                            .bg(app.theme.body_background_color()),
                    ),
                    cell_rect,
                );
                continue;
            }
            let Some(cell) = matrix_cell(&app.palette, r, c, wcag_th, apca_bar) else {
                frame.render_widget(
                    Paragraph::new(" ? ").style(
                        Style::default()
                            .fg(app.theme.warning_color())
                            .bg(app.theme.body_background_color()),
                    ),
                    cell_rect,
                );
                continue;
            };
            let glyph = cell.glyph();
            let glyph_color = match glyph {
                '✓' => app.theme.success_color(),
                '✗' => app.theme.error_color(),
                _ => app.theme.warning_color(),
            };
            let mut bg = app.theme.body_background_color();
            let mut fg = app.theme.text_primary_color();
            if selected {
                bg = app.theme.selected_background_color();
                fg = app.theme.text_active_focus_color();
            }
            let mut glyph_style = Style::default().fg(glyph_color).bg(bg);
            if hovered {
                glyph_style = glyph_style.add_modifier(Modifier::UNDERLINED);
            }
            let lines = if h >= 2 {
                vec![
                    Line::from(Span::styled(format!(" {glyph}"), glyph_style)),
                    Line::from(Span::styled(
                        format!(" {:.1}", cell.ratio),
                        Style::default().fg(fg).bg(bg),
                    )),
                ]
            } else {
                vec![Line::from(Span::styled(format!(" {glyph}"), glyph_style))]
            };
            frame.render_widget(
                Paragraph::new(lines).style(Style::default().bg(bg)),
                cell_rect,
            );
        }
    }
}

fn render_palette_pair_list(frame: &mut Frame, app: &mut App, area: Rect, map: &mut LayoutMap) {
    let focused = app.focus == FocusId::Matrix;
    frame.render_widget(
        Block::default()
            .title(Line::styled(
                "Pairs",
                Style::default().fg(if focused {
                    app.theme.text_active_focus_color()
                } else {
                    app.theme.text_labels_color()
                }),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if focused {
                app.theme.border_active_color()
            } else {
                app.theme.border_default_color()
            }))
            .style(Style::default().bg(app.theme.body_background_color())),
        area,
    );
    let inner = area.inner(ratatui::layout::Margin::new(1, 1));
    map.pair_list = inner;
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let (wcag_th, apca_bar) = app.contrast_thresholds();
    let pairs = off_diagonal_pairs();
    let visible = inner.height as usize;
    app.palette.pair_max_scroll = pairs.len().saturating_sub(visible);
    if let Some(idx) = pair_list_index(app.palette.matrix_row, app.palette.matrix_col) {
        if idx < app.palette.pair_scroll {
            app.palette.pair_scroll = idx;
        } else if visible > 0 && idx >= app.palette.pair_scroll + visible {
            app.palette.pair_scroll = idx + 1 - visible;
        }
    }
    app.palette.pair_scroll = app.palette.pair_scroll.min(app.palette.pair_max_scroll);
    let scroll = app.palette.pair_scroll;
    for (i, &(r, c)) in pairs.iter().skip(scroll).take(visible).enumerate() {
        let row = Rect {
            x: inner.x,
            y: inner.y.saturating_add(i as u16),
            width: inner.width,
            height: 1,
        };
        let selected = app.palette.matrix_row == r && app.palette.matrix_col == c;
        let text = MatrixAxis::from_index(r).short_label();
        let surface = MatrixAxis::from_index(c).short_label();
        let (body, style) = match matrix_cell(&app.palette, r, c, wcag_th, apca_bar) {
            Some(cell) => {
                let glyph = cell.glyph();
                let color = match glyph {
                    '✓' => app.theme.success_color(),
                    '✗' => app.theme.error_color(),
                    _ => app.theme.warning_color(),
                };
                let line = format!(
                    "{text} on {surface}  {:.2}{glyph}  Lc{:.0}{glyph}",
                    cell.ratio, cell.lc
                );
                let st = if selected {
                    Style::default()
                        .fg(app.theme.text_active_focus_color())
                        .bg(app.theme.selected_background_color())
                } else {
                    Style::default()
                        .fg(color)
                        .bg(app.theme.body_background_color())
                };
                (line, st)
            }
            None => (
                format!("{text} on {surface}  ?"),
                Style::default()
                    .fg(app.theme.warning_color())
                    .bg(app.theme.body_background_color()),
            ),
        };
        frame.render_widget(Paragraph::new(body).style(style), row);
    }
}

fn render_palette_detail(frame: &mut Frame, app: &mut App, area: Rect) -> Rect {
    let selected = app.palette.selected_family();
    let (wcag_th, apca_bar) = app.contrast_thresholds();
    let mut lines: Vec<Line> = Vec::new();
    if let Some(cell) = matrix_cell(
        &app.palette,
        app.palette.matrix_row,
        app.palette.matrix_col,
        wcag_th,
        apca_bar,
    ) {
        let glyph = cell.glyph();
        let color = match glyph {
            '✓' => app.theme.success_color(),
            '✗' => app.theme.error_color(),
            _ => app.theme.warning_color(),
        };
        lines.push(Line::from(vec![Span::styled(
            format!("{} on {}", cell.text.label(), cell.surface.label()),
            Style::default()
                .fg(app.theme.text_active_focus_color())
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(vec![Span::styled(
            format!(
                "WCAG {:.2} {glyph}   APCA Lc {:.0} {glyph}",
                cell.ratio, cell.lc
            ),
            Style::default().fg(color),
        )]));
        lines.push(Line::from(format!(
            "needs ≥ {wcag_th:.1}:1 and |Lc| ≥ {apca_bar:.0}"
        )));
        lines.push(Line::from(vec![
            Span::styled(
                " Aa sample ",
                Style::default()
                    .fg(cell.fg.to_tui_color())
                    .bg(cell.bg.to_tui_color()),
            ),
            Span::raw(format!("  {} on {}", cell.fg.to_hex(), cell.bg.to_hex())),
        ]));
    } else {
        lines.push(Line::from("Selected pair has an invalid color."));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            selected.label(),
            Style::default()
                .fg(app.theme.text_active_focus_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" theme builder"),
    ]));
    lines.push(Line::from(format!(
        "Base: {}",
        app.palette.input_for(selected)
    )));

    match crate::palette::parse_palette_color(app.palette.input_for(selected)) {
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
            let (tag, color) = if check.passes {
                ("PASS ", app.theme.success_color())
            } else if check.blocking {
                ("FAIL ", app.theme.error_color())
            } else {
                ("WARN ", app.theme.warning_color())
            };
            lines.push(Line::from(vec![
                Span::styled(tag, Style::default().fg(color)),
                Span::raw(format!(
                    "{:.2}:1 >= {:.1}:1 {}{}",
                    check.ratio,
                    check.threshold,
                    check.label,
                    if check.blocking || check.passes {
                        ""
                    } else {
                        " (advisory)"
                    }
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
        Line::from("Home / End / Delete: caret and forward-delete in a text field"),
        Line::from(
            "Ctrl+V or terminal paste: paste into the focused field (valid color replaces it)",
        ),
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
        Line::from("Ctrl+C: copy focused hex   Ctrl+V: paste   Ctrl+F: Fix   Ctrl+O: web preview"),
        Line::from("Click WCAG / APCA (or Tab there, then Enter/←/→): cycle AA↔AAA / Lc45–90"),
        Line::from(
            "Fix: Ctrl+N next candidate  Enter Apply/Next/Close  [ ] lightness  { } hue  drag gauges",
        ),
        Line::from(
            "Fix → Palette: FG→ / BG→ chips set Pri/Sec/Ter/Sup. p/s/t/u send the focused axis",
        ),
        Line::from("PageUp / PageDown: scroll the left column when it does not fit"),
        Line::from("Mouse wheel over the left column: scroll (size/weight still step)"),
        Line::from(""),
        Line::from("Palette"),
        Line::from("Ctrl+G: generate _palette.scss and select the worst failing pair"),
        Line::from("Enter: edit role / generate / open Fix for the selected matrix pair"),
        Line::from("Arrows on the matrix skip the diagonal   Space: swap text \\ surface"),
        Line::from("Click a cell to select it   Double-click or Enter: open Fix"),
        Line::from("Click Text: select the Text axis   PageUp/Down: scroll generated output"),
        Line::from("Ctrl+S: save SCSS + Penpot/Figma tokens JSON   Ctrl+C: copy SCSS"),
        Line::from(""),
        Line::from("F1: this help   F2: theme source and tokens"),
        Line::from(""),
        Line::from("Mouse"),
        Line::from("Click a field: focus + place caret   Click tab / WCAG / APCA: switch or cycle"),
        Line::from("Click ↓/↑: step size or weight   Wheel over size/weight: same as Ctrl+Up/Down"),
        Line::from("Click a style chip: apply it   Click Swap / Copy / Fix / Web: that action"),
        Line::from("Click toast: dismiss   Click outside F1/F2: close"),
        Line::from("Shift+click a swatch: copy that hex   Drag a scrollbar: jump in the list"),
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
    if let Some(editor) = &app.theme_editor {
        render_theme_editor(frame, app, popup, editor);
        return;
    }
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

fn render_theme_editor(
    frame: &mut Frame,
    app: &App,
    popup: Rect,
    editor: &ldnddev_theme::ThemeEditor,
) {
    use ldnddev_theme::{ThemeEditorRow, theme_editor_rows};
    let rows = theme_editor_rows(&editor.fields);
    let channel = ["R", "G", "B"][editor.channel.min(2)];
    let target = editor.save_target.label().to_uppercase();
    let mut lines = vec![
        Line::from(format!(
            "Save: {target} (Tab)   Channel: {channel} ([/])   Y save   R reset   Esc revert"
        )),
        Line::from(if editor.editing_hex {
            format!("Hex: {}█   Enter apply", editor.hex_draft)
        } else {
            format!("Hex: {}   Enter to type   +/- nudge", editor.hex_draft)
        }),
        Line::from(""),
    ];
    let view_h = popup.height.saturating_sub(6) as usize;
    let start = rows
        .iter()
        .position(|row| match row {
            ThemeEditorRow::Color(idx) => *idx >= editor.scroll,
            ThemeEditorRow::Header(_) => false,
        })
        .unwrap_or(0);
    let start = if start > 0 && matches!(rows[start - 1], ThemeEditorRow::Header(_)) {
        start - 1
    } else {
        start
    };
    for row in rows.iter().skip(start).take(view_h.max(1)) {
        match row {
            ThemeEditorRow::Header(name) => lines.push(Line::from(*name)),
            ThemeEditorRow::Color(idx) => {
                let field = editor.fields[*idx];
                let hex = editor
                    .palette
                    .get(field.key)
                    .map(|c| c.to_hex())
                    .unwrap_or_else(|| "#000000".into());
                let cursor = if *idx == editor.selected { ">" } else { " " };
                lines.push(Line::from(format!("{cursor} {:<22} {hex}", field.key)));
            }
        }
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
                    .title("F2 Theme editor")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.border_active_color())),
            ),
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
