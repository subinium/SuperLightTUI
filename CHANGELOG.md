# Changelog

## [0.6.0] — 2026-03-14

### Features
- **Select/Dropdown widget**: `select()` with `SelectState` — collapsible dropdown with keyboard and mouse support
- **Radio buttons**: `radio()` with `RadioState` — mutually exclusive option group with ●/○ markers
- **Multi-select**: `multi_select()` with `MultiSelectState` — checkbox-style [x]/[ ] selection with Space toggle
- **Tree view**: `tree()` with `TreeNode`/`TreeState` — hierarchical expandable tree with ▾/▸ icons
- **Virtual list**: `virtual_list()` — renders only visible items for large datasets with ↑/↓ indicators
- **Command palette**: `command_palette()` with `CommandPaletteState` — modal search overlay with fuzzy filtering
- **Markdown rendering**: `markdown()` — renders headings (#/##/###), bold, italic, lists, code, and horizontal rules
- **Key sequences**: `key_seq("gg")` — matches multi-character key sequences within a single frame
- **Password masking**: `TextInputState.masked` — displays input as `•` characters
- **Percentage-based sizing**: `w_pct()` / `h_pct()` — set container width/height as percentage of parent
- **Per-side borders**: `border_top()`, `border_right()`, `border_bottom()`, `border_left()`, `border_sides()` — show/hide individual border sides with `BorderSides` type

### Improvements
- 30+ widgets total (up from 20+)
- New state types exported: `SelectState`, `RadioState`, `MultiSelectState`, `TreeNode`, `TreeState`, `CommandPaletteState`, `PaletteCommand`

## [0.5.1] — 2026-03-14

### Documentation
- Added module-level rustdoc (`//!`) to all 10 public modules
- Documented `EventBuilder` and `TestBackend` public API in `test_utils`
- Documented `Direction` enum variants in `layout`
- Documented `max_length` fields and methods in `TextInputState` / `TextareaState`
- Removed hardcoded line count from README Architecture section

### Bug Fixes
- Fixed RNG infinite loop in demo_game — replaced LCG with xorshift64 for all 3 games
- Fixed MSRV clippy error — replaced `is_multiple_of(2)` with `% 2 == 0` (requires Rust 1.74+)
- Fixed game layout — nav pinned to top, game content centered vertically

## [0.5.0] — 2026-03-14

### Features
- **Design system overhaul**: container `bg()` now propagates to child text, borders, titles, and scroll indicators — no more split background/text rendering
- **Theme expansion**: added `surface_text`, `surface_hover` fields to Theme struct for readable text on elevated surfaces
- **5 new themes**: Dracula, Catppuccin, Nord, Solarized Dark, Tokyo Night (total 7 built-in themes)
- **Color utilities**: `Color::luminance()`, `Color::contrast_fg()`, `Color::blend()`, `Color::lighten()`, `Color::darken()`
- **Focus events**: `Event::FocusGained` / `Event::FocusLost` for terminal focus tracking; hover clears on focus loss
- **New widgets**: `button_with()` variants (Primary, Secondary, Danger, Ghost, Outline), `form_field()`, `form_submit()`, `bar_chart_grouped()`, `histogram()`, `line_chart()`, `bar_chart_styled()`
- **Justify modes**: `SpaceBetween`, `SpaceAround`, `SpaceEvenly` for flexbox-style distribution
- **Links**: `ui.link()` renders OSC 8 clickable hyperlinks
- **Canvas**: braille-based vector drawing with `line()`, `circle()`, `rect()`, `point()`
- **Animation**: `Sequence` chaining, `Stagger` for list animations, `LoopMode::PingPong`
- **Snapshot testing**: `TestBackend::to_string_trimmed()` for insta-based UI regression tests

### Bug Fixes
- Container background now correctly inherits to border characters, title text, and scroll indicators
- Modal centering respects `min_width` / `max_width` constraints
- Hover state properly clears when terminal loses focus (via `EnableFocusChange`)

### Improvements
- Demo example fully redesigned: tabbed navigation, theme-aware cards, all widgets showcased
- Demo website example: surface_text applied for readable text on colored backgrounds
- 4 new regression tests for background color inheritance
- 162 total tests passing

## [0.4.1] — 2025-12-26

### Features
- IME/Korean input support for text_input and textarea
- Text selection with mouse drag (border cell exclusion)
- Click-to-focus for interactive widgets

## [0.3.0] — 2025-12-21

### Features
- Data visualization: chart, histogram, bar_chart, sparkline
- Grid layout
- Error boundary with panic recovery
- Serde support (optional feature)
- Viewport culling for off-screen widgets
- FPS cap via `RunConfig::max_fps`

## [0.2.2] — 2025-12-18

### Features
- TestBackend for headless rendering
- Synchronized output (DECSET 2026)
- State safety improvements

## [0.2.0] — 2025-12-15

### Features
- Initial public release
- Immediate-mode API with row/col layout
- 15+ built-in widgets
- Double-buffer diff rendering
- Dark and light themes
