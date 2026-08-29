use crate::app::{App, FocusId, StylePreset};
use crate::color::{apca_lookup_lc, is_large_text};
use crate::layout::Breakpoint;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

#[derive(Debug, Clone, Copy, Default)]
pub struct ContrastRects {
    pub fg_input: Rect,
    pub fg_swatch: Rect,
    pub bg_input: Rect,
    pub bg_swatch: Rect,
    pub size_input: Rect,
    pub size_dec: Rect,
    pub size_inc: Rect,
    pub weight_input: Rect,
    pub weight_dec: Rect,
    pub weight_inc: Rect,
    pub style_btns: [Rect; 4],
    pub preview_text: Rect,
    pub font_family: Rect,
    pub swap_btn: Rect,
    pub copy_btn: Rect,
    pub fix_btn: Rect,
    pub web_btn: Rect,
    pub preview: Rect,
    pub scores_wcag: Rect,
    pub scores_apca: Rect,
}

pub fn render_contrast(frame: &mut Frame, app: &App, area: Rect, bp: Breakpoint) -> ContrastRects {
    let mut rects = ContrastRects::default();

    match bp.contrast_form_width() {
        Some(form_w) => {
            let [form, preview_col] =
                Layout::horizontal([Constraint::Length(form_w), Constraint::Fill(1)]).areas(area);
            let [preview, scores] =
                Layout::vertical([Constraint::Fill(1), Constraint::Length(6)]).areas(preview_col);
            fill_form(frame, app, form, &mut rects);
            render_preview(frame, app, preview);
            rects.preview = preview;
            fill_scores(frame, app, scores, &mut rects);
        }
        None => {
            let [form, preview, scores] = Layout::vertical([
                Constraint::Length(9),
                Constraint::Min(5),
                Constraint::Length(4),
            ])
            .areas(area);
            fill_form(frame, app, form, &mut rects);
            render_preview(frame, app, preview);
            rects.preview = preview;
            fill_scores(frame, app, scores, &mut rects);
        }
    }

    rects
}

fn fill_form(frame: &mut Frame, app: &App, area: Rect, rects: &mut ContrastRects) {
    let block = Block::default()
        .title("Input")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border_default_color()))
        .style(Style::default().bg(app.theme.body_background_color()));
    let inner = area.inner(Margin::new(1, 1));
    frame.render_widget(block, area);

    let [row0, row1, row2, row3, row4, row5, row6] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    let fg_text = field_text(app, FocusId::FgHex, &app.foreground_input);
    split_color_row(
        frame,
        app,
        row0,
        "FG",
        &fg_text,
        app.foreground.to_tui_color(),
        app.focus == FocusId::FgHex,
        &mut rects.fg_input,
        &mut rects.fg_swatch,
    );

    let bg_text = field_text(app, FocusId::BgHex, &app.background_input);
    split_color_row(
        frame,
        app,
        row1,
        "BG",
        &bg_text,
        app.background.to_tui_color(),
        app.focus == FocusId::BgHex,
        &mut rects.bg_input,
        &mut rects.bg_swatch,
    );

    fill_size_weight(frame, app, row2, rects);
    fill_style_chips(frame, app, row3, rects);

    let preview_value = field_text(app, FocusId::PreviewText, &app.preview_text).replace('\n', "\\n");
    split_labeled_input(
        frame,
        app,
        row4,
        "Text",
        &preview_value,
        app.focus == FocusId::PreviewText,
        &mut rects.preview_text,
    );

    let font_value = field_text(app, FocusId::FontFamily, &app.preview_font_family);
    split_labeled_input(
        frame,
        app,
        row5,
        "Font",
        &font_value,
        app.focus == FocusId::FontFamily,
        &mut rects.font_family,
    );

    fill_actions(frame, app, row6, rects);
}

fn field_text(app: &App, focus: FocusId, stored: &str) -> String {
    if app.focus == focus {
        app.current_input.clone()
    } else {
        stored.to_string()
    }
}

fn split_color_row(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    label: &str,
    value: &str,
    swatch: ratatui::style::Color,
    focused: bool,
    input_rect: &mut Rect,
    swatch_rect: &mut Rect,
) {
    let [lab, input, swatch_area] = Layout::horizontal([
        Constraint::Length(4),
        Constraint::Fill(1),
        Constraint::Length(3),
    ])
    .areas(area);
    paint_label(frame, app, lab, label, focused);
    paint_input(frame, app, input, value, focused);
    frame.render_widget(
        Paragraph::new("  ").style(Style::default().bg(swatch)),
        swatch_area,
    );
    *input_rect = input;
    *swatch_rect = swatch_area;
}

