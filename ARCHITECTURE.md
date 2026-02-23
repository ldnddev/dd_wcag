# dd_wcag Architecture

## Overview

dd_wcag is a terminal-based WCAG color contrast utility built with Rust, Ratatui for the TUI, and Crossterm for terminal handling. It allows users to input foreground/background colors, preview text, and evaluate WCAG/ APCA contrast compliance with live updates.

## Modules

- **main.rs**: Entry point, terminal setup, event loop, key handling.
- **app.rs**: App state (colors, inputs, font size, bold, tabs, errors).
- **color.rs**: Color parsing (HEX/RGB/HSL), conversions, WCAG contrast ratio, APCA Lc calculation and pass checks.
- **ui.rs**: Rendering for tabs (Input, Conversions, Contrast with WCAG/APCA, Preview), popups, cursor positioning.
- **web_preview.rs**: Generates and opens HTML preview with real CSS styling and compliance info.
- **theme.rs**: Loads custom theme from YML for TUI colors.

## Key Flows

1. **Input Handling**: Tab cycles focus, auto-applies valid colors, syncs preview.
2. **Contrast Evaluation**: WCAG ratio and APCA Lc computed in color.rs, displayed in ui.rs with pass/fail based on size/bold.
3. **Preview**: TUI preview in terminal; web preview for accurate font sizing and compliance display.
4. **Event Loop**: Handles keys for navigation, adjustments, open preview.

## New Features

- **APCA Integration**: Added to color.rs with Lc calculation and size/weight-based thresholds. Displayed alongside WCAG in Contrast tab and web preview.
- **Palette Tab**: New tab for managing a list of brand colors and testing contrast against FG/BG. Logic in app.rs for palette storage/add/remove; rendering in ui.rs with contrast results; keybindings for navigation/adding colors.

## Definition of Done (Phase 1+)

- Basic TUI setup and event loop.
- Color parsing and conversions.
- WCAG contrast.
- APCA contrast.
- Browser preview with compliance info.
- Tests for all core functions.

For contributions, see README.md.

