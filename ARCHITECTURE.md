# SLT Architecture

This document describes how the code is organized and how data flows through the system.
For design philosophy and conventions, see [DESIGN_PRINCIPLES.md](DESIGN_PRINCIPLES.md).

---

## Module Map

```
src/
├── lib.rs                      # Crate root
│   ├── Re-exports (public API surface)
│   ├── Backend trait
│   ├── AppState, RunConfig
│   └── run(), run_with(), run_inline(), run_async(), frame()
│
├── context.rs                  # The "UI handle" — passed to user closures
│   ├── Context struct (25+ fields: layout, focus, scroll, animation, hooks, theme, events, debug)
│   ├── ContainerBuilder        # Fluent builder for row/col/grid containers
│   ├── Response                # Widget interaction result { clicked, hovered, changed, focused, rect }
│   └── State<T> / use_state()  # Hook system for component-local state
│
├── context/
│   ├── widgets_display.rs      # impl Context: text, styled, button, tabs, modal, overlay, markdown, code_block...
│   ├── widgets_interactive.rs  # impl Context: list, table, select, radio, multi_select, tree, virtual_list...
│   ├── widgets_input.rs        # impl Context: text_input, textarea, form_field, validation
│   └── widgets_viz.rs          # impl Context: chart, bar_chart, sparkline, histogram, canvas, scatter, candlestick...
│
├── widgets.rs                  # State structs: TextInputState, TableState, ListState, SelectState, TabsState...
│                               # (30+ state types — one per interactive widget)
│
├── layout.rs                   # Layout engine
│   ├── Command enum            # Flat representation of UI calls
│   ├── LayoutNode              # Tree node with resolved children
│   ├── build_tree()            # Command list → LayoutNode tree
│   └── collect_all()           # Single DFS pass — collects focus, scroll, hits, animations, draws, modals, toasts
│
├── layout/
│   ├── flexbox.rs              # compute(), layout_row(), layout_column(), gap/grow/shrink resolution
│   └── render.rs               # render(), render_inner(), render_border(), clipping, viewport culling
│
├── style.rs                    # Style struct, Border, Padding, Margin, Constraints, Modifiers, Align, Justify
├── style/
│   ├── color.rs                # Color enum (Named, Indexed, Rgb), ColorDepth, color blending
│   └── theme.rs                # Theme struct, 7 presets (dark, light, dracula, catppuccin, nord, solarized, tokyo_night), ThemeBuilder
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
│   ├── render.rs               # Chart rendering, histogram
│   ├── axis.rs                 # Axis struct, label formatting
│   ├── bar.rs                  # Bar chart rendering
│   ├── grid.rs                 # Grid lines
│   └── braille.rs              # Braille dot patterns for line/scatter charts
│
├── buffer.rs                   # Double-buffer with clip stack and diff tracking
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

Every frame follows this exact sequence:

```
1. EVENT POLL
   └── Terminal polls for keyboard/mouse events (non-blocking)
   └── Events stored in Context for widget consumption

2. USER CLOSURE
   └── User's closure runs: ui.text(), ui.button(), ui.col(), etc.
   └── Each call pushes a Command to Context's internal command list
   └── No layout is computed yet — just recording intent

3. BUILD TREE — build_tree()
   └── Flat Command list → nested LayoutNode tree
   └── Parent-child relationships resolved from open/close markers

4. COLLECT ALL — collect_all()
   └── Single DFS traversal of the LayoutNode tree
   └── Collects: focus targets, scroll regions, hit areas,
       animation values, draw closures, modals, toasts
   └── This single pass replaced 7 separate traversals (v0.9)

5. FLEXBOX LAYOUT
   └── layout_row() / layout_column()
   └── Resolves: sizes, gaps, grow factors, min/max constraints
   └── Breakpoint-conditional styles evaluated against terminal width

6. RENDER
   └── render() → render_inner() → render_border()
   └── Writes Cell values to the back buffer
   └── Clip stack ensures children don't overflow parent bounds
   └── Viewport culling: nodes fully outside the viewport are skipped

7. DIFF + FLUSH
   └── Compare front buffer (previous frame) vs back buffer (current frame)
   └── apply_style_delta() — only emit ANSI attributes that changed
   └── Synchronized output (DECSET 2026) prevents tearing on supported terminals
   └── Swap front ↔ back buffers
```

---

## One-Frame Delay Feedback

Layout-computed data feeds back to the **next** frame via `prev_*` fields on Context:

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
  │     ├── layout.rs ←── layout/flexbox.rs, layout/render.rs
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
- `context.rs` is the central hub — it depends on almost everything
- `terminal.rs` is isolated — it only knows about `buffer` and `event`
- `style`, `layout`, `anim` are independent of each other
- Widget submodules (`context/widgets_*.rs`) add `impl Context` blocks — they don't define new types

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
