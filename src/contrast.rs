use crate::app::{App, FocusId, StylePreset};
use crate::color::{apca_lookup_lc, is_large_text};
use crate::layout::{
    Breakpoint, caret_line, scroll_to_show, view_scroll, visible_rect, visual_cursor, visual_lines,
};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

const FORM_HEIGHT: u16 = 23;
const CONVERSIONS_HEIGHT: u16 = 6;
const WCAG_HEIGHT: u16 = 12;
const APCA_HEIGHT: u16 = 13;
const LEFT_NATURAL_HEIGHT: u16 = FORM_HEIGHT + CONVERSIONS_HEIGHT + WCAG_HEIGHT + APCA_HEIGHT;

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
    pub panel: Rect,
    pub scrollbar: Rect,
}

pub fn render_contrast(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    bp: Breakpoint,
) -> ContrastRects {
    let mut rects = ContrastRects::default();

    let (viewport, preview) = if bp.contrast_side_by_side() {
        let [left, preview] =
            Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                .areas(area);
        (left, preview)
    } else {
        let [left, preview] =
            Layout::vertical([Constraint::Fill(1), Constraint::Min(8)]).areas(area);
        (left, preview)
    };

    render_preview(frame, app, preview);
    rects.preview = preview;
    rects.panel = viewport;

    if viewport.width == 0 || viewport.height == 0 {
        return rects;
    }

    let extra = viewport.height.saturating_sub(LEFT_NATURAL_HEIGHT);
    let wcag_h = WCAG_HEIGHT.saturating_add(extra / 2);
    let apca_h = APCA_HEIGHT.saturating_add(extra - extra / 2);
    let content_h = FORM_HEIGHT
        .saturating_add(CONVERSIONS_HEIGHT)
        .saturating_add(wcag_h)
        .saturating_add(apca_h);

    let max_scroll = content_h.saturating_sub(viewport.height);
    app.contrast_max_scroll = max_scroll;
    let show_bar = max_scroll > 0;
    let content_width = viewport.width.saturating_sub(if show_bar { 1 } else { 0 });
    if show_bar {
        rects.scrollbar = Rect {
            x: viewport.x.saturating_add(viewport.width.saturating_sub(1)),
            y: viewport.y,
            width: 1,
            height: viewport.height,
        };
    }

    let content_area = Rect {
        x: 0,
        y: 0,
        width: content_width.max(1),
        height: content_h.max(1),
    };
    let [form, conversions, scores] = Layout::vertical([
        Constraint::Length(FORM_HEIGHT),
        Constraint::Length(CONVERSIONS_HEIGHT),
        Constraint::Min(WCAG_HEIGHT + APCA_HEIGHT),
    ])
    .areas(content_area);
    let [wcag, apca] =
        Layout::vertical([Constraint::Length(wcag_h), Constraint::Length(apca_h)]).areas(scores);

    let mut buf = Buffer::empty(content_area);
    buf.set_style(
        content_area,
        Style::default().bg(app.theme.body_background_color()),
    );
    fill_form(&mut buf, app, form, &mut rects);
    fill_conversions(&mut buf, app, conversions);
    fill_wcag_panel(&mut buf, app, wcag);
    fill_apca_panel(&mut buf, app, apca);
    rects.scores_wcag = wcag;
    rects.scores_apca = apca;

    let content_view = Rect {
        x: viewport.x,
        y: viewport.y,
        width: content_width.max(1),
        height: viewport.height,
    };
    if app.contrast_scroll_focus != Some(app.focus) {
        if let Some(target) = focused_virtual_rect(app, &rects) {
            app.contrast_scroll = scroll_to_show(app.contrast_scroll, viewport.height, target);
        }
        app.contrast_scroll_focus = Some(app.focus);
    }
    app.contrast_scroll = app.contrast_scroll.min(max_scroll);

    blit_visible(frame, &buf, content_view, app.contrast_scroll);
    translate_rects(&mut rects, content_view, app.contrast_scroll);

    rects
}

fn paint(buf: &mut Buffer, widget: impl Widget, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    widget.render(area, buf);
}

