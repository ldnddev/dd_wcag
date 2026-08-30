# dd_wcag — Product spec

**Single source of truth** for layout, mouse, keyboard, contrast math, palette generation, theming, and implementation order.

Supersedes and replaces `layout-plan.md`, `mouse-support-plan.md`, and `UI-THEME-BUILDER.md`.

Chrome theme **tokens** (the YAML schema) still live in `THEME_STRUCTURE_STANDARD.md` — that file is the shared ldnddev TUI standard, not an app spec. Do not duplicate or fork it here. Add new chrome tokens there first, then to `THEME_KEYS` / `Theme`.

**Status:** spec for the `layout-mouse` rewrite. Current `src/` is still the five-tab app until that branch lands.

Related split: **~60% Contrast pair checker**, **~40% Palette check/generator** (theme builder + pair matrix).

---

## 0. Locked decisions

| Topic | Decision |
|---|---|
| Product | Contrast pair checker **and** SCSS theme builder. Palette does not become a generic 5-role playground. |
| Text roles | **Not editable.** Fixed tokens from §8. Generation must never modify them. |
| Palette inputs | Required: Primary, Secondary, Tertiary. Optional: Support / Utility. |
| APCA header | User **pass bar** (`Lc45 / 60 / 75 / 90`) for scores + matrix glyphs. Size/weight still drive WCAG large-text and an **advisory** lookup line. See §7. |
| WCAG header | `AA` or `AAA`. Changes which WCAG column is the official pass/fail. |
| Weight | Explicit `100..=900` (step 100). Style chips are presets. Italic is visual only; it does not change math. |
| Keep | Browser preview (`Ctrl+O`), FontFamily, editable PreviewText, `_palette.scss` save/copy, F1 help, F2 theme info, toasts (5s), YAML theme file. |
| Quit / help | `Ctrl+Q` quits. `F1` help. `F2` theme source/status. **Not** bare `q`. **Not** `?` for help. `Esc` never quits. |
| Steppers | Focus the control, then the same keys: `Ctrl+Up` / `Ctrl+Down`. Size, weight, and Fix OKLab gauges all use this. Unfocused steppers ignore those keys. |
| Fix lightness | **OKLab L** `0..=1`. Default axis is FG/text; BG axis only if the user focuses the BG gauge. `keep_hue` default true. |
| Narrow fields | **One row** per field (not 3-line Blocks). |
| Geometry | `render` returns a `LayoutMap`. Store it on `App` as last frame. Paint and hit-test use the same splits. Do not re-split in event handlers. |
| Field mouse | Click-to-focus + click-to-caret. **No** selection range in v1. |
| Double-click | 420ms, same cell. Matrix cell / generated token: open Fix. Color field: ignored in v1 (no select-all). Palette role row: begin/commit edit. |
| Shift+click swatch | Copy that color’s hex (same payload as `c` on a focused color). |
| Right-click menu | **Deferred.** Paste / random / darken / lighten / swap stay on keys. |
| Apply invalid color | Same as Tab: block focus move, toast the parse error. |
| Shell heights | Header `Length(3)`, footer `Length(1)`, body `Fill(1)`. |
| Chrome colors | Always from `Theme` / YAML. Never hardcode after load. User FG/BG/roles use the user’s colors in previews and swatches only. |

---

## 1. Goal

A keyboard-first TUI with full mouse support that:

1. Checks one foreground/background pair against **WCAG 2.x** and **APCA** for a given font size, weight, and style.
2. Takes brand bases (Primary / Secondary / Tertiary, optional Support), generates the derived `_palette.scss` theme (light + dark), and audits it against **fixed** text roles.
3. Shows a directed pair matrix of the five conceptual colors (three bases + Support + fixed Text) so APCA/WCAG pair failures are visible at a glance.
4. Offers a Fix pane that previews a nearby passing candidate and lets the user nudge OKLab lightness.

Terminal rendering cannot show real CSS px or a real font family. Label TUI previews `approx`. Contrast math must still use the numeric size/weight the user entered. `Ctrl+O` remains the only true CSS preview (`/tmp/dd_wcag_preview.html`).

---

## 2. Non-negotiable UX rules

- Always show **both** WCAG ratio and APCA Lc.
- APCA is **directional**. `Text on Primary` ≠ `Primary on Text`. Matrix axes are `text \ surface`.
- Font size and weight are **shared** across Contrast and Palette. APCA advisory lookup is invalid without them.
- WCAG “large text” = 18px+ **or** 14px+ and (bold **or** weight ≥ 700).
- Color fields accept `#hex`, `rgb()` / `rgba()`, `hsl()`, and show a 2-cell truecolor swatch (`Color::Rgb`).
- Style control is a **chip row** of presets (Regular / Bold / Italic / Bold+Italic) plus a **weight stepper 100–900**.
- One function owns all `Rect` splits per frame. `LayoutMap` is the hit-test geometry.
- Mouse click sets keyboard focus to the same `FocusId`. Hover never steals focus.
- Invalid color input blocks focus movement and toasts. No blocking error modals for non-interactive feedback.
- I/O stays out of key/mouse handlers: they return `KeyEffects` (`quit`, `sync_preview`, `open_preview`, `save_palette`, `copy_palette`, `copy_hex`).

