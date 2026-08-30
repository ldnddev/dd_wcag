//! dd_wcag TUI entry point.

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{self, Stdout, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

mod app;
mod color;
mod contrast;
mod fix;
mod layout;
mod palette;
mod theme;
mod ui;
mod web_preview;

use app::{App, FocusId, Mode, StylePreset};
use fix::FixAxis;
use layout::{Hit, char_index_at, char_index_at_xy, view_scroll, visual_cursor};
use palette::PALETTE_EXPORT_PATH;
use palette::PaletteInput;
use theme::Theme;

fn main() -> Result<()> {
    let mut terminal = setup_terminal()?;

    let loaded_theme = Theme::load();
    let source = loaded_theme.source;
    let path = loaded_theme.path.clone();
    let mut app = App::with_theme(loaded_theme.theme, source);
    let path_label = path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "built-in defaults".to_string());
    app.notify_status(format!(
        "Theme health: {} theme v{} active ({path_label}).",
        source.label(),
        app.theme.version
    ));
    if let Some(warning) = loaded_theme.warning {
        app.notify_error(warning);
    }
    sync_web_preview(&mut app);

    let res = run_loop(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    res
}

fn sync_web_preview(app: &mut App) {
    if let Err(err) = web_preview::sync(app) {
        app.notify_error(format!("Failed to update web preview: {err}"));
    }
}

#[derive(Default)]
struct KeyEffects {
    quit: bool,
    sync_preview: bool,
    open_preview: bool,
    save_palette: bool,
    copy_palette: bool,
    copy_hex: bool,
}

fn try_apply_active_input(app: &mut App) -> bool {
    if !app.focus.is_text_field() {
        return true;
    }
    app.sync_active_input();
    app.submit_input()
}

fn dispatch_effects(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    effects: KeyEffects,
) -> Result<bool> {
    if effects.open_preview {
        sync_web_preview(app);
        match web_preview::open_in_browser() {
            Ok(()) => app.notify_status("Opened web preview. Click toast to dismiss."),
            Err(err) => app.notify_error(format!(
                "Failed to open browser preview ({}): {err}",
                web_preview::preview_path().display()
            )),
        }
    }
    if effects.save_palette {
        save_palette_with_dialog(terminal, app)?;
    }
    if effects.copy_palette {
        copy_palette(app);
    }
    if effects.copy_hex {
        if let Some(hex) = app.copy_focused_hex() {
            match copy_to_clipboard(&hex) {
                Ok(()) => app.notify_status(format!("Copied {hex}")),
                Err(err) => app.notify_error(format!("Clipboard unavailable: {err}")),
            }
        }
    }
    if effects.sync_preview {
        sync_web_preview(app);
    }
    Ok(effects.quit)
}

fn handle_key_event(app: &mut App, key: KeyEvent) -> KeyEffects {
    let mut effects = KeyEffects::default();
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    if ctrl {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => effects.quit = true,
            KeyCode::Char('b') | KeyCode::Char('B') => {
                app.toggle_bold_preset();
                effects.sync_preview = true;
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                app.toggle_fix();
                if app.fix_open {
                    effects.sync_preview = true;
                }
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                app.set_mode(Mode::Palette);
                app.generate_palette();
                app.set_focus(FocusId::Detail);
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                app.cycle_font_family();
                effects.sync_preview = true;
            }
            KeyCode::Char('o') | KeyCode::Char('O') => effects.open_preview = true,
            KeyCode::Char('s') | KeyCode::Char('S') => {
                if app.mode == Mode::Palette {
                    effects.save_palette = true;
                } else {
                    app.cycle_style();
                    effects.sync_preview = true;
                }
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                if app.mode == Mode::Palette {
                    effects.copy_palette = true;
                } else {
                    effects.copy_hex = true;
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if app.fix_open {
                    app.next_fix_candidate();
                    app.set_focus(FocusId::NextFix);
                }
            }
            KeyCode::Up => {
                step_focused(app, true, shift, &mut effects);
            }
            KeyCode::Down => {
                step_focused(app, false, shift, &mut effects);
            }
            _ => {}
        }
        return effects;
    }

    match key.code {
        KeyCode::F(1) => {
            app.show_keybindings = !app.show_keybindings;
            app.show_theme_debug = false;
        }
        KeyCode::F(2) => {
            app.show_theme_debug = !app.show_theme_debug;
            app.show_keybindings = false;
        }
        KeyCode::Esc => {
            if app.show_keybindings {
                app.show_keybindings = false;
            } else if app.show_theme_debug {
                app.show_theme_debug = false;
            } else if app.fix_open {
                app.close_fix();
            } else if app.mode == Mode::Palette && app.palette.editing {
                app.palette.cancel_edit();
            } else if app.editing && app.focus.is_text_field() {
                app.editing = false;
            }
        }
        KeyCode::Char('1') if !is_typing(app) => app.set_mode(Mode::Contrast),
        KeyCode::Char('2') if !is_typing(app) => app.set_mode(Mode::Palette),
        KeyCode::Tab => {
            if try_apply_active_input(app) {
                effects.sync_preview = true;
                app.cycle_focus(false);
            }
        }
        KeyCode::BackTab => {
            if try_apply_active_input(app) {
                effects.sync_preview = true;
                app.cycle_focus(true);
            }
        }
        KeyCode::Left => {
            if app.focus == FocusId::Style {
                app.move_style_chip(-1);
                effects.sync_preview = true;
            } else if matches!(app.focus, FocusId::SendFg | FocusId::SendBg) {
                app.move_fix_send_chip(-1);
            } else if app.mode == Mode::Palette && app.palette.editing {
                app.palette.move_cursor_left();
            } else if app.focus.is_text_field() {
                app.move_cursor_left();
            }
        }
        KeyCode::Right => {
            if app.focus == FocusId::Style {
                app.move_style_chip(1);
                effects.sync_preview = true;
            } else if matches!(app.focus, FocusId::SendFg | FocusId::SendBg) {
                app.move_fix_send_chip(1);
            } else if app.mode == Mode::Palette && app.palette.editing {
                app.palette.move_cursor_right();
            } else if app.focus.is_text_field() {
                app.move_cursor_right();
            }
        }
        KeyCode::Enter => {
            if matches!(app.focus, FocusId::SendFg | FocusId::SendBg) {
                app.send_fixed_selected_chip();
            } else if app.mode == Mode::Palette {
                if app.palette.editing {
                    app.palette.commit_edit();
                } else if matches!(app.focus, FocusId::Role(_)) || app.focus == FocusId::Generate {
                    if app.focus == FocusId::Generate {
                        app.generate_palette();
                        app.set_focus(FocusId::Detail);
                    } else {
                        app.palette.begin_edit();
                    }
                } else {
                    app.palette.begin_edit();
                }
            } else if app.focus == FocusId::PreviewText {
                app.insert_newline_at_cursor();
                app.sync_active_input();
                effects.sync_preview = true;
            } else if app.focus == FocusId::Swap {
                app.swap_colors();
                effects.sync_preview = true;
            } else if app.focus == FocusId::CopyHex {
                effects.copy_hex = true;
            } else if app.focus == FocusId::FixBtn {
                app.toggle_fix();
            } else if app.focus == FocusId::ApplyFix {
                app.apply_fix();
                effects.sync_preview = true;
            } else if app.focus == FocusId::NextFix {
                app.next_fix_candidate();
            } else if app.focus == FocusId::CloseFix {
                app.close_fix();
            } else if app.focus == FocusId::OpenPreview {
                effects.open_preview = true;
            } else if app.focus == FocusId::Style {
                app.apply_style_preset(StylePreset::from_index(app.style_chip));
                effects.sync_preview = true;
            } else if app.focus.is_text_field() {
                let _ = try_apply_active_input(app);
                effects.sync_preview = true;
            }
        }
        KeyCode::Backspace => {
            if app.mode == Mode::Palette && app.palette.editing {
                app.palette.backspace_at_cursor();
            } else if app.focus.is_text_field() {
                app.backspace_at_cursor();
                app.sync_active_input();
                if app.focus == FocusId::PreviewText {
                    effects.sync_preview = true;
                }
            }
        }
        KeyCode::Up => {
            if app.focus == FocusId::Size || app.focus == FocusId::Weight {
                step_focused(app, true, shift, &mut effects);
            } else if app.focus == FocusId::Style {
                app.move_style_chip(-1);
                effects.sync_preview = true;
            } else if app.focus == FocusId::SendBg {
                app.set_focus(FocusId::SendFg);
            } else if app.focus == FocusId::Detail {
                app.palette.scroll_detail_by(if shift { -8 } else { -1 });
            } else if app.mode == Mode::Palette && !app.palette.editing {
                app.palette.select_previous();
                if let FocusId::Role(_) = app.focus {
                    app.set_focus(FocusId::Role(app.palette.selected_idx));
                }
            }
        }
        KeyCode::Down => {
            if app.focus == FocusId::Size || app.focus == FocusId::Weight {
                step_focused(app, false, shift, &mut effects);
            } else if app.focus == FocusId::Style {
                app.move_style_chip(1);
                effects.sync_preview = true;
            } else if app.focus == FocusId::SendFg {
                app.set_focus(FocusId::SendBg);
            } else if app.focus == FocusId::Detail {
                app.palette.scroll_detail_by(if shift { 8 } else { 1 });
            } else if app.mode == Mode::Palette && !app.palette.editing {
                app.palette.select_next();
                if let FocusId::Role(_) = app.focus {
                    app.set_focus(FocusId::Role(app.palette.selected_idx));
                }
            }
        }
        KeyCode::PageUp => {
            if app.mode == Mode::Palette {
                app.set_focus(FocusId::Detail);
                app.palette.scroll_detail_by(-10);
            } else if app.mode == Mode::Contrast {
                app.scroll_contrast_by(if shift { -16 } else { -8 });
            }
        }
        KeyCode::PageDown => {
            if app.mode == Mode::Palette {
                app.set_focus(FocusId::Detail);
                app.palette.scroll_detail_by(10);
            } else if app.mode == Mode::Contrast {
                app.scroll_contrast_by(if shift { 16 } else { 8 });
            }
        }
        KeyCode::Char('[') if !is_typing(app) || is_fix_nudge_focus(app) => {
            let axis = fix_nudge_axis(app);
            let delta = if shift { -0.10 } else { -0.02 };
            if app.fix_open {
                app.nudge_fix(axis, delta);
            } else {
                nudge_live_color(app, axis, delta);
                effects.sync_preview = true;
            }
        }
        KeyCode::Char(']') if !is_typing(app) || is_fix_nudge_focus(app) => {
            let axis = fix_nudge_axis(app);
            let delta = if shift { 0.10 } else { 0.02 };
            if app.fix_open {
                app.nudge_fix(axis, delta);
            } else {
                nudge_live_color(app, axis, delta);
                effects.sync_preview = true;
            }
        }
        KeyCode::Char(' ') if !app.editing && app.mode == Mode::Contrast => {
            if app.focus == FocusId::Style {
                app.apply_style_preset(StylePreset::from_index(app.style_chip));
                effects.sync_preview = true;
            } else {
                app.swap_colors();
                effects.sync_preview = true;
            }
        }
        KeyCode::Char(c) => {
            if app.show_keybindings || app.show_theme_debug {
                return effects;
            }
            if app.fix_open && !is_typing(app) {
                if let Some(role) = palette_role_from_key(c) {
                    if is_fix_focus(app) {
                        let axis = if matches!(app.focus, FocusId::NudgeBg | FocusId::SendBg) {
                            FixAxis::Bg
                        } else {
                            FixAxis::Fg
                        };
                        app.send_fixed_to_role(axis, role);
                        return effects;
                    }
                }
            }
            if app.mode == Mode::Palette && app.palette.editing {
                app.palette.insert_char_at_cursor(c);
                return effects;
            }
            if app.editing && app.focus.is_text_field() {
                app.insert_char_at_cursor(c);
                app.sync_active_input();
                if app.focus == FocusId::PreviewText {
                    effects.sync_preview = true;
                }
            }
        }
        _ => {}
    }

    effects
}

fn is_typing(app: &App) -> bool {
    app.editing || app.palette.editing
}

fn is_fix_nudge_focus(app: &App) -> bool {
    matches!(app.focus, FocusId::NudgeFg | FocusId::NudgeBg)
}

fn is_fix_focus(app: &App) -> bool {
    matches!(
        app.focus,
        FocusId::NudgeFg
            | FocusId::NudgeBg
            | FocusId::SendFg
            | FocusId::SendBg
            | FocusId::ApplyFix
            | FocusId::NextFix
            | FocusId::CloseFix
    )
}

fn palette_role_from_key(c: char) -> Option<PaletteInput> {
    match c {
        'p' | 'P' => Some(PaletteInput::Primary),
        's' | 'S' => Some(PaletteInput::Secondary),
        't' | 'T' => Some(PaletteInput::Tertiary),
        'u' | 'U' => Some(PaletteInput::Support),
        _ => None,
    }
}

fn fix_nudge_axis(app: &App) -> FixAxis {
    match app.focus {
        FocusId::NudgeBg | FocusId::BgHex => FixAxis::Bg,
        _ => FixAxis::Fg,
    }
}

fn nudge_live_color(app: &mut App, axis: FixAxis, delta: f32) {
    match axis {
        FixAxis::Fg => {
            app.foreground = app.foreground.nudge_oklab_l(delta);
            app.foreground_input = app.foreground.to_hex();
            if app.focus == FocusId::FgHex {
                app.current_input = app.foreground_input.clone();
                app.cursor_char_idx = app.current_input.chars().count();
            }
        }
        FixAxis::Bg => {
            app.background = app.background.nudge_oklab_l(delta);
            app.background_input = app.background.to_hex();
            if app.focus == FocusId::BgHex {
                app.current_input = app.background_input.clone();
                app.cursor_char_idx = app.current_input.chars().count();
            }
        }
    }
    app.update_contrast();
}

fn is_contrast_scroll_hit(hit: Hit) -> bool {
    matches!(
        hit,
        Hit::ContrastPanel
            | Hit::ContrastScrollbar
            | Hit::FgInput
            | Hit::FgSwatch
            | Hit::BgInput
            | Hit::BgSwatch
            | Hit::Style(_)
            | Hit::PreviewText
            | Hit::FontFamily
            | Hit::Swap
            | Hit::Copy
            | Hit::FixBtn
            | Hit::WebBtn
    )
}

fn step_focused(app: &mut App, up: bool, shift: bool, effects: &mut KeyEffects) {
    let sign = if up { 1 } else { -1 };
    match app.focus {
        FocusId::Size => {
            let delta = if shift { 4 } else { 1 };
            app.adjust_font_size(sign * delta);
            effects.sync_preview = true;
        }
        FocusId::Weight => {
            let delta = if shift { 200 } else { 100 };
            app.adjust_weight(sign * delta);
            effects.sync_preview = true;
        }
        FocusId::Style => {
            app.move_style_chip(sign);
            effects.sync_preview = true;
        }
        FocusId::Detail => {
            let step = if shift { 8 } else { 3 };
            app.palette.scroll_detail_by(i32::from(sign) * step);
        }
        FocusId::NudgeFg => {
            let delta = if shift { 0.10 } else { 0.02 };
            app.nudge_fix(FixAxis::Fg, sign as f32 * delta);
        }
        FocusId::NudgeBg => {
            let delta = if shift { 0.10 } else { 0.02 };
            app.nudge_fix(FixAxis::Bg, sign as f32 * delta);
        }
        _ => {}
    }
}

fn handle_mouse_event(app: &mut App, mouse: MouseEvent) -> KeyEffects {
    let mut effects = KeyEffects::default();
    let (col, row) = (mouse.column, mouse.row);

    match mouse.kind {
        MouseEventKind::Moved | MouseEventKind::Drag(_) => {
            app.mouse_pos = Some((col, row));
            app.hovered = app.layout.hit(col, row);
            if let Some(axis) = app.nudge_dragging {
                let gauge = match axis {
                    FixAxis::Fg => app.layout.nudge_fg,
                    FixAxis::Bg => app.layout.nudge_bg,
                };
                app.set_fix_l_from_x(axis, col, gauge);
            }
        }
        MouseEventKind::Up(_) => {
            app.scrollbar_dragging = false;
            app.nudge_dragging = None;
        }
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
            let up = matches!(mouse.kind, MouseEventKind::ScrollUp);
            let shift = mouse.modifiers.contains(KeyModifiers::SHIFT);
            if let Some(hit) = app.layout.hit(col, row) {
                match hit {
                    Hit::SizeInput | Hit::SizeDec | Hit::SizeInc => {
                        app.set_focus(FocusId::Size);
                        step_focused(app, up, shift, &mut effects);
                    }
                    Hit::WeightInput | Hit::WeightDec | Hit::WeightInc => {
                        app.set_focus(FocusId::Weight);
                        step_focused(app, up, shift, &mut effects);
                    }
                    Hit::Detail | Hit::DetailScrollbar | Hit::PairList => {
                        app.set_focus(FocusId::Detail);
                        let step = if shift { 8 } else { 3 };
                        app.palette.scroll_detail_by(if up { -step } else { step });
                    }
                    Hit::NudgeFg => {
                        app.set_focus(FocusId::NudgeFg);
                        let delta = if shift { 0.10 } else { 0.02 };
                        app.nudge_fix(FixAxis::Fg, if up { delta } else { -delta });
                    }
                    Hit::NudgeBg => {
                        app.set_focus(FocusId::NudgeBg);
                        let delta = if shift { 0.10 } else { 0.02 };
                        app.nudge_fix(FixAxis::Bg, if up { delta } else { -delta });
                    }
                    hit if is_contrast_scroll_hit(hit) => {
                        let step = if shift { 8 } else { 3 };
                        app.scroll_contrast_by(if up { -step } else { step });
                    }
                    _ => {}
                }
            }
        }
        MouseEventKind::Down(MouseButton::Left) => {
            let now = Instant::now();
            let is_double = app.last_mouse_click_pos.is_some_and(|(lx, ly, lt)| {
                lx == col && ly == row && now.duration_since(lt).as_millis() < 420
            });
            app.last_mouse_click_pos = Some((col, row, now));

            let Some(hit) = app.layout.hit(col, row) else {
                return effects;
            };
            match hit {
                Hit::Toast => app.clear_notification(),
                Hit::Popup => {}
                Hit::PopupOutside => {
                    app.show_keybindings = false;
                    app.show_theme_debug = false;
                }
                Hit::TabContrast => {
                    if try_apply_active_input(app) {
                        app.set_mode(Mode::Contrast);
                        effects.sync_preview = true;
                    }
                }
                Hit::TabPalette => {
                    if try_apply_active_input(app) {
                        app.set_mode(Mode::Palette);
                        effects.sync_preview = true;
                    }
                }
                Hit::TargetWcag => {
                    app.targets.wcag = app.targets.wcag.cycle();
                    app.set_focus(FocusId::TargetWcag);
                }
                Hit::TargetApca => {
                    app.targets.apca = app.targets.apca.cycle();
                    app.set_focus(FocusId::TargetApca);
                }
                Hit::FgInput | Hit::FgSwatch => {
                    if try_apply_active_input(app) {
                        app.set_focus(FocusId::FgHex);
                        if matches!(hit, Hit::FgInput) {
                            app.cursor_char_idx = char_index_at(
                                app.layout.fg_input,
                                col,
                                app.current_input.chars().count(),
                            );
                        }
                    }
                }
                Hit::BgInput | Hit::BgSwatch => {
                    if try_apply_active_input(app) {
                        app.set_focus(FocusId::BgHex);
                        if matches!(hit, Hit::BgInput) {
                            app.cursor_char_idx = char_index_at(
                                app.layout.bg_input,
                                col,
                                app.current_input.chars().count(),
                            );
                        }
                    }
                }
                Hit::PreviewText => {
                    if try_apply_active_input(app) {
                        app.set_focus(FocusId::PreviewText);
                        let area = app.layout.preview_text;
                        let (cursor_row, _) =
                            visual_cursor(&app.current_input, app.cursor_char_idx, area.width);
                        let scroll = view_scroll(cursor_row, area.height.max(1));
                        app.cursor_char_idx =
                            char_index_at_xy(&app.current_input, area, col, row, scroll);
                    }
                }
                Hit::FontFamily => {
                    if try_apply_active_input(app) {
                        app.set_focus(FocusId::FontFamily);
                        app.cursor_char_idx = char_index_at(
                            app.layout.font_family,
                            col,
                            app.current_input.chars().count(),
                        );
                    }
                }
                Hit::SizeInput | Hit::SizeDec | Hit::SizeInc => {
                    app.set_focus(FocusId::Size);
                    if matches!(hit, Hit::SizeInc) {
                        step_focused(
                            app,
                            true,
                            mouse.modifiers.contains(KeyModifiers::SHIFT),
                            &mut effects,
                        );
                    } else if matches!(hit, Hit::SizeDec) {
                        step_focused(
                            app,
                            false,
                            mouse.modifiers.contains(KeyModifiers::SHIFT),
                            &mut effects,
                        );
                    }
                }
                Hit::WeightInput | Hit::WeightDec | Hit::WeightInc => {
                    app.set_focus(FocusId::Weight);
                    if matches!(hit, Hit::WeightInc) {
                        step_focused(
                            app,
                            true,
                            mouse.modifiers.contains(KeyModifiers::SHIFT),
                            &mut effects,
                        );
                    } else if matches!(hit, Hit::WeightDec) {
                        step_focused(
                            app,
                            false,
                            mouse.modifiers.contains(KeyModifiers::SHIFT),
                            &mut effects,
                        );
                    }
                }
                Hit::Style(i) => {
                    app.apply_style_preset(StylePreset::from_index(i));
                    app.set_focus(FocusId::Style);
                    effects.sync_preview = true;
                }
                Hit::Swap => {
                    app.swap_colors();
                    app.set_focus(FocusId::Swap);
                    effects.sync_preview = true;
                }
                Hit::Copy => {
                    app.set_focus(FocusId::CopyHex);
                    effects.copy_hex = true;
                }
                Hit::FixBtn => {
                    app.toggle_fix();
                }
                Hit::ApplyFix => {
                    app.set_focus(FocusId::ApplyFix);
                    app.apply_fix();
                    effects.sync_preview = true;
                }
                Hit::NextFix => {
                    app.set_focus(FocusId::NextFix);
                    app.next_fix_candidate();
                }
                Hit::NudgeFg => {
                    app.set_focus(FocusId::NudgeFg);
                    app.nudge_dragging = Some(FixAxis::Fg);
                    app.set_fix_l_from_x(FixAxis::Fg, col, app.layout.nudge_fg);
                }
                Hit::NudgeBg => {
                    app.set_focus(FocusId::NudgeBg);
                    app.nudge_dragging = Some(FixAxis::Bg);
                    app.set_fix_l_from_x(FixAxis::Bg, col, app.layout.nudge_bg);
                }
                Hit::SendFg(i) => {
                    app.fix_send_chip = i.min(3);
                    app.set_focus(FocusId::SendFg);
                    app.send_fixed_to_role(FixAxis::Fg, PaletteInput::from_index(i));
                }
                Hit::SendBg(i) => {
                    app.fix_send_chip = i.min(3);
                    app.set_focus(FocusId::SendBg);
                    app.send_fixed_to_role(FixAxis::Bg, PaletteInput::from_index(i));
                }
                Hit::WebBtn => {
                    app.set_focus(FocusId::OpenPreview);
                    effects.open_preview = true;
                }
                Hit::Role(i) => {
                    if try_apply_active_input(app) {
                        app.palette.selected_idx = i.min(3);
                        app.set_focus(FocusId::Role(app.palette.selected_idx));
                        if is_double {
                            app.palette.begin_edit();
                        }
                    }
                }
                Hit::Generate => {
                    app.generate_palette();
                    app.set_focus(FocusId::Detail);
                }
                Hit::Detail | Hit::DetailScrollbar => {
                    app.set_focus(FocusId::Detail);
                }
                Hit::ContrastScrollbar => {
                    let track = app.layout.contrast_scrollbar;
                    if track.height > 0 && app.contrast_max_scroll > 0 {
                        let rel = row.saturating_sub(track.y);
                        let next = (u32::from(rel) * u32::from(app.contrast_max_scroll))
                            / u32::from(track.height.max(1));
                        app.contrast_scroll = (next as u16).min(app.contrast_max_scroll);
                    }
                }
                Hit::FixOutside | Hit::CloseFix => app.close_fix(),
                _ => {}
            }
        }
        _ => {}
    }

    effects
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(Into::into)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    disable_raw_mode()?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::render(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let effects = handle_key_event(app, key);
                    if dispatch_effects(terminal, app, effects)? {
                        return Ok(());
                    }
                }
                Event::Mouse(mouse) => {
                    let effects = handle_mouse_event(app, mouse);
                    if dispatch_effects(terminal, app, effects)? {
                        return Ok(());
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
            app.expire_notification(last_tick);
        }
    }
}

