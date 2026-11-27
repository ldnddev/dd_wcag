# dd_wcag

WCAG 2.x and APCA contrast checker, plus an SCSS palette builder. The same checker runs in the terminal and in a desktop window. Both read one command line and one theme file. `check` and `palette` print and exit. They never open a UI.

**[Illustrated tutorial](https://ldnddev.github.io/dd_wcag/tutorial.html)** — setup, install, Contrast, Fix, Palette, theming, and how to refresh the screenshots.

![Contrast tab with a light brand color on a dark background](docs/images/contrast-pair.png)

## Quick start

```bash
curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash
dd_wcag
```

With no flag, the installer asks for the terminal (`dd_wcag`), the desktop window (`dd_wcag-gui`), or both. A non-interactive shell installs the terminal. `INSTALL_GUI=1` with no target flag installs both. Pin the choice:

```bash
curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash -s -- --tui
curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash -s -- --gui
curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash -s -- --both
```

The installer detects OS and CPU, downloads the matching GitHub Release package when one exists (Linux glibc x86_64/arm64, macOS Intel/Apple Silicon), and puts the binary in `~/.local/bin`. Alpine/musl and unpublished targets fall back to a cargo build. A default theme is copied to `~/.config/ldnddev/` only if that file is missing. On Linux it writes a launcher under `~/.local/share/applications/` (`dd_wcag.desktop` opens a terminal; `dd_wcag-gui.desktop` opens the window). Pin a release with `--version`, or force a cargo build with `--from-source`:

```bash
curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash -s -- --version v0.9.5
curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash -s -- --from-source
```

From a local checkout:

```bash
./install.sh --both
dd_wcag
dd_wcag-gui
```

Developers can run either UI without installing:

```bash
cargo run
cargo run -p dd_wcag_gui --bin dd_wcag-gui
```

`cargo build` skips the window. Build it with `-p dd_wcag_gui`. The window needs Vulkan, Wayland or X11, fontconfig, and FreeType.

Uninstall asks which binary to remove. `--tui`, `--gui`, or `--both` skip the question. `-uninstall` leaves the theme file in place. Pass the same `--prefix`, `--config-dir`, and `XDG_DATA_HOME` you used at install time.

```bash
./install.sh -uninstall --both
curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash -s -- -uninstall
```

## What it does

Contrast is about 60% of the product. Palette is the other 40%.

- Check one foreground/background pair against **WCAG 2.x** (AA or AAA) and **APCA** (Lc 45 / 60 / 75 / 90). Both scores stay on screen. APCA is directional: text on a surface is not the same pair as that surface on text.
- Size 6–120px and weight 100–900 (step 100), shared by Contrast and Palette. Large text is 18px or larger, or 14px or larger at weight 700 or more. AA needs 4.5:1 (3:1 for large text). AAA needs 7:1 (4.5:1 for large text).
- The APCA header is the official pass bar (`|Lc|` greater than or equal to 45, 60, 75, or 90). Size and weight also drive an advisory lookup line. They do not change the official bar.
- Style chips: Regular (400), Bold (700), Italic (400, italic), Bold+Italic (700, italic). Italic is visual only. It does not change the math. A weight that is not 400 or 700 leaves the chips unset.
- Live conversions for the pair: hex, `rgb()`, and `hsl()`. Fields accept `#hex` (3 or 6 digits), `rgb()` / `rgba()`, and `hsl()`. A  truecolor swatch sits beside each field.
- Preview sentence and font family. `Ctrl+T` cycles Roboto, Open Sans, Lato, Montserrat, and Poppins. The font field also accepts a typed family.
- Fix pane. It searches OKLab lightness for a nearby pair that passes the current WCAG level and APCA bar, then lets you nudge lightness or hue.
- Browser preview (`Ctrl+O`) writes `/tmp/dd_wcag_preview.html` and opens it. That page is the true CSS pixel size. The terminal preview is marked approximate. The window draws the sample at the real size, weight, and italic.
- Palette builder. Required bases are Primary, Secondary, and Tertiary. Support is optional and defaults to `#46BE8C` when the field is empty. Generate writes a WCAG-gated `_palette.scss` and a Penpot/Figma `_palette.tokens.json`. Text tokens are fixed (`#1C1E21` on light surfaces, `#F5F6F7` on dark). Generate never edits them.
- Directed 5×5 pair matrix. Rows are the text role and columns are the surface role: Primary, Secondary, Tertiary, Text, Support. The diagonal is not a pair. Glyphs are `✓` when WCAG and APCA both pass, `~` when they disagree, and `✗` when both fail. Enter or a double-click (420ms, same cell) opens Fix for that pair.
- Toasts for 5 seconds. Errors, saves, and theme health use the toast. Esc does not quit.

Defaults are black `#000000` on white `#FFFFFF`, 16px, weight 400, WCAG AA, APCA Lc75, preview text “The quick brown fox jumps over the lazy dog.”, and font Roboto. Palette bases start at Primary `#88D9F7`, Secondary `#FFCA76`, Tertiary `#F98971`, Support `#46BE8C`. The matrix starts on Text on Primary.

## Command line

No arguments opens Contrast. Color flags fill the UI before the first paint. `dd_wcag` opens the terminal. `dd_wcag-gui` opens the window. The flags are the same. `check` and `palette` print and exit from either binary.

```bash
dd_wcag --fg '#1C1E21' --bg '#F5F6F7' --size 18 --weight 700 --wcag aaa --apca 90
dd_wcag --tab palette --primary '#0B3D91' --secondary '#C45508' --tertiary '#1F7A4D' --support '#46BE8C' --generate

dd_wcag check --fg '#1C1E21' --bg '#FFFFFF' --json
dd_wcag check --pairs pairs.txt
dd_wcag check --pairs -   # fg bg pairs on stdin

dd_wcag palette --primary '#0B3D91' --secondary '#C45508' --tertiary '#1F7A4D' --out ./theme
dd_wcag palette --primary '#0B3D91' --secondary '#C45508' --tertiary '#1F7A4D' --stdout scss
```

`check` prints the WCAG ratio, AA and AAA, and APCA Lc. Exit 0 when the pair passes the selected WCAG level (default AA) and APCA bar (default Lc75). Exit 1 when it fails. Exit 2 when an argument or a color is invalid. `--size` and `--weight` use the same large-text rule as the UI. A pairs file is one `fg bg` pair per line. `--json` prints one object, or an array when there is more than one pair.

`palette` writes `_palette.scss` and `_palette.tokens.json`. Primary, Secondary, and Tertiary are required. Support defaults to `#46BE8C`. Exit 1 when a blocking contrast check fails. The files are still written. `--stdout scss` or `--stdout json` prints that artifact instead. `--json` prints the compliance summary.

Launch flags: `--tab contrast|palette`, `--fg`, `--bg`, `--primary`, `--secondary`, `--tertiary`, `--support`, `--generate`, `--size` (6–120), `--weight` (100–900, step 100), `--wcag aa|aaa`, `--apca 45|60|75|90`. The tab is inferred from which color flags you pass when `--tab` is omitted.

`dd_wcag --help` lists every flag. The long-form contract is SPEC.md §25.

## Everyday keys

These keys are the same in the terminal and the window. A text field swallows bare characters, including `1` and `2`. Esc blurs the field first. Bare `q` does not quit. Esc does not quit.

| Key | Action |
|---|---|
| `1` / `2` | Contrast / Palette, when no field is being typed in |
| `Tab` / `Shift+Tab` | Next / previous control. An invalid color blocks the move and toasts |
| `Up` / `Down` | Step the focused size (1px), weight (100), or style chip. Move a matrix row, a palette role, or between the Fix send rows |
| `Shift+Up` / `Shift+Down` | Larger step: size 4px, weight 200 |
| `Ctrl+Up` / `Ctrl+Down` | Step the focused control, including the Fix gauge by 0.02 (with Shift, 0.10) |
| `[` / `]` | Nudge OKLab lightness of the focused color by 0.02 (Shift: 0.10) |
| `{` / `}` | Nudge hue of the focused color by 10° (Shift: 30°) |
| `Ctrl+B` | Toggle weight 400 / 700 |
| `Ctrl+S` | Contrast: cycle style. Palette: save `_palette.scss` and `_palette.tokens.json` |
| `Ctrl+C` | Contrast: copy the focused hex. Palette: copy the SCSS |
| `Ctrl+V` | Paste into the focused field. A valid color replaces the field |
| `Ctrl+T` | Cycle the preview font |
| `Ctrl+F` | Toggle Fix |
| `Ctrl+N` | Next Fix candidate |
| `Ctrl+G` | Switch to Palette, generate, and focus the matrix |
| `Ctrl+O` | Write and open the browser preview |
| `Enter` | Activate the focused control (apply, generate, edit a role, open Fix on a matrix cell, newline in the preview) |
| `Space` | Swap the pair, unless style, a target, or the matrix is focused |
| `Home` / `End` / `Delete` | Caret motion in a text field |
| `PageUp` / `PageDown` | Scroll the contrast column (Shift scrolls farther), or the palette detail |
| `P` `S` `T` `U` | While Fix is focused, send the candidate to Primary, Secondary, Tertiary, or Support |
| `F1` / `F2` / `Ctrl+/` / `Ctrl+,` | Settings. Themes is first, Help is last. The same keys close it |
| `Esc` | Close Fix, cancel a palette edit, or blur a field. In Settings, close and revert the theme preview |
| `Ctrl+Q` | Quit |

In the palette matrix, arrows move the selection and skip the diagonal. Space swaps the text and surface axes. Enter opens Fix. Apply writes the candidate back to the brand base. It never writes the fixed Text token. Text on a surface changes the surface color only.

Mouse matches the focused control. Click focuses. Click a swatch to focus that color. Shift+click a swatch or a palette role copies that hex. Click a style chip, a target, a matrix cell, a pair-list row, or a palette role. Double-click a matrix cell or a pair-list row (420ms, same cell) to open Fix. Shift+click a size or weight stepper uses the larger step. Scroll the contrast column and the palette detail. Hover does not move focus. Click a toast to dismiss it.

The window adds a Settings page named Window (Comfortable / Compact density, and vim `j`/`k` inside Settings). That page is not in the terminal. The terminal Settings sheet is Themes and Help.

## Palette export

Generate requires Primary, Secondary, and Tertiary to parse. An empty Support field becomes `#46BE8C`. After Generate, the matrix selects the worst failing pair, or Text on Primary when every pair passes.

Blocking checks use 4.5:1 for text on surfaces and for action text, and 3:1 for borders and disabled text. Support-border and focus checks are advisory at 3:1. Save and copy refuse while a blocking check fails, and they toast the first failure. A passing export writes both files. The tokens JSON sits next to the SCSS (`theme.tokens.json` when the SCSS is `theme.scss`).

## Theme

Lookup order: `./dd_wcag_theme.yml`, then `~/.config/ldnddev/dd_wcag_theme.yml`, then built-in defaults. The file needs `version: 1`. Chrome colors come from that theme. The pair you are checking uses your colors in the swatches and the preview only.

F1 opens Settings on Themes. When Omarchy themes are installed, that page lists them. Otherwise it opens the color editor. `t` toggles the save target (Global / Local). `y` or `s` writes the theme YAML. Esc and closing Settings revert the preview to the palette from when the sheet opened. Window preferences (density, vim keys) live in `~/.config/ldnddev/dd_wcag/gui.json`. They are not written into the theme file.

## Docs

- [Tutorial](https://ldnddev.github.io/dd_wcag/) — user guide with screenshots (GitHub Pages)
- [SPEC.md](SPEC.md) — product spec
- [THEME_STRUCTURE_STANDARD.md](THEME_STRUCTURE_STANDARD.md) — shared ldnddev theme schema
