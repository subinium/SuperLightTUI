# Changelog

## [0.9.3] — 2026-03-15

### Refactoring
- **Run loop deduplication**: extracted `run_frame()` generic over `TerminalBackend` trait — 3 near-identical ~300-line loops replaced with 1 shared frame function + 3 thin wrappers. `lib.rs` reduced from 940 to 732 lines
- **FrameState struct**: bundled 15+ per-frame local variables into `FrameState`, eliminating `Context::new()` 17-parameter constructor and removing `#[allow(clippy::too_many_arguments)]`
- **TerminalBackend trait**: `Terminal` and `InlineTerminal` now implement a shared trait with `size()`, `buffer_mut()`, `flush()`, `handle_resize()`
- **style.rs split**: extracted `style/color.rs` (Color enum + ColorDepth, 316 lines) and `style/theme.rs` (Theme + ThemeBuilder, 353 lines). `style.rs` reduced from 1429 to 765 lines
- **ContainerBuilder field unification**: renamed `bg_color` → `bg`, `dark_bg_color` → `dark_bg` to match `ContainerStyle` field names

## [0.9.2] — 2026-03-15

### Features
- **`gap_at(bp, value)`**: unified breakpoint API — `ui.container().gap_at(Md, 2)` replaces `ui.container().md_gap(2)`. Added 7 `_at` methods: `gap_at`, `w_at`, `h_at`, `min_w_at`, `max_w_at`, `grow_at`, `p_at`. Existing methods kept for backward compatibility

### Performance
- **String clone elimination**: `ContainerBuilder::finish()` changed to `mut self`, replacing `group_name.clone()` with `group_name.take()` — eliminates one heap allocation per container per frame

### Refactoring
- **context.rs split** (6527 → 2163 lines): widget methods extracted to `context/widgets_display.rs` (896), `context/widgets_input.rs` (540), `context/widgets_viz.rs` (3012)
- **layout.rs split** (2294 → 1411 lines): flexbox algorithm extracted to `layout/flexbox.rs` (343), rendering to `layout/render.rs` (548)
- **terminal.rs split** (1044 → 880 lines): selection logic extracted to `terminal/selection.rs` (175)

## [0.9.1] — 2026-03-15

### Bug Fixes
- **draw_raw focus_id**: `pending_focus_id.take()` was called twice in `RawDraw` node creation — second call clobbered the first with `None`, breaking `FocusMarker` on draw_raw regions

### Improvements
- **Hook panic messages**: `use_state` type mismatch now reports hook index and expected type name (`use_state type mismatch at hook index 3 — expected i32`) instead of bare `"use_state type mismatch"`
- **draw_raw docs**: added `'static` bound explanation with workaround code example to `ContainerBuilder::draw()` rustdoc

### Tests
- 7 new draw_raw tests: `draw_raw_with_grow_fills_available_width`, `draw_raw_alongside_normal_widgets`, `draw_raw_with_fixed_size`, `draw_raw_styled_content`, `draw_raw_multiple_regions`, `collect_all_focus_rects_match_tab_navigation`, `collect_all_scroll_works_after_merge`

## [0.9.0] — 2026-03-15

### Features
- **`draw_raw()`**: direct buffer access via `ContainerBuilder::draw()` — write to `&mut Buffer` with computed `Rect` after layout. Clip protection prevents writes outside allocated area. Enables custom renderers, game-like effects, and protocol visualizers without the Command pipeline overhead
- **`Buffer` and `Rect` re-exported**: `slt::Buffer` and `slt::Rect` now available at crate root for `draw_raw()` users

### Performance
- **7× fewer tree traversals per frame**: merged 7 independent `collect_*` functions into a single `collect_all()` DFS pass returning a `FrameData` struct — 1000-node trees go from 7000 to 1000 node visits per frame
- **Keyframes: zero allocations per frame**: `Keyframes::value()` no longer clones+sorts the stop list every frame — stops are sorted once at construction time via `stop()` builder
- **Delta-based style flushing**: `terminal::flush()` now emits only changed attributes (fg/bg/modifiers) instead of full `ResetColor + SetAttribute(Reset) + apply_style()` on every style transition — reduces escape sequences by ~50% for typical UIs