fn save_palette_with_dialog(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<()> {
    let scss = match app.prepare_palette_export("saving") {
        Ok(scss) => scss,
        Err(err) => {
            app.notify_error(err);
            return Ok(());
        }
    };

    restore_terminal(terminal)?;
    let chosen = rfd::FileDialog::new()
        .set_title("Save palette")
        .set_file_name(PALETTE_EXPORT_PATH)
        .add_filter("SCSS", &["scss", "css"])
        .save_file();
    *terminal = setup_terminal()?;
    terminal.clear()?;

    match chosen {
        Some(path) => match std::fs::write(&path, scss) {
            Ok(()) => app.notify_status(format!("Palette saved to {}.", path.display())),
            Err(err) => app.notify_error(format!("Failed to save {}: {err}", path.display())),
        },
        None => app.notify_status("Save cancelled."),
    }
    Ok(())
}

fn copy_palette(app: &mut App) {
    match app.prepare_palette_export("copying") {
        Ok(scss) => match copy_to_clipboard(&scss) {
            Ok(()) => {
                app.copied_palette = Some(scss);
                app.notify_status("Palette copied to clipboard.");
            }
            Err(err) => {
                app.copied_palette = Some(scss);
                app.notify_error(format!(
                    "Could not access a system clipboard command: {err}. Palette is available in the app copy buffer."
                ));
            }
        },
        Err(err) => {
            app.notify_error(err);
        }
    }
}

fn copy_to_clipboard(content: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        return write_to_command_stdin("pbcopy", &[], content);
    }

    #[cfg(target_os = "windows")]
    {
        return write_to_command_stdin("clip", &[], content);
    }

    #[cfg(target_os = "linux")]
    {
        let attempts: [(&str, &[&str]); 4] = [
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
            ("termux-clipboard-set", &[]),
        ];
        let mut last_err = None;
        for (program, args) in attempts {
            match write_to_command_stdin(program, args, content) {
                Ok(()) => return Ok(()),
                Err(err) => last_err = Some(err),
            }
        }
        return Err(last_err.unwrap_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no clipboard command configured",
            )
        }));
    }

    #[allow(unreachable_code)]
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "clipboard copy is not supported on this platform",
    ))
}

