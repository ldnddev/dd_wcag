//! Contrast and Palette body. Paint reads the shared app, the same state as the tests.

use dd_wcag::app::{FocusId, Mode, StylePreset};
use dd_wcag::fix::FixAxis;
use dd_wcag::palette::{self, MatrixAxis, PaletteInput};
use gpui::{
    ClickEvent, Context, FontWeight, IntoElement, ParentElement, Styled, div, prelude::*, px,
};

use crate::paint::{self, Packed};
use crate::product::Click;
use crate::shell::Shell;

pub fn header(cx: &mut Context<Shell>, shell: &Shell, pack: Packed, bar: f32) -> gpui::Div {
    let contrast = shell.product.app.mode == Mode::Contrast;
    div()
        .h(px(bar))
        .px(px(12.0))
        .flex()
        .items_center()
        .gap(px(12.0))
        .bg(pack.raised)
        .text_color(pack.text)
        .border_b_1()
        .border_color(pack.border)
        .child(div().font_weight(FontWeight::BOLD).child("dd_wcag"))
        .child(mode_tab(cx, "tab-contrast", "Contrast", contrast, pack))
        .child(mode_tab(cx, "tab-palette", "Palette", !contrast, pack))
}

pub fn toast(
    cx: &mut Context<Shell>,
    shell: &Shell,
    pack: Packed,
) -> Option<gpui::Stateful<gpui::Div>> {
    let message = shell
        .product
        .app
        .error
        .clone()
        .or_else(|| shell.product.app.status.clone())
        .filter(|message| !message.is_empty())?;
    let color = if shell.product.app.error.is_some() {
        pack.error
    } else {
        pack.success
    };
    Some(
        div()
            .id("toast")
            .px(px(12.0))
            .py(px(6.0))
            .bg(pack.panel)
            .text_color(color)
            .border_t_1()
            .border_color(pack.border)
            .cursor_pointer()
            .on_click(cx.listener(|shell, _: &ClickEvent, _, cx| {
                shell.product.app.clear_notification();
                shell.clear_toast_arm();
                cx.stop_propagation();
                cx.notify();
            }))
            .child(message),
    )
}

pub fn body(cx: &mut Context<Shell>, shell: &Shell, pack: Packed) -> gpui::Stateful<gpui::Div> {
    let mut column = div()
        .id("product-body")
        .flex_1()
        .min_h(px(0.0))
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .p(px(12.0))
        .gap(px(12.0))
        .track_scroll(&shell.product_scroll);
    column = if shell.product.app.mode == Mode::Contrast {
        column.child(contrast(cx, shell, pack))
    } else {
        column.child(palette_page(cx, shell, pack))
    };
    if shell.product.app.fix_open {
        column = column.child(fix_pane(cx, shell, pack));
    }
    column
}