fn blit_visible(frame: &mut Frame, src: &Buffer, viewport: Rect, scroll: u16) {
    let dest = frame.buffer_mut();
    for dy in 0..viewport.height {
        let src_y = scroll.saturating_add(dy);
        for dx in 0..viewport.width {
            let Some(cell) = src.cell((dx, src_y)) else {
                continue;
            };
            if let Some(target) =
                dest.cell_mut((viewport.x.saturating_add(dx), viewport.y.saturating_add(dy)))
            {
                *target = cell.clone();
            }
        }
    }
}

fn focused_virtual_rect(app: &App, rects: &ContrastRects) -> Option<Rect> {
    match app.focus {
        FocusId::FgHex => Some(rects.fg_input.union(rects.fg_swatch)),
        FocusId::BgHex => Some(rects.bg_input.union(rects.bg_swatch)),
        FocusId::Size => Some(rects.size_input.union(rects.size_dec).union(rects.size_inc)),
        FocusId::Weight => Some(
            rects
                .weight_input
                .union(rects.weight_dec)
                .union(rects.weight_inc),
        ),
        FocusId::Style => rects.style_btns.into_iter().reduce(|a, b| a.union(b)),
        FocusId::PreviewText => Some(rects.preview_text),
        FocusId::FontFamily => Some(rects.font_family),
        FocusId::Swap => Some(rects.swap_btn),
        FocusId::CopyHex => Some(rects.copy_btn),
        FocusId::FixBtn => Some(rects.fix_btn),
        FocusId::OpenPreview => Some(rects.web_btn),
        _ => None,
    }
}

fn translate_rects(rects: &mut ContrastRects, viewport: Rect, scroll: u16) {
    let t = |r: Rect| visible_rect(r, viewport, scroll);
    rects.fg_input = t(rects.fg_input);
    rects.fg_swatch = t(rects.fg_swatch);
    rects.bg_input = t(rects.bg_input);
    rects.bg_swatch = t(rects.bg_swatch);
    rects.size_input = t(rects.size_input);
    rects.size_dec = t(rects.size_dec);
    rects.size_inc = t(rects.size_inc);
    rects.weight_input = t(rects.weight_input);
    rects.weight_dec = t(rects.weight_dec);
    rects.weight_inc = t(rects.weight_inc);
    for btn in &mut rects.style_btns {
        *btn = t(*btn);
    }
    rects.preview_text = t(rects.preview_text);
    rects.font_family = t(rects.font_family);
    rects.swap_btn = t(rects.swap_btn);
    rects.copy_btn = t(rects.copy_btn);
    rects.fix_btn = t(rects.fix_btn);
    rects.web_btn = t(rects.web_btn);
    rects.scores_wcag = t(rects.scores_wcag);
    rects.scores_apca = t(rects.scores_apca);
}

fn fill_form(buf: &mut Buffer, app: &App, area: Rect, rects: &mut ContrastRects) {
    let [row0, row1, row2, row3, text_row, font_row, actions] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(5),
        Constraint::Length(3),
        Constraint::Length(3),
    ])
    .areas(area);

    let fg_text = field_text(app, FocusId::FgHex, &app.foreground_input);
    split_color_row(
        buf,
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
        buf,
        app,
        row1,
        "BG",
        &bg_text,
        app.background.to_tui_color(),
        app.focus == FocusId::BgHex,
        &mut rects.bg_input,
        &mut rects.bg_swatch,
    );

    fill_size_weight(buf, app, row2, rects);
    fill_style_chips(buf, app, row3, rects);

    let text_focused = app.focus == FocusId::PreviewText;
    paint(buf, field_block(app, "Text", text_focused), text_row);
    let text_inner = text_row.inner(Margin::new(1, 1));
    let preview_value = field_text(app, FocusId::PreviewText, &app.preview_text);
    paint_multiline(buf, app, text_inner, &preview_value, text_focused);
    rects.preview_text = text_inner;

    let font_value = field_text(app, FocusId::FontFamily, &app.preview_font_family);
    split_labeled_input(
        buf,
        app,
        font_row,
        "Font",
        &font_value,
        app.focus == FocusId::FontFamily,
        &mut rects.font_family,
    );

    fill_actions(buf, app, actions, rects);
}

fn field_text(app: &App, focus: FocusId, stored: &str) -> String {
    if app.focus == focus {
        app.current_input.clone()
    } else {
        stored.to_string()
    }
}

