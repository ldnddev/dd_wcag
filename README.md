# dd_wcag

Terminal WCAG color utility built with Rust, `ratatui`, and `crossterm`.

## Features
- Foreground/background color input in `HEX`, `RGB/RGBA`, and `HSL`
- Live conversion panel (`hex`, `rgb`, `hsl`)
- WCAG AA contrast evaluation for current size/weight
- APCA (Advanced Perceptual Contrast Algorithm) evaluation alongside WCAG
- TUI preview with color + bold styling
- Browser preview with real CSS pixel font size (now includes WCAG and APCA info)
- Palette theme builder for Primary/Secondary/Tertiary brand colors with WCAG-gated `_palette.scss` output, scrollable generated output, and visible scrollbar
- Bottom-right toast notifications that auto-close after 5 seconds

## Run
```bash
cargo run
```

## Build Binary
```bash
cargo build --release
./target/release/dd_wcag
```

The app loads theme files in this order:
- `./dd_wcag_theme.yml`
- `~/.config/ldnddev/dd_wcag_theme.yml`
- Built-in defaults

Theme files must include `version: 1`; unsupported or missing versions fall back to built-in defaults with a warning.

## Install Script
Install from current checkout:
```bash
./install.sh
```

Install by cloning a repo:
```bash
./install.sh --repo https://github.com/your-org/dd_wcag.git --branch main
```

Uninstall:
```bash
./install.sh -uninstall
```

The installer:
- Builds `dd_wcag` in release mode
- Installs binary to `~/.local/bin/dd_wcag` (default)
- Installs default theme to `~/.config/ldnddev/dd_wcag_theme.yml` if missing
- Uninstall removes the installed binary and `dd_wcag_theme.yml`, then removes `~/.config/ldnddev` only if it is empty

## Keybindings
- `Tab` / `Shift+Tab`: cycle focus and auto-apply FG/BG/PreviewText input
- `Enter`: insert newline when focus is `PreviewText`
- `Left` / `Right`: move cursor in active input field
- `Ctrl+Up` / `Ctrl+Down`: increase/decrease font size (`6..=120`)
- `Ctrl+B`: toggle bold
- `G`: generate palette when focused on the Palette tab
- `Up` / `Down`: select Palette inputs before generation, scroll generated output after generation
- `F` then `G`: apply selected Palette color to foreground preview
- `B` then `G`: apply selected Palette color to background preview
- `Ctrl+S`: save generated palette to `./_palette.scss`
- `Ctrl+C`: copy generated palette values to the system clipboard when available
- `F1`: open keybindings popup
- `F2`: open theme debug popup
- `Ctrl+O`: open web preview (`/tmp/dd_wcag_preview.html`)
- `Esc`: cancel active Palette edit/apply flow or quit
- `Ctrl+Q`: quit

Non-interactive status and error messages appear as bottom-right toasts and close automatically after 5 seconds.

## Palette Builder
- Required base colors: `Primary`, `Secondary`, and `Tertiary`
- Optional base color: `Support`
- Text colors are fixed and are not changed by palette generation
- Press `G` to generate the palette and WCAG compliance checks
- Generated variables and compliance checks appear in a scrollable detail panel
- A visible scrollbar appears when the generated detail panel has overflow content
- `Ctrl+S` writes the generated palette to `./_palette.scss`
- `Ctrl+C` copies the generated palette values when a system clipboard command is available

## Manual Test Checklist
- Valid color parse and conversion updates:
  - Enter valid `HEX`, `RGB/RGBA`, and `HSL` for FG/BG.
  - Verify Conversions tab updates correctly after tabbing out.
- Invalid color handling:
  - Enter invalid `HEX` (e.g. `#12zz34`) and confirm error says HEX.
  - Confirm the error appears as a bottom-right toast and auto-closes.
- Focus/apply flow:
  - Cycle `FG -> BG -> PreviewText` using `Tab`, and reverse with `Shift+Tab`.
  - Confirm FG/BG drafts persist independently.
- Preview text editing:
  - Focus `PreviewText`, type text, use `Enter` for newline.
  - Use `Left/Right` and insert in the middle of text.
- Font size and weight:
  - In Contrast/Preview tabs, use `Ctrl+Up/Down` and verify size changes.
  - Toggle `Ctrl+B` and verify bold state changes.
- Contrast checks:
  - Switch to Contrast tab and verify WCAG ratio, pass/fail, and quick reference table.
  - Verify APCA Lc value, pass/fail, and quick reference table update with color/size/bold changes.
  - Test high/low contrast cases to see differences between WCAG and APCA.
- Web preview sync:
  - Press `Ctrl+O` to open `/tmp/dd_wcag_preview.html`.
  - Confirm FG/BG/PreviewText/size/bold changes reflect in browser preview.
  - Verify WCAG and APCA info is displayed in the meta section.
