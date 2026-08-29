use crate::color::Color;
use palette::{FromColor, Hsl, IntoColor, Srgb};

pub const PALETTE_EXPORT_PATH: &str = "_palette.scss";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteInput {
    Primary,
    Secondary,
    Tertiary,
    Support,
}

impl PaletteInput {
    pub const ALL: [Self; 4] = [
        Self::Primary,
        Self::Secondary,
        Self::Tertiary,
        Self::Support,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Primary => "Primary",
            Self::Secondary => "Secondary",
            Self::Tertiary => "Tertiary",
            Self::Support => "Support",
        }
    }

    pub fn required(self) -> bool {
        !matches!(self, Self::Support)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteApplyTarget {
    Foreground,
    Background,
}

impl PaletteApplyTarget {
    pub fn label(self) -> &'static str {
        match self {
            Self::Foreground => "FG",
            Self::Background => "BG",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PaletteState {
    pub primary_input: String,
    pub secondary_input: String,
    pub tertiary_input: String,
    pub support_input: String,
    pub selected_idx: usize,
    pub editing: bool,
    pub edit_input: String,
    pub edit_cursor_char_idx: usize,
    pub pending_apply: Option<PaletteApplyTarget>,
    pub detail_scroll: usize,
    pub detail_max_scroll: usize,
    pub generated: Option<GeneratedPalette>,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self {
            primary_input: "#88D9F7".to_string(),
            secondary_input: "#FFCA76".to_string(),
            tertiary_input: "#F98971".to_string(),
            support_input: "#46BE8C".to_string(),
            selected_idx: 0,
            editing: false,
            edit_input: String::new(),
            edit_cursor_char_idx: 0,
            pending_apply: None,
            detail_scroll: 0,
            detail_max_scroll: 0,
            generated: None,
        }
    }
}

impl PaletteState {
    pub fn selected(&self) -> PaletteInput {
        PaletteInput::ALL[self.selected_idx.min(PaletteInput::ALL.len() - 1)]
    }

    pub fn selected_input(&self) -> &str {
        self.input_for(self.selected())
    }

    pub fn input_for(&self, input: PaletteInput) -> &str {
        match input {
            PaletteInput::Primary => &self.primary_input,
            PaletteInput::Secondary => &self.secondary_input,
            PaletteInput::Tertiary => &self.tertiary_input,
            PaletteInput::Support => &self.support_input,
        }
    }

    fn input_for_mut(&mut self, input: PaletteInput) -> &mut String {
        match input {
            PaletteInput::Primary => &mut self.primary_input,
            PaletteInput::Secondary => &mut self.secondary_input,
            PaletteInput::Tertiary => &mut self.tertiary_input,
            PaletteInput::Support => &mut self.support_input,
        }
    }

    pub fn select_next(&mut self) {
        self.pending_apply = None;
        self.detail_scroll = 0;
        if self.selected_idx + 1 < PaletteInput::ALL.len() {
            self.selected_idx += 1;
        }
    }

    pub fn select_previous(&mut self) {
        self.pending_apply = None;
        self.detail_scroll = 0;
        self.selected_idx = self.selected_idx.saturating_sub(1);
    }

    pub fn begin_edit(&mut self) {
        self.pending_apply = None;
        self.editing = true;
        self.edit_input = self.selected_input().to_string();
        self.edit_cursor_char_idx = self.edit_input.chars().count();
    }

    pub fn cancel_edit(&mut self) {
        self.editing = false;
        self.edit_input.clear();
        self.edit_cursor_char_idx = 0;
    }

    pub fn commit_edit(&mut self) {
        let selected = self.selected();
        let value = self.edit_input.clone();
        *self.input_for_mut(selected) = value;
        self.cancel_edit();
        self.generated = None;
        self.detail_scroll = 0;
    }

    pub fn insert_char_at_cursor(&mut self, c: char) {
        self.clamp_cursor();
        let byte_idx = byte_index_at_char(&self.edit_input, self.edit_cursor_char_idx);
        self.edit_input.insert(byte_idx, c);
        self.edit_cursor_char_idx += 1;
    }

    pub fn backspace_at_cursor(&mut self) {
        self.clamp_cursor();
        if self.edit_cursor_char_idx == 0 {
            return;
        }
        let end = byte_index_at_char(&self.edit_input, self.edit_cursor_char_idx);
        let start = byte_index_at_char(&self.edit_input, self.edit_cursor_char_idx - 1);
        self.edit_input.replace_range(start..end, "");
        self.edit_cursor_char_idx -= 1;
    }

    pub fn move_cursor_left(&mut self) {
        self.edit_cursor_char_idx = self.edit_cursor_char_idx.saturating_sub(1);
    }

    pub fn move_cursor_right(&mut self) {
        let len = self.edit_input.chars().count();
        if self.edit_cursor_char_idx < len {
            self.edit_cursor_char_idx += 1;
        }
    }

    pub fn cursor_col(&self) -> u16 {
        self.edit_cursor_char_idx.min(u16::MAX as usize) as u16
    }

    pub fn scroll_detail_up(&mut self) {
        self.scroll_detail_by(-1);
    }

    pub fn scroll_detail_down(&mut self) {
        self.scroll_detail_by(1);
    }

    pub fn scroll_detail_by(&mut self, delta: i32) {
        if delta < 0 {
            self.detail_scroll = self
                .detail_scroll
                .saturating_sub(delta.unsigned_abs() as usize);
        } else {
            self.detail_scroll = self
                .detail_scroll
                .saturating_add(delta as usize)
                .min(self.detail_max_scroll);
        }
    }

    fn clamp_cursor(&mut self) {
        let len = self.edit_input.chars().count();
        if self.edit_cursor_char_idx > len {
            self.edit_cursor_char_idx = len;
        }
    }
}

#[derive(Debug, Clone)]
pub struct GeneratedPalette {
    pub tokens: Vec<PaletteToken>,
    pub checks: Vec<ComplianceCheck>,
    pub scss: String,
}

impl GeneratedPalette {
    pub fn blocking_failures(&self) -> Vec<&ComplianceCheck> {
        self.checks
            .iter()
            .filter(|check| check.blocking && !check.passes)
            .collect()
    }

    pub fn advisory_failures(&self) -> Vec<&ComplianceCheck> {
        self.checks
            .iter()
            .filter(|check| !check.blocking && !check.passes)
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct PaletteToken {
    pub name: String,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub struct ComplianceCheck {
    pub label: String,
    pub ratio: f64,
    pub threshold: f64,
    pub passes: bool,
    pub blocking: bool,
}

#[derive(Debug, Clone)]
struct PaletteBaseColors {
    primary: Color,
    secondary: Color,
    tertiary: Color,
    support: Color,
}

pub fn generate_palette(state: &PaletteState) -> Result<GeneratedPalette, String> {
    let base = parse_base_colors(state)?;
    let mut tokens = Vec::new();

    push_brand_tokens(&mut tokens, "primary", base.primary);
    push_brand_tokens(&mut tokens, "secondary", base.secondary);
    push_brand_tokens(&mut tokens, "tertiary", base.tertiary);
    push_action_tokens(&mut tokens, "primary", base.primary);
    push_action_tokens(&mut tokens, "secondary", base.secondary);
    push_action_tokens(&mut tokens, "tertiary", base.tertiary);
    push_semantic_tokens(&mut tokens);
    push_text_tokens(&mut tokens);
    push_neutral_tokens(&mut tokens);
    push_support_tokens(&mut tokens, base.support);

    let checks = build_checks(&tokens);
    let scss = render_scss(&tokens);

    Ok(GeneratedPalette {
        tokens,
        checks,
        scss,
    })
}

pub fn validate_export(
    generated: Option<&GeneratedPalette>,
    action: &str,
) -> Result<String, String> {
    let Some(generated) = generated else {
        return Err(format!("Generate palette before {action}"));
    };

    let failures = generated.blocking_failures();
    if let Some(first) = failures.first() {
        let remaining = failures.len().saturating_sub(1);
        return Err(format!(
            "{} failed ({:.2}:1, needs {:.1}:1); {remaining} more blocking failure(s)",
            first.label, first.ratio, first.threshold
        ));
    }

    Ok(generated.scss.clone())
}

pub fn parse_palette_color(input: &str) -> Result<Color, String> {
    let value = input.trim();
    let lower = value.to_lowercase();
    if lower.starts_with("rgba(") || lower.starts_with("rgb(") {
        Color::parse_rgb(value).map_err(|err| format!("Invalid RGB/RGBA format: {err}"))
    } else if lower.starts_with("hsl(") {
        Color::parse_hsl(value).map_err(|err| format!("Invalid HSL format: {err}"))
    } else {
        Color::parse_hex(value).map_err(|err| format!("Invalid HEX format: {err}"))
    }
}

fn parse_base_colors(state: &PaletteState) -> Result<PaletteBaseColors, String> {
    let primary = parse_required_base(state, PaletteInput::Primary)?;
    let secondary = parse_required_base(state, PaletteInput::Secondary)?;
    let tertiary = parse_required_base(state, PaletteInput::Tertiary)?;
    let support = if state.support_input.trim().is_empty() {
        Color::parse_hex("#46BE8C").expect("default support color is valid")
    } else {
        parse_palette_color(&state.support_input)
            .map_err(|err| format!("Support color error: {err}"))?
    };

    Ok(PaletteBaseColors {
        primary,
        secondary,
        tertiary,
        support,
    })
}

fn parse_required_base(state: &PaletteState, input: PaletteInput) -> Result<Color, String> {
    let raw = state.input_for(input);
    if raw.trim().is_empty() {
        return Err(format!("{} color is required", input.label()));
    }
    parse_palette_color(raw).map_err(|err| format!("{} color error: {err}", input.label()))
}

fn push_brand_tokens(tokens: &mut Vec<PaletteToken>, family: &str, base: Color) {
    push_token(tokens, format!("$c_{family}_default"), base);
    push_token(
        tokens,
        format!("$c_{family}_default--dark"),
        adjust_lightness(base, 0.68),
    );
    push_token(
        tokens,
        format!("$c_{family}_strong"),
        ensure_contrast(base, fixed_text_primary(), 4.5),
    );
    push_token(
        tokens,
        format!("$c_{family}_strong--dark"),
        ensure_contrast(adjust_lightness(base, 0.78), fixed_text_inverse_dark(), 4.5),
    );
    push_token(
        tokens,
        format!("$c_{family}_subtle"),
        adjust_lightness(base, 0.94),
    );
    push_token(
        tokens,
        format!("$c_{family}_subtle--dark"),
        adjust_lightness(base, 0.18),
    );
}

fn push_action_tokens(tokens: &mut Vec<PaletteToken>, family: &str, base: Color) {
    let text = best_fixed_text_for(base).unwrap_or_else(fixed_text_primary);
    let dark_surface = adjust_lightness(base, 0.66);
    let dark_text = best_fixed_text_for(dark_surface).unwrap_or_else(fixed_text_inverse_dark);

    for (state, surface, dark_surface) in [
        ("default", base, dark_surface),
        (
            "hover",
            adjust_lightness(base, 0.64),
            adjust_lightness(base, 0.72),
        ),
        (
            "pressed",
            adjust_lightness(base, 0.54),
            adjust_lightness(base, 0.58),
        ),
        (
            "disabled",
            adjust_lightness(base, 0.82),
            adjust_lightness(base, 0.30),
        ),
    ] {
        let light_text = if state == "disabled" {
            fixed_text_disabled()
        } else {
            best_fixed_text_for(surface).unwrap_or(text)
        };
        let dark_text = if state == "disabled" {
            fixed_text_disabled_dark()
        } else {
            best_fixed_text_for(dark_surface).unwrap_or(dark_text)
        };

        push_token(
            tokens,
            format!("$c_{family}_action_{state}_surface"),
            surface,
        );
        push_token(
            tokens,
            format!("$c_{family}_action_{state}_surface--dark"),
            dark_surface,
        );
        push_token(
            tokens,
            format!("$c_{family}_action_{state}_text"),
            light_text,
        );
        push_token(
            tokens,
            format!("$c_{family}_action_{state}_text--dark"),
            dark_text,
        );
        push_token(
            tokens,
            format!("$c_{family}_action_{state}_border"),
            ensure_non_text_contrast(adjust_lightness(surface, 0.42), surface),
        );
        push_token(
            tokens,
            format!("$c_{family}_action_{state}_border--dark"),
            ensure_non_text_contrast(adjust_lightness(dark_surface, 0.86), dark_surface),
        );
    }
}

fn push_support_tokens(tokens: &mut Vec<PaletteToken>, base: Color) {
    push_token(
        tokens,
        "$c_support_overlay",
        Color::parse_hex("#000000").expect("overlay"),
    );
    push_token(
        tokens,
        "$c_support_overlay--dark",
        Color::parse_hex("#000000").expect("overlay"),
    );
    push_token(
        tokens,
        "$c_support_border",
        Color::parse_hex("#E0E3E7").expect("support border"),
    );
    push_token(
        tokens,
        "$c_support_border--dark",
        Color::parse_hex("#2A2D31").expect("support border dark"),
    );
    push_token(tokens, "$c_support_focus", base);
    push_token(
        tokens,
        "$c_support_focus--dark",
        adjust_lightness(base, 0.72),
    );
    push_token(tokens, "$c_support_disabled", fixed_text_disabled());
    push_token(
        tokens,
        "$c_support_disabled--dark",
        fixed_text_disabled_dark(),
    );
}

fn push_semantic_tokens(tokens: &mut Vec<PaletteToken>) {
    for (name, hex) in [
        ("$c_success_surface", "#E9F7EF"),
        ("$c_success_surface--dark", "#0B1C13"),
        ("$c_success_text", "#1E8449"),
        ("$c_success_text--dark", "#82E0AA"),
        ("$c_success_border", "#82E0AA"),
        ("$c_success_border--dark", "#27AE60"),
        ("$c_warning_surface", "#FEF5E7"),
        ("$c_warning_surface--dark", "#1B1607"),
        ("$c_warning_text", "#B9770E"),
        ("$c_warning_text--dark", "#F5C469"),
        ("$c_warning_border", "#F5CBA7"),
        ("$c_warning_border--dark", "#B9770E"),
        ("$c_error_surface", "#FDEDEC"),
        ("$c_error_surface--dark", "#240C0C"),
        ("$c_error_text", "#A93226"),
        ("$c_error_text--dark", "#E57373"),
        ("$c_error_border", "#F1948A"),
        ("$c_error_border--dark", "#CB4335"),
        ("$c_info_surface", "#21618C"),
        ("$c_info_surface--dark", "#5DADE2"),
        ("$c_info_text", "#EBF5FB"),
        ("$c_info_text--dark", "#0B141C"),
        ("$c_info_border", "#85C1E9"),
        ("$c_info_border--dark", "#2874A6"),
    ] {
        push_token(
            tokens,
            name,
            Color::parse_hex(hex).expect("semantic token hex is valid"),
        );
    }
}

fn push_text_tokens(tokens: &mut Vec<PaletteToken>) {
    for (name, color) in [
        ("$c_text_primary", fixed_text_primary()),
        ("$c_text_primary--dark", fixed_text_primary_dark()),
        ("$c_text_secondary", fixed_text_secondary()),
        ("$c_text_secondary--dark", fixed_text_secondary_dark()),
        ("$c_text_disabled", fixed_text_disabled()),
        ("$c_text_disabled--dark", fixed_text_disabled_dark()),
        ("$c_text_inverse", fixed_text_inverse()),
        ("$c_text_inverse--dark", fixed_text_inverse_dark()),
    ] {
        push_token(tokens, name, color);
    }
}

fn push_neutral_tokens(tokens: &mut Vec<PaletteToken>) {
    for (name, color) in [
        ("$c_ui_neutral_100", neutral_100()),
        ("$c_ui_neutral_100--dark", neutral_100_dark()),
        (
            "$c_ui_neutral_200",
            Color::parse_hex("#F2F3F5").expect("valid neutral"),
        ),
        (
            "$c_ui_neutral_200--dark",
            Color::parse_hex("#1A1C1F").expect("valid neutral"),
        ),
        (
            "$c_ui_neutral_300",
            Color::parse_hex("#D9DCE1").expect("valid neutral"),
        ),
        (
            "$c_ui_neutral_300--dark",
            Color::parse_hex("#2A2D31").expect("valid neutral"),
        ),
        ("$c_ui_neutral_600", fixed_text_secondary()),
        ("$c_ui_neutral_600--dark", fixed_text_secondary_dark()),
        ("$c_ui_neutral_900", fixed_text_primary()),
        ("$c_ui_neutral_900--dark", fixed_text_primary_dark()),
    ] {
        push_token(tokens, name, color);
    }
}

fn build_checks(tokens: &[PaletteToken]) -> Vec<ComplianceCheck> {
    let mut checks = Vec::new();
    for family in ["primary", "secondary", "tertiary"] {
        for role in ["default", "strong", "subtle"] {
            for suffix in ["", "--dark"] {
                let name = format!("$c_{family}_{role}{suffix}");
                if let Some(surface) = find_token(tokens, &name) {
                    let best = best_fixed_text_for(surface.color);
                    let (ratio, passes) = best
                        .map(|text| {
                            let ratio = text.contrast_ratio(&surface.color);
                            (ratio, ratio >= 4.5)
                        })
                        .unwrap_or((0.0, false));
                    checks.push(ComplianceCheck {
                        label: format!("{name} fixed text recommendation"),
                        ratio,
                        threshold: 4.5,
                        passes,
                        blocking: true,
                    });
                }
            }
        }

        for state in ["default", "hover", "pressed", "disabled"] {
            for suffix in ["", "--dark"] {
                let surface_name = format!("$c_{family}_action_{state}_surface{suffix}");
                let text_name = format!("$c_{family}_action_{state}_text{suffix}");
                let border_name = format!("$c_{family}_action_{state}_border{suffix}");
                if let (Some(surface), Some(text)) = (
                    find_token(tokens, &surface_name),
                    find_token(tokens, &text_name),
                ) {
                    let ratio = text.color.contrast_ratio(&surface.color);
                    checks.push(ComplianceCheck {
                        label: format!("{text_name} on {surface_name}"),
                        ratio,
                        threshold: 4.5,
                        passes: ratio >= 4.5,
                        blocking: state != "disabled",
                    });
                }
                if let (Some(surface), Some(border)) = (
                    find_token(tokens, &surface_name),
                    find_token(tokens, &border_name),
                ) {
                    let ratio = border.color.contrast_ratio(&surface.color);
                    checks.push(ComplianceCheck {
                        label: format!("{border_name} against {surface_name}"),
                        ratio,
                        threshold: 3.0,
                        passes: ratio >= 3.0,
                        blocking: state != "disabled",
                    });
                }
            }
        }
    }

    for (name, background) in [
        ("$c_support_border", neutral_100()),
        ("$c_support_focus", neutral_100()),
        ("$c_support_border--dark", neutral_100_dark()),
        ("$c_support_focus--dark", neutral_100_dark()),
    ] {
        if let Some(token) = find_token(tokens, name) {
            let ratio = token.color.contrast_ratio(&background);
            checks.push(ComplianceCheck {
                label: format!("{name} against neutral surface"),
                ratio,
                threshold: 3.0,
                passes: ratio >= 3.0,
                blocking: false,
            });
        }
    }

    checks
}

fn scss_rgba(tokens: &[PaletteToken], name: &str) -> String {
    find_token(tokens, name)
        .map(|token| rgba_string(token.color))
        .unwrap_or_else(|| "rgba(0, 0, 0, 1)".to_string())
}

fn emit_comment(out: &mut String, lines: &[&str]) {
    out.push_str("/**\n");
    for line in lines {
        out.push_str(" * ");
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(" */\n");
}

fn emit_pair(out: &mut String, tokens: &[PaletteToken], light: &str, dark: &str) {
    out.push_str(&format!("{light}: {};\n", scss_rgba(tokens, light)));
    out.push_str(&format!("{dark}: {};\n", scss_rgba(tokens, dark)));
}

fn emit_blanks(out: &mut String, n: usize) {
    for _ in 0..n {
        out.push('\n');
    }
}

fn emit_action(out: &mut String, tokens: &[PaletteToken], family: &str, title: &str) {
    emit_comment(
        out,
        &[
            title,
            "Action (CTAs / Buttons / Interactions)",
            "This is your engagement system — designed for clarity, energy, and ADA-compliant interaction states.Every state (default, hover, pressed, disabled) has coordinated surface, border, and text colors for intuitive feedback across light and dark UIs.",
        ],
    );
    for state in ["default", "hover", "pressed", "disabled"] {
        for part in ["surface", "text", "border"] {
            emit_pair(
                out,
                tokens,
                &format!("$c_{family}_action_{state}_{part}"),
                &format!("$c_{family}_action_{state}_{part}--dark"),
            );
            emit_blanks(out, 1);
        }
    }
    emit_blanks(out, 3);
}

fn render_scss(tokens: &[PaletteToken]) -> String {
    let mut out = String::new();

    emit_comment(
        &mut out,
        &[
            "Primary (Brand Placeholder)",
            "This is your primary identity color system — the flexible accent that becomes the brand.",
            "It drives recognition and hierarchy without locking you into a specific color palette. Swap these tones for any client brand.",
        ],
    );
    emit_pair(
        &mut out,
        tokens,
        "$c_primary_default",
        "$c_primary_default--dark",
    );
    emit_blanks(&mut out, 1);
    emit_pair(
        &mut out,
        tokens,
        "$c_primary_strong",
        "$c_primary_strong--dark",
    );
    emit_blanks(&mut out, 1);
    emit_pair(
        &mut out,
        tokens,
        "$c_primary_subtle",
        "$c_primary_subtle--dark",
    );
    emit_blanks(&mut out, 4);

    emit_comment(
        &mut out,
        &[
            "Secondary (Brand Placeholder)",
            "This is your secondary identity color system — the flexible accent that becomes the brand.",
            "It drives recognition and hierarchy without locking you into a specific color palette. Swap these tones for any client brand.",
        ],
    );
    emit_pair(
        &mut out,
        tokens,
        "$c_secondary_default",
        "$c_secondary_default--dark",
    );
    emit_blanks(&mut out, 1);
    emit_pair(
        &mut out,
        tokens,
        "$c_secondary_strong",
        "$c_secondary_strong--dark",
    );
    emit_blanks(&mut out, 1);
    emit_pair(
        &mut out,
        tokens,
        "$c_secondary_subtle",
        "$c_secondary_subtle--dark",
    );
    emit_blanks(&mut out, 4);

    emit_comment(
        &mut out,
        &[
            "Tertiary (Brand Placeholder)",
            "This is your tertiary identity color system — the flexible accent that becomes the brand.",
            "It drives recognition and hierarchy without locking you into a specific color palette. Swap these tones for any client brand.",
        ],
    );
    emit_pair(
        &mut out,
        tokens,
        "$c_tertiary_default",
        "$c_tertiary_default--dark",
    );
    emit_blanks(&mut out, 1);
    emit_pair(
        &mut out,
        tokens,
        "$c_tertiary_strong",
        "$c_tertiary_strong--dark",
    );
    emit_blanks(&mut out, 1);
    emit_pair(
        &mut out,
        tokens,
        "$c_tertiary_subtle",
        "$c_tertiary_subtle--dark",
    );
    emit_blanks(&mut out, 4);

    emit_action(
        &mut out,
        tokens,
        "primary",
        "Primary Action (cyan-blue family)",
    );
    emit_action(
        &mut out,
        tokens,
        "secondary",
        "Secondary Action (warm peach family)",
    );
    emit_action(
        &mut out,
        tokens,
        "tertiary",
        "Tertiary Action (coral family)",
    );

    emit_comment(&mut out, &["Semantic (Status / Feedback / Alerts)"]);
    for name in ["success", "warning", "error", "info"] {
        emit_pair(
            &mut out,
            tokens,
            &format!("$c_{name}_surface"),
            &format!("$c_{name}_surface--dark"),
        );
        emit_blanks(&mut out, 1);
        emit_pair(
            &mut out,
            tokens,
            &format!("$c_{name}_text"),
            &format!("$c_{name}_text--dark"),
        );
        emit_blanks(&mut out, 1);
        emit_pair(
            &mut out,
            tokens,
            &format!("$c_{name}_border"),
            &format!("$c_{name}_border--dark"),
        );
        emit_blanks(&mut out, 1);
    }
    emit_blanks(&mut out, 3);

    emit_comment(&mut out, &["Text Roles (Color Mapping)"]);
    emit_pair(&mut out, tokens, "$c_text_primary", "$c_text_primary--dark");
    emit_blanks(&mut out, 1);
    emit_pair(
        &mut out,
        tokens,
        "$c_text_secondary",
        "$c_text_secondary--dark",
    );
    emit_blanks(&mut out, 1);
    emit_pair(
        &mut out,
        tokens,
        "$c_text_disabled",
        "$c_text_disabled--dark",
    );
    emit_blanks(&mut out, 1);
    emit_pair(&mut out, tokens, "$c_text_inverse", "$c_text_inverse--dark");
    emit_blanks(&mut out, 4);

    emit_comment(&mut out, &["Neutrals (Core UI / Backgrounds)"]);
    emit_pair(
        &mut out,
        tokens,
        "$c_ui_neutral_100",
        "$c_ui_neutral_100--dark",
    );
    emit_blanks(&mut out, 1);
    emit_pair(
        &mut out,
        tokens,
        "$c_ui_neutral_200",
        "$c_ui_neutral_200--dark",
    );
    emit_blanks(&mut out, 1);
    emit_pair(
        &mut out,
        tokens,
        "$c_ui_neutral_300",
        "$c_ui_neutral_300--dark",
    );
    emit_blanks(&mut out, 1);
    emit_pair(
        &mut out,
        tokens,
        "$c_ui_neutral_600",
        "$c_ui_neutral_600--dark",
    );
    emit_blanks(&mut out, 1);
    emit_pair(
        &mut out,
        tokens,
        "$c_ui_neutral_900",
        "$c_ui_neutral_900--dark",
    );
    emit_blanks(&mut out, 3);

    emit_comment(
        &mut out,
        &[
            "Support / Utility",
            "Used for default overlays, borders, and hover / focus states.",
        ],
    );
    out.push_str("$c_support_overlay: rgba(0, 0, 0, 0.8);\n");
    out.push_str("$c_support_overlay--dark: rgba(0, 0, 0, 0.8);\n");
    emit_blanks(&mut out, 1);
    emit_pair(
        &mut out,
        tokens,
        "$c_support_border",
        "$c_support_border--dark",
    );
    emit_blanks(&mut out, 1);
    emit_pair(
        &mut out,
        tokens,
        "$c_support_focus",
        "$c_support_focus--dark",
    );
    emit_blanks(&mut out, 1);
    emit_pair(
        &mut out,
        tokens,
        "$c_support_disabled",
        "$c_support_disabled--dark",
    );
    emit_blanks(&mut out, 4);

    emit_comment(&mut out, &["admin button colors"]);
    out.push_str(" $c_black: #000000;\n");
    out.push_str(" $c_red: #bb0000;\n");
    out.push_str(" $c_blue: #0074BD;\n");
    out.push_str(" $c_green: #4d8f46;\n");
    out.push_str(" $c_yellow: #e7ff6f;\n");
    out.push_str(" $c_white: #ffffff;\n");
    emit_blanks(&mut out, 4);

    emit_comment(&mut out, &["Social Media Colors"]);
    out.push_str(" $c_facebook: #3b5998;\n");
    out.push_str(" $c_twitter: #00aced;\n");
    out.push_str(" $c_pinterest: #bb0000;\n");
    out.push_str(" $c_instagram: #777777;\n");
    out.push_str(" $c_youtube: #777777;\n");
    out.push_str(" $c_comments: #222222;\n");
    emit_blanks(&mut out, 4);

    emit_comment(&mut out, &["Default widths"]);
    out.push_str("$width_xs: 20.5em;\n");
    out.push_str("$width_sm: 33.5em;\n");
    out.push_str("$width_md: 48em;\n");
    out.push_str("$width_lg: 64em;\n");
    out.push_str("$width_xl: 80em;\n");
    out.push_str("$width_xxl: 120em;\n");
    out.push_str("$width_xxxl: 160em;\n");
    out.push_str("$width_x4k: 240em;\n");
    out.push_str("$max-width: $width_xxl;\n");

    out
}

fn find_token<'a>(tokens: &'a [PaletteToken], name: &str) -> Option<&'a PaletteToken> {
    tokens.iter().find(|token| token.name == name)
}

fn push_token(tokens: &mut Vec<PaletteToken>, name: impl Into<String>, color: Color) {
    tokens.push(PaletteToken {
        name: name.into(),
        color,
    });
}

fn best_fixed_text_for(surface: Color) -> Option<Color> {
    fixed_text_candidates()
        .into_iter()
        .filter(|text| text.contrast_ratio(&surface) >= 4.5)
        .max_by(|a, b| {
            a.contrast_ratio(&surface)
                .total_cmp(&b.contrast_ratio(&surface))
        })
}

fn fixed_text_candidates() -> [Color; 6] {
    [
        fixed_text_primary(),
        fixed_text_secondary(),
        fixed_text_inverse(),
        fixed_text_primary_dark(),
        fixed_text_secondary_dark(),
        fixed_text_inverse_dark(),
    ]
}

fn ensure_contrast(mut color: Color, text: Color, threshold: f64) -> Color {
    let mut hsl = color_to_hsl(color);
    let prefer_darker = text.luminance() < 0.5;
    for _ in 0..80 {
        if color.contrast_ratio(&text) >= threshold {
            return color;
        }
        hsl.lightness = if prefer_darker {
            (hsl.lightness - 0.025).max(0.0)
        } else {
            (hsl.lightness + 0.025).min(1.0)
        };
        color = hsl_to_color(hsl);
    }
    color
}

fn ensure_non_text_contrast(mut color: Color, background: Color) -> Color {
    let mut hsl = color_to_hsl(color);
    let prefer_darker = fixed_text_inverse_dark().contrast_ratio(&background)
        >= fixed_text_inverse().contrast_ratio(&background);
    for _ in 0..80 {
        if color.contrast_ratio(&background) >= 3.0 {
            return color;
        }
        hsl.lightness = if prefer_darker {
            (hsl.lightness - 0.025).max(0.0)
        } else {
            (hsl.lightness + 0.025).min(1.0)
        };
        color = hsl_to_color(hsl);
    }
    color
}

fn adjust_lightness(color: Color, lightness: f32) -> Color {
    let mut hsl = color_to_hsl(color);
    hsl.lightness = lightness.clamp(0.0, 1.0);
    hsl_to_color(hsl)
}

fn color_to_hsl(color: Color) -> Hsl {
    color.0.into_color()
}

fn hsl_to_color(hsl: Hsl) -> Color {
    Color(Srgb::from_color(hsl))
}

fn rgba_string(color: Color) -> String {
    let r = channel_to_u8(color.0.red);
    let g = channel_to_u8(color.0.green);
    let b = channel_to_u8(color.0.blue);
    format!("rgba({r}, {g}, {b}, 1)")
}

fn channel_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn byte_index_at_char(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

fn fixed_text_primary() -> Color {
    Color::parse_hex("#1C1E21").expect("fixed text token is valid")
}

fn fixed_text_primary_dark() -> Color {
    Color::parse_hex("#F5F6F7").expect("fixed text token is valid")
}

fn fixed_text_secondary() -> Color {
    Color::parse_hex("#5A5F66").expect("fixed text token is valid")
}

fn fixed_text_secondary_dark() -> Color {
    Color::parse_hex("#9EA3AA").expect("fixed text token is valid")
}

fn fixed_text_disabled() -> Color {
    Color::parse_hex("#A0A4A8").expect("fixed text token is valid")
}

fn fixed_text_disabled_dark() -> Color {
    Color::parse_hex("#5A5F66").expect("fixed text token is valid")
}

fn fixed_text_inverse() -> Color {
    Color::parse_hex("#F9FAFB").expect("fixed text token is valid")
}

fn fixed_text_inverse_dark() -> Color {
    Color::parse_hex("#0F1114").expect("fixed text token is valid")
}

fn neutral_100() -> Color {
    Color::parse_hex("#F9FAFB").expect("neutral token is valid")
}

fn neutral_100_dark() -> Color {
    Color::parse_hex("#0F1114").expect("neutral token is valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_copy_ready_scss_with_required_groups() {
        let generated = generate_palette(&PaletteState::default()).expect("palette generates");

        assert!(generated.scss.contains("$c_primary_default: rgba("));
        assert!(generated.scss.contains("$c_primary_action_default_surface"));
        assert!(
            generated
                .scss
                .contains("$c_text_primary: rgba(28, 30, 33, 1);")
        );
        assert!(generated.scss.contains("$c_support_focus--dark"));
        assert!(generated.scss.contains("Primary (Brand Placeholder)"));
        assert!(generated.scss.contains("$c_support_disabled"));
        assert!(generated.scss.contains("$c_black: #000000"));
        assert!(generated.scss.contains("$c_facebook:"));
        assert!(generated.scss.contains("$width_xs: 20.5em"));
        assert!(generated.scss.contains("$max-width: $width_xxl"));
        assert!(
            generated
                .scss
                .contains("$c_support_overlay: rgba(0, 0, 0, 0.8)")
        );
    }

    #[test]
    fn generated_scss_lists_every_framework_variable_name() {
        let generated = generate_palette(&PaletteState::default()).expect("palette generates");
        let framework = include_str!("../_variables.scss");
        for line in framework.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with('$') {
                continue;
            }
            let name = trimmed.split(':').next().expect("variable name");
            assert!(
                generated.scss.contains(name),
                "generated SCSS missing framework variable {name}"
            );
        }
    }

    #[test]
    fn default_palette_has_no_blocking_failures() {
        let generated = generate_palette(&PaletteState::default()).expect("palette generates");

        let failures = generated.blocking_failures();
        assert!(
            failures.is_empty(),
            "blocking failures: {:?}",
            failures
                .iter()
                .map(|check| format!("{} {:.2}/{:.1}", check.label, check.ratio, check.threshold))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn invalid_required_base_color_blocks_generation() {
        let state = PaletteState {
            primary_input: "nope".to_string(),
            ..PaletteState::default()
        };

        let err = generate_palette(&state).expect_err("invalid input fails");

        assert!(err.contains("Primary color error"));
    }

    #[test]
    fn validate_export_requires_generated_palette() {
        let err = validate_export(None, "saving").expect_err("missing palette fails");

        assert_eq!(err, "Generate palette before saving");
    }
}
