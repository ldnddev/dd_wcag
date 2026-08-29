use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Breakpoint {
    Wide,
    Medium,
    #[default]
    Narrow,
}

impl Breakpoint {
    pub fn label(self) -> &'static str {
        match self {
            Self::Wide => "Wide",
            Self::Medium => "Medium",
            Self::Narrow => "Narrow",
        }
    }

    pub fn contrast_side_by_side(self) -> bool {
        !matches!(self, Self::Narrow)
    }

    pub fn palette_roles_width(self) -> Option<u16> {
        match self {
            Self::Wide => Some(28),
            Self::Medium => Some(24),
            Self::Narrow => None,
        }
    }

    pub fn fix_strip_height(self) -> Option<u16> {
        match self {
            Self::Wide => Some(7),
            Self::Medium => Some(6),
            Self::Narrow => None,
        }
    }
}

pub fn breakpoint(area: Rect) -> Breakpoint {
    match (area.width, area.height) {
        (w, h) if w >= 120 && h >= 28 => Breakpoint::Wide,
        (w, h) if w >= 100 && h >= 24 => Breakpoint::Medium,
        _ => Breakpoint::Narrow,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Shell {
    pub header: Rect,
    pub body: Rect,
    pub footer: Rect,
}

pub fn split_shell(area: Rect) -> Shell {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);
    Shell {
        header,
        body,
        footer,
    }
}

pub fn split_body_with_fix(body: Rect, bp: Breakpoint, fix_open: bool) -> (Rect, Option<Rect>) {
    if !fix_open {
        return (body, None);
    }
    match bp.fix_strip_height() {
        Some(height) => {
            let [main, fix] =
                Layout::vertical([Constraint::Fill(1), Constraint::Length(height)]).areas(body);
            (main, Some(fix))
        }
        None => (body, None),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HeaderLayout {
    pub tabs_contrast: Rect,
    pub tabs_palette: Rect,
    pub target_wcag: Rect,
    pub target_apca: Rect,
}

pub fn split_header(header: Rect) -> HeaderLayout {
    let inner = header.inner(Margin::new(1, 1));
    let [tabs, _spacer, targets] = Layout::horizontal([
        Constraint::Length(24),
        Constraint::Fill(1),
        Constraint::Length(32),
    ])
    .areas(inner);

    let [tabs_contrast, tabs_palette] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(tabs);
    let [target_wcag, target_apca] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(targets);

    HeaderLayout {
        tabs_contrast,
        tabs_palette,
        target_wcag,
        target_apca,
    }
}

pub fn centered(area: Rect, w_pct: u16, h_pct: u16) -> Rect {
    let w = area.width.saturating_mul(w_pct) / 100;
    let h = area.height.saturating_mul(h_pct) / 100;
    Rect {
        x: area.x + area.width.saturating_sub(w) / 2,
        y: area.y + area.height.saturating_sub(h) / 2,
        width: w.max(40),
        height: h.max(12),
    }
}

pub fn bottom_right_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width.saturating_sub(2)).max(16);
    let height = height.min(area.height.saturating_sub(3)).max(2);
    Rect {
        x: area
            .x
            .saturating_add(area.width.saturating_sub(width.saturating_add(1))),
        y: area
            .y
            .saturating_add(area.height.saturating_sub(height.saturating_add(2))),
        width,
        height,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    TabContrast,
    TabPalette,
    TargetWcag,
    TargetApca,
    FgInput,
    FgSwatch,
    BgInput,
    BgSwatch,
    SizeInput,
    SizeDec,
    SizeInc,
    WeightInput,
    WeightDec,
    WeightInc,
    Style(usize),
    PreviewText,
    FontFamily,
    Swap,
    Copy,
    FixBtn,
    WebBtn,
    ContrastPanel,
    ContrastScrollbar,
    Role(usize),
    TextRow,
    Generate,
    MatrixCell(usize, usize),
    PairList,
    Detail,
    DetailScrollbar,
    NudgeFg,
    NudgeBg,
    ApplyFix,
    NextFix,
    CloseFix,
    FixOutside,
    Toast,
    Popup,
    PopupOutside,
}

#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
pub struct LayoutMap {
    pub breakpoint: Breakpoint,
    pub tabs_contrast: Rect,
    pub tabs_palette: Rect,
    pub target_wcag: Rect,
    pub target_apca: Rect,
    pub footer: Rect,
    pub body: Rect,
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
    pub contrast_panel: Rect,
    pub contrast_scrollbar: Rect,
    pub role_rows: [Rect; 4],
    pub text_row: Rect,
    pub generate_btn: Rect,
    pub matrix_area: Rect,
    pub matrix_cells: [[Rect; 5]; 5],
    pub pair_list: Rect,
    pub detail: Rect,
    pub detail_scrollbar: Rect,
    pub fix_area: Rect,
    pub now_area: Rect,
    pub fixed_area: Rect,
    pub nudge_fg: Rect,
    pub nudge_bg: Rect,
    pub apply_btn: Rect,
    pub next_btn: Rect,
    pub close_fix: Rect,
    pub popup_area: Option<Rect>,
    pub toast_area: Option<Rect>,
}

pub fn contains(area: Rect, col: u16, row: u16) -> bool {
    area.width > 0
        && area.height > 0
        && col >= area.x
        && col < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}

pub fn char_index_at(area: Rect, x: u16, len: usize) -> usize {
    if area.width <= 2 || x <= area.x {
        return 0;
    }
    let text_x = area.x.saturating_add(1);
    if x <= text_x {
        return 0;
    }
    ((x - text_x) as usize).min(len)
}

/// Wrap `text` to `width` columns, honoring explicit newlines.
pub fn visual_lines(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut ended_with_newline = false;
    for ch in text.chars() {
        if ch == '\n' {
            lines.push(std::mem::take(&mut current));
            ended_with_newline = true;
            continue;
        }
        ended_with_newline = false;
        current.push(ch);
        if current.chars().count() >= width {
            lines.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() || lines.is_empty() || ended_with_newline {
        lines.push(current);
    }
    lines
}

pub fn visual_cursor(text: &str, char_idx: usize, width: u16) -> (u16, u16) {
    let width = width.max(1);
    let mut row = 0u16;
    let mut col = 0u16;
    for (i, ch) in text.chars().enumerate() {
        if i >= char_idx {
            break;
        }
        if ch == '\n' {
            row = row.saturating_add(1);
            col = 0;
        } else {
            col = col.saturating_add(1);
            if col >= width {
                row = row.saturating_add(1);
                col = 0;
            }
        }
    }
    (row, col)
}

/// Highlight the character at `cursor` (or a trailing space if the caret is at the end).
pub fn caret_line(text: &str, cursor: usize, style: Style, caret: Style) -> Line<'static> {
    let chars: Vec<char> = text.chars().collect();
    let idx = cursor.min(chars.len());
    let mut spans = Vec::new();
    if idx > 0 {
        spans.push(Span::styled(chars[..idx].iter().collect::<String>(), style));
    }
    let under = if idx < chars.len() {
        chars[idx].to_string()
    } else {
        " ".to_string()
    };
    spans.push(Span::styled(under, caret));
    if idx + 1 < chars.len() {
        spans.push(Span::styled(
            chars[idx + 1..].iter().collect::<String>(),
            style,
        ));
    }
    Line::from(spans)
}

pub fn view_scroll(cursor_row: u16, height: u16) -> u16 {
    let height = height.max(1);
    cursor_row.saturating_sub(height.saturating_sub(1))
}

/// Map a rect laid out in a virtual (0, 0) canvas into `viewport`, shifted by `scroll` rows.
pub fn visible_rect(virtual_rect: Rect, viewport: Rect, scroll: u16) -> Rect {
    if virtual_rect.width == 0
        || virtual_rect.height == 0
        || viewport.width == 0
        || viewport.height == 0
    {
        return Rect::default();
    }
    let x = i32::from(viewport.x) + i32::from(virtual_rect.x);
    let y = i32::from(viewport.y) + i32::from(virtual_rect.y) - i32::from(scroll);
    let x2 = x + i32::from(virtual_rect.width);
    let y2 = y + i32::from(virtual_rect.height);
    let vx2 = i32::from(viewport.x) + i32::from(viewport.width);
    let vy2 = i32::from(viewport.y) + i32::from(viewport.height);
    let ix = x.max(i32::from(viewport.x));
    let iy = y.max(i32::from(viewport.y));
    let ix2 = x2.min(vx2);
    let iy2 = y2.min(vy2);
    if ix2 <= ix || iy2 <= iy {
        return Rect::default();
    }
    Rect {
        x: ix as u16,
        y: iy as u16,
        width: (ix2 - ix) as u16,
        height: (iy2 - iy) as u16,
    }
}

/// Keep `target` (virtual coords) inside a viewport of `view_h` rows.
pub fn scroll_to_show(scroll: u16, view_h: u16, target: Rect) -> u16 {
    if target.height == 0 || view_h == 0 {
        return scroll;
    }
    let top = target.y;
    let bottom = target.y.saturating_add(target.height);
    if top < scroll {
        top
    } else if bottom > scroll.saturating_add(view_h) {
        bottom.saturating_sub(view_h)
    } else {
        scroll
    }
}

/// Map a click inside a wrapped text rect to a char index.
pub fn char_index_at_xy(text: &str, area: Rect, x: u16, y: u16, scroll: u16) -> usize {
    if area.width == 0 {
        return text.chars().count();
    }
    let width = area.width.max(1);
    let rel_x = x.saturating_sub(area.x).min(width.saturating_sub(1));
    let rel_y = y.saturating_sub(area.y);
    let target_row = scroll.saturating_add(rel_y);
    let mut row = 0u16;
    let mut col = 0u16;
    for (i, ch) in text.chars().enumerate() {
        if row == target_row && (ch == '\n' || col >= rel_x) {
            return i;
        }
        if row > target_row {
            return i.saturating_sub(1);
        }
        if ch == '\n' {
            row = row.saturating_add(1);
            col = 0;
        } else {
            col = col.saturating_add(1);
            if col >= width {
                row = row.saturating_add(1);
                col = 0;
            }
        }
    }
    text.chars().count()
}

impl LayoutMap {
    pub fn hit(&self, col: u16, row: u16) -> Option<Hit> {
        if let Some(toast) = self.toast_area
            && contains(toast, col, row)
        {
            return Some(Hit::Toast);
        }
        if let Some(popup) = self.popup_area {
            if contains(popup, col, row) {
                return Some(Hit::Popup);
            }
            return Some(Hit::PopupOutside);
        }

        if self.fix_area.width > 0 {
            if contains(self.apply_btn, col, row) {
                return Some(Hit::ApplyFix);
            }
            if contains(self.next_btn, col, row) {
                return Some(Hit::NextFix);
            }
            if contains(self.close_fix, col, row) {
                return Some(Hit::CloseFix);
            }
            if contains(self.nudge_fg, col, row) {
                return Some(Hit::NudgeFg);
            }
            if contains(self.nudge_bg, col, row) {
                return Some(Hit::NudgeBg);
            }
            if matches!(self.breakpoint, Breakpoint::Narrow)
                && contains(self.body, col, row)
                && !contains(self.fix_area, col, row)
            {
                return Some(Hit::FixOutside);
            }
        }

        if contains(self.tabs_contrast, col, row) {
            return Some(Hit::TabContrast);
        }
        if contains(self.tabs_palette, col, row) {
            return Some(Hit::TabPalette);
        }
        if contains(self.target_wcag, col, row) {
            return Some(Hit::TargetWcag);
        }
        if contains(self.target_apca, col, row) {
            return Some(Hit::TargetApca);
        }

        if contains(self.contrast_scrollbar, col, row) {
            return Some(Hit::ContrastScrollbar);
        }

        if contains(self.fg_input, col, row) {
            return Some(Hit::FgInput);
        }
        if contains(self.fg_swatch, col, row) {
            return Some(Hit::FgSwatch);
        }
        if contains(self.bg_input, col, row) {
            return Some(Hit::BgInput);
        }
        if contains(self.bg_swatch, col, row) {
            return Some(Hit::BgSwatch);
        }
        if contains(self.size_dec, col, row) {
            return Some(Hit::SizeDec);
        }
        if contains(self.size_inc, col, row) {
            return Some(Hit::SizeInc);
        }
        if contains(self.size_input, col, row) {
            return Some(Hit::SizeInput);
        }
        if contains(self.weight_dec, col, row) {
            return Some(Hit::WeightDec);
        }
        if contains(self.weight_inc, col, row) {
            return Some(Hit::WeightInc);
        }
        if contains(self.weight_input, col, row) {
            return Some(Hit::WeightInput);
        }
        for (i, rect) in self.style_btns.iter().enumerate() {
            if contains(*rect, col, row) {
                return Some(Hit::Style(i));
            }
        }
        if contains(self.preview_text, col, row) {
            return Some(Hit::PreviewText);
        }
        if contains(self.font_family, col, row) {
            return Some(Hit::FontFamily);
        }
        if contains(self.swap_btn, col, row) {
            return Some(Hit::Swap);
        }
        if contains(self.copy_btn, col, row) {
            return Some(Hit::Copy);
        }
        if contains(self.fix_btn, col, row) {
            return Some(Hit::FixBtn);
        }
        if contains(self.web_btn, col, row) {
            return Some(Hit::WebBtn);
        }
        if contains(self.contrast_panel, col, row) {
            return Some(Hit::ContrastPanel);
        }

        if contains(self.generate_btn, col, row) {
            return Some(Hit::Generate);
        }
        for (i, rect) in self.role_rows.iter().enumerate() {
            if contains(*rect, col, row) {
                return Some(Hit::Role(i));
            }
        }
        if contains(self.text_row, col, row) {
            return Some(Hit::TextRow);
        }
        if contains(self.detail_scrollbar, col, row) {
            return Some(Hit::DetailScrollbar);
        }
        if contains(self.detail, col, row) {
            return Some(Hit::Detail);
        }
        if contains(self.pair_list, col, row) {
            return Some(Hit::PairList);
        }
        for r in 0..5 {
            for c in 0..5 {
                if contains(self.matrix_cells[r][c], col, row) {
                    return Some(Hit::MatrixCell(r, c));
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: u16, y: u16, w: u16, h: u16) -> Rect {
        Rect::new(x, y, w, h)
    }

    #[test]
    fn breakpoint_uses_width_and_height_thresholds() {
        assert_eq!(breakpoint(r(0, 0, 120, 28)), Breakpoint::Wide);
        assert_eq!(breakpoint(r(0, 0, 100, 24)), Breakpoint::Medium);
        assert_eq!(breakpoint(r(0, 0, 119, 40)), Breakpoint::Medium);
        assert_eq!(breakpoint(r(0, 0, 140, 20)), Breakpoint::Narrow);
        assert_eq!(breakpoint(r(0, 0, 80, 40)), Breakpoint::Narrow);
    }

    #[test]
    fn split_shell_uses_fixed_header_and_footer_heights() {
        let shell = split_shell(r(0, 0, 120, 30));
        assert_eq!(shell.header.height, 3);
        assert_eq!(shell.footer.height, 1);
        assert_eq!(shell.body.height, 26);
        assert_eq!(shell.header.y, 0);
        assert_eq!(shell.footer.y, 29);
    }

    #[test]
    fn hit_prefers_toast_then_popup_then_tabs() {
        let mut map = LayoutMap {
            tabs_contrast: r(1, 1, 10, 1),
            toast_area: Some(r(80, 20, 20, 3)),
            popup_area: Some(r(10, 8, 40, 10)),
            ..LayoutMap::default()
        };
        assert_eq!(map.hit(82, 21), Some(Hit::Toast));
        map.toast_area = None;
        assert_eq!(map.hit(12, 9), Some(Hit::Popup));
        assert_eq!(map.hit(0, 0), Some(Hit::PopupOutside));
        map.popup_area = None;
        assert_eq!(map.hit(2, 1), Some(Hit::TabContrast));
    }

    #[test]
    fn char_index_at_clamps_to_field() {
        let area = r(4, 2, 12, 1);
        assert_eq!(char_index_at(area, 0, 8), 0);
        assert_eq!(char_index_at(area, 5, 8), 0);
        assert_eq!(char_index_at(area, 8, 8), 3);
        assert_eq!(char_index_at(area, 80, 8), 8);
    }

    #[test]
    fn caret_line_marks_start_middle_and_end() {
        let style = Style::default();
        let caret = Style::default().add_modifier(ratatui::style::Modifier::REVERSED);
        let start = caret_line("ab", 0, style, caret);
        assert_eq!(start.spans.len(), 2);
        assert_eq!(start.spans[0].content, "a");
        let mid = caret_line("ab", 1, style, caret);
        assert_eq!(mid.spans[0].content, "a");
        assert_eq!(mid.spans[1].content, "b");
        let end = caret_line("ab", 2, style, caret);
        assert_eq!(end.spans.last().map(|s| s.content.as_ref()), Some(" "));
    }

    #[test]
    fn visual_lines_wrap_and_keep_newlines() {
        assert_eq!(visual_lines("abcdef", 3), vec!["abc", "def"]);
        assert_eq!(visual_lines("ab\ncd", 10), vec!["ab", "cd"]);
    }

    #[test]
    fn char_index_at_xy_hits_second_row() {
        let area = r(0, 0, 10, 3);
        let text = "hello\nworld";
        assert_eq!(char_index_at_xy(text, area, 0, 1, 0), 6);
        assert_eq!(char_index_at_xy(text, area, 1, 1, 0), 7);
    }

    #[test]
    fn visible_rect_clips_scrolled_content() {
        let viewport = r(2, 3, 40, 10);
        assert_eq!(visible_rect(r(0, 0, 40, 3), viewport, 0), r(2, 3, 40, 3));
        assert_eq!(visible_rect(r(0, 0, 40, 3), viewport, 5).height, 0);
        assert_eq!(visible_rect(r(0, 8, 40, 3), viewport, 5), r(2, 6, 40, 3));
        assert_eq!(visible_rect(r(0, 0, 40, 3), viewport, 1), r(2, 3, 40, 2));
    }

    #[test]
    fn scroll_to_show_brings_target_into_view() {
        assert_eq!(scroll_to_show(0, 10, r(0, 0, 8, 3)), 0);
        assert_eq!(scroll_to_show(0, 10, r(0, 12, 8, 3)), 5);
        assert_eq!(scroll_to_show(8, 10, r(0, 0, 8, 3)), 0);
    }

    #[test]
    fn wide_fix_split_steals_a_bottom_strip() {
        let body = r(0, 3, 120, 24);
        let (main, fix) = split_body_with_fix(body, Breakpoint::Wide, true);
        assert_eq!(fix.map(|a| a.height), Some(7));
        assert_eq!(main.height + fix.unwrap().height, 24);
        let (main_closed, fix_closed) = split_body_with_fix(body, Breakpoint::Wide, false);
        assert!(fix_closed.is_none());
        assert_eq!(main_closed, body);
    }
}
