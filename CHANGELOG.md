# Changelog

## [0.12.12] — 2026-03-18

### Improvements

- Add `Default` impl for 8 widget state types: `FormField`, `ToastMessage`, `ListState`, `FileEntry`, `TabsState`, `TableState`, `SelectState`, `RadioState`
- Replace ~35 duplicated breakpoint methods on `ContainerBuilder` with `define_breakpoint_methods!` macro
- Split long widget functions into focused helpers: `table()`, `select()`, `bar_chart_styled()`
- Improve `use_memo` panic message with type information and guidance
- Add `PartialEq` derive to `WidgetColors`

### Documentation

- Add doc comments to `ThemeBuilder` (17 methods), `Palette` (16 constants), `ContainerStyle` fields, `WidgetColors` methods, `Modifiers` constants
- Reduce `missing_docs` warnings from 229 to 68
- Update `CLAUDE.md` architecture section (add `widgets_interactive.rs`, update line counts)

### Tests

- Add 49 new unit tests for `style.rs` (24), `style/theme.rs` (11), `widgets.rs` (14)
- Test suite: 52 → 393 total tests

## [0.12.11] — 2026-03-18

### Documentation

- Add `DESIGN_PRINCIPLES.md` — core design philosophy, widget contract, error handling guide, API stability policy
- Add `ARCHITECTURE.md` — module map, frame lifecycle, data flow, visibility rules
- Add `SECURITY.md` — vulnerability reporting policy
- Add PR template with quality checklist
- Add issue templates (bug report, feature request)
- Add `CODEOWNERS`
- Enhance `CONTRIBUTING.md` with widget creation checklist and design principles reference

### Internal

- Add crate-level lints: `forbid(unsafe_code)`, `deny(clippy::unwrap_in_result)`, `warn(clippy::unwrap_used)`, `warn(clippy::dbg_macro)`, `warn(clippy::print_stdout)`, `warn(clippy::print_stderr)`, rustdoc link lints
- Add doc coverage CI check (non-blocking) for `missing_docs` tracking
- Add `doc_auto_cfg` — feature-gated items now display their required feature on docs.rs
- Add `cargo-semver-checks` to CI (informational, non-blocking)
- Reduce crates.io package size (exclude `AUDIT-REPORT.md`, `CLAUDE.md`)

## [0.12.10] — 2026-03-17

### Features

- **`flex_center()`**: ContainerBuilder shorthand for `.justify(Center).align(Center)` — center children on both axes in one call
- **`border_x()` / `border_y()`**: ContainerBuilder shorthands for showing only horizontal (left+right) or vertical (top+bottom) borders
- **`text_center()` / `text_right()`**: Text chain shorthands for `.align(Align::Center)` / `.align(Align::End)` — horizontal text alignment
- **`text_color(Color)`**: ContainerBuilder style inheritance — set a default text color that propagates to all child text elements. Individual `.fg()` calls override.
- **`row_gap()` / `col_gap()`**: ContainerBuilder axis-specific gap control. `row_gap(v)` applies to `.col()` containers, `col_gap(v)` applies to `.row()` containers. `.gap()` still sets both.
- **`align_self(Align)`**: ContainerBuilder per-child cross-axis alignment override, like CSS `align-self`. Each child can independently override the parent's `align()`.
- **`truncate()`**: Text chain method for overflow with ellipsis (`…`). Truncates text to its allocated width when it exceeds the container or `.w()` constraint.
- **`ContainerStyle`**: All 7 new properties available as const methods: `text_color()`, `row_gap()`, `col_gap()`, `align_self()` + composable via `.apply()`
- **`demo.rs`**: New "v0.12.10" tab showcasing all 7 features with interactive examples

## [0.12.9] — 2026-03-17

### Features

- **`border_fg(Color)`**: ContainerBuilder shorthand for border foreground color
- **`separator_colored(Color)`**: Separator with custom color
- **`separator()` chaining**: Now returns `&mut Self` — `.fg()`, `.dim()` etc. chainable
- **`help_colored(bindings, key_color, text_color)`**: Help bar with custom key/text colors