fn contrast(cx: &mut Context<Shell>, shell: &Shell, pack: Packed) -> gpui::Div {
    let app = &shell.product.app;
    let fg = shown(app, FocusId::FgHex, &app.foreground_input);
    let bg = shown(app, FocusId::BgHex, &app.background_input);
    let summary = dd_wcag::contrast::contrast_body_text(
        &app.foreground,
        &app.background,
        app.font_size_px,
        app.weight,
    );
    let lc = app.foreground.apca_lc(&app.background);
    let lookup = dd_wcag::color::apca_lookup_lc(app.font_size_px, app.weight);
    let polarity = if lc >= 0.0 { "light text" } else { "dark text" };
    let apca = if lc.abs() >= app.targets.apca.value() {
        "pass"
    } else {
        "fail"
    };
    section(
        "Pair",
        pack,
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(color_row(
                cx,
                "fg-hex",
                "Foreground",
                &fg,
                paint::hsla_bytes(app.foreground.rgb_u8()),
                app.focus == FocusId::FgHex,
                pack,
                FocusId::FgHex,
            ))
            .child(color_row(
                cx,
                "bg-hex",
                "Background",
                &bg,
                paint::hsla_bytes(app.background.rgb_u8()),
                app.focus == FocusId::BgHex,
                pack,
                FocusId::BgHex,
            )),
    )
    .child(div().h(px(12.0)))
    .child(section(
        "Type",
        pack,
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(metric(
                cx,
                shell,
                "size-field",
                "Size",
                app.font_size_px,
                FocusId::Size,
                pack,
            ))
            .child(metric(
                cx,
                shell,
                "weight-field",
                "Weight",
                app.weight,
                FocusId::Weight,
                pack,
            ))
            .child(style_row(cx, shell, pack)),
    ))
    .child(div().h(px(12.0)))
    .child(section(
        "Targets",
        pack,
        div()
            .flex()
            .gap(px(8.0))
            .child(chip(
                cx,
                "wcag-target",
                &format!("WCAG {}", app.targets.wcag.label()),
                app.focus == FocusId::TargetWcag,
                pack,
                Click::Wcag,
            ))
            .child(chip(
                cx,
                "apca-target",
                &format!("APCA {}", app.targets.apca.label()),
                app.focus == FocusId::TargetApca,
                pack,
                Click::Apca,
            )),
    ))
    .child(div().h(px(12.0)))
    .child(section(
        "Scores",
        pack,
        div().flex().flex_col().gap(px(2.0)).children(
            summary
                .lines()
                .map(|line| line.to_string())
                .chain(std::iter::once(format!(
                    "APCA Lc {lc:.2} {apca} lookup Lc{lookup:.0} {polarity}"
                )))
                .map(|line| div().text_color(score_color(&line, pack)).child(line)),
        ),
    ))
    .child(div().h(px(12.0)))
    .child(section(
        "Conversions",
        pack,
        div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(format!(
                "HEX  {}  {}",
                app.foreground.to_hex(),
                app.background.to_hex()
            ))
            .child(format!(
                "RGB  {}  {}",
                app.foreground.to_rgb_str(),
                app.background.to_rgb_str()
            ))
            .child(format!(
                "HSL  {}  {}",
                app.foreground.to_hsl_str(),
                app.background.to_hsl_str()
            )),
    ))
    .child(div().h(px(12.0)))
    .child(section("Preview", pack, preview_block(cx, shell, pack)))
    .child(div().h(px(12.0)))
    .child(section(
        "Actions",
        pack,
        div()
            .flex()
            .flex_wrap()
            .gap(px(8.0))
            .child(chip(
                cx,
                "swap",
                "Swap",
                app.focus == FocusId::Swap,
                pack,
                Click::Swap,
            ))
            .child(chip(
                cx,
                "copy-hex",
                "Copy hex",
                app.focus == FocusId::CopyHex,
                pack,
                Click::CopyPair,
            ))
            .child(chip(
                cx,
                "fix-btn",
                "Fix",
                app.focus == FocusId::FixBtn,
                pack,
                Click::Fix,
            ))
            .child(chip(
                cx,
                "web-preview",
                "Browser preview",
                app.focus == FocusId::OpenPreview,
                pack,
                Click::Preview,
            )),
    ))
}