---

## 3. Stack, startup, event loop

```
ratatui + crossterm + palette + anyhow
```

Setup: `enable_raw_mode`, `EnterAlternateScreen`, `EnableMouseCapture`.

Teardown: `DisableMouseCapture`, `LeaveAlternateScreen`, `disable_raw_mode`.

```rust
loop {
    terminal.draw(|frame| {
        app.layout = render(frame, &app);
    })?;
    // poll + tick (250ms) so toasts expire via App::expire_notification
    match event::read()? {
        Event::Key(k) if k.kind == KeyEventKind::Press => {
            let effects = handle_key_event(&mut app, k);
            dispatch_effects(&mut app, effects);
        }
        Event::Mouse(m) => {
            let effects = handle_mouse_event(&mut app, m);
            dispatch_effects(&mut app, effects);
        }
        Event::Resize(_, _) => {}
        _ => {}
    }
    if effects_or_flag_quit { break; }
}
```

Truecolor: `Color::Rgb`. If the terminal lacks truecolor, fall back to indexed color and mark the preview `approx`.

---

## 4. App state (target)

```rust
struct App {
    mode: Mode,                 // Contrast | Palette
    fix_open: bool,
    focus: FocusId,
    editing: bool,              // hex / preview text / font family field is in edit mode
    hovered: Option<Hit>,
    drag: Drag,                 // None | NudgeFg | NudgeBg | Scrollbar
    layout: LayoutMap,
    contrast: ContrastState,    // fg, bg, preview_text, font_family
    palette: PaletteState,      // theme-builder state + matrix selection
    fix: FixState,
    size_px: u16,               // shared, default 16, clamp 6..=120
    weight: u16,                // shared, 100..=900 step 100, default 400
    italic: bool,
    targets: Targets,           // wcag AA|AAA, apca Lc45|60|75|90
    help_open: bool,            // F1
    theme_debug_open: bool,     // F2
    theme: Theme,
    theme_source: ThemeSource,
    error: Option<String>,
    status: Option<String>,
    notification_updated_at: Option<Instant>,
    mouse_pos: Option<(u16, u16)>,
    scrollbar_dragging: bool,
    last_mouse_click_pos: Option<(u16, u16, Instant)>,
    copied_palette: Option<String>,
}

enum Mode { Contrast, Palette }

struct Targets {
    wcag: WcagLevel,            // AA | AAA
    apca: ApcaTarget,           // Lc45 | Lc60 | Lc75 | Lc90
}

enum FocusId {
    FgHex, BgHex, Size, Weight, Style, PreviewText, FontFamily,
    Swap, CopyHex, FixBtn, OpenPreview,
    Role(usize),                // 0..=2 required, 3 Support; Text is not a FocusId
    Generate, Matrix, Detail,
    NudgeFg, NudgeBg, ApplyFix, NextFix, CloseFix,
    Tabs, TargetWcag, TargetApca,
}

enum Drag { None, NudgeFg, NudgeBg, Scrollbar }
```

`fix_open` is independent of tab. `Ctrl+F` toggles it. Opening Fix from a failing matrix cell or a generated-token row loads that pair into `FixState`.

Toast TTL remains 5 seconds.

---

## 5. Breakpoints

```rust
enum Breakpoint { Wide, Medium, Narrow }

fn breakpoint(area: Rect) -> Breakpoint {
    match (area.width, area.height) {
        (w, h) if w >= 120 && h >= 28 => Breakpoint::Wide,
        (w, h) if w >= 100 && h >= 24 => Breakpoint::Medium,
        _ => Breakpoint::Narrow,
    }
}
```

| Breakpoint | Contrast | Palette | Fix |
|---|---|---|---|
| Wide (≥120×28) | A: form 34 cols \| preview+scores | D: roles 28 cols \| matrix+detail | bottom strip, 7 rows |
| Medium (≥100×24) | A: form 30 cols \| preview+scores | D: roles 24 cols \| matrix+detail | bottom strip, 6 rows |
| Narrow | E stacked | stacked roles + pairs + detail | overlay ~80% height, click-outside closes |

If `fix_open` on Wide/Medium: split `body` **vertically first** (`Fill(1)` + `Length(6|7)`), then split the remaining `main` as the tab content.

If `fix_open` on Narrow: do **not** steal a strip. Centered overlay with `Clear` + bordered `Block`. Esc or click outside closes it.

---

## 6. Shell (every frame)

```
┌ header  Length(3) ────────────────────────────────────────────┐
│ Contrast │ Palette              WCAG AA ▾     APCA Lc75 ▾     │
├ body  Fill(1) ────────────────────────────────────────────────┤
│ tab content  (+ optional Fix strip / overlay)                 │
├ footer  Length(1) ────────────────────────────────────────────┘
│ width-adaptive key hints                                      │
```

