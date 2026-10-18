# SLT Architecture

This document describes how the code is organized and how data flows
through the system. It is the **Macro tier** of SLT's convention stack —
see [`DESIGN_PRINCIPLES.md`](DESIGN_PRINCIPLES.md) §4 for the full
4-tier map. For naming and signature conventions, see
[`NAMING.md`](NAMING.md) and [`API_DESIGN.md`](API_DESIGN.md).

Related docs:
- [QUICK_START.md](QUICK_START.md)
- [WIDGETS.md](WIDGETS.md)
- [PATTERNS.md](PATTERNS.md)
- [EXAMPLES.md](EXAMPLES.md)
- [BACKENDS.md](BACKENDS.md)
- [DEBUGGING.md](DEBUGGING.md)
- [TESTING.md](TESTING.md)

---

## The 5 Layers

Every public method belongs to exactly one of five layers. The layer
determines the file the method lives in, what it can mutate, and what
shape it returns.

```
┌─────────────────────────────────────────────────────────┐
│ 1. Context                  (frame state + events)       │
│    └── 2. ContainerBuilder  (layout + style chain)       │
│         └── 3. Widget       (text, button, gauge, ...)   │
│              └── 4. State   (XxxState; persists)         │
│                   └── 5. Response (XxxResponse: Deref)   │
└─────────────────────────────────────────────────────────┘
```

The arrow is "calls into" — Context creates ContainerBuilders,
ContainerBuilders host Widgets, Widgets read & write State, Widgets
return Response.

### Layer 1: Context

**Owns**: frame buffer, hooks (`use_state`, `use_effect`), focus state,
event queue, theme + spacing, command list, modal stack, name maps.

**Methods belong here when**: they affect frame-level state (not visual
output) or they orchestrate (focus, quit, notify).

**Source**: `src/context/core.rs`, `src/context/runtime.rs`.

**Examples**:

```rust
ui.quit();
ui.notify("saved", ToastLevel::Success);
ui.register_focusable_named("search");
ui.focus_by_name("search");
let count = ui.use_state(|| 0);
let theme = ui.theme();
```

### Layer 2: ContainerBuilder

**Owns**: layout configuration (col/row/line direction, gap, grow),
styling (border, padding, background, foreground), per-subtree theme
override, alignment.

**Lifecycle**: created via `ui.container()` or shortcut entry points
(`ui.bordered(B)`, `ui.col(...)`, `ui.row(...)`, `ui.row_gap(g, ...)`),
mutated via chained methods, finalized by `.col(closure)` /
`.row(closure)` / `.line(closure)` / `Drop`.

**Source**: `src/context/container.rs`.

**Examples**:

```rust
ui.container().border(Border::Rounded).p(2).gap(1).grow(1).col(|ui| { /* ... */ });
ui.bordered(Border::Single).title("Settings").p(1).col(|ui| { /* ... */ });
ui.container().theme(Theme::dark()).fill().col(|ui| { /* ... */ });
```

### Layer 3: Widget

**Owns**: rendering primitives — text, buttons, gauges, tables, charts,
inputs.