## [0.12.8] — 2026-03-17

### Features

- **`kitty_image_fit(rgba, w, h, cols)`**: Aspect-ratio-preserving image display. Height auto-calculated from image ratio. Terminal handles scaling via Kitty protocol `c`/`r` params — no software resize.
- **`normalize_rgba()`**: RGBA data resilience. Short data is zero-padded, long data truncated. Images never fail silently.
- **`kitty_image()` / `kitty_image_fit()`**: Now return `Response` (was `()`) for API consistency.

### Bug Fixes

- **Jennie image not rendering**: `jpeg_decoder` hardcoded 237px height — now preserves original dimensions per image.
- **Kitty image ghost on tab switch**: Delete all previous images before rendering new frame.

## [0.12.7] — 2026-03-17

### Features

- **`kitty_image_fit()`**: Auto-resize + center-crop images to fill container. Nearest-neighbor scaling, no external dependencies.
- **Kitty image cleanup**: Previous-frame images are deleted before rendering new ones (fixes tab-switch ghost images)
- **demo_wiki**: BLACKPINK wiki-style demo with real namu wiki photos via Kitty protocol

## [0.12.6] — 2026-03-17

### Features

- **Kitty graphics protocol**: `kitty_image()` renders pixel-perfect images via Kitty protocol (Ghostty, Kitty, WezTerm)
- **demo_wiki**: BLACKPINK wiki-style demo with Kitty images and tabbed member profiles

## [0.12.5] — 2026-03-17

### Bug Fixes

- **`candlestick()` container sizing fix**: Switched rendering to `ContainerBuilder::draw()` with layout-provided `Rect`, so chart width/height now match the allocated container area instead of caller-provided dimensions.
- **`candlestick()` API update**: Removed explicit `width`/`height` parameters. New signature is `candlestick(candles, up_color, down_color)`.
- **`button_colored()` layout stability**: Unified label format to `[ label ]` and removed custom-bg-dependent text width/style branching that caused focus/hover layout shifts.
- **`demo_trading` stability**: Migrated to the new `candlestick()` API, added `Esc` quit handling, fixed right column/bottom panel heights, and set `page_size = 5` on all synced tables.

## [0.12.4] — 2026-03-16

### Features

- **`WidgetColors`**: New per-widget color override system. Pass `&WidgetColors` to any `_colored()` variant to override theme colors on individual widgets. Theme remains the default fallback.
  ```rust
  let red = WidgetColors::new().fg(Color::White).bg(Color::Red).accent(Color::LightRed);
  ui.button_colored("Delete", &red);
  ```

- **Per-widget `_colored()` methods**: 10 widgets now support individual color customization:
  - `button_colored()`, `text_input_colored()`, `checkbox_colored()`, `toggle_colored()`
  - `progress_bar_colored()`, `tabs_colored()`, `select_colored()`, `radio_colored()`
  - `list_colored()`, `table_colored()`

- **Text size/margin setters**: Text and link elements now support size constraints and margin via style chaining:
  ```rust
  ui.text("hello").w(20).m(1);
  ui.text("padded").mx(2).min_w(10);
  ```
  New methods: `w()`, `h()`, `min_w()`, `max_w()`, `min_h()`, `max_h()`, `m()`, `mx()`, `my()`, `mt()`, `mr()`, `mb()`, `ml()`

- **Color light variants**: 8 new ANSI bright color variants:
  `DarkGray`, `LightRed`, `LightGreen`, `LightYellow`, `LightBlue`, `LightMagenta`, `LightCyan`, `LightWhite`

## [0.12.3] — 2026-03-16

### Chart Rendering Engine Overhaul

Design principle: **"Chart = content, Container = chrome"** — charts render plot area + data decorations; containers handle borders, padding, and titles. Eliminates the Tailwind-style API conflict where chart frames duplicated container borders.