```rust
let [header, body, footer] = Layout::vertical([
    Constraint::Length(3),
    Constraint::Fill(1),
    Constraint::Length(1),
]).areas(frame.area());

let [tabs, _spacer, targets] = Layout::horizontal([
    Constraint::Length(28),
    Constraint::Fill(1),
    Constraint::Length(36),
]).areas(header);

let [target_wcag, target_apca] = Layout::horizontal([
    Constraint::Fill(1),
    Constraint::Fill(1),
]).areas(targets);
```

Store clickable halves: `tabs_contrast`, `tabs_palette`, `target_wcag`, `target_apca`. If using `Tabs`, still store **per-label** rects.

Header is **interactive** (tabs + targets). No branding tagline in v1.

Footer is help text, not a hit target (except F1 remains a key). Width-adaptive:

- `< 75`: `F1:Help  F2:Theme  Tab:Focus  f:Fix  g:Gen  Ctrl+O:Web  Ctrl+Q:Quit`
- mid: slightly longer
- `>= 110`: append `(mouse: click/scroll/drag)`

Theme source is **not** persistent in the footer. It appears in F2 and as a startup toast.

---

## 7. Contrast math (UI contract)

Pure module, no widgets. Fix pane, scores, matrix, palette compliance, and web preview all call the same functions.

### WCAG 2.x

Relative luminance + contrast ratio (1..=21).

| | AA | AAA |
|---|---|---|
| Normal text | 4.5:1 | 7:1 |
| Large text (18px+ or 14px+ and weight≥700) | 3:1 | 4.5:1 |
| UI / non-text | 3:1 | 3:1 |

Official pass/fail for the **current pair** uses the header WCAG level plus large-text classification from `size_px` + `weight`.

### APCA

Signed Lc (existing `apca_lc`). Display polarity from the sign of Lc.

**Official pass/fail** (scores glyphs, matrix `✓/~ /✗`, Fix “did we pass”): `|Lc| >=` header APCA target (45 / 60 / 75 / 90).

**Advisory lookup** (not a second fail): a table of recommended `|Lc|` for `(size_px, weight)`. Show as `lookup Lc{n} for {size}px/{weight}`. If the header bar is **stricter** than lookup, say so; if **looser**, warn that body text may still need the lookup value (`body ~` / +15 note when relevant).

This is intentional: the header is “what I am auditing to”; size/weight tell you whether that bar matches typical body/UI use.

Weight 100–900 lookup (minimum viable, extend later if needed):

| size | 100–300 | 400–500 | 600–900 |
|---|---|---|---|
| ≤12 | 90 | 90 / 75 if ≥700 | 75 |
| 13–18 | 75 | 75 / 60 if ≥700 | 60 |
| 19–24 | 60 | 60 / 45 if ≥700 | 45 |
| ≥25 | 45 | 45 / 30 if ≥700 | 30 |

Italic does not change this table.

### Conversions

No Conversions tab. The Contrast left column includes a Conversions box under the form (and above stacked WCAG / APCA) showing Hex / RGB / HSL for both foreground and background.

---

## 8. Palette — theme builder (load-bearing)

This section is the former `UI-THEME-BUILDER.md`. Naming, grouping, compliance, and export rules are **not** optional.

### 8.1 Inputs

User-provided brand bases (same parse rules as Contrast: `#rgb` / `#rrggbb`, `rgb()` / `rgba()`, `hsl()`):

```scss
$base_primary: rgba(136, 217, 247, 1);
$base_secondary: rgba(255, 202, 118, 1);
$base_tertiary: rgba(249, 137, 113, 1);
$base_support: rgba(70, 190, 140, 1);
```

- Primary, Secondary, Tertiary are **required**.
- Support / Utility is **optional**; if omitted, use `#46BE8C`.
- Alpha may be accepted in `rgba()`; generated variables are opaque `rgba(r, g, b, 1)`.
- Invalid bases block generation and toast the field-specific parse error.

### 8.2 Fixed text colors (never edited, never generated)

```scss
$c_text_primary: rgba(28, 30, 33, 1);
$c_text_primary--dark: rgba(245, 246, 247, 1);
$c_text_secondary: rgba(90, 95, 102, 1);
$c_text_secondary--dark: rgba(158, 163, 170, 1);
$c_text_disabled: rgba(160, 164, 168, 1);
$c_text_disabled--dark: rgba(90, 95, 102, 1);
$c_text_inverse: rgba(249, 250, 251, 1);
$c_text_inverse--dark: rgba(15, 17, 20, 1);
```

- The builder may **choose** which fixed text token sits on a generated surface.
- It must not invent new text colors to force compliance.
- If no fixed text token passes, **adjust the generated surface**, not the text color.
- Disabled states use disabled text roles and are reported as advisory.

In the UI, Text is a **read-only row** (name + swatch + hex) so the matrix has a text axis. Clicking it selects the matrix “Text” axis; it does not enter edit mode.

### 8.3 Generated variable groups (order for `_palette.scss`)