fn write_to_command_stdin(program: &str, args: &[&str], content: &str) -> std::io::Result<()> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(content.as_bytes())?;
    }
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "{program} exited with {status}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn tab_auto_applies_foreground_and_moves_focus() {
        let mut app = App::new();
        app.set_focus(FocusId::FgHex);
        app.current_input = "#00ff00".to_string();

        let effects = handle_key_event(&mut app, key(KeyCode::Tab, KeyModifiers::NONE));

        assert_eq!(app.foreground.to_hex(), "#00ff00");
        assert_eq!(app.focus, FocusId::BgHex);
        assert!(effects.sync_preview);
        assert!(!effects.quit);
    }

    #[test]
    fn tab_with_invalid_input_keeps_focus_and_sets_error() {
        let mut app = App::new();
        app.set_focus(FocusId::FgHex);
        app.current_input = "#zzzzzz".to_string();

        let effects = handle_key_event(&mut app, key(KeyCode::Tab, KeyModifiers::NONE));

        assert_eq!(app.focus, FocusId::FgHex);
        assert!(app.error.is_some());
        assert!(!effects.sync_preview);
    }

    #[test]
    fn ctrl_up_down_steps_focused_size() {
        let mut app = App::new();
        app.set_focus(FocusId::Size);
        app.font_size_px = 16;

        handle_key_event(&mut app, key(KeyCode::Up, KeyModifiers::CONTROL));
        assert_eq!(app.font_size_px, 17);

        handle_key_event(&mut app, key(KeyCode::Down, KeyModifiers::CONTROL));
        assert_eq!(app.font_size_px, 16);

        app.set_focus(FocusId::FgHex);
        handle_key_event(&mut app, key(KeyCode::Up, KeyModifiers::CONTROL));
        assert_eq!(app.font_size_px, 16);
    }

    #[test]
    fn ctrl_up_down_steps_focused_weight() {
        let mut app = App::new();
        app.set_focus(FocusId::Weight);
        handle_key_event(&mut app, key(KeyCode::Up, KeyModifiers::CONTROL));
        assert_eq!(app.weight, 500);
        handle_key_event(&mut app, key(KeyCode::Down, KeyModifiers::CONTROL));
        assert_eq!(app.weight, 400);
    }

    #[test]
    fn esc_does_not_quit() {
        let mut app = App::new();
        app.notify_error("error");
        let effects = handle_key_event(&mut app, key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!effects.quit);
        assert!(app.error.is_some());
    }

    #[test]
    fn ctrl_q_quits() {
        let mut app = App::new();
        let effects = handle_key_event(&mut app, key(KeyCode::Char('q'), KeyModifiers::CONTROL));
        assert!(effects.quit);
    }

    #[test]
    fn f1_toggles_help() {
        let mut app = App::new();
        handle_key_event(&mut app, key(KeyCode::F(1), KeyModifiers::NONE));
        assert!(app.show_keybindings);
        handle_key_event(&mut app, key(KeyCode::F(1), KeyModifiers::NONE));
        assert!(!app.show_keybindings);
    }

    #[test]
    fn f2_opens_theme_debug_popup() {
        let mut app = App::new();
        handle_key_event(&mut app, key(KeyCode::F(2), KeyModifiers::NONE));
        assert!(app.show_theme_debug);
    }

    #[test]
    fn keys_1_and_2_type_into_palette_color_edit() {
        let mut app = App::new();
        app.set_mode(Mode::Palette);
        app.set_focus(FocusId::Role(1));
        app.palette.selected_idx = 1;
        app.palette.begin_edit();
        app.palette.edit_input.clear();
        app.palette.edit_cursor_char_idx = 0;

        handle_key_event(&mut app, key(KeyCode::Char('1'), KeyModifiers::NONE));
        handle_key_event(&mut app, key(KeyCode::Char('2'), KeyModifiers::NONE));

        assert_eq!(app.mode, Mode::Palette);
        assert!(app.palette.editing);
        assert_eq!(app.palette.edit_input, "12");
    }

    #[test]
    fn keys_1_and_2_switch_mode() {
        let mut app = App::new();
        app.set_focus(FocusId::Swap);
        handle_key_event(&mut app, key(KeyCode::Char('2'), KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Palette);
        app.set_focus(FocusId::Generate);
        handle_key_event(&mut app, key(KeyCode::Char('1'), KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Contrast);
    }

    #[test]
    fn arrows_select_style_chips_when_style_is_focused() {
        let mut app = App::new();
        app.set_focus(FocusId::Style);
        assert_eq!(app.style_chip, 0);

        handle_key_event(&mut app, key(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(app.weight, 700);
        assert!(!app.italic);
        assert_eq!(app.style_chip, 1);

        handle_key_event(&mut app, key(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(app.weight, 400);
        assert!(app.italic);
        assert_eq!(app.style_chip, 2);

        handle_key_event(&mut app, key(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.weight, 700);
        assert!(!app.italic);
    }

    #[test]
    fn space_swaps_colors_when_not_editing() {
        let mut app = App::new();
        app.editing = false;
        app.set_focus(FocusId::Swap);
        app.foreground_input = "#000000".to_string();
        app.background_input = "#ffffff".to_string();
        let fg = app.foreground;
        handle_key_event(&mut app, key(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(app.background, fg);
    }

    #[test]
    fn enter_in_preview_text_adds_newline_and_syncs_preview() {
        let mut app = App::new();
        app.set_focus(FocusId::PreviewText);
        app.current_input = "Line 1".to_string();
        app.cursor_char_idx = app.current_input.chars().count();

        let effects = handle_key_event(&mut app, key(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(app.current_input, "Line 1\n");
        assert_eq!(app.preview_text, "Line 1\n");
        assert!(effects.sync_preview);
    }

    #[test]
    fn palette_g_generates_scss() {
        let mut app = App::new();
        handle_key_event(&mut app, key(KeyCode::Char('g'), KeyModifiers::CONTROL));
        assert_eq!(app.mode, Mode::Palette);
        assert!(app.palette.generated.is_some());
        assert_eq!(app.focus, FocusId::Detail);
    }

    #[test]
    fn contrast_page_keys_scroll_the_left_column() {
        let mut app = App::new();
        app.contrast_max_scroll = 20;
        handle_key_event(&mut app, key(KeyCode::PageDown, KeyModifiers::NONE));
        assert_eq!(app.contrast_scroll, 8);
        handle_key_event(&mut app, key(KeyCode::PageUp, KeyModifiers::NONE));
        assert_eq!(app.contrast_scroll, 0);
        handle_key_event(&mut app, key(KeyCode::PageDown, KeyModifiers::SHIFT));
        assert_eq!(app.contrast_scroll, 16);
    }

    #[test]
    fn generated_detail_scrolls_with_arrows() {
        let mut app = App::new();
        handle_key_event(&mut app, key(KeyCode::Char('g'), KeyModifiers::CONTROL));
        app.palette.detail_max_scroll = 20;
        handle_key_event(&mut app, key(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.palette.detail_scroll, 1);
        handle_key_event(&mut app, key(KeyCode::PageDown, KeyModifiers::NONE));
        assert_eq!(app.palette.detail_scroll, 11);
        handle_key_event(&mut app, key(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.palette.detail_scroll, 10);
    }

    #[test]
    fn bare_g_types_into_a_focused_color_field() {
        let mut app = App::new();
        app.set_focus(FocusId::FgHex);
        app.current_input = "#00".to_string();
        app.cursor_char_idx = 3;
        handle_key_event(&mut app, key(KeyCode::Char('g'), KeyModifiers::NONE));
        assert!(app.current_input.contains('g'));
        assert!(app.palette.generated.is_none());
    }

    #[test]
    fn ctrl_g_generates_even_when_a_color_field_is_focused() {
        let mut app = App::new();
        app.set_focus(FocusId::FgHex);
        handle_key_event(&mut app, key(KeyCode::Char('g'), KeyModifiers::CONTROL));
        assert_eq!(app.mode, Mode::Palette);
        assert!(app.palette.generated.is_some());
    }

    #[test]
    fn fix_tab_order_is_gauges_then_buttons_then_send() {
        let mut app = App::new();
        app.set_focus(FocusId::Swap);
        assert!(try_apply_active_input(&mut app));
        handle_key_event(&mut app, key(KeyCode::Char('f'), KeyModifiers::CONTROL));
        assert_eq!(app.focus, FocusId::NudgeFg);
        let expected = [
            FocusId::NudgeBg,
            FocusId::ApplyFix,
            FocusId::NextFix,
            FocusId::CloseFix,
            FocusId::SendFg,
            FocusId::SendBg,
        ];
        for focus in expected {
            handle_key_event(&mut app, key(KeyCode::Tab, KeyModifiers::NONE));
            assert_eq!(app.focus, focus);
        }
    }

    #[test]
    fn ctrl_f_toggles_fix() {
        let mut app = App::new();
        handle_key_event(&mut app, key(KeyCode::Char('f'), KeyModifiers::CONTROL));
        assert!(app.fix_open);
        assert_eq!(app.focus, FocusId::NudgeFg);
        handle_key_event(&mut app, key(KeyCode::Char('f'), KeyModifiers::CONTROL));
        assert!(!app.fix_open);
    }

    fn gray_on_gray(app: &mut App) {
        app.set_focus(FocusId::FgHex);
        app.current_input = "#808080".to_string();
        assert!(app.submit_input());
        app.set_focus(FocusId::BgHex);
        app.current_input = "#808080".to_string();
        assert!(app.submit_input());
        app.update_contrast();
    }

    #[test]
    fn fix_apply_writes_candidate_into_contrast_pair() {
        let mut app = App::new();
        gray_on_gray(&mut app);
        let original = app.foreground.to_hex();
        handle_key_event(&mut app, key(KeyCode::Char('f'), KeyModifiers::CONTROL));
        assert!(app.fix_open);
        let candidate = app.fix.candidate_fg.to_hex();
        assert_ne!(candidate, original);
        app.set_focus(FocusId::ApplyFix);
        let effects = handle_key_event(&mut app, key(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.foreground.to_hex(), candidate);
        assert!(effects.sync_preview);
    }

    #[test]
    fn ctrl_n_advances_fix_candidate() {
        let mut app = App::new();
        gray_on_gray(&mut app);
        handle_key_event(&mut app, key(KeyCode::Char('f'), KeyModifiers::CONTROL));
        let first = app.fix.candidate_fg.to_hex();
        handle_key_event(&mut app, key(KeyCode::Char('n'), KeyModifiers::CONTROL));
        if app.fix.candidate_count() > 1 {
            assert_ne!(app.fix.candidate_fg.to_hex(), first);
        }
    }

    #[test]
    fn fix_sends_fg_and_bg_to_palette_roles() {
        let mut app = App::new();
        gray_on_gray(&mut app);
        app.generate_palette();
        assert!(app.palette.generated.is_some());
        handle_key_event(&mut app, key(KeyCode::Char('f'), KeyModifiers::CONTROL));
        let fg = app.fix.candidate_fg.to_hex();
        let bg = app.fix.candidate_bg.to_hex();
        handle_key_event(&mut app, key(KeyCode::Char('p'), KeyModifiers::NONE));
        assert_eq!(app.palette.primary_input, fg);
        app.set_focus(FocusId::NudgeBg);
        handle_key_event(&mut app, key(KeyCode::Char('s'), KeyModifiers::NONE));
        assert_eq!(app.palette.secondary_input, bg);
        app.set_focus(FocusId::SendFg);
        app.fix_send_chip = 2;
        handle_key_event(&mut app, key(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.palette.tertiary_input, fg);
        assert!(app.palette.generated.is_none());
        assert_eq!(app.palette.selected_idx, 2);
    }

    #[test]
    fn brackets_nudge_fix_oklab_l() {
        let mut app = App::new();
        gray_on_gray(&mut app);
        handle_key_event(&mut app, key(KeyCode::Char('f'), KeyModifiers::CONTROL));
        let before = app.fix.candidate_fg.oklab_l();
        handle_key_event(&mut app, key(KeyCode::Char(']'), KeyModifiers::NONE));
        assert!(app.fix.candidate_fg.oklab_l() > before);
        handle_key_event(&mut app, key(KeyCode::Char('['), KeyModifiers::NONE));
        assert!((app.fix.candidate_fg.oklab_l() - before).abs() < 0.015);
    }

    #[test]
    fn ctrl_s_and_ctrl_c_set_palette_effects() {
        let mut app = App::new();
        app.set_mode(Mode::Palette);
        let save = handle_key_event(&mut app, key(KeyCode::Char('s'), KeyModifiers::CONTROL));
        let copy = handle_key_event(&mut app, key(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(save.save_palette);
        assert!(copy.copy_palette);
    }

    #[test]
    fn mouse_click_switches_tabs() {
        let mut app = App::new();
        app.layout.tabs_palette.x = 10;
        app.layout.tabs_palette.y = 1;
        app.layout.tabs_palette.width = 8;
        app.layout.tabs_palette.height = 1;
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 12,
            row: 1,
            modifiers: KeyModifiers::NONE,
        };
        handle_mouse_event(&mut app, mouse);
        assert_eq!(app.mode, Mode::Palette);
    }
}