**Source**: `src/context/widgets/<family>.rs` (plus a `<family>/` directory
when the family is large). The file name is the widget family; see
[Widget Families](#widget-families) for the full table.

**Stateless widgets** take primitive args:
```rust
ui.text("hello");
ui.gauge(0.6).label("60%").width(24);
ui.button("Save");
```

**Stateful widgets** take `&mut <Widget>State`:
```rust
let mut input = TextInputState::default();
ui.text_input(&mut input);
```

### Layer 4: State

**Owns**: persistent per-widget state.

**Pattern**: `pub struct <Widget>State { ... }` with `Default` impl. Field
visibility is `pub` for trivial fields and `pub(crate)` for invariants.

**Source**: `src/widgets/<family>.rs` — the same family names as the
`Context` method files.

**Examples**: `TextInputState`, `TextareaState`, `TabsState`,
`ScrollState`, `SplitPaneState`, `TreeState`.

### Layer 5: Response

**Owns**: interaction results.

**Pattern**: every interactive widget returns `Response` or a compound
`<Widget>Response: Deref<Target = Response>`.

**Source**: `src/widgets/<family>.rs`, next to the state of the family that
returns them (e.g. `GaugeResponse` in `feedback.rs`, `SplitPaneResponse` in
`layout.rs`).

**Why Deref**: callers can write `r.hovered`, `r.rect` regardless of
whether `r` is `Response` or `BreadcrumbResponse`. The compound fields
(`r.clicked_segment`) are accessed normally.

---

## Layer Cross-Cutting Rules

### M1 — One method, one layer

A method must not exist on more than one layer with the same name and similar
semantics unless the overlap is an explicitly reviewed ergonomic exception.
`scripts/api_audit.sh --strict` rejects every unapproved overlap in CI; the V1
allowlist in that script is the machine-readable source of approved exceptions.

**Representative approved exceptions**:

| Name | Context | Builder | Resolution |
|------|---------|---------|------------|
| `text` | unbordered shortcut | inside-builder form | both keep |
| `theme` | getter | per-subtree override | both keep (different semantics) |

Fluent layout modifiers and container finalizers may also appear at both layers
when their return type and composition semantics differ. Every new overlap still
requires review and an explicit V1 allowlist entry; the table above is illustrative,
not the complete allowlist.

**Documented ergonomic shortcut**: `ui.bordered(...)` remains a convenience
entry point for `ui.container().border(...)`. New low-level examples prefer the
explicit builder form; neither path is deprecated in v0.23.

### M2 — Composition, not inheritance

A widget that wants container-like behaviour uses `ContainerBuilder` via
`ui.container()`, not by re-implementing layout. Compound widgets like
`code_block` internally call `self.bordered(...).col(|ui| ...)`.

### M3 — State is `&mut` always

No widget mutates frame state through `&self`. State changes go through
the `&mut Context` parameter or `&mut <Widget>State`.

### M4 — Response is the only return shape

Interactive widgets return `Response` or a compound `<Widget>Response: Deref`.
Display chains (`text`, `link`, `line`, `form_field`, `toast`, ...) return
`&mut Self` instead. Known `()` returns: `screen`, overlay helpers such
as `keymap_help_overlay`, and hook/side-effect methods.

---

## Adding a new widget — checklist

1. **Family**: pick one from [Widget Families](#widget-families).
2. **Stateful**: needs persistence? If yes, create `<Widget>State` in
   `src/widgets/<family>.rs`.
3. **Return shape**: simple `Response` or compound `<Widget>Response`?
4. **Builder vs immediate**: see [API_DESIGN.md](API_DESIGN.md) rule 1
   (builder when ≥4 optional fields).
5. **Implement** in `src/context/widgets/<family>.rs` (or a file in its
   `<family>/` directory) — the same family name as the state file.
6. **Document** per [RUSTDOC_GUIDE.md](RUSTDOC_GUIDE.md) — 4-part
   docstring with at least one runnable example.
7. **Audit**: run `scripts/api_audit.sh`. Update DESIGN_PRINCIPLES.md
   matrix if the new widget changes a cell's status.

## Widget Families

Every widget belongs to exactly one family. The family name is shared by
the state file and the `Context` method file, so a widget's state and its
render method are always found under the same name:

| Family | State (`src/widgets/<family>.rs`) | Methods (`src/context/widgets/<family>.rs` + dir) |
|---|---|---|
| `text_input` | `TextInputState`, `Validator`, `AsyncValidation`, `ValidateTrigger`, `FormField`, `FormState`, `FormFieldResponse`, `TextareaState`, `NumberInputState`, `CodeEditorState` (validators in `widgets/validators.rs`, code editor state in `widgets/text_input/code_editor.rs`) | `text_input`, `textarea`, `number_input`, `form`, `form_field`, `form_field_response`, `form_submit`, `code_editor` |
| `selection` | `SelectState`, `RadioState`, `MultiSelectState`, `TabsState`, `ButtonVariant`, `PickerMode`, `ColorPickerState` | `tabs`, `button`, `button_with`, `checkbox`, `toggle`, `select`, `radio`, `multi_select`, `color_picker` |
| `collections` | `ListState`, `ListResponse`, `FilePickerState`, `FileEntry`, `TableState`, `TableColumn`, `PaginatorState`, `PaginatorStyle`, `TreeNode`, `TreeState`, `DirectoryTreeState`, `CalendarState`, `CalDate`, `CalendarSelect` | `list`, `list_reorderable`, `virtual_list`, `table`, `paginator`, `tree`, `directory_tree`, `calendar`, `file_picker` |
| `feedback` | `ToastState`, `ToastMessage`, `ToastLevel`, `AlertLevel`, `SpinnerState`, `SpinnerPreset`, `SliderOpts`, `GaugeResponse` | `spinner`, `toast`, `slider`, `alert`, `confirm`, `progress`, `progress_bar`, `gauge`, `line_gauge` |
| `rich` | `RichLogState`, `RichLogEntry`, `StreamingTextState`, `StreamingMarkdownState`, `ApprovalAction`, `ToolApprovalState`, `ContextItem`, `DiffState`, `DiffResponse`, `DiffLayout`, `DiffKeys`, `DiffColors`, `HunkId`, `HunkAction`, `HunkDecision`, `DiffParseError`, `DiffFile`, `DiffHunk`, `DiffLine` (diff state in `widgets/rich/diff.rs`) | `rich_log`, `markdown`, `streaming_text`, `streaming_markdown`, `tool_approval`, `context_bar`, `code_block`, `diff` |
| `commanding` | `ChordState`, `CommandPaletteState`, `PaletteCommand`, `ScreenState`, `ModeState` | `screen`, `push_screen` / `pop_screen` / `reset_screen`, `command_palette`, `key_chord`, `key_seq` |
| `layout` | `ScrollState`, `HighlightRange`, `SplitPaneState`, `GridColumn`, `SplitPaneResponse`, `GutterResponse` | `col` / `row` / `line`, `separator`, `group`, `container`, `scrollable`, `scrollbar`, `bordered`, `grid`, `split_pane`, `scrollable_with_gutter` |
| `display` | `Trend`, `BreadcrumbResponse` | `text` and style chains, `link`, `timer_display`, `help`, `badge`, `key_hint`, `stat`, `breadcrumb`, `accordion`, `definition_list`, `divider_text`, `empty_state` |
| `overlay` | `MenuState`, `MenuItem`, `MenuBarState`, `PopoverResponse` | `modal`, `modal_with` / `ModalOpts`, `overlay`, `overlay_at(_offset)`, `modal_at(_offset)` (deprecated), `popover` / `Placement`, `context_menu`, `menu_bar`, `tooltip`, `Anchor` |
| `media` | — | `big_text`, `image`, `kitty_image(_fit)`, `sixel_image`, `iterm_image(_fit)` |
| `viz` | — (bar data types in `context/bars.rs`, chart config in `crate::chart`) | `viz/bars.rs`, `viz/charts.rs`, `viz/heat_tree.rs`, `viz/canvas.rs`, `viz/qr.rs` |
| `events` | — | keyboard / mouse / paste / scroll queries, theme, size, tick, quit, debug toggles |
| `runtime` | `StaticOutput`, `SchedulerState` (non-widget runtime state) | `src/context/runtime.rs` |

Borderline assignments: `ButtonVariant` is `selection` because only
`button_with` consumes it; `Trend` is `display` (`stat_trend`);
`SchedulerState` and `StaticOutput` are runtime state rather than widget
state, so they live in `widgets/runtime.rs` next to `context/runtime.rs`;
compound `*Response` types live with the family that returns them.

---

## Module Map

```
src/
├── lib.rs                      # Crate root: docs, mod decls, public re-exports, Backend / AppState / frame()
├── runtime.rs                  # Private run-loop facade; entry points re-exported from lib.rs
├── runtime/
│   ├── config.rs               # RunConfig + builder methods
│   ├── frame_state.rs          # FrameState, FocusState, LayoutFeedbackState, DiagnosticsState, DebugLayer
│   ├── kernel.rs               # run_frame_kernel, run_frame, input_frame, run-loop event handling
│   ├── native.rs               # run / run_with / run_inline* / run_static*, event polling (`crossterm`)
│   ├── async_run.rs            # run_async*, AsyncSender, AsyncRunHandle (`async`)
│   ├── panic.rs                # Panic hook + recoverable-panic scopes
│   ├── suspend.rs              # Ctrl+Z suspend / resume (`crossterm`, unix)
│   └── tests.rs                # Run-loop tests
├── context.rs                  # Facade for core context types + widget impl modules
├── context/
│   ├── state.rs                # State<T>, Response
│   ├── bars.rs                 # BarDirection, Bar, BarChartConfig, BarGroup
│   ├── widget.rs               # Widget trait
│   ├── core.rs                 # Context struct + checkpoint / rollback state
│   ├── container.rs            # ContainerBuilder + CanvasContext
│   ├── runtime.rs              # Core Context methods (hooks, focus, notifications)
│   ├── helpers.rs              # Shared helper functions for widget impls, previous-frame geometry queries
│   ├── async_tasks.rs          # In-frame async task API (spawn/poll, TaskHandle) — `async` feature
│   ├── tests.rs, async_tasks_tests.rs, scheduler_tests.rs  # Context unit tests
│   ├── widgets.rs              # Widget-method facade: one `<family>` module per widget family (see Widget Families)
│   └── widgets/
│       ├── display.rs          # Display family facade
│       ├── display/
│       │   ├── text.rs         # text, link, timer_display, help_from_keymap, style chains, size/margin helpers
│       │   ├── status.rs       # help, breadcrumb, accordion, definition_list, divider_text, badge, key_hint, stat, empty_state
│       │   └── tests.rs        # Display widget tests
│       ├── layout.rs           # separator, col/row/line/line_wrap, group, container, scrollable, scrollbar, bordered
│       ├── layout/
│       │   ├── grid.rs         # grid / grid_with
│       │   ├── gutter.rs       # scrollable_with_gutter, GutterOpts
│       │   ├── split.rs        # split_pane / vsplit_pane
│       │   └── line_wrap_tests.rs
│       ├── overlay.rs          # Anchor, ModalOpts, modal*, overlay*, tooltip
│       ├── overlay/
│       │   ├── popover.rs      # Placement, flip/clamp placement, popover, floating panels
│       │   ├── menu.rs         # context_menu, menu_bar
│       │   └── tooltip_wrap.rs # tooltip word wrapping
│       ├── commanding.rs       # screen stack, command_palette, key_chord / key_seq
│       ├── text_input.rs       # text_input, form / form_field / form_submit
│       ├── text_input/
│       │   ├── textarea.rs     # textarea
│       │   ├── number_input.rs # number_input
│       │   └── tests.rs        # Text-input family tests
│       ├── selection.rs        # tabs, button, checkbox, toggle, select, radio, multi_select, color_picker
│       ├── collections.rs      # Collections family facade
│       ├── collections/
│       │   ├── list.rs         # list, list_reorderable, virtual_list(_variable)
│       │   ├── table.rs        # table, paginator
│       │   ├── tree.rs         # tree, directory_tree
│       │   ├── calendar.rs     # calendar
│       │   └── file_picker.rs  # file_picker
│       ├── feedback.rs         # spinner, toast, slider, alert, confirm, progress / progress_bar
│       ├── feedback/
│       │   ├── gauge.rs        # gauge / line_gauge builders
│       │   └── tests.rs        # Feedback family tests
│       ├── rich.rs             # Rich-content family facade
│       ├── rich/
│       │   ├── rich_log.rs     # rich_log
│       │   ├── markdown.rs     # markdown and inline-markdown helpers
│       │   ├── streaming.rs    # streaming_text, streaming_markdown, tool_approval, context_bar
│       │   ├── code_block.rs   # code_block builder
│       │   ├── fallback_highlight.rs # keyword fallback highlighter, tree-sitter line renderer
│       │   └── tests.rs        # Rich-content family tests
│       ├── media.rs            # big_text, image, kitty_image, sixel_image, iterm_image
│       ├── media/
│       │   ├── prepare.rs      # image preparation caches, encoders, half-block / RGBA fallback
│       │   ├── font8x8.rs      # 8x8 bitmap font for big_text
│       │   └── tests.rs        # Media family tests
│       ├── viz.rs              # Viz family facade + shared viz-root helpers
│       ├── viz/
│       │   ├── bars.rs         # bar_chart, grouped and stacked bars
│       │   ├── charts.rs       # sparkline, line/area, candlestick(_hd), chart, scatter, histogram
│       │   ├── heat_tree.rs    # heatmap, heatmap_halfblock, treemap, TreemapItem
│       │   ├── canvas.rs       # canvas / try_canvas / canvas_with
│       │   ├── qr.rs           # qr_code
│       │   └── tests.rs        # Viz family tests
│       └── events.rs           # keyboard, mouse, paste, scroll, theme, size, tick, quit, debug helpers
│
├── widgets.rs                  # Facade: private family submodules below, glob re-exported (`pub use text_input::*;` …)
│                               # so every state type stays at `slt::widgets::*`; `validators` is `pub mod`
├── widgets/
│   ├── display.rs              # Trend, BreadcrumbResponse
│   ├── layout.rs               # ScrollState, HighlightRange, SplitPaneState, GridColumn, SplitPaneResponse, GutterResponse
│   ├── overlay.rs              # MenuItem, MenuState, MenuBarState, PopoverResponse
│   ├── commanding.rs           # ChordState, CommandPaletteState, PaletteCommand, ScreenState, ModeState
│   ├── text_input.rs           # TextInputState, Validator, AsyncValidation, ValidateTrigger, FormField, FormState, FormFieldResponse, TextareaState, NumberInputState
│   ├── selection.rs            # SelectState, RadioState, MultiSelectState, TabsState, ButtonVariant, PickerMode, ColorPickerState
│   ├── collections.rs          # ListState, ListResponse, FilePickerState, FileEntry, TableState, TableColumn, PaginatorState, TreeState, TreeNode, DirectoryTreeState, CalendarState, CalDate
│   ├── feedback.rs             # ToastState, ToastMessage, ToastLevel, AlertLevel, SpinnerState, SpinnerPreset, SliderOpts, GaugeResponse
│   ├── rich.rs                 # RichLogState, RichLogEntry, StreamingTextState, StreamingMarkdownState, ToolApprovalState, ApprovalAction, ContextItem
│   ├── runtime.rs              # Non-widget runtime state: StaticOutput, SchedulerState
│   ├── validators.rs           # Built-in FormField validators (`slt::validators`)
│   └── tests.rs                # Widget state tests (`mod tests`)
│
├── layout.rs                   # Thin facade re-exporting layout kernels
├── layout/
│   ├── command.rs              # Command enum recorded by Context
│   ├── tree.rs                 # LayoutNode, NodeKind, build_tree(), wrap helpers
│   ├── collect.rs              # collect_all(), FrameData, raw-draw collection helpers
│   ├── flexbox.rs              # compute(), layout_row(), layout_column(), gap/grow/shrink resolution
│   ├── focus.rs                # Layout clip, focus/scroll geometry feedback, follow-focus scroll adjustments
│   ├── render.rs               # render(), render_inner(), render_container_border(), clipping, viewport culling
│   └── tests.rs                # Layout-focused kernel tests
│
├── style.rs                    # Style struct, Border, Padding, Margin, Constraints, Modifiers, Align, Justify
├── style/
│   ├── color.rs                # Color enum (Named, Indexed, Rgb), ColorDepth, color blending
│   ├── theme.rs                # Theme struct, Spacing, ThemeColor, 11 color presets + 3 density variants, ThemeBuilder, contrast helpers
│   └── theme_io.rs             # TOML ThemeFile loading (`serde`) and ThemeWatcher hot reload (`theme-watch`)
│
├── terminal.rs                 # Terminal backend facade: buffer helpers, Sink, Terminal, InlineTerminal
│   ├── Terminal                # Full-screen mode — alternate screen, raw mode, mouse capture
│   ├── InlineTerminal          # Inline mode — renders below cursor, no alternate screen
│   └── submodule re-exports    # Capabilities, ColorScheme, clipboard, bench hooks
│
├── terminal/
│   ├── kitty.rs                # KittyImageManager: upload dedup, placement diff, compression
│   ├── cell_pixel.rs           # CSI 16 t cell-pixel probe + cache
│   ├── capabilities.rs         # Capabilities, BlitterSupport, Blitter, host / multiplexer / env policy
│   ├── query.rs                # Reply reader, DA1/DA2/DECRPM/XTGETTCAP parsing, sync-output resolution
│   ├── session.rs              # Session registry/guard, enter/exit/cleanup writers, panic cleanup, suspend/resume
│   ├── osc.rs                  # OSC 11 color scheme, OSC 52 clipboard, base64
│   ├── flush.rs                # flush_buffer_diff*, cursor, SGR style delta
│   ├── sprixel.rs              # Graphics emission policy, raw-sequence + sprixel flush
│   ├── bench_hooks.rs          # `#[doc(hidden)]` __bench_* fixtures
│   ├── selection.rs            # SelectionState, text selection overlay rendering
│   ├── tests.rs                # Test facade
│   └── tests/                  # capability / detection / flush / graphics / lifecycle / query / selection tests
│
├── anim.rs                     # Animation primitives
│   ├── Tween                   # Linear interpolation with 9 easing functions
│   ├── Spring                  # Physics-based spring animation
│   ├── Keyframes               # Timeline with stops and loop modes
│   ├── Sequence                # Chained tween segments
│   └── Stagger                 # Delayed animation for list items
│
├── chart.rs                    # ChartBuilder, ChartConfig, Dataset, Marker
├── chart/
│   ├── render.rs               # Chart rendering
│   ├── axis.rs                 # TickSpec, tick generation and formatting helpers
│   ├── bar.rs                  # Bar chart rendering
│   ├── grid.rs                 # Grid lines
│   └── braille.rs              # Braille dot patterns for line/scatter charts
│
├── buffer.rs                   # Double-buffer with clip stack and diff tracking
├── syntax.rs                   # Tree-sitter-based syntax highlighting helpers
├── sixel.rs                    # Sixel image protocol support
├── iterm.rs                    # iTerm2 OSC 1337 inline-image encoder
├── clock.rs                    # Platform clocks for frame timing, input and scheduling (native + wasm)
├── cell.rs                     # Cell = one grapheme (CompactString symbol) + Style + optional OSC 8 hyperlink
├── rect.rs                     # Rect struct, bounds checking, intersection
├── event.rs                    # Event, KeyCode, KeyModifiers, MouseEvent, MouseButton
├── event/
│   ├── reply.rs                # Terminal query replies; preserves input consumed during probes (`crossterm`)
│   ├── native.rs               # Owned nonblocking Unix input reader (`crossterm`, unix)
│   ├── native/parse.rs         # Escape-sequence parser adapted from Crossterm 0.28.1 (unchanged in 0.29)
│   └── native/lifecycle_tests.rs, reply/review_tests.rs  # Input tests
├── halfblock.rs                # Half-block (▀▄) image rendering
├── keymap.rs                   # KeyMap, Binding structs
├── palette.rs                  # 256-color palette definitions
└── test_utils.rs               # TestBackend, EventBuilder for headless testing
```

---

## Frame Lifecycle

Every frame follows this exact sequence. The engine performs **four top-level DFS traversals** of the layout tree per frame (build → layout → collect → render) in the common case. When follow-focus scrolling moves a viewport (v0.25), it adds two more walks — `apply_scroll_adjustments()` and a second `collect_all()` — so rendering and hit-testing share the adjusted tree. The debug overlay adds optional passes on top.

```
1. EVENT POLL
   └── Terminal polls for keyboard/mouse events (non-blocking)
   └── Events stored in Context for widget consumption

2. USER CLOSURE
   └── User's closure runs: ui.text(), ui.button(), ui.col(), etc.
   └── Each call pushes a Command to Context's internal command list
   └── No layout is computed yet — just recording intent

3. POST-CLOSURE NORMALIZATION
   └── process_focus_keys()
   └── render_notifications()
   └── emit_pending_tooltips()
   └── Scoped stacks settle before layout; quit can short-circuit here

4. BUILD TREE — build_tree()                         [DFS pass 1 of 4]
   └── Flat Command list → nested LayoutNode tree
   └── Parent-child relationships resolved from open/close markers

5. FLEXBOX LAYOUT — flexbox::compute()               [DFS pass 2 of 4]
   └── layout_row() / layout_column() walk the tree
   └── Resolves: sizes, gaps, grow factors, min/max constraints
   └── Breakpoint-conditional styles evaluated against terminal width

6. COLLECT ALL — collect_all()                       [DFS pass 3 of 4]
   └── One DFS over the laid-out tree; returns a FrameData bundle
   └── Gathers, in a single walk: scroll regions, hit areas, group rects,
       content rects, focus rects/groups, raw-draw viewport rects
   └── See "The collect_all consolidation" below for what this replaced
   └── Follow-focus check: if a focus transition or fresh bounds require a
       scroll correction, apply_scroll_adjustments() rewrites the scroll
       offsets in place and collect_all() runs once more (src/runtime/kernel.rs)

7. RENDER + DEFERRED DRAW — layout::render()         [DFS pass 4 of 4]
   └── render() → render_inner() → render_container_border()
   └── Writes Cell values to the back buffer
   └── Clip stack ensures children don't overflow parent bounds
   └── Viewport culling: nodes fully outside the viewport are skipped
   └── Deferred raw-draw callbacks replay into collected raw-draw rects

8. DIFF + FLUSH
   └── Compare front buffer (previous frame) vs back buffer (current frame)
   └── apply_style_delta() — only emit ANSI attributes that changed
   └── Synchronized output (DECSET 2026) prevents tearing on supported terminals
   └── Swap front ↔ back buffers

9. DEBUG OVERLAY (optional, F12)
   └── render_debug_overlay() adds 1–2 extra DFS passes when enabled
   └── Off by default; pure diagnostic path
```

For the custom-backend entry point that drives this lifecycle manually, see `docs/BACKENDS.md`.
Terminal-owned run loops add selection overlay and clipboard handling around the shared kernel before the final flush.

### The `collect_all` consolidation

The per-frame DFS count used to be higher. Before consolidation, the collect phase performed seven independent tree walks — one each for scroll regions, hit areas, group rects, content rects, focus rects, focus groups, and raw-draw viewport rects. That was 7 DFS traversals stacked on top of the build, layout, and render walks — 10 traversals per frame in total.

`collect_all()` folds those seven collect-phase walks into **one** DFS that produces a single `FrameData` struct holding every vector the runtime needs for the next frame's hit-testing and scroll feedback. The top-level pipeline is still four DFS passes (build, layout, collect, render) — `collect_all` did not fuse the phases, it fused the sub-walks inside one phase.

Net effect: **10 traversals per frame → 4** (6 on frames where follow-focus scrolling corrects a viewport). That is the real story; the marketing line "single DFS" was shorthand for "collect no longer does seven walks" and understated what is actually a four-stage pipeline.

---

## One-Frame Delay Feedback

Layout-computed data feeds back to the **next** frame via settled `prev_*` fields on `Context`, sourced from session state carried between frames:

```
Frame N:   closure runs → layout computed → focus_count, hit_areas, scroll_bounds stored
                                            ↓
Frame N+1: closure reads prev_focus_count, prev_hit_areas → makes decisions
```

This is an intentional design choice of immediate-mode UI:
- Widget positions are not known until layout runs (after the closure)
- So interaction checks (hover, click) use positions from the previous frame
- This introduces a one-frame delay that is imperceptible at 60 FPS

Interactive widgets depend on `prev_*` data for hit testing, scroll bounds, and focus count.

---

## Module Dependency Flow

```
lib.rs (entry point) ←── runtime.rs ←── runtime/*.rs (run loops, frame kernel)
  ├── context.rs ←── context/widgets_*.rs (impl blocks on Context)
  │     ↑
  │     ├── widgets.rs (state types)
  │     ├── style.rs ←── style/color.rs, style/theme.rs
  │     ├── layout.rs ←── layout/command.rs, layout/tree.rs, layout/collect.rs, layout/flexbox.rs, layout/render.rs
  │     ├── buffer.rs ←── cell.rs
  │     ├── anim.rs
  │     ├── event.rs
  │     └── rect.rs
  │
  ├── terminal.rs ←── terminal/{kitty, cell_pixel, capabilities, query, session, osc, flush, sprixel, bench_hooks, selection}.rs
  │     ↑
  │     └── buffer.rs, event.rs (for flush and polling)
  │
  └── chart.rs ←── chart/render.rs, chart/axis.rs, chart/bar.rs, chart/grid.rs, chart/braille.rs
```

Key observations:
- `context.rs` stays the public hub, but heavy logic is now split into smaller files under `src/context/`
- `widgets.rs` stays the public state catalog, but the concrete state types are grouped under `src/widgets/`
- `terminal.rs` is isolated — it only knows about `buffer` and `event`
- `layout.rs` is now only a facade; the real kernels live under `src/layout/`
- `style`, `layout`, `anim` are largely independent of each other
- `src/context/widgets.rs` indexes the widget families; each family name matches a state file in `src/widgets/`

- The `Backend` / `AppState` / `frame()` path in `src/lib.rs` is the low-level escape hatch when SLT does not own the event loop

---

## Visibility Rules

| Visibility | Use when | Example |
|------------|----------|---------|
| `pub` | Part of the user-facing API | `pub fn text()`, `pub struct Style` |
| `pub(crate)` | Shared across modules, not for users | `pub(crate) struct FrameData` |
| `pub(super)` | Shared with parent module's submodules only | `pub(super) fn render_border()` |
| Private (no modifier) | Implementation detail within a single file | Helper functions, internal state |

### Re-export Rule

**The public API is defined by `lib.rs` re-exports.** Users should never need deep imports like `slt::context::widgets::display::...`. If something is public, it must be re-exported from the crate root.

### Why This Matters for Semver

Every `pub use` in `lib.rs` is a semver commitment. Adding a re-export is non-breaking. Removing one is breaking. Be deliberate about what gets re-exported.

---

## File Conventions

- **Module pattern**: `module.rs` + `module/` directory (Rust 2018 style, NOT `mod.rs`)
- **Submodule imports**: `use super::*;` to access parent types
- **Splitting safety**: When splitting a file, keep `#[derive(...)]` and `#[cfg_attr(...)]` attached to their type definitions — they must not get separated by the split boundary