#### Breaking Default Changes
- **`frame_visible` defaults to `false`**: Charts no longer draw their own `┌─┐`/`└─┘` border frame. Use container `.bordered()` for borders. Opt back in with `c.frame(true)`.
- **Histogram title removed**: `histogram()` no longer renders a hardcoded "Histogram" title row. Use container `.title("Histogram")` instead.

#### Rendering Quality
- **X-axis integrated rendering**: Axis tick line and labels merged into a single row — saves 1 row of overhead per chart. ~33% more plot area for small charts.
- **Smarter tick algorithm for small charts**: Plot heights < 4 rows gracefully degrade to min/max boundary ticks instead of producing broken or missing labels. Heights 4–14 allow denser tick spacing (1 row per interval vs 2).
- **Subtler grid**: Default grid color changed from dim white to `Color::Indexed(238)` — grid dots no longer compete visually with data points.
- **Y-label truncation fix**: Vertical y-axis labels (ylabel) hidden when plot area is too short to render them meaningfully, preventing garbled single-character display.

#### Bar Chart Overhaul (ratatui-inspired)
- **Horizontal sub-cell precision**: Bars now use `▏▎▍▌▋▊▉█` for 8x resolution instead of full `█` blocks. Applies to `bar_chart()`, `bar_chart_styled()`, and `bar_chart_grouped()`.
- **`Bar::text_value()`**: Custom display text per bar (e.g., `Bar::new("Q1", 72.0).text_value("72%")`). Falls back to `format_compact_number()` when unset.
- **`Bar::value_style()`**: Override value label styling per bar.
- **`BarChartConfig` builder**: New `bar_chart_with()` and `bar_chart_grouped_with()` APIs with `bar_width`, `bar_gap`, `group_gap`, `max_value`, and `direction` controls.
- **Wide vertical bars**: `bar_width > 1` renders multi-cell bars. `bar_width >= 3` embeds value text inside the bar with inverted colors (ratatui pattern).

#### Bug Fixes
- **Tab click off-by-one**: `tabs()` widget had `interaction_count` incremented 68 lines after capture (all other widgets increment immediately). Caused `prev_hit_map` to reference the wrong rect, making some tabs unclickable.

#### Demo
- **demo_infoviz tabbed layout**: 4-tab navigation (Overview / Lines / Bars / Advanced). Overview shows all chart types at a glance. Detail tabs give each chart full height (~16 rows plot area vs previous 2-3 rows). Bars tab showcases `bar_chart_with(bar_width=3)`, `bar_chart_grouped_with(group_gap=2)`, and `Bar::text_value()`.

## [0.12.2] — 2026-03-16

### Refactor
- **chart.rs split into 6 modules**: `chart.rs` (533 lines) + `chart/render.rs` (485), `chart/grid.rs` (228), `chart/braille.rs` (184), `chart/bar.rs` (181), `chart/axis.rs` (161). No API or logic changes — pure file reorganization following Rust 2018 module pattern.

## [0.12.1] — 2026-03-16

### Chart System Overhaul — matplotlib-level customization

#### New Chart Types
- **`GraphType::Area`**: Area fill rendering — fills below the line to baseline with braille dots. Use via `c.area(&data)` or `area_chart()` / `area_chart_colored()`.
- **`candlestick(candles, w, h, up_color, down_color)`**: OHLC candlestick chart using `│` (wick) and `█` (body) block characters with automatic Y-axis scaling.
- **`heatmap(data, w, h, low_color, high_color)`**: 2D data grid rendered as colored `█` blocks with RGB color blending.