1. Primary (`$c_primary_default`, `_strong`, `_subtle`, each with `--dark`)
2. Secondary (same)
3. Tertiary (same)
4. Primary Action — for each of default / hover / pressed / disabled: `surface`, `text`, `border`, each with `--dark`
5. Secondary Action (same)
6. Tertiary Action (same)
7. Semantic (success, warning, error, info — built-in defaults)
8. Text Roles (the fixed tokens above)
9. Neutrals (built-in defaults)
10. Support / Utility (`$c_support_overlay`, `_border`, `_focus`, each with `--dark`)

Example primary action default:

```scss
$c_primary_action_default_surface
$c_primary_action_default_surface--dark
$c_primary_action_default_text
$c_primary_action_default_text--dark
$c_primary_action_default_border
$c_primary_action_default_border--dark
```

Derivation (predictable, accessible):

- `default` stays visually close to the base.
- `strong` for emphasis / high-contrast text-or-icon use.
- `subtle` for light fills, badges, quiet surfaces.
- `--dark` is tuned for dark UI backgrounds, not mechanically inverted.
- Action `hover_surface` ≠ `default_surface`; `pressed_surface` ≠ `hover_surface`.
- Disabled should look inactive while remaining legible. State must not rely on color alone; notes say when border/surface carries the state.

### 8.4 Compliance testing (generated tokens)

Minimum WCAG (export gating; independent of the header AA/AAA **display** target — export stays AA as today unless we later add an export-level control):

- Normal text: `>= 4.5`
- Large text / UI labels: `>= 3.0`
- Non-text borders/focus vs adjacent surfaces: `>= 3.0`
- Disabled: reported, `disabled advisory`, does not block export

Required checks:

For each family `primary` / `secondary` / `tertiary`, test `default` / `strong` / `subtle` against `$c_text_primary`, `_secondary`, `_inverse` and their `--dark` variants. Recommend the best passing fixed text token. If none pass, fail that surface.

For each action state: text vs surface; border vs surface; light action surfaces vs light neutrals; dark vs dark.

For support: `support_border` and `support_focus` vs light/dark neutrals; `support_overlay` vs fixed text if used as a readable surface.

### 8.5 Export (`Ctrl+S` / `Ctrl+C`)

Same payload. Default path `./_palette.scss`.

Block when:

- No generation yet → `Generate palette before saving|copying`
- Required bases invalid → field-specific parse error
- Blocking compliance failures → first failing group + remaining count

Allow with warning when only advisory (disabled / decorative accent) failures exist.

`rgba(r, g, b, 1)` only. Preserve group order in §8.3. Short comments only for usage or compliance notes.

Include a concise WCAG notes section: recommended text token per default/strong/subtle; failing combos; suggested use; action-state notes.

---

## 9. Contrast tab layout

### 9.1 Wide / Medium (Layout A)

One-row fields. Form width 34 (Wide) or 30 (Medium).

```
body
┌ INPUT ────────────────┬ PREVIEW + SCORES ─────────────────────┐
│ FG  [#2563EB] ██      │ ┌ preview ──────────────────────────┐ │
│ BG  [#0F172A] ██      │ │ {preview_text}                    │ │
│ Size [16] −+  Wt 400  │ │ Aa  Heading  [Button]             │ │
│ ●Reg ○Bld ○Itl ○B+I   │ └───────────────────────────────────┘ │
│ Text [The quick…]     │ WCAG 4.61  AA✓ AAA✗  UI✓              │
│ Font [Roboto    ]     │ APCA Lc 62  bar Lc75 ✓  lookup Lc75   │
│ [Swap] [Copy] [Fix]   │ approx · Roboto 16px/400 · dark on lt │
│ [Web]                 │ hex / rgb / hsl of focused color      │
└───────────────────────┴───────────────────────────────────────┘
```

Vertical form rows (all `Length(1)` content + optional section padding; do **not** use `Length(3)` Blocks per field):

1. FG label + input + swatch
2. BG label + input + swatch
3. Size input + −/+  and Weight input + −/+
4. Four style chips
5. PreviewText
6. FontFamily
7. Swap | Copy | Fix | Web (`Ctrl+O`)

Preview content (minimum):

- Heading line (bold if weight ≥ 700)
- Body: current `preview_text` (wrap)
- UI line: `[ Action ]`
- Caption: `approx · {family} {size}px/{weight}` plus polarity `dark on light` | `light on dark`

TUI cannot honor `font_family`; the caption and web preview do.

Preview chrome uses the same `body_background` as the left column so the title stays readable. The FG/BG sample is inset ~12px (2 columns / 1 row inside the border); sample text has one extra cell of padding.

When the left column’s natural height (form 23 + conversions 6 + WCAG/APCA tables) exceeds the viewport, the column **scrolls as a unit** instead of shrinking field blocks. Mouse wheel over the left column (except size/weight, which still step), PageUp/PageDown, the edge scrollbar, and Tab (keeps the focused field in view).

Scores (minimum), stacked under Conversions in the left column:

**WCAG** — current size/weight result (`{size}px {normal|bold} | ratio n.nn`, `needs >= {header+large-text threshold} | PASS/FAIL`); AA normal/large; AAA normal/large; UI 3:1; quick-reference table at 12/14/16/18px using the header level + large-text at that size. Pass/fail uses header level + green/red theme tokens.

