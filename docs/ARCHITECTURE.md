# SLT Architecture

This document describes how the code is organized and how data flows through the system.
For design philosophy and conventions, see [Design Principles](DESIGN_PRINCIPLES.md).

Related docs:
- [QUICK_START.md](QUICK_START.md)
- [WIDGETS.md](WIDGETS.md)
- [PATTERNS.md](PATTERNS.md)
- [EXAMPLES.md](EXAMPLES.md)
- [BACKENDS.md](BACKENDS.md)
- [DEBUGGING.md](DEBUGGING.md)
- [TESTING.md](TESTING.md)

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
│   └── widgets_viz.rs          # Charts, sparklines, canvas, QR, and other data-viz widgets
│
├── widgets.rs                  # Facade for widget state types
├── widgets/
│   ├── input.rs                # StaticOutput, TextInputState, FormField, FormState, TextareaState, SpinnerState
│   ├── collections.rs          # ListState, FilePickerState, TabsState, TableState, ScrollState
│   ├── feedback.rs             # RichLogState, CalendarState, Toast types, ButtonVariant, Trend
│   ├── selection.rs            # SelectState, RadioState, MultiSelectState, TreeState, DirectoryTreeState, PaletteCommand
│   └── commanding.rs           # CommandPaletteState, streaming states, ScreenState, tool approval types, ContextItem
│
├── layout.rs                   # Thin facade re-exporting layout kernels
├── layout/
│   ├── command.rs              # Command enum recorded by Context
│   ├── tree.rs                 # LayoutNode, NodeKind, build_tree(), wrap helpers
│   ├── collect.rs              # collect_all(), FrameData, raw-draw collection helpers
│   ├── flexbox.rs              # compute(), layout_row(), layout_column(), gap/grow/shrink resolution
│   ├── render.rs               # render(), render_inner(), render_border(), clipping, viewport culling
│   └── tests.rs                # Layout-focused kernel tests
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

Every frame follows this exact sequence:

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

4. BUILD TREE — build_tree()
   └── Flat Command list → nested LayoutNode tree
   └── Parent-child relationships resolved from open/close markers

5. FLEXBOX LAYOUT
   └── layout_row() / layout_column()
   └── Resolves: sizes, gaps, grow factors, min/max constraints
   └── Breakpoint-conditional styles evaluated against terminal width

6. COLLECT ALL — collect_all()
   └── Single DFS traversal of the laid-out LayoutNode tree
   └── Collects: scroll regions, hit areas, group rects, content rects,
       focus rects/groups, raw-draw viewport rects
   └── This single pass replaced multiple feedback traversals

7. RENDER + DEFERRED DRAW
   └── render() → render_inner() → render_border()
   └── Writes Cell values to the back buffer
   └── Clip stack ensures children don't overflow parent bounds
   └── Viewport culling: nodes fully outside the viewport are skipped
   └── Deferred raw-draw callbacks replay into collected raw-draw rects

8. DIFF + FLUSH
   └── Compare front buffer (previous frame) vs back buffer (current frame)
   └── apply_style_delta() — only emit ANSI attributes that changed
   └── Synchronized output (DECSET 2026) prevents tearing on supported terminals
   └── Swap front ↔ back buffers
```

For the custom-backend entry point that drives this lifecycle manually, see `docs/BACKENDS.md`.
Terminal-owned run loops add selection overlay and clipboard handling around the shared kernel before the final flush.

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