#### New Chart Customization
- **Manual ticks**: `c.xticks(&[0.0, 5.0, 10.0])`, `c.yticks(&[...])` — override auto-computed tick positions.
- **Tick labels**: `c.xtick_labels(&[0.0, 6.0, 11.0], &["Jan", "Jul", "Dec"])` — custom text labels at tick positions.
- **Reference lines**: `c.axhline(50.0, style)`, `c.axvline(5.0, style)` — horizontal/vertical reference lines with custom styling.
- **Direction coloring**: `c.line(&data).color_by_direction(green, red)` — per-segment up/down coloring for price charts.
- **Style overrides**: `c.title_style(style)`, `c.grid_style(style)`, `c.x_axis_style(style)`, `c.y_axis_style(style)`.
- **Visibility toggles**: `c.frame(false)`, `c.x_axis_visible(false)`, `c.y_axis_visible(false)` — hide frame, axes independently.

#### New Convenience Methods
- **`line_chart_colored(data, w, h, color)`**: Line chart with custom color (vs theme.primary).
- **`area_chart(data, w, h)`**: Filled area chart with theme color.
- **`area_chart_colored(data, w, h, color)`**: Filled area chart with custom color.

### New Types
- **`Candle`**: `{ open, high, low, close }` for candlestick data.

## [0.12.0] — 2026-03-16

### Features
- **Custom Backend API**: `pub trait Backend { size, buffer_mut, flush }` — implement custom rendering targets (WebGL, egui, SSH, test harnesses). Pair with `AppState` and `slt::frame()` to drive the render loop from external event loops.
- **`streaming_markdown()`**: New widget combining streaming text with markdown rendering — headings, bold, italic, inline code, bullet lists, code blocks with blinking cursor during streaming.

### Bug Fixes
- **`confirm()` hook panic on tab switch**: Removed internal `use_state()` from `confirm()` widget — was the only widget using internal hooks, causing panic when conditionally rendered across tab switches. Now uses the `result: &mut bool` parameter directly for selection state.

### Improvements
- **`parse_inline_segments()` visibility**: Changed from private to `pub(crate)` — enables inline markdown formatting reuse across widget modules.
- **README architecture section**: Added Custom Backends guide with code example and AI-Native Widgets table.

## [0.11.0] — 2026-03-16

### BREAKING: Response Pattern
- **All widgets return `Response`**: `button()`, `checkbox()`, `toggle()`, `list()`, `table()`, `tabs()`, `select()`, `radio()`, `multi_select()`, `text_input()`, `textarea()`, `accordion()`, `alert()`, `tool_approval()`, and all display/viz widgets now return `Response { clicked, hovered, changed, focused, rect }` instead of `bool`, `&mut Self`, or `()`.
- **Migration**: `if ui.button("x") {` → `if ui.button("x").clicked {`; `ui.checkbox("x", &mut v);` → check `.changed` field.
- `text()`, `styled()`, `link()` are unchanged — they still return the text builder for `.bold().fg()` chaining.
- `command_palette()` is unchanged — still returns `Option<usize>`.

### New Widgets
- **`slider("label", &mut value, range)`**: Horizontal slider for numeric input. Left/Right/h/l to adjust, returns `Response` with `.changed`.
- **`confirm("question?", &mut bool)`**: Yes/No button pair. y/n shortcuts, Tab to switch focus. Returns `Response` with `.clicked` when answered.
- **`file_picker(&mut FilePickerState)`**: Directory browser with Enter to navigate, Backspace to go up, extension filter, hidden file toggle.
- **`notify("message", ToastLevel)`**: App-level toast notification — no external `ToastState` needed. Auto-dismisses after ~3 seconds.
- **`help_from_keymap(&KeyMap)`**: Renders help bar automatically from a `KeyMap` struct.

### New Types
- **`KeyMap`** + **`Binding`**: Declarative key binding management with builder pattern. `.bind('q', "quit")`, `.bind_code(KeyCode::Up, "up")`, `.bind_mod('s', KeyModifiers::CONTROL, "save")`, `.bind_hidden(...)`.
- **`FilePickerState`** + **`FileEntry`**: State for the file picker widget.
- **`Palette`**: Color palette struct with 11 shades (c50–c950).
- **`palette::tailwind`**: 22 Tailwind CSS color palettes (slate through rose) as `const` values. Usage: `slt::palette::tailwind::BLUE.c500`.