**APCA** — current size/weight result (`{size}px {normal|bold} | Lc n.nn`, `needs >= {header bar} | PASS/FAIL`); advisory lookup line (`lookup Lc{n} · polarity`); quick-reference table at 12/14/16/18/24px using lookup thresholds for the current weight. Official pass/fail is the header bar.

### 9.2 Narrow (Layout E)

```
form     Length(7)   one-row FG/BG/size+wt/style/text/font/actions (scroll or clip with Min)
preview  Min(5)
scores   Length(4)
```

If 7 rows do not fit, clip actions into a second line or drop labels to 2-char (`FG` `BG`). Keep size and weight steppers. Collapse style to a **single cycling chip** if four chips do not fit.

---

## 10. Palette tab layout

### 10.1 Wide / Medium (Layout D)

```
┌ ROLES (28 or 24) ─────┬ MATRIX (text \ surface) ──────────────┐
│ ● Primary    #88D9F7  │      Pri  Sec  Ter  Text Sup          │
│   Secondary  #FFCA76  │ Pri   ·    ✗    ✗    ✓    ✓           │
│   Tertiary   #F98971  │ ...                                   │
│   Support    #46BE8C  │                                       │
│   Text       #1C1E21  │ Selected: Secondary on Primary        │
│   (read-only)         │ WCAG 2.14 ✗   APCA Lc 28 ✗            │
│ Size 16  Wt 400       ├───────────────────────────────────────┤
│ [Generate] [Web]      │ Generated tokens + compliance (scroll)│
└───────────────────────┴───────────────────────────────────────┘
```

Roles column:

- Four editable rows: Primary*, Secondary*, Tertiary*, Support (optional).
- Fifth row: **Text**, read-only, shows `$c_text_primary` (light) swatch. Not in `FocusId` as an editor. Used as matrix axis.
- Shared size/weight (same `App` fields) + `[Generate]` + `[Web]`.

Matrix:

- Rows = text role, columns = surface role.
- Five axes: Primary, Secondary, Tertiary, Text (fixed `$c_text_primary` or `_--dark` chosen by polarity), Support.
- Diagonal `·`, not selectable.
- Glyph: `✓` both official WCAG-header and APCA-bar pass; `~` they disagree; `✗` fail.
- Two lines per cell if height allows: ratio and Lc.
- Arrows move selection, skip diagonal.
- `Enter` or double-click (420ms) opens Fix for that pair.
- Detail under the matrix: selected pair preview + numbers, then generated token list + compliance for the selected **brand family** when `generated` is `Some`. Scrollbar when overflow.

Generate (`Ctrl+G` or button):

1. Parse required bases (block + toast if invalid).
2. Derive the full token set in §8.
3. Run compliance checks.
4. Refresh the matrix from the five conceptual colors (bases + support + fixed text), **not** from every derived token.
5. Select the worst failing **matrix** pair (largest shortfall vs current header targets). If none, select `Text on Primary`.
6. Do not randomize colors. Do not rewrite text tokens.

Do **not** bind Shift+F / Shift+B (or F-then-G) to send a palette color to Contrast FG/BG. Contrast pair fields are not visible on this tab.

### 10.2 Narrow Palette

```
roles     Length(6)   four inputs + read-only Text, compact
pairs     Fill(1)     "Sec on Pri   2.14✗  Lc28✗"
detail    Length(6)   preview + numbers + tokens
```

`List` + `ListState` + `Scrollbar`. Wheel scrolls the focused list (roles vs pairs vs detail).

---

## 11. Fix pane (Layout F)

### 11.1 Wide / Medium — bottom strip

```
┌ NOW ──────────────┬ FIXED ────────────┬ NUDGE (28 cols) ──────┐
│ current swatch    │ candidate swatch  │ FG L ──●────          │
│ sample text       │ sample text       │ BG L ────●──          │
│ WCAG/APCA fail    │ WCAG/APCA pass    │ [Apply] [Next] [Close]│
└───────────────────┴───────────────────┴───────────────────────┘
```

`LineGauge` sliders. Gauge `Rect` is the drag target.

Apply writes the candidate back into the active pair: Contrast FG/BG, or the two palette **bases** involved (never into a fixed text token — if the pair is `Text on Primary`, only Primary may change). Next generates another nearby candidate (OKLab L first; hue only if `keep_hue` is off or lightness cannot pass).

FIXED FG and BG can be sent independently to Palette **Primary / Secondary / Tertiary / Support** (`FG→` / `BG→` chips, or `p` / `s` / `t` / `u` for the focused axis). That writes the hex into the role, clears generated output, and does not change Contrast unless Apply was used.

### 11.2 Narrow — overlay

Centered ~80% box. Render order: tab content, `Clear` overlay, Fix stacked: NOW, FIXED, sliders, buttons. Click outside `fix_area` closes. Esc closes Fix (or help/theme debug if those are on top).

### 11.3 Candidate logic (minimum)

Keep hue. Search OKLab L on the focused axis (default FG/text) until header WCAG **and** header APCA bar pass, or bounds hit. If one metric still fails, mark `~` and still offer Apply. Do not block Apply on perfect both.

---