- Palette builder:
  - Tab to `Palette`, edit Primary/Secondary/Tertiary/Support with `Enter`.
  - Press `G` and confirm the generated summary has no blocking failures.
  - Confirm generated variables and compliance checks can be scrolled with `Up` / `Down`.
  - Confirm the scrollbar appears when the generated panel overflows.
  - Press `F` then `G` and confirm the selected color becomes the foreground.
  - Press `B` then `G` and confirm the selected color becomes the background.
  - Press `Ctrl+S` and confirm `_palette.scss` is written.
  - Press `Ctrl+C` and confirm the generated SCSS is copied when a clipboard command is available.

## Notes
- Terminal rendering cannot change real per-widget font size.
- For true font-size rendering, use the browser preview opened by `Ctrl+O`.
- APCA provides more accurate contrast for modern displays, often stricter than WCAG for small text.

## Architecture

### Tech stack
- `ratatui` for TUI rendering
- `crossterm` for keyboard input and terminal control
- `palette` for color conversion/math
- `anyhow` for error handling

### Module layout (`src/`)
- `main.rs` — terminal lifecycle, event loop, pure `handle_key_event` returning a `KeyEffects` struct, side-effect dispatch (browser open, palette save, clipboard, web-preview sync).
- `app.rs` — central `App` state: input-target switching, draft syncing, format-aware parse/apply, cursor-aware text editing (insert/backspace/move), font-size clamped adjustment (`6..=120`), preview font-family preset cycling, toast notifications with 5-second TTL.
- `color.rs` — parsing (`parse_hex`, `parse_rgb` incl. `rgba`, `parse_hsl`), formatting (`to_hex`, `to_rgb_str`, `to_hsl_str`), WCAG luminance and contrast math.
- `palette.rs` — Palette tab state, brand/action/support token derivation in light + dark, WCAG compliance gating, `_palette.scss` generation.
- `theme.rs` — YAML theme loading with version validation and the local → global → default fallback chain.
- `ui.rs` — ratatui layout/rendering for the five tabs, keybindings popup (`F1`), theme debug popup (`F2`), bottom-right toasts, cursor placement for active inputs.
- `web_preview.rs` — writes `/tmp/dd_wcag_preview.html` with real CSS `font-size` and opens it in the default browser.

### Runtime state (`App`)
- Colors: `foreground`, `background`, per-field drafts `foreground_input` / `background_input`, `last_parsed_format` (`HEX | RGB | RGBA | HSL | None`).
- Navigation: `input_target` (`Foreground | Background | PreviewText | FontFamily | None`), `active_tab` (`Input | Conversions | Contrast | Preview | Palette`).
- Text editing: `current_input` buffer, `cursor_char_idx`.
- Contrast/preview: `contrast_ratio`, `preview_text`, `preview_font_family`, `font_size_px` (default `12`, clamped `6..=120`), `is_bold`.
- Feedback: `error`, `status`, `notification_updated_at` (5s TTL).
- Popups: `show_keybindings`, `show_theme_debug`.
- Theming: `theme`, `theme_source` (`Local | Global | Default`).
- Palette: `palette` (`PaletteState`), `copied_palette`.

### Key-handler pattern
`handle_key_event(&mut App, KeyEvent) -> KeyEffects` is pure of I/O. It mutates `App` and returns a `KeyEffects` struct with intent flags (`quit`, `sync_preview`, `open_preview`, `save_palette`, `copy_palette`). The real side effects are performed by `run_loop` based on those flags. This split keeps key handling unit-testable.

### Input parsing rules
- Accepted: `#rgb` / `#rrggbb` (or without `#`); `rgb(r,g,b)` or `r,g,b`; `rgba(r,g,b,a)` (alpha validated, RGB used); `hsl(h,s,l)` (percent signs accepted on `s`/`l`).
- Errors are format-aware (`Invalid HEX format: ...`, `Invalid RGB format: ...`, etc.) with a fallback supported-format guidance message.

### Contrast logic
- WCAG ratio: `(Llighter + 0.05) / (Ldarker + 0.05)`.
- AA thresholds: normal text `>= 4.5`; large text `>= 3.0`. Large text means `>= 18px` normal or `>= 14px` bold.
- The Contrast tab shows the verdict for the active size/weight plus a quick-reference table for `12 / 14 / 16 / 18` at the current weight, alongside APCA Lc.

### Preview strategy
TUI preview shows accurate colors, weight, and size metadata, but terminal widgets cannot render true per-widget pixel font size. The browser preview at `/tmp/dd_wcag_preview.html` (`Ctrl+O`) is the only path that renders real CSS `font-size`.

### Test coverage
Tests live in `#[cfg(test)] mod tests` blocks alongside the code:
- `color.rs` — parsing, formatting, luminance, contrast, style conversion.
- `app.rs` — font-size clamping, FG/BG submit behavior, invalid input labeling, draft persistence, preview-text updates, cursor edit helpers.
- `main.rs` — Tab auto-apply success/failure, `Ctrl+Up/Down` direction + bounds, `Esc` dismiss-before-quit, `Enter` newline insertion, cursor movement via key events.
- `palette.rs` — palette generation, compliance checks, export validation.
- `theme.rs` — YAML parsing, version validation, fallback behavior.

CI (`.github/workflows/ci.yml`) runs `cargo check --locked` and `cargo test --locked` on stable Rust.
