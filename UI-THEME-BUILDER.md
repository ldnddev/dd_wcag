# UI Theme Builder

Use this document as the product spec for the `dd_wcag` Palette tab when it is used to generate a UI theme palette.

The Palette tab must let the user provide brand base colors, generate the derived SCSS variables from those colors, and test each generated color against fixed text roles for WCAG compliance.

## Goals

- Accept user-provided `Primary`, `Secondary`, and `Tertiary` brand colors.
- Keep text colors fixed; the theme builder must not modify text role values.
- Generate light and dark mode palette variables that match the structure defined below.
- Test every generated color where it can be used as a text/background/border/surface value.
- Make failures visible before export.
- Provide copy-ready `_palette.scss` output.

## Palette Tab Scope

The Palette tab should support two related workflows:

1. **Brand input**
   - User enters `Primary`, `Secondary`, and `Tertiary` colors.
   - Optional: user enters `Support / Utility`.
   - Inputs must accept the same formats as the rest of `dd_wcag`: `#rgb`, `#rrggbb`, `rgb()`, `rgba()`, and `hsl()`.

2. **Generated theme review**
   - App generates derived variables for light and dark mode.
   - App shows WCAG pass/fail status for each generated usage.
   - App exposes copy-ready and save-ready `_palette.scss` output.

The first implementation should keep the generated palette in memory until the user explicitly saves or copies it.

## Base Colors

These are user-provided brand inputs. They are the only colors the user must supply for the core palette.

```scss
$base_primary: rgba(136, 217, 247, 1);
$base_secondary: rgba(255, 202, 118, 1);
$base_tertiary: rgba(249, 137, 113, 1);
$base_support: rgba(70, 190, 140, 1);
```

Rules:

- `Primary`, `Secondary`, and `Tertiary` are required.
- `Support / Utility` is optional; if omitted, use the app default support color.
- Alpha may be accepted in `rgba()`, but generated palette variables must be opaque `rgba(r, g, b, 1)`.
- Invalid base colors must block generation and show the field-specific parse error.

## Fixed Text Colors

These values must not be changed by palette generation.

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

Rules:

- The builder may choose which fixed text token is used on a generated surface.
- The builder must not invent new text colors to force compliance.
- If no fixed text token passes on a generated surface, the generated surface must be adjusted, not the text color.
- Disabled states may use disabled text roles and must still be reported in the compliance table.

## Generated Variable Groups

The output must follow the variable naming and grouping defined below.

### Primary Colors

Derived from `Primary`.

```scss
$c_primary_default
$c_primary_default--dark
$c_primary_strong
$c_primary_strong--dark
$c_primary_subtle
$c_primary_subtle--dark
```

### Secondary Colors

Derived from `Secondary`.

```scss
$c_secondary_default
$c_secondary_default--dark
$c_secondary_strong
$c_secondary_strong--dark
$c_secondary_subtle
$c_secondary_subtle--dark
```

### Tertiary Colors

Derived from `Tertiary`.

```scss
$c_tertiary_default
$c_tertiary_default--dark
$c_tertiary_strong
$c_tertiary_strong--dark
$c_tertiary_subtle
$c_tertiary_subtle--dark
```

### Action Colors

Generate action variables for `Primary`, `Secondary`, and `Tertiary`.

Each action family must include `default`, `hover`, `pressed`, and `disabled` states.

Each state must include:

- `surface`
- `text`
- `border`

Example for primary:

```scss
$c_primary_action_default_surface
$c_primary_action_default_surface--dark
$c_primary_action_default_text
$c_primary_action_default_text--dark
$c_primary_action_default_border
$c_primary_action_default_border--dark
```

Repeat the same pattern for:

- `$c_primary_action_hover_*`
- `$c_primary_action_pressed_*`
- `$c_primary_action_disabled_*`
- `$c_secondary_action_*`
- `$c_tertiary_action_*`

### Support / Utility

Derived from `Support / Utility`.

```scss
$c_support_overlay
$c_support_overlay--dark
$c_support_border
$c_support_border--dark
$c_support_focus
$c_support_focus--dark
```

### Additional Token Groups

The generated `_palette.scss` must also include the following groups, which are not derived from user inputs and use built-in defaults:

- Semantic status variables: success, warning, error, info
- Fixed text role variables
- Neutral UI variables

These groups may use existing defaults unless the builder later adds explicit inputs for them.

## Derivation Requirements

Generated colors should be predictable and accessible.

For each brand family:

- `default` should stay visually close to the supplied base color.
- `strong` should be suitable for emphasis, active states, and high-contrast text/icon usage.
- `subtle` should be suitable for light fills, badges, table highlights, and quiet surfaces.
- `--dark` variants should be tuned for dark UI backgrounds, not mechanically inverted.

For action states:

- `hover_surface` should be visibly different from `default_surface`.
- `pressed_surface` should be visibly different from `hover_surface`.
- `disabled_surface`, `disabled_text`, and `disabled_border` should look inactive while remaining legible.
- State changes must not rely on color alone; generated notes should identify when border or surface changes communicate the state.

## Compliance Testing

The Palette tab must test generated variables against the fixed text colors.

### Minimum WCAG Targets

- Normal text: WCAG AA, contrast ratio `>= 4.5`.
- Large text and UI labels: WCAG AA, contrast ratio `>= 3.0`.
- Non-text UI boundaries such as borders/focus rings against adjacent surfaces: contrast ratio `>= 3.0`.
- Disabled states are reported, but may be marked `disabled advisory` instead of hard failing the export.