### New Features
- **`TextInputState::set_suggestions()`**: Autocomplete dropdown with prefix matching. Tab accepts, Up/Down navigates, Esc closes.
- **`TextInputState::add_validator()`**: Multiple validators with multi-error collection. `.errors()` returns all validation errors.
- **`Context::light_dark(light, dark)`**: Returns the appropriate color based on current theme's dark/light mode.
- **`ListState::set_items()`**: Safe item replacement with automatic view index rebuild.
- **`Rect` helpers**: `.centered(w,h)`, `.union()`, `.intersection()`, `.contains(x,y)`, `.rows()`, `.positions()`.

### Bug Fixes
- **`use_memo` panic messages**: Now include hook index and expected type name (matching `use_state` quality).
- **InlineTerminal background**: `flush()` now respects `theme_bg` via `reset_with_bg()`.
- **`Color::blend()` rounding**: Changed truncation (`as u8`) to rounding (`.round() as u8`). `blend(White, Black, 0.5)` now correctly returns `(128,128,128)`.
- **README signature fixes**: `stat()`, `key_hint()`, `code_block()`, `accordion()` examples corrected.
- **ListState direct mutation crash**: `pkg_list.items = items` without rebuild caused stale view indices. Fixed with `set_items()`.

### Improvements
- **Re-exports**: Easing functions (`ease_in_quad`, `ease_out_bounce`, etc.), `ContainerBuilder`, `Cell`, `Direction`, `Palette` now exported from crate root.
- **Default impls**: `ListState`, `TabsState`, `TableState`, `SelectState`, `RadioState`, `MultiSelectState`, `TreeState`, `CommandPaletteState`, `ToolApprovalState` all implement `Default`.
- **Refactoring**: `table()` (229→3 helpers), `select()` (138→2 helpers), `bar_chart_styled()` (228→2 helpers) split into smaller functions. Vertical nav pattern extracted into shared `handle_vertical_nav()` from 7 widgets.

### Demo
- Consolidated 19 → 14 examples. Removed debug tools (`test_mouse`, `debug_selection`). Absorbed `demo_table` and `demo_ime` into main demo.
- New "v0.11.0" tab in `demo.rs` showcasing all new features.
- All help bars now correctly show `Ctrl+Q` / `Ctrl+T` modifiers.

## [0.10.1] — 2026-03-16

### Performance
- **Cell.symbol**: `String` → `CompactString` — eliminates heap allocation for ≤24-byte symbols (99%+ of terminal cells). Same approach as ratatui.
- **Cell.hyperlink**: `Option<String>` → `Option<CompactString>` — reduces per-cell overhead for hyperlinks.
- **diff+flush inline**: Removed intermediate `Vec<(u32, u32, &Cell)>` allocation in `Terminal::flush()`. Now diffs and writes to stdout in a single pass.
- **reset_with_bg()**: Theme background applied during buffer reset instead of a separate O(w×h) loop per frame.

### Changes
- **MSRV**: 1.74 → 1.81 (required by `compact_str` 0.9)
- **New dependency**: `compact_str` 0.9 (no-default-features) — adds 4 small transitive deps (castaway, ryu, static_assertions, rustversion)

## [0.10.0] — 2026-03-15

### Bug Fixes
- **error_boundary terminal recovery**: panic hook fires before `catch_unwind`, destroying terminal state. Now re-enters raw mode + alternate screen after catching the panic.
- **error_boundary rollback scope**: previously only restored 2 fields (`commands`, `last_text_idx`). Now captures and restores all 13 mutable per-frame fields via `ContextSnapshot` — prevents focus/hook/modal/group state corruption after caught panics.
- **`Theme::light()` dark_mode**: `dark_mode` was hardcoded to `true` regardless of theme. Now reads `theme.is_dark`.

