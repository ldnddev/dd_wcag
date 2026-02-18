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
- `Ctrl+Up` / `Ctrl+Down`: increase/decrease font size (`6..=120`)
- `Ctrl+B`: toggle bold
- `Ctrl+O`: open web preview (`/tmp/dd_wcag_preview.html`)
- `Esc`: dismiss error popup (or quit when no popup)
- `Ctrl+Q`: quit

## Notes
- Terminal rendering cannot change real per-widget font size.
- For true font-size rendering, use the browser preview opened by `Ctrl+O`.