### Required Checks

For each generated color family (`primary`, `secondary`, `tertiary`):

- Test `default`, `strong`, and `subtle` against:
  - `$c_text_primary`
  - `$c_text_secondary`
  - `$c_text_inverse`
  - `$c_text_primary--dark`
  - `$c_text_secondary--dark`
  - `$c_text_inverse--dark`
- Identify the recommended fixed text token for each generated surface.
- If no fixed text token passes, mark the generated color as failing and recommend adjusting that generated surface.

For each action state:

- Test `*_action_*_text` against `*_action_*_surface`.
- Test `*_action_*_border` against `*_action_*_surface`.
- Test light-mode action surfaces against light neutral backgrounds.
- Test dark-mode action surfaces against dark neutral backgrounds.

For support / utility:

- Test `support_border` and `support_focus` against light and dark neutral surfaces.
- Test `support_overlay` against fixed text tokens if it is used as a readable surface.

## Palette Tab Layout

Use the Option 1 split layout.

```text
┌──────────────────────────────┬─────────────────────────────────────────────┐
│ Palette Inputs               │ Selected / Generated Detail                 │
│                              │                                             │
│ > Primary      #88D9F7       │ Primary                                     │
│   Secondary    #FFCA76       │ Base: #88D9F7                               │
│   Tertiary     #F98971       │ Generated: default / strong / subtle         │
│   Support      #46BE8C       │                                             │
│                              │ Text colors: fixed                          │
│                              │ Best text: $c_text_primary                  │
│                              │ WCAG: 8.92 PASS                             │
│                              │                                             │
│                              │ Enter: edit color                           │
│                              │ G: generate palette                         │
│                              │ F then G: preview as FG                     │
│                              │ B then G: preview as BG                     │
│                              │ Ctrl+S: save _palette.scss                  │
│                              │ Ctrl+C: copy generated values               │
└──────────────────────────────┴─────────────────────────────────────────────┘
```

### Detail Modes

The right panel should be able to show:

- Base color parse details: hex, rgb, hsl.
- Generated tokens for the selected family.
- Compliance checks for selected generated token.
- Full family summary.
- Export preview.

## Palette Keybindings

- `Up` / `Down`:
  - Before a palette has been generated: move through `Primary`, `Secondary`, `Tertiary`, and `Support`.
  - After generation: scroll the generated detail panel.
- `Enter`: edit the selected base color.
- `G`: generate or regenerate the palette from the current base colors.
- `F` then `G`: push the actively selected palette color to the app foreground input for preview.
- `B` then `G`: push the actively selected palette color to the app background input for preview.
- `Ctrl+S`: save/export `_palette.scss`.
- `Ctrl+C`: copy the generated `_palette.scss` values so the user can paste them elsewhere.
- `Esc`: cancel edit mode or a pending preview apply sequence.

The detail panel must show a visible scrollbar when its content overflows.

Non-interactive status and error messages must render as bottom-right toasts and close automatically after 5 seconds.

### Save Behavior

`Ctrl+S` must write the generated SCSS to `_palette.scss`.

Rules:

- If no palette has been generated yet, show `Generate palette before saving`.
- If required base colors are invalid, block save and show the field-specific parse error.
- If export-blocking compliance failures exist, block save and show the first failing group plus a count of remaining failures.
- If only advisory warnings exist, allow save and show a warning summary after saving.
- Default save target should be `./_palette.scss`.

### Copy Behavior

`Ctrl+C` must copy the generated SCSS values to the clipboard.

Rules:

- If no palette has been generated yet, show `Generate palette before copying`.
- If required base colors are invalid, block copy and show the field-specific parse error.
- If export-blocking compliance failures exist, block copy and show the first failing group plus a count of remaining failures.
- If only advisory warnings exist, allow copy and show a warning summary after copying.
- Copied content must match the same `_palette.scss` output that `Ctrl+S` saves.

## Copy-Ready Output

The builder must provide a copy-ready `_palette.scss` section.

Output requirements:

- Use the exact variable names listed above.
- Use `rgba(r, g, b, 1)` values.
- Preserve this group order:
  - Primary
  - Secondary
  - Tertiary
  - Primary Action
  - Secondary Action
  - Tertiary Action
  - Semantic
  - Text Roles
  - Neutrals
  - Support / Utility
- Include short comments only where they explain usage or compliance notes.

## WCAG Notes Output

The builder must provide a concise compliance summary with the generated palette.

For each family, include:

- Recommended text token for `default`, `strong`, and `subtle`.
- Any failing combinations.
- Suggested use:
  - `default`: normal brand surfaces / accents
  - `strong`: emphasis, active state, text/icon color when compliant
  - `subtle`: quiet background surface only
- Action state notes:
  - default, hover, pressed, disabled
  - chosen text token
  - pass/fail ratio

## Export Blocking Rules

The app may export a palette only when:

- Required base colors parse successfully.
- All non-disabled action text/surface pairs pass WCAG AA normal text.
- All focus and border tokens pass the `>= 3.0` non-text contrast target against their intended surfaces.
- Every generated surface has at least one compliant fixed text token recommendation.

The app may export with warnings when:

- Disabled state contrast is below normal text AA but still visually communicates disabled state.
- A generated decorative accent is not recommended for text.

## Terms

- Use `Secondary`, not `Seconday`.
- Use `Tertiary`, not `Tertiery`.
- Use `Primary Action`, not `Primary Action Action`.