fn split_labeled_input(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    label: &str,
    value: &str,
    focused: bool,
    input_rect: &mut Rect,
) {
    let [lab, input] =
        Layout::horizontal([Constraint::Length(5), Constraint::Fill(1)]).areas(area);
    paint_label(frame, app, lab, label, focused);
    paint_input(frame, app, input, value, focused);
    *input_rect = input;
}

fn fill_size_weight(frame: &mut Frame, app: &App, area: Rect, rects: &mut ContrastRects) {
    let [size_lab, size_in, size_dec, size_inc, _gap, wt_lab, wt_in, wt_dec, wt_inc] =
        Layout::horizontal([
            Constraint::Length(5),
            Constraint::Length(4),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(area);

    paint_label(frame, app, size_lab, "Size", app.focus == FocusId::Size);
    paint_input(
        frame,
        app,
        size_in,
        &app.font_size_px.to_string(),
        app.focus == FocusId::Size,
    );
    paint_stepper(frame, app, size_dec, "↓", app.focus == FocusId::Size);
    paint_stepper(frame, app, size_inc, "↑", app.focus == FocusId::Size);
    paint_label(frame, app, wt_lab, "Wt", app.focus == FocusId::Weight);
    paint_input(
        frame,
        app,
        wt_in,
        &app.weight.to_string(),
        app.focus == FocusId::Weight,
    );
    paint_stepper(frame, app, wt_dec, "↓", app.focus == FocusId::Weight);
    paint_stepper(frame, app, wt_inc, "↑", app.focus == FocusId::Weight);

    rects.size_input = size_in;
    rects.size_dec = size_dec;
    rects.size_inc = size_inc;
    rects.weight_input = wt_in;
    rects.weight_dec = wt_dec;
    rects.weight_inc = wt_inc;
}

fn fill_style_chips(frame: &mut Frame, app: &App, area: Rect, rects: &mut ContrastRects) {
    let chips: [Rect; 4] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .areas(area);
    let selected = app.active_style_preset();
    let row_focused = app.focus == FocusId::Style;
    for (i, preset) in StylePreset::ALL.iter().enumerate() {
        let on = selected == Some(*preset);
        let chip_focused = row_focused && app.style_chip == i;
        let mut style = if on {
            Style::default()
                .fg(app.theme.text_active_focus_color())
                .bg(app.theme.selected_background_color())
                .add_modifier(Modifier::BOLD)
        } else if chip_focused {
            Style::default().fg(app.theme.text_active_focus_color())
        } else {
            Style::default().fg(app.theme.text_secondary_color())
        };
        if chip_focused {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        let mark = if on { "●" } else { "○" };
        frame.render_widget(
            Paragraph::new(format!("{mark}{}", preset.label())).style(style),
            chips[i],
        );
        rects.style_btns[i] = chips[i];
    }
}

fn fill_actions(frame: &mut Frame, app: &App, area: Rect, rects: &mut ContrastRects) {
    let [swap, copy, fix, web] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .areas(area);
    paint_button(frame, app, swap, "Swap", app.focus == FocusId::Swap);
    paint_button(frame, app, copy, "Copy", app.focus == FocusId::CopyHex);
    paint_button(frame, app, fix, "Fix", app.focus == FocusId::FixBtn);
    paint_button(frame, app, web, "Web", app.focus == FocusId::OpenPreview);
    rects.swap_btn = swap;
    rects.copy_btn = copy;
    rects.fix_btn = fix;
    rects.web_btn = web;
}

fn paint_label(frame: &mut Frame, app: &App, area: Rect, text: &str, focused: bool) {
    let style = if focused {
        Style::default().fg(app.theme.text_active_focus_color())
    } else {
        Style::default().fg(app.theme.text_labels_color())
    };
    frame.render_widget(Paragraph::new(text).style(style), area);
}

fn paint_input(frame: &mut Frame, app: &App, area: Rect, text: &str, focused: bool) {
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

fn paint_stepper(frame: &mut Frame, app: &App, area: Rect, glyph: &str, focused: bool) {
    let style = if focused {
        Style::default().fg(app.theme.text_active_focus_color())
    } else {
        Style::default().fg(app.theme.text_secondary_color())
    };
    frame.render_widget(Paragraph::new(glyph).style(style), area);
}

fn paint_button(frame: &mut Frame, app: &App, area: Rect, label: &str, focused: bool) {
    let hovered = app.hovered.is_some_and(|hit| match label {
        "Swap" => matches!(hit, crate::layout::Hit::Swap),
        "Copy" => matches!(hit, crate::layout::Hit::Copy),
        "Fix" => matches!(hit, crate::layout::Hit::FixBtn),
        "Web" => matches!(hit, crate::layout::Hit::WebBtn),
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

fn render_preview(frame: &mut Frame, app: &App, area: Rect) {
    let mut style = app.foreground.to_style().bg(app.background.to_tui_color());
    if app.weight >= 700 {
        style = style.add_modifier(Modifier::BOLD);
    }
    if app.italic {
        style = style.add_modifier(Modifier::ITALIC);
    }
    let polarity = if app.foreground.luminance() < app.background.luminance() {
        "dark on light"
    } else {
        "light on dark"
    };
    let caption = format!(
        "approx · {} {}px/{} · {polarity}",
        app.preview_font_family, app.font_size_px, app.weight
    );
    let heading_style = style.add_modifier(Modifier::BOLD);
    let mut lines = vec![Line::styled("Heading", heading_style)];
    for body in app.preview_text.lines() {
        lines.push(Line::styled(body.to_string(), style));
    }
    if app.preview_text.is_empty() {
        lines.push(Line::from(""));
    }
    lines.push(Line::styled("[ Action ]", style));
    lines.push(Line::from(""));
    lines.push(Line::styled(caption, style));
    lines.push(Line::styled(
        "Ctrl+O opens a true CSS size/weight preview in the browser.",
        style,
    ));

    frame.render_widget(
        Paragraph::new(lines)
            .style(style)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .title("Preview")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.border_default_color())),
            ),
        area,
    );
}

fn fill_scores(frame: &mut Frame, app: &App, area: Rect, rects: &mut ContrastRects) {
    let inner = area.inner(Margin::new(1, 0));
    let [wcag, apca] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(inner);
    rects.scores_wcag = wcag;
    rects.scores_apca = apca;

    let ratio = app.contrast_ratio;
    let large = is_large_text(app.font_size_px, app.weight);
    let threshold = app.targets.wcag.text_threshold(large);
    let wcag_pass = ratio >= threshold;
    let aa_normal = ratio >= 4.5;
    let aa_large = ratio >= 3.0;
    let aaa_normal = ratio >= 7.0;
    let aaa_large = ratio >= 4.5;
    let ui = ratio >= 3.0;

    let wcag_lines = vec![
        Line::from(Span::styled(
            "WCAG",
            Style::default().fg(app.theme.text_labels_color()),
        )),
        Line::from(format!("{ratio:.2}:1  {} {}", app.targets.wcag.label(), pass_fail(app, wcag_pass))),
        Line::from(format!(
            "AA n{} l{}  AAA n{} l{}  UI{}",
            mark(aa_normal),
            mark(aa_large),
            mark(aaa_normal),
            mark(aaa_large),
            mark(ui)
        )),
    ];
    frame.render_widget(
        Paragraph::new(wcag_lines)
            .style(
                Style::default()
                    .fg(app.theme.text_primary_color())
                    .bg(app.theme.body_background_color()),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.border_default_color()))
                    .style(Style::default().bg(app.theme.body_background_color())),
            ),
        wcag,
    );

    let lc = app.foreground.apca_lc(&app.background);
    let abs_lc = lc.abs();
    let bar = app.targets.apca.value();
    let apca_pass = abs_lc >= bar;
    let lookup = apca_lookup_lc(app.font_size_px, app.weight);
    let polarity = if lc >= 0.0 { "light text" } else { "dark text" };
    let apca_lines = vec![
        Line::from(Span::styled(
            "APCA",
            Style::default().fg(app.theme.text_labels_color()),
        )),
        Line::from(format!(
            "Lc {lc:.0}  {} {}",
            app.targets.apca.label(),
            pass_fail(app, apca_pass)
        )),
        Line::from(format!("lookup Lc{lookup:.0} · {polarity}")),
    ];
    frame.render_widget(
        Paragraph::new(apca_lines)
            .style(
                Style::default()
                    .fg(app.theme.text_primary_color())
                    .bg(app.theme.body_background_color()),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.border_default_color()))
                    .style(Style::default().bg(app.theme.body_background_color())),
            ),
        apca,
    );
}

fn mark(pass: bool) -> &'static str {
    if pass { "✓" } else { "✗" }
}

fn pass_fail(app: &App, pass: bool) -> Span<'static> {
    if pass {
        Span::styled("PASS", Style::default().fg(app.theme.success_color()))
    } else {
        Span::styled("FAIL", Style::default().fg(app.theme.error_color()))
    }
}