fn field_block(app: &App, title: &str, focused: bool) -> Block<'static> {
    let border = if focused {
        app.theme.input_border_focus_color()
    } else {
        app.theme.input_border_default_color()
    };
    let title_style = if focused {
        Style::default().fg(app.theme.text_active_focus_color())
    } else {
        Style::default().fg(app.theme.text_labels_color())
    };
    Block::default()
        .title(Line::styled(title.to_string(), title_style))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .style(Style::default().bg(app.theme.body_background_color()))
}

fn split_color_row(
    buf: &mut Buffer,
    app: &App,
    area: Rect,
    label: &str,
    value: &str,
    swatch: ratatui::style::Color,
    focused: bool,
    input_rect: &mut Rect,
    swatch_rect: &mut Rect,
) {
    paint(buf, field_block(app, label, focused), area);
    let inner = area.inner(Margin::new(1, 1));
    let [input, swatch_area] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(2)]).areas(inner);
    paint_input(buf, app, input, value, focused);
    paint(
        buf,
        Paragraph::new("  ").style(Style::default().bg(swatch)),
        swatch_area,
    );
    *input_rect = input;
    *swatch_rect = swatch_area;
}

fn split_labeled_input(
    buf: &mut Buffer,
    app: &App,
    area: Rect,
    label: &str,
    value: &str,
    focused: bool,
    input_rect: &mut Rect,
) {
    paint(buf, field_block(app, label, focused), area);
    let inner = area.inner(Margin::new(1, 1));
    paint_input(buf, app, inner, value, focused);
    *input_rect = inner;
}

fn fill_size_weight(buf: &mut Buffer, app: &App, area: Rect, rects: &mut ContrastRects) {
    let [size_area, wt_area] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(area);

    let size_focused = app.focus == FocusId::Size;
    paint(buf, field_block(app, "Size", size_focused), size_area);
    let size_inner = size_area.inner(Margin::new(1, 1));
    let [size_in, size_dec, size_inc] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(size_inner);
    paint_input(
        buf,
        app,
        size_in,
        &app.font_size_px.to_string(),
        size_focused,
    );
    paint_stepper(buf, app, size_dec, "↓", size_focused);
    paint_stepper(buf, app, size_inc, "↑", size_focused);

    let wt_focused = app.focus == FocusId::Weight;
    paint(buf, field_block(app, "Wt", wt_focused), wt_area);
    let wt_inner = wt_area.inner(Margin::new(1, 1));
    let [wt_in, wt_dec, wt_inc] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(wt_inner);
    paint_input(buf, app, wt_in, &app.weight.to_string(), wt_focused);
    paint_stepper(buf, app, wt_dec, "↓", wt_focused);
    paint_stepper(buf, app, wt_inc, "↑", wt_focused);

    rects.size_input = size_in;
    rects.size_dec = size_dec;
    rects.size_inc = size_inc;
    rects.weight_input = wt_in;
    rects.weight_dec = wt_dec;
    rects.weight_inc = wt_inc;
}

fn fill_style_chips(buf: &mut Buffer, app: &App, area: Rect, rects: &mut ContrastRects) {
    let row_focused = app.focus == FocusId::Style;
    paint(buf, field_block(app, "Style", row_focused), area);
    let inner = area.inner(Margin::new(1, 1));
    let chips: [Rect; 4] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .areas(inner);
    let selected = app.active_style_preset();
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
        paint(
            buf,
            Paragraph::new(format!("{mark}{}", preset.label())).style(style),
            chips[i],
        );
        rects.style_btns[i] = chips[i];
    }
}

fn fill_actions(buf: &mut Buffer, app: &App, area: Rect, rects: &mut ContrastRects) {
    let focused = matches!(
        app.focus,
        FocusId::Swap | FocusId::CopyHex | FocusId::FixBtn | FocusId::OpenPreview
    );
    paint(buf, field_block(app, "Actions", focused), area);
    let inner = area.inner(Margin::new(1, 1));
    let [swap, copy, fix, web] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .areas(inner);
    paint_button(buf, app, swap, "Swap", app.focus == FocusId::Swap);
    paint_button(buf, app, copy, "Copy", app.focus == FocusId::CopyHex);
    paint_button(buf, app, fix, "Fix", app.focus == FocusId::FixBtn);
    paint_button(buf, app, web, "Web", app.focus == FocusId::OpenPreview);
    rects.swap_btn = swap;
    rects.copy_btn = copy;
    rects.fix_btn = fix;
    rects.web_btn = web;
}

