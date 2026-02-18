# dd_wcag

Terminal WCAG color utility built with Rust, `ratatui`, and `crossterm`.

## Features
- Foreground/background color input in `HEX`, `RGB/RGBA`, and `HSL`
- Live conversion panel (`hex`, `rgb`, `hsl`)
- WCAG AA contrast evaluation for current size/weight
- TUI preview with color + bold styling
- Browser preview with real CSS pixel font size

## Run
```bash
cargo run
```

## Keybindings
- `Tab` / `Shift+Tab`: cycle focus and auto-apply FG/BG/PreviewText input
- `Enter`: insert newline when focus is `PreviewText`
- `Left` / `Right`: move cursor in active input field
- `Ctrl+Up` / `Ctrl+Down`: increase/decrease font size (`6..=120`)
- `Ctrl+B`: toggle bold
- `Ctrl+O`: open web preview (`/tmp/dd_wcag_preview.html`)
- `Esc`: dismiss error popup (or quit when no popup)
- `Ctrl+Q`: quit

## Manual Test Checklist
- Valid color parse and conversion updates:
  - Enter valid `HEX`, `RGB/RGBA`, and `HSL` for FG/BG.
  - Verify Conversions tab updates correctly after tabbing out.
- Invalid color handling:
  - Enter invalid `HEX` (e.g. `#12zz34`) and confirm error says HEX.
  - Press `Esc` and confirm popup closes without quitting.
- Focus/apply flow:
  - Cycle `FG -> BG -> PreviewText` using `Tab`, and reverse with `Shift+Tab`.
  - Confirm FG/BG drafts persist independently.
- Preview text editing:
  - Focus `PreviewText`, type text, use `Enter` for newline.
  - Use `Left/Right` and insert in the middle of text.
- Font size and weight:
  - In Contrast/Preview tabs, use `Ctrl+Up/Down` and verify size changes.
  - Toggle `Ctrl+B` and verify bold state changes.
- Web preview sync:
  - Press `Ctrl+O` to open `/tmp/dd_wcag_preview.html`.
  - Confirm FG/BG/PreviewText/size/bold changes reflect in browser preview.

## Notes
- Terminal rendering cannot change real per-widget font size.
- For true font-size rendering, use the browser preview opened by `Ctrl+O`.