## 12. LayoutMap and hit testing

Rebuild every draw. Mouse uses the **last frame**.

```rust
struct LayoutMap {
    breakpoint: Breakpoint,
    tabs_contrast: Rect,
    tabs_palette: Rect,
    target_wcag: Rect,
    target_apca: Rect,
    footer: Rect,
    body: Rect,
    // contrast
    fg_input: Rect, fg_swatch: Rect,
    bg_input: Rect, bg_swatch: Rect,
    size_input: Rect, size_dec: Rect, size_inc: Rect,
    weight_input: Rect, weight_dec: Rect, weight_inc: Rect,
    style_btns: [Rect; 4],
    preview_text: Rect,
    font_family: Rect,
    swap_btn: Rect, copy_btn: Rect, fix_btn: Rect, web_btn: Rect,
    preview: Rect,
    scores_wcag: Rect, scores_apca: Rect,
    // palette
    role_rows: [Rect; 4],      // P/S/T/Support
    text_row: Rect,            // read-only
    generate_btn: Rect,
    matrix_area: Rect,
    matrix_cells: [[Rect; 5]; 5],
    pair_list: Rect,
    detail: Rect,
    detail_scrollbar: Rect,
    // fix
    fix_area: Rect, now_area: Rect, fixed_area: Rect,
    nudge_fg: Rect, nudge_bg: Rect,
    apply_btn: Rect, next_btn: Rect, close_fix: Rect,
    // overlays
    popup_area: Option<Rect>,
    toast_area: Option<Rect>,
}
```

`hit(col, row)` order:

1. Toast
2. F1 / F2 popup (inside = no-op except wheel if needed; outside = close)
3. Fix overlay controls if Narrow + `fix_open`
4. Fix strip controls if Wide/Medium + `fix_open`
5. Chrome tabs/targets
6. Active tab controls
7. `FixOutside` if Narrow + `fix_open` + point in `body` but not `fix_area`

`Rect::contains`. Matrix cell splitter must match the painted `Table` (header row + 5 body rows, gutter + 5 role columns). Diagonal cells exist in the map; `dispatch_click` ignores them. Text-role editor clicks on `text_row` only select the Text matrix axis.

---

## 13. Mouse

| Action | Target | Behavior |
|---|---|---|
| Left click tab | `tabs_*` | switch `mode`; auto-apply focused field first |
| Left click target | WCAG / APCA | cycle AA↔AAA / Lc45→60→75→90→45 |
| Left click input | color / size / weight / preview / font | focus + caret at click column |
| Left click swatch | FG/BG/role | focus that color |
| Shift+click swatch | same | copy hex (`copy_hex`) |
| Left click − / + | size / weight steppers | size ±1 (Shift ±4); weight ±100 |
| Left click style chip | `style_btns[i]` | apply preset (see §15) |
| Left click Swap / Copy / Fix / Generate / Web | buttons | same as keys |
| Left click role row | `role_rows[i]` | select role, focus hex |
| Double-click role row | same, 420ms | begin or commit edit |
| Left click Text row | `text_row` | select Text axis only |
| Left click matrix cell | off-diagonal | select pair; load detail |
| Double-click matrix cell | off-diagonal | open Fix for that pair |
| Drag nudge gauge | `nudge_fg` / `nudge_bg` | map `column` → OKLab L 0..=1 |
| Wheel over size / weight | those rects | same as `Ctrl+Up`/`Down` on that focused control (also focuses it) |
| Wheel over nudge | gauge | ±0.02 L (same as `Ctrl+Up`/`Down` when that gauge is focused) |
| Wheel over pair list / matrix / detail | list/table/detail | scroll; Shift = faster (3 / 8) |
| Wheel over Contrast left column | form / conversions / scores / scrollbar | scroll the column; Shift = faster. Size/weight still step. |
| Click toast | toast | dismiss |
| Click outside F1/F2 | not `popup_area` | close |
| Click outside Fix overlay | body \ fix_area | close Fix (Narrow only) |
| Mouse move | any | `hovered`; do **not** change focus |
| Mouse up | any | `drag = None`, `scrollbar_dragging = false` |
| Scrollbar drag | detail track | proportional scroll; continues if pointer leaves the column while held |
| Hover scrollbar | track | `scrollbar_hover` token |

**Not v1:** drag-select text, double-click select-all, right-click context menu, click Preview body to toggle bold (use style chips / `Ctrl+S` on Contrast / weight).

Leaving a color field (click another control) runs the same apply/parse path as Tab.

---

## 14. Keyboard

