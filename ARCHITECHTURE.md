# dd_wcag Project Architecture

## Overview
dd_wcag is a TUI CSS color picker/converter with WCAG 2.1 AA contrast checker. Built in Rust using Ratatui for UI, Crossterm for events, Palette for color math, and Anyhow for errors. Supports foreground/background color input, conversions (hex/rgb/hsl), previews, and contrast checks across font sizes (12-18px) and weights (normal/bold).

## Architecture

### Modules
- `main.rs`: Terminal setup, Crossterm event loop, Ratatui rendering.
- `app.rs`: State management (App struct).
- `color.rs`: Color parsing, conversions, luminance, contrast calculations.
- `ui.rs`: Widget rendering and layout.

### Dependencies
- ratatui
- crossterm
- palette
- anyhow

### App State (app.rs)
- `foreground`: Color (using palette::Srgb)
- `background`: Color
- `input_target`: Enum { Foreground, Background, None }
- `current_input`: String (active text field)
- `parsed_fg`, `parsed_bg`: Option<Color>
- `contrast_ratio`: f64
- `preview_text`: String (editable sample text)
- `font_size_idx`: usize (index into [12.0, 14.0, 16.0, 18.0])
- `is_bold`: bool
- `error`: Option<String>
- `active_tab`: Enum { Input, Conversions, Contrast, Preview }

### Color Struct (color.rs)
- Methods:
  - `parse_hex`, `parse_rgb`, `parse_hsl` → Result<Color>
  - `to_hex()`, `to_rgb_str()`, `to_hsl_str()`
  - `luminance()` → f64
  - `contrast_ratio(other: &Color)` → f64 ((L1 + 0.05) / (L2 + 0.05))
  - `to_style()` → ratatui::Style

### Contrast Logic
- WCAG 2.1 AA thresholds:
  - Normal text: ≥4.5:1
  - Large text (≥18px normal or ≥14px bold): ≥3:1
- Font sizes: [12.0, 14.0, 16.0, 18.0] px
- `passes_aa(size: f32, bold: bool, ratio: f64) → bool`

### UI Layout (ui.rs)
- Vertical split:
  - Top: Input area (FG/BG fields, active indicator)
  - Middle: Tabs (Input / Conversions / Contrast / Preview)
  - Bottom: Help bar ("Tab: switch FG/BG | Enter: edit | Arrow: size | B: toggle bold | q/Esc: quit")
- Widgets:
  - Text input fields
  - Conversions Table: (Format | Value)
  - Preview: Paragraph with FG/BG styles + bold modifier
  - Contrast Table:
    - Columns: Size | Normal Ratio | Normal Pass | Bold Ratio | Bold Pass
    - Rows: 12px, 14px, 16px, 18px
    - Green/red styling for PASS/FAIL

### State Flow
- Input → Parse → Convert/Compute contrast → Render tabs/panels
- Update contrast on color changes

## Build Plan – Phased

### Phase 1 – Proof of Concept
1. `cargo new dd_wcag && cd dd_wcag`  
2. `cargo add ratatui crossterm palette anyhow`  
3. Create src/{app.rs,color.rs,ui.rs}  
4. color.rs: basic hex parse → Srgb, to_hex(), luminance()  
5. main.rs: minimal terminal setup + event loop (q to quit)  
6. ui.rs: empty frame with title + help bar  
7. Commit: "Phase 1: basic project setup & terminal loop"  
   Push if remote exists

### Phase 2 – MVP (usable core)
8. color.rs: add rgb/hsl parse, to_rgb_str(), to_hsl_str(), contrast_ratio()  
9. app.rs: App struct with fg, bg, input_target, current_input, contrast_ratio, font_size_idx, is_bold  
10. app.rs: update_contrast(), passes_aa() logic  
11. main.rs: handle keys (tab switch FG/BG, chars/enter for input, B toggle bold, arrows cycle size, q/Esc quit)  
12. ui.rs: basic layout (top inputs, preview paragraph, simple contrast table)  
13. Parse on enter, show preview with FG/BG styles  
14. Commit: "Phase 2: MVP – color input, preview, basic contrast table"  
   Push

### Phase 3 – Polish
15. ui.rs: full contrast table (size/normal/bold/ratio/pass, green/red)  
16. ui.rs: add conversions table (hex/rgb/hsl)  
17. ui.rs: tabs (Input/Conversions/Contrast/Preview)  
18. Add error display in UI  
19. Improve help bar + active field indicator  
20. Commit: "Phase 3: polished UI, tabs, error handling"  
   Push

### Phase 4 – Additions & Documentation
21. Add preview_text editing (simple multi-line input)  
22. Optional: closest Tailwind color match (hardcoded or basic)  
23. Heavy documentation:  
   - README.md: project overview, usage, build/run  
   - Inline doc comments on every public fn/struct/method  
   - Module-level comments in each file  
24. Final refactor & clean-up  
25. Commit: "Phase 4: final features + full documentation"  
   Push

## Notes
- Document heavily: every function, struct, and module.
- Git commit after each major change.
- Push before starting new phase.
- Test with samples: #000/#fff, #333/#ccc, invalid input.