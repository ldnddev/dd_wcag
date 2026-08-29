# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`dd_wcag` is a Rust TUI utility for WCAG / APCA color contrast evaluation and SCSS palette generation. Built on `ratatui` + `crossterm` + `palette` + `anyhow` (Rust edition 2024).

## Common commands

```bash
cargo run                       # run the TUI
cargo build --release           # produce ./target/release/dd_wcag
cargo check --locked            # CI parity (matches .github/workflows/ci.yml)
cargo test --locked             # full test suite (CI command)
cargo test <name>               # run a single test by name substring
cargo test --test <file>        # run a specific integration test file
./install.sh                    # build + install binary to ~/.local/bin and theme to ~/.config/ldnddev
./install.sh -uninstall         # remove installed binary + theme (and config dir if empty)
```

Tests live in `#[cfg(test)] mod tests` blocks alongside the code in `main.rs`, `app.rs`, and `color.rs`.

## Architecture

### Module layout (all under `src/`)
- `main.rs` — terminal lifecycle, event loop, **pure** `handle_key_event` and the side-effect dispatcher. Also: clipboard integration (`pbcopy` / `clip` / `wl-copy` / `xclip` / `xsel`) and palette save.
- `app.rs` — central `App` state, input-target switching, draft syncing, cursor-aware text editing, font-size clamping (`6..=120`), toast notification TTL.
- `color.rs` — color parsing (`hex`, `rgb`, `rgba`, `hsl`), formatting, WCAG luminance/contrast math.
- `palette.rs` — Palette tab state, derivation of light/dark variants, WCAG compliance gating, `_palette.scss` generation.
- `theme.rs` — YAML theme loading, version validation, fallback chain.
- `ui.rs` — ratatui rendering for all tabs, popups (keybindings F1, theme debug F2), bottom-right toast.
- `web_preview.rs` — writes `/tmp/dd_wcag_preview.html` and opens it in the default browser (true CSS pixel font-size).

### Key-handling pattern (important)
`handle_key_event(&mut App, KeyEvent) -> KeyEffects` is **pure of I/O**. It mutates `App` and returns a `KeyEffects` struct indicating intents: `quit`, `sync_preview`, `open_preview`, `save_palette`, `copy_palette`. The real side effects (browser open, file write, clipboard, web-preview regeneration) are performed by `run_loop` based on those flags. Preserve this split when adding actions — it is what makes the key handler unit-testable.

### Tabs and focus
Five tabs: `Input | Conversions | Contrast | Preview | Palette`. `Tab`/`Shift+Tab` cycles tabs and, while inside `Input`, cycles `InputTarget` (`Foreground → Background → PreviewText → FontFamily`) auto-applying the current draft. Invalid input on auto-apply blocks focus movement and surfaces a toast.

### Palette tab — derived from product spec
`SPEC.md` is the product spec (layout, mouse, keyboard, and palette export). When changing palette code:
- Required base inputs: `Primary`, `Secondary`, `Tertiary`. Optional: `Support`. Fixed text tokens must never be modified by generation.
- Generated SCSS must use `rgba(r, g, b, 1)` and preserve the variable group order documented in the spec (Primary, Secondary, Tertiary, Primary Action, Secondary Action, Tertiary Action, Semantic, Text Roles, Neutrals, Support/Utility).
- Export (`Ctrl+S`) and clipboard (`Ctrl+C`) share the same SCSS payload via `App::prepare_palette_export` and are **gated on WCAG compliance** — see `validate_export` in `palette.rs`. Disabled-state failures are advisory; non-disabled action text/surface and border/focus failures block export.
- Keys: `Ctrl+G` generates; `Ctrl+F` toggles Fix. Bare letters type into focused fields. Palette does not push colors to Contrast FG/BG (those fields are not visible on the Palette tab).

### Theming (cross-app standard)
Themes are loaded in this order, first hit wins:
1. `./dd_wcag_theme.yml` (project local)
2. `~/.config/ldnddev/dd_wcag_theme.yml` (global)
3. Built-in defaults

Theme files **must** declare `version: 1`; unsupported versions fall back to defaults with a warning toast. The full set of required keys lives in `THEME_KEYS` in `src/theme.rs` and matches `THEME_STRUCTURE_STANDARD.md` (the shared ldnddev TUI standard). When adding a new themable color, add the token to the standard doc first, then to `THEME_KEYS`, then to `Theme` and `Default for Theme`. Do not hardcode colors in render paths after the theme is loaded.

### Notifications
Status and error feedback render as bottom-right toasts (`TOAST_TTL = 5s`, expired by `App::expire_notification` on each tick). Do not introduce blocking error popups for non-interactive feedback.

### Browser preview
`Ctrl+O` opens `/tmp/dd_wcag_preview.html`. The file is rewritten on every `sync_preview` effect — it is the only path that renders true CSS pixel font sizes (TUI cannot). Web preview includes WCAG and APCA info in the meta block.

## Reference docs in repo
- `SPEC.md` — single app product spec (layout, mouse, keyboard, contrast math, palette generation/export). Authoritative for the `layout-mouse` rewrite. Current `src/` may still be the five-tab app until that branch lands.
- `README.md` — user-facing keybindings, manual test checklist, and the architecture section (tech stack, module layout, key-handler pattern, runtime state, contrast logic, test coverage).
- `THEME_STRUCTURE_STANDARD.md` — shared theme schema across ldnddev TUIs.