| Key | Action |
|---|---|
| `1` / `2` | Contrast / Palette. While a text field or palette color is being edited, these type the digits. |
| `Tab` / `Shift+Tab` | next / prev `FocusId` in the active mode (and Fix controls if open); auto-apply; block on invalid |
| `Space` | Contrast, not editing: swap FG/BG. Palette, matrix selected: swap text ↔ surface axes |
| `[` `]` | OKLab L ±0.02 on the focused **color** (FG / BG / role), including when Fix is closed |
| `{` `}` | hue ± on the focused color |
| `Ctrl+Up` / `Ctrl+Down` | **shared stepper**, only for the focused control (see below). No global size/weight shortcut. |
| `Ctrl+S` | Contrast: cycle style presets. Palette: save `_palette.scss` (gated) |
| `Ctrl+F` | toggle Fix |
| `Ctrl+G` | generate + audit palette (switches to Palette if needed) |
| `Ctrl+C` | Palette: copy `_palette.scss` (gated). Else: copy focused hex |
| `Ctrl+B` | toggle bold preset (weight 400↔700, keep italic) |
| `Ctrl+T` | cycle font-family presets (Roboto, Open Sans, Lato, Montserrat, Poppins) |
| `Ctrl+O` | open web preview |
| `Enter` | commit field; or apply Fix candidate; or open Fix from matrix; or newline in PreviewText while editing it |
| `Ctrl+N` | next Fix suggestion (only if Fix open) |
| `Esc` | blur edit; else close Fix; else close F1/F2; else no-op (does **not** quit) |
| `F1` | toggle help |
| `F2` | toggle theme debug (source, version, path, status) |
| `Ctrl+Q` | quit |
| `Up` / `Down` | Palette: before generate, move role selection; after generate, if focus is Detail/Matrix, scroll or move cell |

While a text/hex field is being edited, most keys go to the input. `Esc` / `Enter` leave edit mode (`Enter` in PreviewText inserts newline and stays editing). `Tab` commits and moves. `Ctrl+Up` / `Ctrl+Down` on Size, Weight, or a Fix gauge still step even if that numeric field is in edit mode.

### Shared stepper keys

One chord, meaning depends on `focus`:

| Focus | `Ctrl+Up` | `Ctrl+Down` | Shift held |
|---|---|---|---|
| Size | `size_px + 1` | `size_px - 1` | ±4, clamp `6..=120` |
| Weight | `weight + 100` | `weight - 100` | ±200, clamp `100..=900` |
| NudgeFg / NudgeBg | OKLab L +0.02 | L −0.02 | ±0.10 |
| Anything else | no-op | no-op | no-op |

Mouse `−` / `+` on a row, and wheel over that row, perform the same steps and set focus to that control. Do not also bind `-` / `+` / `=` as global size keys (they collide with typing in hex/`rgb()`).

Focus order — Contrast: `FgHex → BgHex → Size → Weight → Style → PreviewText → FontFamily → Swap → CopyHex → FixBtn → OpenPreview` then Fix if open (`NudgeFg → NudgeBg → ApplyFix → NextFix → CloseFix → SendFg → SendBg`).

Focus order — Palette: `Role(0..3) → Generate → Matrix → Detail` then Fix if open. Size/weight on Palette are the shared controls; include them in the cycle after Support.

---

## 15. Style chips vs weight

| Chip | weight | italic |
|---|---|---|
| Regular | 400 | false |
| Bold | 700 | false |
| Italic | 400 | true |
| Bold+Italic | 700 | true |

Weight stepper may then move 100–900. If weight is not 400 or 700, chips show no exclusive selection (all idle) but italic remains. `Ctrl+S` on Contrast cycles the four presets (and resets weight to 400/700 accordingly).

---

## 16. Widgets

| Region | Widget |
|---|---|
| Tabs | `Tabs` or custom `Line`s with stored label rects |
| Hex / text / font fields | one-row `Block` + `Paragraph`; caret from `cursor_char_idx` |
| Swatches | `Paragraph` `Style::new().bg(Color::Rgb(..))`, width 2 |
| Style chips | four mini `Block`s |
| Buttons | bordered `Paragraph`; focused / hovered / idle from theme |
| Preview | `Paragraph` + `Wrap`; user FG/BG; `BOLD` when weight≥700; `ITALIC` when italic |
| Scores | two `Block`s of `Line`s |
| Role list | `List` + `ListState` |
| Matrix | `Table` + `TableState`, or custom grid if per-cell bg is easier |
| Pair list (narrow) | `List` + `Scrollbar` + `ScrollbarState` |
| Detail | `Paragraph` + custom or ratatui scrollbar (`scrollbar` / `scrollbar_hover`) |
| Nudge | `LineGauge` |
| Fix overlay / F1 / F2 | `Clear` + bordered `Block` (`modal_*` tokens) |
| Toast | bottom-right, 5s, semantic colors |

---

## 17. Chrome theme

Load order (first hit wins):

1. `./dd_wcag_theme.yml`
2. `~/.config/ldnddev/dd_wcag_theme.yml`
3. Built-in defaults

Files must declare `version: 1`. Unsupported versions fall back to defaults with a warning toast.

Map UI to canonical tokens in `THEME_STRUCTURE_STANDARD.md` / `THEME_KEYS`. After load, **never** hardcode chrome colors.

- Shell/header/footer: `base_background` + `text_primary`
- Panes: `body_background`
- Modals/toasts: `modal_background`, `modal_text`, `modal_labels`
- Focus border: `border_active` / `input_border_focus`
- Hover: underline or `text_active_focus`
- Pass/fail/disagree: `success` / `error` / `warning`
- Scrollbar: `scrollbar`, `scrollbar_hover`
- Cursor: `cursor`