### Internal
- Removed 204 lines of dead `collect_*` code after merge
- Added `FrameData` struct and `collect_all()` to layout.rs
- Added `RawDrawCallback` type alias for deferred draw closures
- 3 new tests: `draw_raw_renders_to_buffer`, `draw_raw_respects_constraints`, `draw_raw_clips_outside_rect`
- New example: `demo_raw_draw` showcasing gradient, plasma, and box drawing effects

## [0.8.4] — 2026-03-15

### Bug Fixes
- **Tabs empty labels crash**: guard modulo-by-zero when `TabsState::new(vec![])` — no longer panics
- **Sparkline div-by-zero**: already guarded (verified, no change needed)

### Improvements
- **`State<T>`**: now `Copy + Clone + Debug + PartialEq + Eq` — pass by value, no `&` needed
- **`ContainerStyle`**: now `Copy` — eliminates unnecessary `.clone()` calls
- **`ContainerStyle`**: added `min_h()`, `max_h()`, `w_pct()`, `h_pct()` builder methods
- **`full` feature flag**: `features = ["full"]` enables async + serde + image
- **docs.rs metadata**: `all-features = true` — async/serde/image APIs now visible on docs.rs

## [0.8.3] — 2026-03-15

### Features
- **ContainerStyle**: reusable composable style recipes — `const CARD: ContainerStyle = ContainerStyle::new().border(Border::Rounded).p(1)` + `ui.container().apply(&CARD)`
- **Rustdoc examples**: added `/// # Example` sections to `modal`, `group`, `use_state`, `use_memo`, `apply`

### Bug Fixes
- **Markdown Korean panic**: `parse_inline_segments` used byte indices on char-indexed positions — panicked on multi-byte CJK text (`**bold**` with Korean). Now uses char-based string operations
- **Example warnings**: removed unused variables and dead code in demo, demo_cli

## [0.8.2] — 2026-03-15

### Features
- **IME cursor always visible**: text_input/textarea cursor no longer blinks — always shown when focused, enabling OS IME popup to anchor correctly for Korean/CJK input
- **text_input horizontal scroll**: long text scrolls within container bounds instead of overflowing — CJK double-width aware via unicode-width

### Added
- `demo_ime.rs` example for Korean/CJK input testing

## [0.8.1] — 2026-03-15

### Bug Fixes
- **ListState filter rendering**: `list()` now renders only filtered items via `view_indices` — previously `set_filter()` updated indices but rendering ignored them
- **ThemeBuilder export**: `ThemeBuilder` now exported from `slt::ThemeBuilder` — was inaccessible in v0.8.0

### Removed
- **Pie chart**: `pie_chart()` removed — not practical for terminal display
- **Area chart**: `GraphType::Area` and `ChartBuilder::area()` removed

### Improvements
- Add rustdoc to group hover/focus public API methods
- Demo: add group hover and use_memo sections to v0.8.0 tab
- Demo: interactive theme builder with Coral/Ocean/Forest presets
- Demo: all keyboard shortcuts changed to Ctrl+key to prevent input conflicts
- Other demos (spreadsheet, dashboard, cli): same Ctrl+key migration

## [0.8.0] — 2026-03-14

### Features
- **Hooks**: `use_state()` / `use_memo()` — React-style persistent state with `State<T>` handle pattern
- **Dark mode prefix**: `dark_bg()`, `dark_border_style()` — conditional container styles for dark/light modes
- **Responsive variants**: `xs_w()` through `xl_w()`, `_h`, `_min_w`, `_max_w`, `_gap`, `_p`, `_grow` (35 methods) — breakpoint-conditional layout
- **Group hover/focus**: `ui.group("card").col(...)` with `group_hover_bg()` — parent hover state affects children
- **Theme builder**: `Theme::builder().primary(Color::Red).build()` — 15-field builder with dark defaults
- **ListState filter**: `list.set_filter("rust")` — multi-token AND matching (same as TableState)
- **Animation callbacks**: `.on_complete()` for Tween/Keyframes/Sequence/Stagger, `.on_settle()` for Spring
- **Scatter plot**: `ui.scatter(&data, w, h)` — standalone braille scatter chart

### Changed
- Demo example: added "v0.8.0" tab showcasing all new features

## [0.7.2] — 2026-03-14

### Changed
- **Multi-token command palette filter**: `CommandPaletteState` search now uses the same multi-token AND logic as `TableState` — e.g. `"save buffer"` matches commands where label contains "save" and description contains "buffer"

### Fixed
- Register `demo_fire` and `demo_game` examples in `Cargo.toml`