fn caret_style(app: &App) -> Style {
    Style::default()
        .fg(app.theme.selected_background_color())
        .bg(app.theme.cursor_color())
        .add_modifier(Modifier::BOLD)
}

fn paint_input(buf: &mut Buffer, app: &App, area: Rect, text: &str, focused: bool) {
    let style = if focused {
        Style::default()
            .fg(app.theme.input_text_focus_color())
            .bg(app.theme.selected_background_color())
    } else {
        Style::default()
            .fg(app.theme.input_text_default_color())
            .bg(app.theme.body_background_color())
    };
    if focused && app.editing {
        paint(
            buf,
            Paragraph::new(caret_line(text, app.cursor_char_idx, style, caret_style(app)))
                .style(style),
            area,
        );
    } else {
        paint(buf, Paragraph::new(text).style(style), area);
    }
}

fn paint_multiline(buf: &mut Buffer, app: &App, area: Rect, text: &str, focused: bool) {
    let style = if focused {
        Style::default()
            .fg(app.theme.input_text_focus_color())
            .bg(app.theme.selected_background_color())
    } else {
        Style::default()
            .fg(app.theme.input_text_default_color())
            .bg(app.theme.body_background_color())
    };
    let width = area.width.max(1);
    let height = area.height.max(1);
    let (crow, ccol) = visual_cursor(text, app.cursor_char_idx, width);
    let scroll = if focused {
        view_scroll(crow, height)
    } else {
        0
    };
    let lines = visual_lines(text, width as usize);
    let shown: Vec<Line> = lines
        .into_iter()
        .skip(scroll as usize)
        .take(height as usize)
        .enumerate()
        .map(|(i, line)| {
            let vis_row = scroll.saturating_add(i as u16);
            if focused && app.editing && vis_row == crow {
                caret_line(&line, ccol as usize, style, caret_style(app))
            } else {
                Line::from(line)
            }
        })
        .collect();
    paint(buf, Paragraph::new(shown).style(style), area);
}

fn paint_stepper(buf: &mut Buffer, app: &App, area: Rect, glyph: &str, focused: bool) {
    let style = if focused {
        Style::default().fg(app.theme.text_active_focus_color())
    } else {
        Style::default().fg(app.theme.text_secondary_color())
    };
    paint(buf, Paragraph::new(glyph).style(style), area);
}

fn paint_button(buf: &mut Buffer, app: &App, area: Rect, label: &str, focused: bool) {
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
    paint(buf, Paragraph::new(format!("[{label}]")).style(style), area);
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

    frame.render_widget(field_block(app, "Preview", false), area);
    // ~12px inset: 2 columns / 1 row inside the chrome so the title stays on body_background.
    let sample = area.inner(Margin::new(1, 1)).inner(Margin::new(2, 1));
    let text = sample.inner(Margin::new(1, 1));
    frame.render_widget(Block::default().style(style), sample);
    frame.render_widget(
        Paragraph::new(lines).style(style).wrap(Wrap { trim: true }),
        text,
    );
}

fn fill_conversions(buf: &mut Buffer, app: &App, area: Rect) {
    paint(buf, field_block(app, "Conversions", false), area);
    let inner = area.inner(Margin::new(1, 1));
    let [header, hex_row, rgb_row, hsl_row] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    let label = Style::default().fg(app.theme.text_labels_color());
    let value = Style::default().fg(app.theme.text_primary_color());

    paint_conversion_row(
        buf,
        header,
        "Format",
        "Foreground",
        "Background",
        label,
        label,
    );
    paint_conversion_row(
        buf,
        hex_row,
        "Hex",
        &app.foreground.to_hex(),
        &app.background.to_hex(),
        label,
        value,
    );
    paint_conversion_row(
        buf,
        rgb_row,
        "RGB",
        &app.foreground.to_rgb_str(),
        &app.background.to_rgb_str(),
        label,
        value,
    );
    paint_conversion_row(
        buf,
        hsl_row,
        "HSL",
        &app.foreground.to_hsl_str(),
        &app.background.to_hsl_str(),
        label,
        value,
    );
}