User pair colors are **only** for preview text, swatches, and matrix cell backgrounds.

F2 shows: source (`local` / `global` / `default`), version, path, health message.

---

## 18. Help overlays

**F1** — keys (§14) + mouse (§13) short form. Esc / F1 / click outside closes. No `?` binding.

**F2** — theme debug. Esc / F2 / click outside closes.

---

## 19. Web preview

`Ctrl+O` / Web button writes `/tmp/dd_wcag_preview.html` and opens the default browser. Rewrite on every `sync_preview`. This is the only true CSS `font-size` / `font-family` / italic path. Include WCAG and APCA (header bar + lookup) in the meta block.

---

## 20. Module layout (target)

```
src/
  main.rs           // setup, mouse capture, loop, dispatch_effects (clipboard, save, open)
  app.rs            // App, mode, focus, drag, handle_key_event, handle_mouse_event, KeyEffects
  layout.rs         // breakpoint, shell splits, LayoutMap, hit()
  contrast.rs       // ContrastState + render_contrast(area) -> partial map
  palette.rs        // theme-builder derive/export + matrix + render_palette
  fix.rs            // OKLab candidates, LineGauge, render_fix
  widgets/          // color_field, chips, swatch, matrix, buttons
  color.rs          // parse hex/rgb/hsl, WCAG, APCA, OKLab lightness nudge
  theme.rs          // YAML load, version, tokens
  web_preview.rs    // /tmp HTML
  ui.rs             // thin: render() merges layout + overlays + toast + caret
```

Each `render_*` takes `area: Rect` and returns the rects it owns. `render()` merges into `LayoutMap`.

Keep the I/O-free handler split. Clipboard backends unchanged (`pbcopy` / `clip` / `wl-copy` / `xclip` / `xsel`).

---

## 21. Tests

- Color parse/format, WCAG ratio, APCA Lc, OKLab L nudge + clamp.
- `hit()`: toast > modal > fix overlay > chrome > tab; outside overlay; diagonal matrix ignored.
- `char_index_at` for one-row fields.
- Tab/click away with invalid HEX: focus stays, toast set.
- Palette generate + `validate_export` blocking vs advisory (existing rules).
- Header APCA bar vs lookup advisory: glyphs follow the bar.
- Fix Apply refuses to write into fixed text tokens.
- KeyEffects: `Ctrl+Q`, `Ctrl+O`, `Ctrl+S`, `Ctrl+C` on Palette.

No automated screenshot harness. Manual: Wide/Medium/Narrow resize; Contrast A↔E; Palette D↔stack; Fix strip vs overlay; mouse table in §13; web preview; theme local/global/default.

---

## 22. Build order (`layout-mouse`)

Do not skip hit-testing until after the first painted layout exists. Prefer incremental commits.

1. Shell + breakpoints + empty tabs + resize + YAML theme chrome. Confirm Wide vs Narrow by shrinking the terminal.
2. Contrast form (one-row fields) + preview + scores. Keyboard only. Shared size/weight. Keep parse/apply/toasts.
3. `LayoutMap` + click-to-focus + caret + steppers + chips + tab/target clicks + toast click + F1/F2 outside click.
4. Palette roles (P/S/T/Support + read-only Text) + matrix + keyboard cell movement. Generate still produces `_palette.scss` as today.
5. Matrix / list / detail mouse + wheel + scrollbar drag/hover.
6. Fix as bottom split on Wide/Medium. OKLab L, Apply cannot mutate text tokens.
7. Narrow Fix overlay + click-outside + Esc.
8. Slider drag sharing `set_lightness` with `[` `]`.
9. Generate/audit + worst-pair selection + `Ctrl+S`/`Ctrl+C` gating.
10. Hover styles, Shift+click copy hex, web preview wiring, footer mouse hint, README/F1 copy.

Done when:

- Contrast A↔E and Palette D↔list switch only from `breakpoint(frame.area())`
- Every visible control has a `Rect` in `LayoutMap` and a keyboard equivalent
- Fix can be opened from Contrast or from a matrix cell
- WCAG and APCA both update live on color, size, weight, and header-target changes
- Theme builder export still matches §8
- F1/F2/toasts/`Ctrl+Q`/`Ctrl+O` work

---

## 23. Out of scope unless asked

- Color-vision simulation
- Persistence / saved palettes / multi-palette libraries
- x-height controls
- Automated screenshot tests
- Drag-select / select-all in fields
- Right-click context menus
- Bare `q` to quit
- Making Text an editable palette role
- Hardcoded chrome colors
- Randomizing role colors on Generate

---

## 24. Doc map

| File | Role |
|---|---|
| `SPEC.md` | This file. App product + layout + mouse + keyboard + palette export. |
| `THEME_STRUCTURE_STANDARD.md` | Shared ldnddev YAML chrome tokens. |
| `README.md` | User-facing install, keys, manual tests. Update when `layout-mouse` ships. |
| `CLAUDE.md` | Agent working notes; points here for product rules. |
| `dd_wcag_theme.yml` | Default/local theme instance. |