### Added
- VHS tape file for DOOM fire demo recording (`demo_fire.tape`)

## [0.7.1] — 2026-03-14

### Changed
- **Multi-token table filter**: `TableState::set_filter` now splits input by whitespace and matches all tokens (AND logic) across any cell in a row — e.g. `"ERROR deploy"` matches rows where one cell contains "error" and another contains "deploy"

## [0.7.0] — 2026-03-14

### Features
- **Dashed borders**: `Border::Dashed` and `Border::DashedThick` variants for dashed/heavy-dashed box drawing
- **Kitty keyboard protocol**: `RunConfig { kitty_keyboard: true }` enables key release/repeat events via `KeyEventKind` — silently ignored on unsupported terminals
- **Color auto-downsampling**: `ColorDepth` enum with auto-detection from `$COLORTERM`/`$TERM`; `Color::downsampled()` converts RGB to 256/16-color; `RunConfig { color_depth }` for override
- **Scrollbar widget**: `ui.scrollbar(&scroll)` renders proportional thumb alongside `scrollable()` containers
- **Responsive breakpoints**: `Breakpoint` enum (`Xs`/`Sm`/`Md`/`Lg`/`Xl`) with `ui.breakpoint()` for terminal-width-adaptive layouts
- **OSC 52 clipboard API**: `ui.copy_to_clipboard(text)` writes to system clipboard via OSC 52 (works over SSH)
- **Enhanced DevTools overlay**: F12 now shows widget count, frame time, FPS, and terminal dimensions
- **Half-block image widget**: `HalfBlockImage` renders images at 2× vertical resolution using `▀` characters; `from_rgb()` always available, `from_dynamic()` behind `image` feature flag
- **AI native widgets**: `streaming_text()` with blinking cursor, `tool_approval()` with approve/reject buttons, `context_bar()` with token counts

### New Types
- `KeyEventKind` — `Press`, `Release`, `Repeat`
- `ColorDepth` — `TrueColor`, `EightBit`, `Basic`
- `Breakpoint` — `Xs`, `Sm`, `Md`, `Lg`, `Xl`
- `HalfBlockImage` — terminal-renderable image grid
- `StreamingTextState` — streaming text accumulator
- `ToolApprovalState` / `ApprovalAction` — tool call approval
- `ContextItem` — context bar entry with token count

### New Methods
- `Color::downsampled(ColorDepth)` — downsample to target depth
- `ColorDepth::detect()` — auto-detect from environment
- `ScrollState::content_height()`, `viewport_height()`, `progress()`
- `Context::scrollbar(&ScrollState)` — vertical scrollbar
- `Context::breakpoint()` — responsive width class
- `Context::copy_to_clipboard(text)` — OSC 52 clipboard
- `Context::image(&HalfBlockImage)` — half-block image render
- `Context::streaming_text(&mut StreamingTextState)` — streaming text
- `Context::tool_approval(&mut ToolApprovalState)` — tool approval widget
- `Context::context_bar(&[ContextItem])` — context bar
- `Context::key_release(char)`, `key_code_release(KeyCode)` — key release detection

### Feature Flags
- `image` — enables `HalfBlockImage::from_dynamic()` (adds `image` crate dependency)

## [0.6.1] — 2026-03-14

### Features
- **Table sorting**: click column header to sort ASC/DESC with ▲/▼ indicator — numeric sort when both values parse as numbers, lexicographic otherwise
- **Table filtering**: `set_filter()` applies case-insensitive substring match across all cells
- **Table pagination**: `page_size` field enables paged display with PageUp/PageDown navigation and "Page X/Y" footer
- **Rich text `line()`**: inline row with gap-0 for composing styled text segments
- **Rich text `line_wrap()`**: segment-aware word wrapping that preserves style boundaries
- **Markdown inline styles**: `markdown()` now renders **bold**, *italic*, and `code` with actual terminal styles

### New Methods on `TableState`
- `toggle_sort(column)` — sort by column, click again to reverse
- `sort_by(column)` — sort ascending by column
- `clear_sort()` — remove sorting
- `set_filter(text)` — filter visible rows
- `next_page()` / `prev_page()` — page navigation
- `total_pages()` — total page count
- `visible_indices()` — filtered + sorted row indices

### New Example
- `demo_table` — interactive showcase for table sorting, filtering, and pagination with 20-row dataset

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
