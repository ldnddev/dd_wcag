# dd_wcag Project Architecture

## Overview
`dd_wcag` is a terminal-first WCAG color utility built with Rust.
It supports:
- Foreground/background color input in `hex`, `rgb/rgba`, and `hsl`
- Live conversions (hex/rgb/hsl)
- WCAG AA contrast evaluation at the current font size
- TUI preview styling (color + bold)
- Browser preview for true CSS pixel font-size rendering

## Tech Stack
- `ratatui` for TUI rendering
- `crossterm` for keyboard input and terminal control
- `palette` for color conversion/math
- `anyhow` for error handling

## Module Layout
- `src/main.rs`
  - App bootstrap, terminal lifecycle, event loop, key handling
  - Syncs web preview file when style/color state changes
- `src/app.rs`
  - Central app state and state transitions
  - Input target switching, draft syncing, submit/apply logic
  - Font-size clamped adjustment (`6..=120`)
- `src/color.rs`
  - Parsing: `parse_hex`, `parse_rgb` (including `rgba`), `parse_hsl`
  - Formatting: `to_hex`, `to_rgb_str`, `to_hsl_str`
  - WCAG math: luminance and contrast ratio
- `src/ui.rs`
  - TUI layout and rendering for Input / Conversions / Contrast / Preview tabs
  - Error popup rendering
- `src/web_preview.rs`
  - Generates `/tmp/dd_wcag_preview.html` with real CSS `font-size`
  - Opens preview file in default browser

## Runtime State (`App`)
- Colors:
  - `foreground`, `background` (committed parsed colors)
  - `foreground_input`, `background_input` (per-field drafts)
  - `current_input` (active input buffer)
- Input/Navigation:
  - `input_target`: `Foreground | Background | None`
  - `active_tab`: `Input | Conversions | Contrast | Preview`
- Contrast/Preview:
  - `contrast_ratio`
  - `font_size_px` (default `12`, clamped `6..=120`)
  - `is_bold`
  - `preview_text`
- Feedback:
  - `error`

## Keybindings
- Navigation:
  - `Tab` / `Shift+Tab`: cycle across all fields/tabs
  - Leaving `FG` or `BG` auto-applies parsed input
- Actions:
  - `Ctrl+Up`: increase size by `1px` (max `120`)
  - `Ctrl+Down`: decrease size by `1px` (min `6`)
  - `Ctrl+B`: toggle bold
  - `Ctrl+O`: open web preview
  - `Ctrl+Q`: quit
  - `Esc`: dismiss error popup if shown, else quit

## Input Parsing Rules
- Accepted input formats:
  - Hex: `#rgb`, `#rrggbb` (or without `#`)
  - RGB: `rgb(r,g,b)` or `r,g,b`
  - RGBA: `rgba(r,g,b,a)` (`a` must be `0.0..=1.0`; alpha is validated, RGB used)
  - HSL: `hsl(h,s,l)` (percent signs accepted on `s` and `l`)

## Contrast Logic
- Ratio formula: `(Llighter + 0.05) / (Ldarker + 0.05)`
- AA thresholds:
  - Normal text: `>= 4.5`
  - Large text: `>= 3.0`
  - Large text means:
    - `>= 18px` normal, or
    - `>= 14px` bold

## Preview Strategy
- TUI preview shows accurate colors/bold state and current size metadata.
- Terminal widgets cannot render true per-widget pixel font sizes.
- Real font-size preview is provided via `/tmp/dd_wcag_preview.html` (opened with `Ctrl+O`).

## Test Coverage
- `src/color.rs` tests:
  - Parsing, formatting, luminance, contrast, style conversion
- `src/app.rs` tests:
  - Font-size clamping
  - Foreground/background submit behavior
  - Invalid input handling
  - FG/BG draft persistence through tab-like apply flow