fn paint_conversion_row(
    buf: &mut Buffer,
    area: Rect,
    format: &str,
    fg: &str,
    bg: &str,
    format_style: Style,
    value_style: Style,
) {
    let [fmt, fg_col, bg_col] = Layout::horizontal([
        Constraint::Length(8),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .areas(area);
    paint(buf, Paragraph::new(format).style(format_style), fmt);
    paint(buf, Paragraph::new(fg).style(value_style), fg_col);
    paint(buf, Paragraph::new(bg).style(value_style), bg_col);
}

fn weight_label(app: &App) -> &'static str {
    if app.weight >= 700 { "bold" } else { "normal" }
}

fn fill_wcag_panel(buf: &mut Buffer, app: &App, area: Rect) {
    let ratio = app.contrast_ratio;
    let size = app.font_size_px;
    let weight = weight_label(app);
    let large = is_large_text(size, app.weight);
    let threshold = app.targets.wcag.text_threshold(large);
    let current_pass = ratio >= threshold;
    let aa_normal = ratio >= 4.5;
    let aa_large = ratio >= 3.0;
    let aaa_normal = ratio >= 7.0;
    let aaa_large = ratio >= 4.5;
    let ui = ratio >= 3.0;
    let labels = Style::default().fg(app.theme.text_labels_color());

    let mut lines = vec![
        Line::from(format!("{size}px {weight} | ratio {ratio:.2}")),
        Line::from(vec![
            Span::raw(format!(
                "needs >= {threshold} ({}) | ",
                app.targets.wcag.label()
            )),
            pass_fail(app, current_pass),
        ]),
        Line::from(format!(
            "AA n{} l{}  AAA n{} l{}  UI{}",
            mark(aa_normal),
            mark(aa_large),
            mark(aaa_normal),
            mark(aaa_large),
            mark(ui)
        )),
        Line::from(""),
        Line::styled(format!("Quick Reference ({weight})"), labels),
        Line::styled("Size  Ratio   Result", labels),
    ];
    for quick_size in [12_u16, 14, 16, 18] {
        let pass = ratio
            >= app
                .targets
                .wcag
                .text_threshold(is_large_text(quick_size, app.weight));
        lines.push(Line::from(vec![
            Span::raw(format!("{:<5} {ratio:<6.2}  ", format!("{quick_size}px"))),
            pass_fail(app, pass),
        ]));
    }

    paint(buf, field_block(app, "WCAG", false), area);
    paint(
        buf,
        Paragraph::new(lines).style(
            Style::default()
                .fg(app.theme.text_primary_color())
                .bg(app.theme.body_background_color()),
        ),
        area.inner(Margin::new(1, 1)),
    );
}

fn fill_apca_panel(buf: &mut Buffer, app: &App, area: Rect) {
    let size = app.font_size_px;
    let weight = weight_label(app);
    let lc = app.foreground.apca_lc(&app.background);
    let bar = app.targets.apca.value();
    let current_pass = lc.abs() >= bar;
    let lookup = apca_lookup_lc(size, app.weight);
    let polarity = if lc >= 0.0 { "light text" } else { "dark text" };
    let labels = Style::default().fg(app.theme.text_labels_color());

    let mut lines = vec![
        Line::from(format!("{size}px {weight} | Lc {lc:.2}")),
        Line::from(vec![
            Span::raw(format!(
                "needs >= {bar:.0} ({}) | ",
                app.targets.apca.label()
            )),
            pass_fail(app, current_pass),
        ]),
        Line::from(format!("lookup Lc{lookup:.0} · {polarity}")),
        Line::from(""),
        Line::styled(format!("Quick Reference ({weight})"), labels),
        Line::styled("Size  Lc      Result", labels),
    ];
    for quick_size in [12_u16, 14, 16, 18, 24] {
        let pass = lc.abs() >= apca_lookup_lc(quick_size, app.weight);
        lines.push(Line::from(vec![
            Span::raw(format!("{:<5} {lc:<7.2} ", format!("{quick_size}px"))),
            pass_fail(app, pass),
        ]));
    }

    paint(buf, field_block(app, "APCA", false), area);
    paint(
        buf,
        Paragraph::new(lines).style(
            Style::default()
                .fg(app.theme.text_primary_color())
                .bg(app.theme.body_background_color()),
        ),
        area.inner(Margin::new(1, 1)),
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
