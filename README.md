# dd_wcag

Terminal WCAG color utility built with Rust, `ratatui`, and `crossterm`.

## Features
- Foreground/background color input in `HEX`, `RGB/RGBA`, and `HSL`
- Live conversion panel (`hex`, `rgb`, `hsl`)
- WCAG AA contrast evaluation for current size/weight
- APCA (Advanced Perceptual Contrast Algorithm) evaluation alongside WCAG
- TUI preview with color + bold styling
- Browser preview with real CSS pixel font size (now includes WCAG and APCA info)
- Palette theme builder for Primary/Secondary/Tertiary brand colors with WCAG-gated `_palette.scss` output

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
  - Press `F` then `G` and confirm the selected color becomes the foreground.
  - Press `B` then `G` and confirm the selected color becomes the background.
  - Press `Ctrl+S` and confirm `_palette.scss` is written.
  - Press `Ctrl+C` and confirm the generated SCSS is copied when a clipboard command is available.

## Notes
- Terminal rendering cannot change real per-widget font size.
- For true font-size rendering, use the browser preview opened by `Ctrl+O`.
- APCA provides more accurate contrast for modern displays, often stricter than WCAG for small text.
