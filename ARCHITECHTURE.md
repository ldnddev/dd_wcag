# dd_wcag Project Architecture

## Overview
`dd_wcag` is a terminal-first WCAG color utility built with Rust.
It supports:
- Foreground/background color input in `hex`, `rgb/rgba`, and `hsl`
- Editable multi-line preview text input
- Live conversions (`hex`, `rgb`, `hsl`)
- WCAG AA contrast evaluation at current size/weight
- TUI preview styling (color + bold)
- Browser preview for true CSS pixel font-size rendering

## Tech Stack
- `ratatui` for TUI rendering
- `crossterm` for keyboard input and terminal control
- `palette` for color conversion/math
- `anyhow` for error handling

## Module Layout
- `src/main.rs`
  - App bootstrap, terminal lifecycle, event loop
  - Testable key-event handler (`handle_key_event`) and side-effect dispatch
- `src/app.rs`
  - Central app state and transitions
  - Input-target switching, draft syncing, format-aware parse/apply logic
  - Cursor-aware text editing primitives (insert/backspace/move)
  - Font-size clamped adjustment (`6..=120`)
- `src/color.rs`
  - Parsing: `parse_hex`, `parse_rgb` (including `rgba`), `parse_hsl`
  - Formatting: `to_hex`, `to_rgb_str`, `to_hsl_str`
  - WCAG math: luminance and contrast ratio
- `src/ui.rs`
  - TUI layout/rendering for Input / Conversions / Contrast / Preview tabs
  - Dynamic help/focus status and error popup
  - Cursor placement logic for active input fields
- `src/web_preview.rs`
  - Generates `/tmp/dd_wcag_preview.html` with real CSS `font-size`
  - Opens preview file in default browser

## Runtime State (`App`)
- Colors:
  - `foreground`, `background` (committed parsed colors)
  - `foreground_input`, `background_input` (per-field drafts)
  - `last_parsed_format` (`HEX`, `RGB`, `RGBA`, `HSL`, or `None`)
- Input/Navigation:
  - `input_target`: `Foreground | Background | PreviewText | None`
  - `active_tab`: `Input | Conversions | Contrast | Preview`
- Text Editing:
  - `current_input` (active input buffer)
  - `cursor_char_idx` (cursor position in chars, for insert/backspace/move)
- Contrast/Preview:
  - `contrast_ratio`
  - `font_size_px` (default `12`, clamped `6..=120`)
  - `is_bold`
  - `preview_text`
- Feedback:
  - `error`

## Keybindings
- Navigation:
  - `Tab` / `Shift+Tab`: cycle all fields/tabs and auto-apply active Input target
- Input Editing:
  - `Left` / `Right`: move cursor in active input field
  - `Backspace`: delete before cursor in active input field
  - `Enter`: insert newline when focused on `PreviewText`
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
  - RGBA: `rgba(r,g,b,a)` (`a` must be `0.0..=1.0`; alpha validated, RGB used)
  - HSL: `hsl(h,s,l)` (percent signs accepted on `s` and `l`)
- Errors are format-aware when possible:
  - `Invalid HEX format: ...`
  - `Invalid RGB format: ...`
  - `Invalid RGBA format: ...`
  - `Invalid HSL format: ...`
  - fallback generic supported-format guidance

## Contrast Logic
- Ratio formula: `(Llighter + 0.05) / (Ldarker + 0.05)`
- AA thresholds:
  - Normal text: `>= 4.5`
  - Large text: `>= 3.0`
  - Large text means:
    - `>= 18px` normal, or
    - `>= 14px` bold
- Contrast tab output:
  - Current-size verdict for active size/weight
  - Quick-reference rows for `12/14/16/18` at current weight

## Preview Strategy
- TUI preview shows accurate colors, weight, and size metadata.
- Terminal widgets cannot render true per-widget pixel font size.
- Real font-size preview is provided via `/tmp/dd_wcag_preview.html` (`Ctrl+O`).

## Test Coverage
- `src/color.rs` tests:
  - Parsing, formatting, luminance, contrast, style conversion
- `src/app.rs` tests:
  - Font-size clamping
  - FG/BG submit behavior
  - Invalid input handling and error labeling
  - Draft persistence and preview-text updates
  - Cursor edit helpers
- `src/main.rs` tests:
  - Tab auto-apply success/failure flows
  - `Ctrl+Up/Down` direction + bounds
  - `Esc` dismiss-before-quit behavior
  - `Enter` newline insertion in `PreviewText`
  - Cursor movement/insertion via key events
