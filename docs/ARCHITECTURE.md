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

**Source**: `src/context/widgets_*/*.rs`. The directory encodes family:
- `widgets_display/` — text, alerts, badges, code blocks
- `widgets_input/` — text input, textarea, sliders, spinners
- `widgets_interactive/` — tables, lists, tabs, palettes
- `widgets_viz/` — charts, sparklines, heatmaps

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

**Source**: `src/widgets/*.rs` (grouped by family).

**Examples**: `TextInputState`, `TextareaState`, `TabsState`,
`ScrollState`, `SplitPaneState`, `TreeState`.

### Layer 5: Response

**Owns**: interaction results.

**Pattern**: every interactive widget returns `Response` or a compound
`<Widget>Response: Deref<Target = Response>`.

**Source**: `src/widgets/responses.rs` (compound types).

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
Stateless rendering returns `Response::none()` — no `()` returns.

---

## Adding a new widget — checklist

1. **Family**: display, input, interactive, or viz?
2. **Stateful**: needs persistence? If yes, create `<Widget>State` in
   `src/widgets/<family>.rs`.
3. **Return shape**: simple `Response` or compound `<Widget>Response`?
4. **Builder vs immediate**: see [API_DESIGN.md](API_DESIGN.md) rule 1
   (builder when ≥4 optional fields).
5. **Implement** in `src/context/widgets_<family>/<file>.rs`.
6. **Document** per [RUSTDOC_GUIDE.md](RUSTDOC_GUIDE.md) — 4-part
   docstring with at least one runnable example.
7. **Audit**: run `scripts/api_audit.sh`. Update DESIGN_PRINCIPLES.md
   matrix if the new widget changes a cell's status.

---

## Module Map

```
src/
├── lib.rs                      # Crate root, public re-exports, run()/frame() entry points
├── context.rs                  # Facade for core context types + widget impl modules
├── context/
│   ├── state.rs                # State<T>, Response
│   ├── bars.rs                 # BarDirection, Bar, BarChartConfig, BarGroup
│   ├── widget.rs               # Widget trait
│   ├── core.rs                 # Context struct + checkpoint / rollback state
│   ├── container.rs            # ContainerBuilder + CanvasContext
│   ├── runtime.rs              # Core Context methods (hooks, focus, notifications)
│   ├── helpers.rs              # Shared helper functions for widget impls
│   ├── widgets_display.rs      # Display/layout facade
│   ├── widgets_display/
│   │   ├── text.rs             # text, style chains, size/margin helpers
│   │   ├── rich_output.rs      # big_text, image, streaming, tool approval, context bar
│   │   ├── status.rs           # alert, breadcrumb, badge, stat, code_block, empty_state
│   │   └── layout.rs           # screen, row/col, modal, tooltip, container, scrollable, form helpers
│   ├── widgets_interactive.rs  # Interactive facade
│   ├── widgets_interactive/
│   │   ├── collections.rs      # grid, list, calendar, file picker
│   │   ├── selection.rs        # table, tabs, button, checkbox, toggle, select, radio, multi_select
│   │   ├── rich_markdown.rs    # rich_log, virtual_list, command palette, markdown, key_seq
│   │   ├── events.rs           # keyboard, mouse, theme, size, quit helpers
│   │   └── tree_widgets.rs     # tree widget internals
│   ├── widgets_input.rs        # Input facade
│   ├── widgets_input/
│   │   ├── text_input.rs       # text input widget
│   │   ├── feedback.rs         # spinner, toast, slider
│   │   └── textarea_progress.rs # textarea and progress widgets
│   └── widgets_viz.rs          # Charts, sparklines, heatmap, treemap, candlestick, stacked bar, canvas, QR
│
├── widgets.rs                  # Facade for widget state types
├── widgets/
│   ├── input.rs                # StaticOutput, TextInputState, FormField, FormState, ToastState, ToastMessage, ToastLevel, AlertLevel, TextareaState, SpinnerState
│   ├── collections.rs          # ListState, FilePickerState, TabsState, TableState, ScrollState
│   ├── feedback.rs             # RichLogState, RichLogEntry, CalendarState, ButtonVariant, Trend
│   ├── selection.rs            # SelectState, RadioState, MultiSelectState, TreeState, DirectoryTreeState, PaletteCommand
│   └── commanding.rs           # CommandPaletteState, streaming states, ScreenState, ModeState, tool approval types, ContextItem
│
├── layout.rs                   # Thin facade re-exporting layout kernels
├── layout/
│   ├── command.rs              # Command enum recorded by Context
│   ├── tree.rs                 # LayoutNode, NodeKind, build_tree(), wrap helpers
│   ├── collect.rs              # collect_all(), FrameData, raw-draw collection helpers
│   ├── flexbox.rs              # compute(), layout_row(), layout_column(), gap/grow/shrink resolution
│   ├── render.rs               # render(), render_inner(), render_container_border(), clipping, viewport culling
│   └── tests.rs                # Layout-focused kernel tests
│
├── style.rs                    # Style struct, Border, Padding, Margin, Constraints, Modifiers, Align, Justify
├── style/
│   ├── color.rs                # Color enum (Named, Indexed, Rgb), ColorDepth, color blending
│   └── theme.rs                # Theme struct, Spacing, ThemeColor, 10 presets, ThemeBuilder, contrast helpers
│
├── terminal.rs                 # Terminal backend
│   ├── Terminal                # Full-screen mode — alternate screen, raw mode, mouse capture
│   ├── InlineTerminal          # Inline mode — renders below cursor, no alternate screen
│   └── ANSI output, synchronized output (DECSET 2026), event polling
│
├── terminal/
│   └── selection.rs            # SelectionState, text selection overlay rendering
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
├── cell.rs                     # Cell = char + Style + optional URL
├── rect.rs                     # Rect struct, bounds checking, intersection
├── event.rs                    # Event, KeyCode, KeyModifiers, MouseEvent, MouseButton
├── halfblock.rs                # Half-block (▀▄) image rendering
├── keymap.rs                   # KeyMap, Binding structs
├── palette.rs                  # 256-color palette definitions
└── test_utils.rs               # TestBackend, EventBuilder for headless testing
```

---

## Frame Lifecycle

Every frame follows this exact sequence. The engine performs **four top-level DFS traversals** of the layout tree per frame (build → layout → collect → render), plus optional passes for the debug overlay.

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

Net effect: **10 traversals per frame → 4**. That is the real story; the marketing line "single DFS" was shorthand for "collect no longer does seven walks" and understated what is actually a four-stage pipeline.

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
lib.rs (entry point)
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
  ├── terminal.rs ←── terminal/selection.rs
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
- Widget facades under `src/context/widgets_*.rs` now act as indexes for narrower implementation files

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

**The public API is defined by `lib.rs` re-exports.** Users should never need deep imports like `slt::context::widgets_display::...`. If something is public, it must be re-exported from the crate root.

### Why This Matters for Semver

Every `pub use` in `lib.rs` is a semver commitment. Adding a re-export is non-breaking. Removing one is breaking. Be deliberate about what gets re-exported.

---

## File Conventions

- **Module pattern**: `module.rs` + `module/` directory (Rust 2018 style, NOT `mod.rs`)
- **Submodule imports**: `use super::*;` to access parent types
- **Splitting safety**: When splitting a file, keep `#[derive(...)]` and `#[cfg_attr(...)]` attached to their type definitions — they must not get separated by the split boundary
