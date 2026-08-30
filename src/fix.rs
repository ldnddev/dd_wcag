use crate::color::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FixAxis {
    #[default]
    Fg,
    Bg,
}

impl FixAxis {
    pub fn label(self) -> &'static str {
        match self {
            Self::Fg => "FG",
            Self::Bg => "BG",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PairVerdict {
    pub ratio: f64,
    pub lc: f64,
    pub wcag: bool,
    pub apca: bool,
}

impl PairVerdict {
    pub fn of(fg: Color, bg: Color, wcag_threshold: f64, apca_bar: f64) -> Self {
        let ratio = fg.contrast_ratio(&bg);
        let lc = fg.apca_lc(&bg);
        Self {
            ratio,
            lc,
            wcag: ratio >= wcag_threshold,
            apca: lc.abs() >= apca_bar,
        }
    }

    pub fn label(self) -> &'static str {
        match (self.wcag, self.apca) {
            (true, true) => "PASS",
            (false, false) => "FAIL",
            _ => "~",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FixState {
    pub candidate_fg: Color,
    pub candidate_bg: Color,
    pub axis: FixAxis,
    #[allow(dead_code)]
    pub keep_hue: bool,
    index: usize,
    candidates: Vec<(Color, Color)>,
}

impl Default for FixState {
    fn default() -> Self {
        let black = Color(palette::Srgb::new(0.0, 0.0, 0.0));
        let white = Color(palette::Srgb::new(1.0, 1.0, 1.0));
        Self {
            candidate_fg: black,
            candidate_bg: white,
            axis: FixAxis::Fg,
            keep_hue: true,
            index: 0,
            candidates: Vec::new(),
        }
    }
}

impl FixState {
    pub fn search(&mut self, now_fg: Color, now_bg: Color, wcag_threshold: f64, apca_bar: f64) {
        self.candidates = collect_candidates(now_fg, now_bg, self.axis, wcag_threshold, apca_bar);
        self.index = 0;
        if let Some(&(fg, bg)) = self.candidates.first() {
            self.candidate_fg = fg;
            self.candidate_bg = bg;
        } else {
            self.candidate_fg = now_fg;
            self.candidate_bg = now_bg;
        }
    }

    pub fn next(&mut self) {
        if self.candidates.len() < 2 {
            return;
        }
        self.index = (self.index + 1) % self.candidates.len();
        let (fg, bg) = self.candidates[self.index];
        self.candidate_fg = fg;
        self.candidate_bg = bg;
    }

    pub fn set_axis_l(&mut self, axis: FixAxis, l: f32) {
        self.axis = axis;
        match axis {
            FixAxis::Fg => self.candidate_fg = self.candidate_fg.with_oklab_l(l),
            FixAxis::Bg => self.candidate_bg = self.candidate_bg.with_oklab_l(l),
        }
    }

    pub fn nudge(&mut self, axis: FixAxis, delta: f32) {
        self.axis = axis;
        match axis {
            FixAxis::Fg => self.candidate_fg = self.candidate_fg.nudge_oklab_l(delta),
            FixAxis::Bg => self.candidate_bg = self.candidate_bg.nudge_oklab_l(delta),
        }
    }

    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }
}

fn collect_candidates(
    now_fg: Color,
    now_bg: Color,
    preferred: FixAxis,
    wcag_threshold: f64,
    apca_bar: f64,
) -> Vec<(Color, Color)> {
    let other = match preferred {
        FixAxis::Fg => FixAxis::Bg,
        FixAxis::Bg => FixAxis::Fg,
    };
    let mut scored: Vec<(u8, u8, f32, Color, Color)> = Vec::new();
    for (axis_penalty, axis) in [(0_u8, preferred), (1, other)] {
        scored.extend(
            collect_axis(now_fg, now_bg, axis, wcag_threshold, apca_bar)
                .into_iter()
                .map(|(rank, dist, fg, bg)| (rank, axis_penalty, dist, fg, bg)),
        );
    }
    let has_apca = scored.iter().any(|(rank, _, _, _, _)| *rank <= 1);
    if !has_apca {
        // Changing only one axis cannot always reach the APCA bar (e.g. gray on gray).
        // Hold the other color at black or white and search L again.
        let extras = match preferred {
            FixAxis::Fg => [0.0, 1.0]
                .into_iter()
                .flat_map(|bg_l| {
                    collect_axis(
                        now_fg,
                        now_bg.with_oklab_l(bg_l),
                        FixAxis::Fg,
                        wcag_threshold,
                        apca_bar,
                    )
                })
                .collect::<Vec<_>>(),
            FixAxis::Bg => [0.0, 1.0]
                .into_iter()
                .flat_map(|fg_l| {
                    collect_axis(
                        now_fg.with_oklab_l(fg_l),
                        now_bg,
                        FixAxis::Bg,
                        wcag_threshold,
                        apca_bar,
                    )
                })
                .collect::<Vec<_>>(),
        };
        scored.extend(
            extras
                .into_iter()
                .map(|(rank, dist, fg, bg)| (rank, 2_u8, dist, fg, bg)),
        );
    }
    if scored.is_empty() {
        return vec![
            (now_fg.with_oklab_l(0.0), now_bg),
            (now_fg.with_oklab_l(1.0), now_bg),
            (now_fg, now_bg.with_oklab_l(0.0)),
            (now_fg, now_bg.with_oklab_l(1.0)),
        ];
    }
    scored.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.cmp(&b.1))
            .then(a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
    });
    scored
        .into_iter()
        .map(|(_, _, _, fg, bg)| (fg, bg))
        .collect()
}

fn collect_axis(
    now_fg: Color,
    now_bg: Color,
    axis: FixAxis,
    wcag_threshold: f64,
    apca_bar: f64,
) -> Vec<(u8, f32, Color, Color)> {
    let start = match axis {
        FixAxis::Fg => now_fg,
        FixAxis::Bg => now_bg,
    };
    let l0 = start.oklab_l();
    let mut scored = Vec::new();
    for i in 0..=50 {
        let l = i as f32 / 50.0;
        if (l - l0).abs() < 0.008 {
            continue;
        }
        let mutated = start.with_oklab_l(l);
        let (fg, bg) = match axis {
            FixAxis::Fg => (mutated, now_bg),
            FixAxis::Bg => (now_fg, mutated),
        };
        let verdict = PairVerdict::of(fg, bg, wcag_threshold, apca_bar);
        let rank = if verdict.wcag && verdict.apca {
            0
        } else if verdict.apca {
            1
        } else if verdict.wcag {
            2
        } else {
            continue;
        };
        scored.push((rank, (l - l0).abs(), fg, bg));
    }
    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    use palette::Srgb;

    #[test]
    fn search_finds_passing_fg_for_gray_on_gray() {
        let gray = Color(Srgb::new(0.5, 0.5, 0.5));
        let mut fix = FixState::default();
        fix.search(gray, gray, 4.5, 75.0);
        assert!(!fix.candidates.is_empty());
        let v = PairVerdict::of(fix.candidate_fg, fix.candidate_bg, 4.5, 75.0);
        assert!(
            v.apca,
            "Fix should meet the APCA bar when black/white L can"
        );
        assert!(v.wcag);
    }

    #[test]
    fn next_cycles_when_multiple_candidates_exist() {
        let gray = Color(Srgb::new(0.5, 0.5, 0.5));
        let mut fix = FixState::default();
        fix.search(gray, gray, 4.5, 75.0);
        let first = fix.candidate_fg.to_hex();
        fix.next();
        if fix.candidate_count() > 1 {
            assert_ne!(fix.candidate_fg.to_hex(), first);
        }
        for _ in 0..fix.candidate_count().saturating_sub(1) {
            fix.next();
        }
        assert_eq!(fix.candidate_fg.to_hex(), first);
    }
}