fn palette_page(cx: &mut Context<Shell>, shell: &Shell, pack: Packed) -> gpui::Div {
    let app = &shell.product.app;
    let mut roles = div().flex().flex_col().gap(px(8.0));
    for role in PaletteInput::ALL {
        let editing = app.palette.editing && app.palette.selected() == role;
        let value = if editing {
            app.palette.edit_input.clone()
        } else {
            app.palette.input_for(role).to_string()
        };
        let index = role.index();
        let on = app.focus == FocusId::Role(index);
        let id = format!("role-{index}");
        roles = roles.child(
            div()
                .id(id)
                .h(px(36.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .border_1()
                .border_color(if on { pack.accent } else { pack.border })
                .bg(pack.panel)
                .cursor_pointer()
                .on_click(cx.listener(move |shell, event: &ClickEvent, _, cx| {
                    if event.modifiers().shift {
                        shell.product.app.set_focus(FocusId::Role(index));
                        shell.activate(Click::CopyFocused);
                    } else {
                        shell.activate(Click::Role(index));
                    }
                    cx.stop_propagation();
                    cx.notify();
                }))
                .child(
                    div()
                        .w(px(110.0))
                        .text_color(pack.secondary)
                        .child(role.label().to_string()),
                )
                .child(value),
        );
    }
    let (wcag, apca) = app.contrast_thresholds();
    section("Brand colors", pack, roles)
        .child(div().h(px(12.0)))
        .child(section(
            "Type",
            pack,
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(metric(
                    cx,
                    shell,
                    "palette-size",
                    "Size",
                    app.font_size_px,
                    FocusId::Size,
                    pack,
                ))
                .child(metric(
                    cx,
                    shell,
                    "palette-weight",
                    "Weight",
                    app.weight,
                    FocusId::Weight,
                    pack,
                ))
                .child(chip(
                    cx,
                    "generate",
                    "Generate",
                    app.focus == FocusId::Generate,
                    pack,
                    Click::Generate,
                )),
        ))
        .child(div().h(px(12.0)))
        .child(section(
            "Pairs  text \\ surface",
            pack,
            matrix(cx, shell, pack, wcag, apca),
        ))
        .child(div().h(px(12.0)))
        .child(section(
            "Pair list",
            pack,
            pair_list(cx, shell, pack, wcag, apca),
        ))
        .child(div().h(px(12.0)))
        .child(section(
            "Selected pair",
            pack,
            pair_detail(shell, pack, wcag, apca),
        ))
}

fn matrix(cx: &mut Context<Shell>, shell: &Shell, pack: Packed, wcag: f64, apca: f64) -> gpui::Div {
    let app = &shell.product.app;
    let mut grid = div().flex().flex_col().gap(px(4.0));
    let mut header = div().flex().gap(px(4.0)).child(div().w(px(52.0)));
    for axis in MatrixAxis::ALL {
        header = header.child(
            div()
                .w(px(52.0))
                .text_color(pack.secondary)
                .child(axis.short_label().to_string()),
        );
    }
    grid = grid.child(header);
    for row in 0..5 {
        let mut line = div().flex().gap(px(4.0)).child(
            div()
                .w(px(52.0))
                .text_color(pack.secondary)
                .child(MatrixAxis::from_index(row).short_label().to_string()),
        );
        for col in 0..5 {
            let selected = app.palette.matrix_row == row && app.palette.matrix_col == col;
            let label = if row == col {
                "·".to_string()
            } else if let Some(cell) = palette::matrix_cell(&app.palette, row, col, wcag, apca) {
                cell.glyph().to_string()
            } else {
                "?".to_string()
            };
            let color = match label.as_str() {
                "✓" => pack.success,
                "✗" => pack.error,
                "~" => pack.accent,
                _ => pack.secondary,
            };
            line = line.child(
                div()
                    .id(("matrix", row * 5 + col))
                    .w(px(52.0))
                    .h(px(32.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_color(if selected { pack.accent } else { pack.border })
                    .bg(if selected { pack.selection } else { pack.panel })
                    .text_color(color)
                    .cursor_pointer()
                    .on_click(cx.listener(move |shell, _: &ClickEvent, _, cx| {
                        shell.activate(Click::Matrix { row, col });
                        cx.stop_propagation();
                        cx.notify();
                    }))
                    .child(label),
            );
        }
        grid = grid.child(line);
    }
    grid
}

fn pair_list(
    cx: &mut Context<Shell>,
    shell: &Shell,
    pack: Packed,
    wcag: f64,
    apca: f64,
) -> gpui::Div {
    let app = &shell.product.app;
    let mut column = div().flex().flex_col().gap(px(2.0));
    for (row, col) in palette::off_diagonal_pairs() {
        let selected = app.palette.matrix_row == row && app.palette.matrix_col == col;
        let text = MatrixAxis::from_index(row).short_label();
        let surface = MatrixAxis::from_index(col).short_label();
        let label = match palette::matrix_cell(&app.palette, row, col, wcag, apca) {
            Some(cell) => format!(
                "{text} on {surface}  {:.2} {}  Lc {:.0}",
                cell.ratio,
                cell.glyph(),
                cell.lc
            ),
            None => format!("{text} on {surface}"),
        };
        column = column.child(
            div()
                .id(("pair", row * 5 + col))
                .h(px(28.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .border_1()
                .border_color(if selected { pack.accent } else { pack.border })
                .bg(if selected { pack.selection } else { pack.panel })
                .cursor_pointer()
                .on_click(cx.listener(move |shell, _: &ClickEvent, _, cx| {
                    shell.activate(Click::Matrix { row, col });
                    cx.stop_propagation();
                    cx.notify();
                }))
                .child(label),
        );
    }
    column
}

fn pair_detail(shell: &Shell, pack: Packed, wcag: f64, apca: f64) -> gpui::Div {
    let app = &shell.product.app;
    let mut column = div().flex().flex_col().gap(px(4.0));
    if let Some(cell) = palette::matrix_cell(
        &app.palette,
        app.palette.matrix_row,
        app.palette.matrix_col,
        wcag,
        apca,
    ) {
        column = column
            .child(format!(
                "{} on {} {}",
                cell.text.label(),
                cell.surface.label(),
                cell.glyph()
            ))
            .child(format!("WCAG {:.2}   APCA Lc {:.0}", cell.ratio, cell.lc))
            .child(format!("needs ≥ {wcag:.1}:1 and |Lc| ≥ {apca:.0}"))
            .child(format!("{} on {}", cell.fg.to_hex(), cell.bg.to_hex()))
            .child(
                div()
                    .px(px(8.0))
                    .py(px(6.0))
                    .bg(paint::hsla_bytes(cell.bg.rgb_u8()))
                    .text_color(paint::hsla_bytes(cell.fg.rgb_u8()))
                    .child("Aa sample"),
            );
    } else {
        column = column.child("Selected pair has an invalid color.");
    }
    column = column.child("Text roles are fixed and will not be changed.");
    if let Some(generated) = &app.palette.generated {
        column = column.child(format!(
            "Generated: {} tokens | {} blocking | {} advisory",
            generated.tokens.len(),
            generated.blocking_failures().len(),
            generated.advisory_failures().len()
        ));
        let start = app.palette.detail_scroll;
        for check in generated.checks.iter().skip(start).take(12) {
            let tag = if check.passes {
                "PASS"
            } else if check.blocking {
                "FAIL"
            } else {
                "WARN"
            };
            column = column.child(format!("{tag} {:.2}:1 {}", check.ratio, check.label));
        }
    } else {
        column = column.child("Press Ctrl+G to generate a compliant _palette.scss draft.");
    }
    let _ = pack;
    column
}

fn fix_pane(cx: &mut Context<Shell>, shell: &Shell, pack: Packed) -> gpui::Div {
    let app = &shell.product.app;
    let source = match app.fix_source {
        dd_wcag::app::FixSource::Contrast => "Contrast pair".to_string(),
        dd_wcag::app::FixSource::Palette { text, surface } => {
            format!("{} on {}", text.label(), surface.label())
        }
    };
    let index = format!(
        "Candidate {} of {}",
        app.fix.candidate_index() + 1,
        app.fix.candidate_count().max(1)
    );
    section(
        "Fix",
        pack,
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(format!("{source} · {index}"))
            .child(format!(
                "{} on {}   L {:.2}",
                app.fix.candidate_fg.to_hex(),
                app.fix.candidate_bg.to_hex(),
                app.fix.candidate_fg.oklab_l()
            ))
            .child(
                div()
                    .px(px(8.0))
                    .py(px(8.0))
                    .bg(paint::hsla_bytes(app.fix.candidate_bg.rgb_u8()))
                    .text_color(paint::hsla_bytes(app.fix.candidate_fg.rgb_u8()))
                    .child("Fixed sample"),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(8.0))
                    .child(chip(
                        cx,
                        "nudge-fg-down",
                        "FG lighter",
                        app.focus == FocusId::NudgeFg,
                        pack,
                        Click::Nudge {
                            axis: FixAxis::Fg,
                            delta: 0.02,
                        },
                    ))
                    .child(chip(
                        cx,
                        "nudge-fg-up",
                        "FG darker",
                        false,
                        pack,
                        Click::Nudge {
                            axis: FixAxis::Fg,
                            delta: -0.02,
                        },
                    ))
                    .child(chip(
                        cx,
                        "nudge-bg-down",
                        "BG lighter",
                        app.focus == FocusId::NudgeBg,
                        pack,
                        Click::Nudge {
                            axis: FixAxis::Bg,
                            delta: 0.02,
                        },
                    ))
                    .child(chip(
                        cx,
                        "nudge-bg-up",
                        "BG darker",
                        false,
                        pack,
                        Click::Nudge {
                            axis: FixAxis::Bg,
                            delta: -0.02,
                        },
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(8.0))
                    .child(chip(
                        cx,
                        "apply-fix",
                        "Apply",
                        app.focus == FocusId::ApplyFix,
                        pack,
                        Click::ApplyFix,
                    ))
                    .child(chip(
                        cx,
                        "next-fix",
                        "Next",
                        app.focus == FocusId::NextFix,
                        pack,
                        Click::NextFix,
                    ))
                    .child(chip(
                        cx,
                        "close-fix",
                        "Close",
                        app.focus == FocusId::CloseFix,
                        pack,
                        Click::CloseFix,
                    )),
            )
            .child(send_row(cx, shell, pack, FixAxis::Fg))
            .child(send_row(cx, shell, pack, FixAxis::Bg)),
    )
}

fn send_row(cx: &mut Context<Shell>, shell: &Shell, pack: Packed, axis: FixAxis) -> gpui::Div {
    let focused = match axis {
        FixAxis::Fg => shell.product.app.focus == FocusId::SendFg,
        FixAxis::Bg => shell.product.app.focus == FocusId::SendBg,
    };
    let mut row = div().flex().flex_wrap().gap(px(8.0));
    for role in PaletteInput::ALL {
        let on = focused && shell.product.app.fix_send_chip == role.index();
        let label = format!("Send {} {}", axis.label(), role.label());
        let id = match (axis, role) {
            (FixAxis::Fg, PaletteInput::Primary) => "send-fg-primary",
            (FixAxis::Fg, PaletteInput::Secondary) => "send-fg-secondary",
            (FixAxis::Fg, PaletteInput::Tertiary) => "send-fg-tertiary",
            (FixAxis::Fg, PaletteInput::Support) => "send-fg-support",
            (FixAxis::Bg, PaletteInput::Primary) => "send-bg-primary",
            (FixAxis::Bg, PaletteInput::Secondary) => "send-bg-secondary",
            (FixAxis::Bg, PaletteInput::Tertiary) => "send-bg-tertiary",
            (FixAxis::Bg, PaletteInput::Support) => "send-bg-support",
        };
        row = row.child(chip(cx, id, &label, on, pack, Click::Send { axis, role }));
    }
    row
}

fn preview_block(cx: &mut Context<Shell>, shell: &Shell, pack: Packed) -> impl IntoElement {
    let app = &shell.product.app;
    let sample = app.preview_text.clone();
    let size = app.font_size_px as f32;
    let weight = FontWeight(app.weight as f32);
    let italic = app.italic;
    let font = if app.focus == FocusId::FontFamily {
        app.current_input.clone()
    } else {
        app.preview_font_family.clone()
    };
    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .id("preview-text")
                .p(px(12.0))
                .bg(paint::hsla_bytes(app.background.rgb_u8()))
                .text_color(paint::hsla_bytes(app.foreground.rgb_u8()))
                .text_size(px(size))
                .font_weight(weight)
                .when(italic, |row| row.italic())
                .border_1()
                .border_color(if app.focus == FocusId::PreviewText {
                    pack.accent
                } else {
                    pack.border
                })
                .cursor_pointer()
                .on_click(cx.listener(|shell, _: &ClickEvent, _, cx| {
                    shell.activate(Click::Focus(FocusId::PreviewText));
                    cx.stop_propagation();
                    cx.notify();
                }))
                .child(sample),
        )
        .child(
            div()
                .id("font-family")
                .h(px(36.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .border_1()
                .border_color(if app.focus == FocusId::FontFamily {
                    pack.accent
                } else {
                    pack.border
                })
                .cursor_pointer()
                .on_click(cx.listener(|shell, _: &ClickEvent, _, cx| {
                    shell.activate(Click::Focus(FocusId::FontFamily));
                    cx.stop_propagation();
                    cx.notify();
                }))
                .child(format!("Font {font}")),
        )
}

fn style_row(cx: &mut Context<Shell>, shell: &Shell, pack: Packed) -> gpui::Div {
    let mut row = div().flex().gap(px(8.0));
    for preset in StylePreset::ALL {
        let on = shell.product.app.active_style_preset() == Some(preset)
            || (shell.product.app.focus == FocusId::Style
                && shell.product.app.style_chip == preset.index());
        let id = match preset {
            StylePreset::Regular => "style-reg",
            StylePreset::Bold => "style-bld",
            StylePreset::Italic => "style-itl",
            StylePreset::BoldItalic => "style-bi",
        };
        row = row.child(chip(
            cx,
            id,
            preset.label(),
            on,
            pack,
            Click::Style(preset.index()),
        ));
    }
    row
}

fn metric(
    cx: &mut Context<Shell>,
    shell: &Shell,
    id: &'static str,
    label: &str,
    value: u16,
    focus: FocusId,
    pack: Packed,
) -> impl IntoElement {
    let on = shell.product.app.focus == focus;
    let step_up = if focus == FocusId::Weight { 100 } else { 1 };
    div()
        .id(id)
        .h(px(36.0))
        .px(px(8.0))
        .flex()
        .items_center()
        .gap(px(8.0))
        .border_1()
        .border_color(if on { pack.accent } else { pack.border })
        .bg(pack.panel)
        .cursor_pointer()
        .on_click(cx.listener(move |shell, _: &ClickEvent, _, cx| {
            shell.activate(Click::Focus(focus));
            cx.stop_propagation();
            cx.notify();
        }))
        .child(
            div()
                .w(px(110.0))
                .text_color(pack.secondary)
                .child(label.to_string()),
        )
        .child(value.to_string())
        .child(div().flex_1())
        .child(stepper(cx, id, false, focus, step_up, pack))
        .child(stepper(cx, id, true, focus, step_up, pack))
}

fn stepper(
    cx: &mut Context<Shell>,
    id: &'static str,
    up: bool,
    focus: FocusId,
    step: i16,
    pack: Packed,
) -> impl IntoElement {
    let label = if up { "+" } else { "−" };
    let name = if up { "up" } else { "down" };
    div()
        .id(format!("{id}-{name}"))
        .w(px(28.0))
        .h(px(28.0))
        .flex()
        .items_center()
        .justify_center()
        .border_1()
        .border_color(pack.border)
        .cursor_pointer()
        .on_click(cx.listener(move |shell, event: &ClickEvent, _, cx| {
            shell.activate(Click::Focus(focus));
            let magnitude = if event.modifiers().shift {
                if focus == FocusId::Weight { 200 } else { 4 }
            } else {
                step
            };
            let signed = if up { magnitude } else { -magnitude };
            if focus == FocusId::Weight {
                shell.product.app.adjust_weight(signed);
            } else {
                shell.product.app.adjust_font_size(signed);
            }
            let _ = dd_wcag::web_preview::sync(&shell.product.app);
            cx.stop_propagation();
            cx.notify();
        }))
        .child(label.to_string())
}

fn color_row(
    cx: &mut Context<Shell>,
    id: &'static str,
    label: &str,
    value: &str,
    swatch: gpui::Hsla,
    on: bool,
    pack: Packed,
    focus: FocusId,
) -> impl IntoElement {
    div()
        .id(id)
        .h(px(36.0))
        .px(px(8.0))
        .flex()
        .items_center()
        .gap(px(8.0))
        .border_1()
        .border_color(if on { pack.accent } else { pack.border })
        .bg(pack.panel)
        .cursor_pointer()
        .on_click(cx.listener(move |shell, _: &ClickEvent, _, cx| {
            shell.activate(Click::Focus(focus));
            cx.stop_propagation();
            cx.notify();
        }))
        .child(
            div()
                .w(px(110.0))
                .text_color(pack.secondary)
                .child(label.to_string()),
        )
        .child(div().flex_1().child(value.to_string()))
        .child(
            div()
                .id(format!("{id}-swatch"))
                .w(px(28.0))
                .h(px(28.0))
                .bg(swatch)
                .border_1()
                .border_color(pack.border)
                .cursor_pointer()
                .on_click(cx.listener(move |shell, event: &ClickEvent, _, cx| {
                    shell.product.app.set_focus(focus);
                    if event.modifiers().shift {
                        shell.activate(Click::CopyFocused);
                    } else {
                        shell.activate(Click::Focus(focus));
                    }
                    cx.stop_propagation();
                    cx.notify();
                })),
        )
}

fn mode_tab(
    cx: &mut Context<Shell>,
    id: &'static str,
    label: &str,
    on: bool,
    pack: Packed,
) -> impl IntoElement {
    let mode = if label == "Contrast" {
        Mode::Contrast
    } else {
        Mode::Palette
    };
    div()
        .id(id)
        .px(px(8.0))
        .cursor_pointer()
        .font_weight(if on {
            FontWeight::BOLD
        } else {
            FontWeight::NORMAL
        })
        .text_color(if on { pack.accent } else { pack.secondary })
        .on_click(cx.listener(move |shell, _: &ClickEvent, _, cx| {
            shell.activate(Click::Mode(mode));
            cx.stop_propagation();
            cx.notify();
        }))
        .child(label.to_string())
}

fn chip(
    cx: &mut Context<Shell>,
    id: &'static str,
    label: &str,
    on: bool,
    pack: Packed,
    click: Click,
) -> impl IntoElement {
    div()
        .id(id)
        .px(px(8.0))
        .h(px(28.0))
        .flex()
        .items_center()
        .border_1()
        .border_color(if on { pack.accent } else { pack.border })
        .bg(if on { pack.selection } else { pack.panel })
        .cursor_pointer()
        .on_click(cx.listener(move |shell, _: &ClickEvent, _, cx| {
            shell.activate(click);
            cx.stop_propagation();
            cx.notify();
        }))
        .child(label.to_string())
}

fn section(title: &str, pack: Packed, body: impl IntoElement) -> gpui::Div {
    div()
        .w_full()
        .border_1()
        .border_color(pack.border)
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(28.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .font_weight(FontWeight::BOLD)
                .bg(pack.raised)
                .border_b_1()
                .border_color(pack.border)
                .child(title.to_string()),
        )
        .child(div().p(px(8.0)).child(body))
}

fn shown(app: &dd_wcag::app::App, focus: FocusId, stored: &str) -> String {
    if app.focus == focus {
        app.current_input.clone()
    } else {
        stored.to_string()
    }
}

fn score_color(line: &str, pack: Packed) -> gpui::Hsla {
    if line.contains('✗') || line.contains("fail") {
        pack.error
    } else if line.contains('✓') || line.contains("pass") {
        pack.success
    } else {
        pack.text
    }
}
