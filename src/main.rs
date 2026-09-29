//! dd_wcag TUI entry point.

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
        MouseEventKind,
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

use app::{App, FocusId, Mode, StylePreset, TOAST_TTL};
use color::Color;
use fix::FixAxis;
use layout::{Hit, char_index_at, char_index_at_xy, view_scroll, visual_cursor};
use palette::{PALETTE_EXPORT_PATH, tokens_json_path_for};
use palette::{PaletteInput, pair_at_list_index};
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
    paste: bool,
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
    if effects.paste {
        match paste_from_clipboard() {
            Ok(text) => {
                let mut paste_effects = KeyEffects::default();
                apply_paste(app, &text, &mut paste_effects);
                if paste_effects.sync_preview {
                    sync_web_preview(app);
                }
            }
            Err(err) => app.notify_error(format!("Clipboard unavailable: {err}")),
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

    if app.theme_editor.is_some() && !ctrl && key.code != KeyCode::F(1) {
        handle_theme_editor(app, key);
        return effects;
    }

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
                app.set_focus(FocusId::Matrix);
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
            KeyCode::Char('v') | KeyCode::Char('V') => effects.paste = true,
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
            app.show_keybindings = false;
            if app.theme_editor.is_some() {
                close_theme_editor(app, true);
            } else {
                app.show_theme_debug = true;
                app.theme_editor = Some(ldnddev_theme::ThemeEditor::new(
                    theme::palette_from_theme(&app.theme),
                    &[],
                ));
            }
        }
        KeyCode::Esc => {
            if app.show_keybindings {
                app.show_keybindings = false;
            } else if app.theme_editor.is_some() {
                close_theme_editor(app, true);
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
            } else if app.focus == FocusId::TargetWcag {
                app.targets.wcag = app.targets.wcag.cycle_back();
            } else if app.focus == FocusId::TargetApca {
                app.targets.apca = app.targets.apca.cycle_back();
            } else if app.focus == FocusId::Matrix {
                app.palette.move_matrix(0, -1);
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
            } else if app.focus == FocusId::TargetWcag {
                app.targets.wcag = app.targets.wcag.cycle();
            } else if app.focus == FocusId::TargetApca {
                app.targets.apca = app.targets.apca.cycle();
            } else if app.focus == FocusId::Matrix {
                app.palette.move_matrix(0, 1);
            } else if matches!(app.focus, FocusId::SendFg | FocusId::SendBg) {
                app.move_fix_send_chip(1);
            } else if app.mode == Mode::Palette && app.palette.editing {
                app.palette.move_cursor_right();
            } else if app.focus.is_text_field() {
                app.move_cursor_right();
            }
        }
        KeyCode::Home => {
            if app.mode == Mode::Palette && app.palette.editing {
                app.palette.move_cursor_home();
            } else if app.focus.is_text_field() {
                app.move_cursor_home();
            }
        }
        KeyCode::End => {
            if app.mode == Mode::Palette && app.palette.editing {
                app.palette.move_cursor_end();
            } else if app.focus.is_text_field() {
                app.move_cursor_end();
            }
        }
        KeyCode::Delete => {
            if app.mode == Mode::Palette && app.palette.editing {
                app.palette.delete_at_cursor();
            } else if app.focus.is_text_field() {
                app.delete_at_cursor();
                app.sync_active_input();
                if app.try_live_apply_color() || app.focus == FocusId::PreviewText {
                    effects.sync_preview = true;
                }
            }
        }
        KeyCode::Enter => {
            if matches!(app.focus, FocusId::SendFg | FocusId::SendBg) {
                app.send_fixed_selected_chip();
            } else if app.mode == Mode::Palette {
                if app.palette.editing {
                    app.palette.commit_edit();
                } else if app.focus == FocusId::Generate {
                    app.generate_palette();
                    app.set_focus(FocusId::Matrix);
                } else if app.focus == FocusId::Matrix {
                    app.open_fix_for_matrix();
                } else if app.focus == FocusId::OpenPreview {
                    effects.open_preview = true;
                } else if matches!(app.focus, FocusId::Role(_)) {
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
            } else if app.focus == FocusId::TargetWcag {
                app.targets.wcag = app.targets.wcag.cycle();
            } else if app.focus == FocusId::TargetApca {
                app.targets.apca = app.targets.apca.cycle();
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
                if app.try_live_apply_color() || app.focus == FocusId::PreviewText {
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
            } else if app.focus == FocusId::Matrix {
                app.palette.move_matrix(-1, 0);
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
            } else if app.focus == FocusId::Matrix {
                app.palette.move_matrix(1, 0);
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
        KeyCode::Char('[') if is_color_nudge_focus(app) => {
            apply_lightness_nudge(app, if shift { -0.10 } else { -0.02 }, &mut effects);
        }
        KeyCode::Char(']') if is_color_nudge_focus(app) => {
            apply_lightness_nudge(app, if shift { 0.10 } else { 0.02 }, &mut effects);
        }
        KeyCode::Char('{') if is_color_nudge_focus(app) => {
            apply_hue_nudge(app, if shift { -30.0 } else { -10.0 }, &mut effects);
        }
        KeyCode::Char('}') if is_color_nudge_focus(app) => {
            apply_hue_nudge(app, if shift { 30.0 } else { 10.0 }, &mut effects);
        }
        KeyCode::Char(' ') if !app.editing => {
            if app.mode == Mode::Palette && app.focus == FocusId::Matrix {
                app.palette.swap_matrix_axes();
            } else if app.mode == Mode::Contrast {
                if app.focus == FocusId::Style {
                    app.apply_style_preset(StylePreset::from_index(app.style_chip));
                    effects.sync_preview = true;
                } else if app.focus == FocusId::TargetWcag {
                    app.targets.wcag = app.targets.wcag.cycle();
                } else if app.focus == FocusId::TargetApca {
                    app.targets.apca = app.targets.apca.cycle();
                } else {
                    app.swap_colors();
                    effects.sync_preview = true;
                }
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
                if app.try_live_apply_color() || app.focus == FocusId::PreviewText {
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

fn is_color_nudge_focus(app: &App) -> bool {
    matches!(
        app.focus,
        FocusId::FgHex | FocusId::BgHex | FocusId::NudgeFg | FocusId::NudgeBg
    )
}

fn apply_lightness_nudge(app: &mut App, delta: f32, effects: &mut KeyEffects) {
    let axis = fix_nudge_axis(app);
    if is_fix_nudge_focus(app)
        || (app.fix_open && !matches!(app.focus, FocusId::FgHex | FocusId::BgHex))
    {
        app.nudge_fix(axis, delta);
    } else {
        nudge_live_color(app, axis, delta);
        effects.sync_preview = true;
    }
}

fn apply_hue_nudge(app: &mut App, degrees: f32, effects: &mut KeyEffects) {
    let axis = fix_nudge_axis(app);
    if is_fix_nudge_focus(app)
        || (app.fix_open && !matches!(app.focus, FocusId::FgHex | FocusId::BgHex))
    {
        app.nudge_fix_hue(axis, degrees);
    } else {
        match axis {
            FixAxis::Fg => {
                app.foreground = app.foreground.nudge_hue(degrees);
                app.foreground_input = app.foreground.to_hex();
                if app.focus == FocusId::FgHex {
                    app.current_input = app.foreground_input.clone();
                    app.cursor_char_idx = app.current_input.chars().count();
                }
            }
            FixAxis::Bg => {
                app.background = app.background.nudge_hue(degrees);
                app.background_input = app.background.to_hex();
                if app.focus == FocusId::BgHex {
                    app.current_input = app.background_input.clone();
                    app.cursor_char_idx = app.current_input.chars().count();
                }
            }
        }
        app.update_contrast();
        effects.sync_preview = true;
    }
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
            } else if app.scrollbar_dragging {
                drag_scrollbar_to(app, row);
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
                    Hit::PairList => {
                        app.set_focus(FocusId::Matrix);
                        let step = if shift { 8 } else { 3 };
                        app.palette.scroll_pair_by(if up { -step } else { step });
                    }
                    Hit::Detail | Hit::DetailScrollbar => {
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
                        if mouse.modifiers.contains(KeyModifiers::SHIFT)
                            && matches!(hit, Hit::FgSwatch)
                        {
                            effects.copy_hex = true;
                        } else if matches!(hit, Hit::FgInput) {
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
                        if mouse.modifiers.contains(KeyModifiers::SHIFT)
                            && matches!(hit, Hit::BgSwatch)
                        {
                            effects.copy_hex = true;
                        } else if matches!(hit, Hit::BgInput) {
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
                        if mouse.modifiers.contains(KeyModifiers::SHIFT) {
                            effects.copy_hex = true;
                        } else if is_double {
                            app.palette.begin_edit();
                        }
                    }
                }
                Hit::Generate => {
                    app.generate_palette();
                    app.set_focus(FocusId::Matrix);
                }
                Hit::TextRow => {
                    app.palette.select_text_axis();
                    app.set_focus(FocusId::Matrix);
                }
                Hit::MatrixCell(r, c) => {
                    if r != c {
                        app.palette.set_matrix(r, c);
                        app.set_focus(FocusId::Matrix);
                        if is_double {
                            app.open_fix_for_matrix();
                        }
                    }
                }
                Hit::PairList => {
                    app.set_focus(FocusId::Matrix);
                    let rel = usize::from(row.saturating_sub(app.layout.pair_list.y));
                    if let Some((r, c)) =
                        pair_at_list_index(app.palette.pair_scroll.saturating_add(rel))
                    {
                        app.palette.set_matrix(r, c);
                        if is_double {
                            app.open_fix_for_matrix();
                        }
                    }
                }
                Hit::Detail | Hit::DetailScrollbar => {
                    app.set_focus(FocusId::Detail);
                    if matches!(hit, Hit::DetailScrollbar) {
                        app.scrollbar_dragging = true;
                        drag_scrollbar_to(app, row);
                    }
                }
                Hit::ContrastScrollbar => {
                    app.scrollbar_dragging = true;
                    drag_scrollbar_to(app, row);
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
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(Into::into)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    execute!(
        terminal.backend_mut(),
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    disable_raw_mode()?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    let mut needs_draw = true;

    loop {
        if needs_draw {
            terminal.draw(|f| ui::render(f, app))?;
            needs_draw = false;
        }

        match next_event(app)? {
            Some(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                let effects = handle_key_event(app, key);
                let quit = dispatch_effects(terminal, app, effects)?;
                if quit {
                    return Ok(());
                }
                needs_draw = true;
            }
            Some(Event::Mouse(mouse)) => {
                let prev_hover = app.hovered;
                let dragging_before = app.nudge_dragging.is_some() || app.scrollbar_dragging;
                let effects = handle_mouse_event(app, mouse);
                let dragging_after = app.nudge_dragging.is_some() || app.scrollbar_dragging;
                needs_draw = mouse_requires_redraw(
                    mouse.kind,
                    prev_hover != app.hovered,
                    dragging_before || dragging_after,
                    &effects,
                );
                if dispatch_effects(terminal, app, effects)? {
                    return Ok(());
                }
            }
            Some(Event::Paste(text)) => {
                let mut effects = KeyEffects::default();
                apply_paste(app, &text, &mut effects);
                if dispatch_effects(terminal, app, effects)? {
                    return Ok(());
                }
                needs_draw = true;
            }
            Some(Event::Resize(_, _)) => needs_draw = true,
            Some(_) => {}
            None => {
                needs_draw = app.expire_notification(Instant::now());
            }
        }
    }
}

fn next_event(app: &App) -> io::Result<Option<Event>> {
    match app.toast_remaining(Instant::now()) {
        Some(remaining) => {
            let timeout = if remaining.is_zero() {
                Duration::ZERO
            } else {
                remaining.min(TOAST_TTL)
            };
            if event::poll(timeout)? {
                event::read().map(Some)
            } else {
                Ok(None)
            }
        }
        None => event::read().map(Some),
    }
}

fn mouse_requires_redraw(
    kind: MouseEventKind,
    hover_changed: bool,
    dragging: bool,
    effects: &KeyEffects,
) -> bool {
    if effects.sync_preview
        || effects.open_preview
        || effects.save_palette
        || effects.copy_palette
        || effects.copy_hex
        || effects.paste
        || effects.quit
    {
        return true;
    }
    match kind {
        MouseEventKind::Moved | MouseEventKind::Drag(_) => hover_changed || dragging,
        _ => true,
    }
}

fn save_palette_with_dialog(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<()> {
    let (scss, tokens_json) = match app.prepare_palette_export("saving") {
        Ok(generated) => (generated.scss.clone(), generated.tokens_json.clone()),
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
            Ok(()) => {
                let json_path = tokens_json_path_for(&path);
                match std::fs::write(&json_path, tokens_json) {
                    Ok(()) => app.notify_status(format!(
                        "Palette saved to {} and {}.",
                        path.display(),
                        json_path.display()
                    )),
                    Err(err) => app.notify_error(format!(
                        "Saved {} but failed to save {}: {err}",
                        path.display(),
                        json_path.display()
                    )),
                }
            }
            Err(err) => app.notify_error(format!("Failed to save {}: {err}", path.display())),
        },
        None => app.notify_status("Save cancelled."),
    }
    Ok(())
}

fn copy_palette(app: &mut App) {
    let scss = match app.prepare_palette_export("copying") {
        Ok(generated) => generated.scss.clone(),
        Err(err) => {
            app.notify_error(err);
            return;
        }
    };
    match copy_to_clipboard(&scss) {
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
    }
}

fn drag_scrollbar_to(app: &mut App, row: u16) {
    match app.mode {
        Mode::Contrast => {
            let track = app.layout.contrast_scrollbar;
            if track.height == 0 || app.contrast_max_scroll == 0 {
                return;
            }
            let rel = row.saturating_sub(track.y);
            let next = (u32::from(rel) * u32::from(app.contrast_max_scroll))
                / u32::from(track.height.max(1));
            app.contrast_scroll = (next as u16).min(app.contrast_max_scroll);
        }
        Mode::Palette => {
            let track = app.layout.detail_scrollbar;
            if track.height == 0 || app.palette.detail_max_scroll == 0 {
                return;
            }
            let rel = usize::from(row.saturating_sub(track.y));
            let max = app.palette.detail_max_scroll;
            let next = (rel * max) / usize::from(track.height.max(1));
            app.palette.detail_scroll = next.min(max);
        }
    }
}

fn apply_paste(app: &mut App, raw: &str, effects: &mut KeyEffects) {
    if app.show_keybindings || app.show_theme_debug || app.theme_editor.is_some() {
        return;
    }
    let text = raw.trim();
    if text.is_empty() {
        return;
    }

    if app.mode == Mode::Palette && app.palette.editing {
        if Color::parse_input(text).is_ok() {
            app.palette.edit_input = text.to_string();
            app.palette.edit_cursor_char_idx = app.palette.edit_input.chars().count();
        } else {
            app.palette.insert_str_at_cursor(text);
        }
        return;
    }

    if !app.focus.is_text_field() {
        return;
    }

    let is_color_field = matches!(app.focus, FocusId::FgHex | FocusId::BgHex);
    if is_color_field && Color::parse_input(text).is_ok() {
        app.current_input = text.to_string();
        app.cursor_char_idx = app.current_input.chars().count();
        app.sync_active_input();
        if app.submit_input() {
            effects.sync_preview = true;
        }
        return;
    }

    app.insert_str_at_cursor(text);
    app.sync_active_input();
    if app.try_live_apply_color() || app.focus == FocusId::PreviewText {
        effects.sync_preview = true;
    }
}

fn paste_from_clipboard() -> std::io::Result<String> {
    #[cfg(target_os = "macos")]
    {
        return read_command_stdout("pbpaste", &[]);
    }

    #[cfg(target_os = "windows")]
    {
        return read_command_stdout("powershell", &["-NoProfile", "-Command", "Get-Clipboard"]);
    }

    #[cfg(target_os = "linux")]
    {
        let attempts: [(&str, &[&str]); 4] = [
            ("wl-paste", &["-n"]),
            ("xclip", &["-selection", "clipboard", "-o"]),
            ("xsel", &["--clipboard", "--output"]),
            ("termux-clipboard-get", &[]),
        ];
        let mut last_err = None;
        for (program, args) in attempts {
            match read_command_stdout(program, args) {
                Ok(text) => return Ok(text),
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
        "clipboard paste is not supported on this platform",
    ))
}

fn read_command_stdout(program: &str, args: &[&str]) -> std::io::Result<String> {
    let output = Command::new(program).args(args).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(std::io::Error::other(format!(
            "{program} exited with {}",
            output.status
        )))
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

fn handle_theme_editor(app: &mut App, key: KeyEvent) {
    if key.code == KeyCode::F(2) {
        close_theme_editor(app, true);
        return;
    }
    let Some(ek) = map_editor_key(key) else {
        return;
    };
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let outcome = {
        let Some(editor) = app.theme_editor.as_mut() else {
            return;
        };
        editor.handle(ek, shift)
    };
    match outcome {
        ldnddev_theme::EditorOutcome::PaletteChanged => {
            if let Some(editor) = &app.theme_editor {
                theme::apply_palette(&mut app.theme, &editor.palette);
            }
        }
        ldnddev_theme::EditorOutcome::RequestSave => {
            if let Err(err) = save_theme_editor(app) {
                app.notify_error(format!("Save failed: {err}"));
            }
        }
        ldnddev_theme::EditorOutcome::Closed { .. } => close_theme_editor(app, false),
        ldnddev_theme::EditorOutcome::HexError(err) => {
            app.notify_error(format!("Invalid hex: {err}"));
        }
        ldnddev_theme::EditorOutcome::None => {}
    }
}

fn close_theme_editor(app: &mut App, revert: bool) {
    if let Some(mut editor) = app.theme_editor.take() {
        if revert {
            editor.revert();
        }
        theme::apply_palette(&mut app.theme, &editor.palette);
    }
    app.show_theme_debug = false;
}

fn save_theme_editor(app: &mut App) -> anyhow::Result<()> {
    let Some(editor) = &app.theme_editor else {
        return Ok(());
    };
    let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let path = ldnddev_theme::save_theme(
        &editor.palette,
        &root,
        theme::PROJECT_THEME_FILE,
        editor.save_target,
        ldnddev_theme::default_config_home().as_deref(),
        &editor.fields,
    )?;
    theme::apply_palette(&mut app.theme, &editor.palette);
    app.theme_source = match editor.save_target {
        ldnddev_theme::ThemeSaveTarget::Local => theme::ThemeSource::Local,
        ldnddev_theme::ThemeSaveTarget::Global => theme::ThemeSource::Global,
    };
    app.notify_status(format!(
        "Saved {} theme to {}",
        editor.save_target.label(),
        path.display()
    ));
    app.theme_editor = None;
    app.show_theme_debug = false;
    Ok(())
}

fn map_editor_key(key: KeyEvent) -> Option<ldnddev_theme::EditorKey> {
    Some(match key.code {
        KeyCode::Up => ldnddev_theme::EditorKey::Up,
        KeyCode::Down => ldnddev_theme::EditorKey::Down,
        KeyCode::Left => ldnddev_theme::EditorKey::Left,
        KeyCode::Right => ldnddev_theme::EditorKey::Right,
        KeyCode::Tab => ldnddev_theme::EditorKey::Tab,
        KeyCode::Enter => ldnddev_theme::EditorKey::Enter,
        KeyCode::Esc => ldnddev_theme::EditorKey::Esc,
        KeyCode::Backspace => ldnddev_theme::EditorKey::Backspace,
        KeyCode::Char(c) => ldnddev_theme::EditorKey::Char(c),
        _ => return None,
    })
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
        assert_eq!(app.focus, FocusId::Matrix);
        assert_ne!(app.palette.matrix_row, app.palette.matrix_col);
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
        app.set_focus(FocusId::Detail);
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

    #[test]
    fn typing_a_valid_hex_updates_the_pair_live() {
        let mut app = App::new();
        app.set_focus(FocusId::FgHex);
        app.current_input.clear();
        app.cursor_char_idx = 0;
        for c in "#00ff00".chars() {
            handle_key_event(&mut app, key(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(app.foreground.to_hex(), "#00ff00");
        assert!(app.error.is_none());
    }

    #[test]
    fn home_end_and_delete_edit_text_fields() {
        let mut app = App::new();
        app.set_focus(FocusId::PreviewText);
        app.current_input = "abcd".to_string();
        app.cursor_char_idx = 1;
        handle_key_event(&mut app, key(KeyCode::Delete, KeyModifiers::NONE));
        assert_eq!(app.current_input, "acd");
        handle_key_event(&mut app, key(KeyCode::End, KeyModifiers::NONE));
        assert_eq!(app.cursor_char_idx, 3);
        handle_key_event(&mut app, key(KeyCode::Home, KeyModifiers::NONE));
        assert_eq!(app.cursor_char_idx, 0);
    }

    #[test]
    fn keyboard_cycles_wcag_and_apca_targets() {
        let mut app = App::new();
        app.set_focus(FocusId::TargetWcag);
        handle_key_event(&mut app, key(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.targets.wcag, crate::app::WcagLevel::Aaa);
        handle_key_event(&mut app, key(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.targets.wcag, crate::app::WcagLevel::Aa);

        app.set_focus(FocusId::TargetApca);
        handle_key_event(&mut app, key(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(app.targets.apca, crate::app::ApcaTarget::Lc90);
    }

    #[test]
    fn braces_nudge_hue_on_focused_color() {
        let mut app = App::new();
        app.set_focus(FocusId::FgHex);
        app.current_input = "#ff0000".to_string();
        assert!(app.submit_input());
        let before = app.foreground.to_hex();
        handle_key_event(&mut app, key(KeyCode::Char('}'), KeyModifiers::NONE));
        assert_ne!(app.foreground.to_hex(), before);
    }

    #[test]
    fn paste_replaces_a_valid_color_field() {
        let mut app = App::new();
        app.set_focus(FocusId::FgHex);
        let mut effects = KeyEffects::default();
        apply_paste(&mut app, "  #336699  \n", &mut effects);
        assert_eq!(app.foreground.to_hex(), "#336699");
        assert!(effects.sync_preview);
    }

    #[test]
    fn shift_click_swatch_copies_hex() {
        let mut app = App::new();
        app.layout.fg_swatch.x = 20;
        app.layout.fg_swatch.y = 2;
        app.layout.fg_swatch.width = 2;
        app.layout.fg_swatch.height = 1;
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 20,
            row: 2,
            modifiers: KeyModifiers::SHIFT,
        };
        let effects = handle_mouse_event(&mut app, mouse);
        assert!(effects.copy_hex);
        assert_eq!(app.focus, FocusId::FgHex);
    }

    #[test]
    fn contrast_scrollbar_click_starts_a_drag() {
        let mut app = App::new();
        app.contrast_max_scroll = 10;
        app.layout.contrast_scrollbar.x = 39;
        app.layout.contrast_scrollbar.y = 3;
        app.layout.contrast_scrollbar.width = 1;
        app.layout.contrast_scrollbar.height = 10;
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 39,
            row: 8,
            modifiers: KeyModifiers::NONE,
        };
        handle_mouse_event(&mut app, mouse);
        assert!(app.scrollbar_dragging);
        assert!(app.contrast_scroll > 0);
    }

    #[test]
    fn idle_mouse_move_without_hover_change_skips_redraw() {
        assert!(!mouse_requires_redraw(
            MouseEventKind::Moved,
            false,
            false,
            &KeyEffects::default(),
        ));
        assert!(mouse_requires_redraw(
            MouseEventKind::Moved,
            true,
            false,
            &KeyEffects::default(),
        ));
        assert!(mouse_requires_redraw(
            MouseEventKind::Down(MouseButton::Left),
            false,
            false,
            &KeyEffects::default(),
        ));
    }

    #[test]
    fn matrix_arrows_and_space_and_enter() {
        let mut app = App::new();
        app.set_mode(Mode::Palette);
        app.set_focus(FocusId::Matrix);
        assert_eq!(app.palette.matrix_row, 3);
        assert_eq!(app.palette.matrix_col, 0);
        handle_key_event(&mut app, key(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(app.palette.matrix_row, 3);
        assert_eq!(app.palette.matrix_col, 1);
        handle_key_event(&mut app, key(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(app.palette.matrix_row, 1);
        assert_eq!(app.palette.matrix_col, 3);
        handle_key_event(&mut app, key(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.fix_open);
        assert!(matches!(
            app.fix_source,
            crate::app::FixSource::Palette { .. }
        ));
    }

    #[test]
    fn mouse_click_selects_a_matrix_cell_and_ignores_diagonal() {
        let mut app = App::new();
        app.set_mode(Mode::Palette);
        app.layout.matrix_cells[0][0] = ratatui::layout::Rect::new(10, 4, 5, 1);
        app.layout.matrix_cells[1][0] = ratatui::layout::Rect::new(10, 5, 5, 1);
        let diagonal = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 11,
            row: 4,
            modifiers: KeyModifiers::NONE,
        };
        handle_mouse_event(&mut app, diagonal);
        assert_eq!(app.palette.matrix_row, 3);
        assert_eq!(app.palette.matrix_col, 0);

        let cell = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 11,
            row: 5,
            modifiers: KeyModifiers::NONE,
        };
        handle_mouse_event(&mut app, cell);
        assert_eq!(app.palette.matrix_row, 1);
        assert_eq!(app.palette.matrix_col, 0);
        assert_eq!(app.focus, FocusId::Matrix);
    }

    #[test]
    fn mouse_text_row_selects_the_text_axis() {
        let mut app = App::new();
        app.set_mode(Mode::Palette);
        app.palette.set_matrix(0, 1);
        app.layout.text_row = ratatui::layout::Rect::new(2, 8, 20, 1);
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 4,
            row: 8,
            modifiers: KeyModifiers::NONE,
        };
        handle_mouse_event(&mut app, mouse);
        assert_eq!(app.palette.matrix_row, 3);
        assert_eq!(app.palette.matrix_col, 1);
        assert_eq!(app.focus, FocusId::Matrix);
    }

    #[test]
    fn tab_from_open_preview_reaches_header_targets() {
        let mut app = App::new();
        app.set_focus(FocusId::OpenPreview);
        handle_key_event(&mut app, key(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.focus, FocusId::TargetWcag);
        handle_key_event(&mut app, key(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.focus, FocusId::TargetApca);
    }
}