### New API
- **`consume_key(c)` / `consume_key_code(code)`**: explicitly consume a key event, preventing widgets from handling it. Unlike `key()`/`key_code()` which peek without consuming.

### Theme
- **`Theme.is_dark`**: new `pub is_dark: bool` field on `Theme`. All 7 built-in presets set it correctly. `ThemeBuilder` supports `.is_dark(bool)`.
- **`Theme::light()` redesign**: Tailwind slate-based high-contrast palette — `Rgb(15,23,42)` text on `Rgb(248,250,252)` background, blue-600 primary, proper contrast for success/warning/error.
- **Default text color**: `ui.text()` now defaults fg to `theme.text` instead of terminal default. Ensures readability in light mode.
- **Root background fill**: screen background filled with `theme.bg` when not `Color::Reset`.

### DX
- **`#[must_use]` message**: `ContainerBuilder` warning now says "does nothing until .col(), .row(), .line(), or .draw() is called"
- **Documentation fixes**: RunConfig docs corrected from 100ms to 16ms (60fps), README `docs.rs/slt` → `docs.rs/superlighttui`, border style count 4 → 6, removed dead `demo_v050` reference.
- **Clippy clean**: `cargo clippy --all-targets --all-features -- -D warnings` now passes (fixed `collapsible_if`, `field_reassign_with_default`, `saturating_sub`, `if_same_then_else`, `too_many_arguments`, `len_zero`).

### Demo
- Theme-aware colors: hardcoded `Color::Green`/`Color::Red`/`Color::Cyan` replaced with `theme.success`/`theme.error`/`theme.primary` for proper light/dark mode rendering.

## [0.9.5] — 2026-03-15

### Tests
- 15 new widget tests: divider_text, alert (render + dismiss), breadcrumb, accordion (open/closed), badge (render + colored bg), key_hint (reversed), stat (render + trend arrow), definition_list, empty_state, code_block (render + numbered)

### Improvements
- **code_block theme auto-switch**: syntax highlighting adapts to dark/light theme — dark uses One Dark palette, light uses One Light
- **Syntax highlighting multi-language**: keywords for Python, JavaScript, Go added alongside Rust
- **breadcrumb Outline style**: segments use `ButtonVariant::Outline` for cleaner navigation look
- **widgets_viz.rs split**: 3012 → 884 lines. Interactive widgets (list, table, tabs, button, etc.) extracted to `widgets_interactive.rs` (2132 lines)
- **demo_dashboard upgraded**: uses `divider_text`, `stat_trend`, `stat_colored`, `badge_colored`

### Documentation
- README.md updated with v0.9.0-v0.9.4 features
- SLT skill updated with new widget API docs

## [0.9.4] — 2026-03-15

### Features — 10 New Widgets

**Tier 1 (not composable from primitives):**
- **`divider_text(label)`**: horizontal rule with centered text label — `──── Settings ────`
- **`alert(message, AlertLevel)`**: persistent inline notification with icon + dismiss — returns `true` when dismissed
- **`breadcrumb(&["Home", "Settings"])`**: clickable path navigation — returns `Some(idx)` on segment click
- **`accordion(title, &mut open, |ui| { ... })`**: collapsible content section with ▾/▸ toggle

**Tier 2 (convenience widgets):**
- **`badge(label)` / `badge_colored(label, color)`**: inline colored tag with auto-contrast foreground
- **`key_hint(key)`**: inline keyboard shortcut display — `[Ctrl+S]` reversed style
- **`stat(label, value)` / `stat_colored` / `stat_trend`**: dashboard metric with optional trend arrow ↑↓
- **`definition_list(&[("key", "value")])`**: auto-aligned key-value pairs
- **`empty_state(title, desc)` / `empty_state_action`**: centered placeholder for empty lists
- **`code_block(code)` / `code_block_numbered`**: bordered code display with optional line numbers

### New Types
- `AlertLevel` enum: `Info`, `Success`, `Warning`, `Error`
- `Trend` enum: `Up`, `Down`

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
