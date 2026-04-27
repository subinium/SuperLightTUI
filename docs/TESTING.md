# Testing Guide

This guide covers the test paths that contributors and AI-assisted editors should reach for first.

## Start with the right lane

SLT testing is easier if you pick the right layer up front.

| Goal | Tool |
|------|------|
| Verify rendering, layout, and widget behavior | `TestBackend` |
| Verify keyboard/mouse/paste/resize interaction | `TestBackend` + `EventBuilder` |
| Verify `frame()` / `Backend` contract semantics | custom `Backend` + `AppState` + `frame()` |
| Verify shared-kernel parity | parity/property tests using both `TestBackend` and `frame()` |

Most tests should stay in the first two lanes.

## Default tool: `TestBackend`

`TestBackend` renders one or more frames into an in-memory buffer without a real terminal.

```rust
use slt::TestBackend;

let mut tb = TestBackend::new(40, 10);
tb.render(|ui| {
    ui.text("Hello");
});

tb.assert_contains("Hello");
```

Use this for:

- text and layout assertions
- widget state changes
- overlay and modal rendering checks
- snapshot-style buffer inspection
- multi-frame state checks for hooks, focus, and previous-frame hit testing

## Simulating input with `EventBuilder`

```rust
use slt::{EventBuilder, KeyCode, TestBackend};

let mut tb = TestBackend::new(40, 10);
let events = EventBuilder::new()
    .key('h')
    .key_code(KeyCode::Tab)
    .click(4, 2)
    .build();

tb.run_with_events(events, |ui| {
    ui.text("interactive");
});
```

Use `EventBuilder` when the widget logic depends on keyboard, mouse, paste, or resize events.

### Mouse and key chain wrappers (v0.19.1+)

`EventBuilder` ships convenience wrappers for events that previously required
constructing raw `Event` values. The most useful ones to know:

| Method | Emits | Use for |
|--------|-------|---------|
| `.click(x, y)` | mouse down + up at `(x, y)` | most click tests |
| `.mouse_up(x, y)` | mouse up only | testing release-only handlers, drag end |
| `.drag(x, y)` | mouse drag at `(x, y)` (button held) | scrubbing sliders, resizing splits |
| `.key_release(c)` | key release for `c` | matched press/release pairs (e.g. modifier holds) |
| `.focus_gained()` | terminal `FocusGained` event | windows/tabs gaining focus |
| `.focus_lost()` | terminal `FocusLost` event | pause-on-blur, autosave |

Click vs drag is the test gap most authors miss — a `click` is two events
in the same cell, a `drag` is movement while a button is held:

```rust
// Click: down then up in the same cell
let events = EventBuilder::new()
    .click(10, 4)
    .build();

// Drag: emit drag events while moving across cells
let events = EventBuilder::new()
    .drag(10, 4)
    .drag(11, 4)
    .drag(12, 4)
    .mouse_up(12, 4)
    .build();
```

Use `mouse_up` when you want to assert that a handler only fires on release
(common for "press-and-hold to drag, release to commit" patterns).

## `render()` vs `run_with_events()` vs `render_with_events()`

| Method | Use when |
|--------|----------|
| `render()` | One static frame, no input needed |
| `run_with_events()` | One frame with events, default focus state is fine |
| `render_with_events()` | One frame with explicit events and explicit focus bookkeeping |

`render_with_events()` is the lowest-level helper.
Use it when you need to control `focus_index` or `prev_focus_count` directly.

## Multi-frame tests are normal in SLT

Immediate-mode UI often uses previous-frame data.
That means some good tests intentionally render two or more frames.

Typical cases:

- hover and click behavior that depends on previous-frame hit maps
- focus movement across Tab / Shift+Tab
- `use_state()` / `use_memo()` persistence
- modal scope and overlay boundaries

If a widget looks “one frame delayed” in implementation, that is not automatically a bug.
It is often the correct immediate-mode contract.

## Backend contract tests

`TestBackend` is not the right tool for everything.
When you want to verify the low-level runtime contract itself, write a tiny custom backend and drive `frame()` directly.

Good contract targets:

- `flush()` errors propagate
- `ui.quit()` returns `Ok(false)`
- `AppState` persists across frames
- resize changes are respected by the next frame

This is the right place to test the runtime boundary, not widget rendering details.

## Parity and property tests

When you touch core lifecycle code, keep one more layer in mind:

- parity tests compare `TestBackend` with a minimal custom `Backend`
- property tests throw event sequences at both and lock the outputs together

Those tests are especially valuable for:

- frame-kernel refactors
- layout feedback changes
- focus and interaction bookkeeping
- backend/path deduplication

If you change internals and the public API stays the same, this test lane is often your best safety net.

## Assertions that age well

```rust
tb.assert_contains("Saved");
tb.assert_line(0, "Header");
tb.assert_line_contains(3, "status");
let snapshot = tb.to_string_trimmed();
```

The most robust pattern is:

1. render one frame
2. inspect one or two stable lines
3. assert a small substring or a focused semantic fact

Avoid asserting giant whole-screen strings unless the UI is intentionally snapshot-tested.

## Snapshot testing with `insta`

`TestBackend::to_string_trimmed()` returns a deterministic multi-line string
suitable for snapshot testing with the [`insta`](https://insta.rs) crate.
SLT itself does not require `insta`, but the in-tree test suite uses it —
see `tests/snapshots.rs` for ~10 live examples covering text, rows, tables,
tabs, calendars, lists, and separators.

Add `insta` to your own project's dev-dependencies:

```toml
[dev-dependencies]
superlighttui = "0.19"
insta = "1"
```

Compare the full buffer against a checked-in snapshot:

```rust,ignore
use slt::TestBackend;

#[test]
fn dashboard_snapshot() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        let _ = ui.container().border(slt::Border::Rounded).p(1).col(|ui| {
            ui.text("Dashboard").bold();
            ui.text("42 events");
        });
    });
    insta::assert_snapshot!(tb.to_string_trimmed());
}
```

Accept or reject candidate snapshots with `cargo insta review`. Inline
snapshots (`insta::assert_snapshot!(tb.to_string_trimmed(), @r"...")`) are
preferred for small fixtures — they keep the expected output next to the
test and never drift from external `.snap` files.

Snapshot tests pair well with `EventBuilder` for interaction journeys, and
with focused `assert_contains` / `assert_line` for narrow invariants.
Prefer small snapshots — one widget or one panel — over full-screen dumps
that churn on every theme or layout tweak.

## Visual snapshot regression tests

`tests/visual_snapshots.rs` renders one frame of each demo example into a
`TestBackend` and stores the buffer output as a plain-text snapshot under
`tests/snapshots/visual__<demo>.snap`. The goal is to catch the kinds of
visual regressions that raw assertions miss — top-border title overflow,
flexbox grow drift, theme color shifts, CJK width handling at the right
edge — by failing CI when the rendered output changes unexpectedly.

### How to run

```bash
cargo test --test visual_snapshots
```

The first run on a clean checkout passes against the committed baselines.
A failing test prints a side-by-side diff of expected vs actual buffer.

### Updating baselines after intentional changes

When you deliberately change visual output (a widget restyling, a layout
tweak, a new badge), the snapshots will fail. Review and accept the new
baseline:

```bash
cargo insta review     # interactive
cargo insta accept     # accept all pending
```

Commit the updated `tests/snapshots/visual__*.snap` files alongside the
code change so reviewers can see the visual diff in the PR.

### What it catches

- Layout drift (flexbox grow/shrink/gap regressions)
- Border rendering bugs (wrong corners, missing edges, **title overflow**)
- Theme color shifts that flip glyph attributes
- CJK / wide-char width handling at the right edge
- Wrap and truncation at small terminal sizes

### What it does NOT catch

- Interactive state transitions (focus, hover, click) — use `EventBuilder`
  with assertion-based tests instead
- Animation and frame timing — use parity / property tests
- Sixel / kitty image output — not represented in plain-text buffer
- Multi-frame state changes (only frame 1 is captured)

### Implementation

Each example file (`examples/demo*.rs`) exposes a `pub fn render(ui: &mut Context)`
entry point that builds fresh state and runs one rendering pass. The
example's own `main` keeps using `slt::run` (or `slt::run_with`) so the
interactive demo still works; the snapshot test imports the example via
Rust's `#[path = "../examples/demo.rs"]` attribute and calls `render`
directly.

Demos with rich internal state (`demo.rs`, `demo_dashboard.rs`,
`demo_infoviz.rs`, `demo_cjk.rs`) use a `render_frame(ui, &mut state)`
helper for runtime, and a thin `render(ui)` wrapper that builds default
state and forwards. Frame-1 snapshots only need that wrapper.

## Testing custom widgets

For custom widgets:

- call `register_focusable()` if keyboard input matters
- use `interaction()` if you need click/hover without a wrapping container
- verify both rendering and return-value semantics

```rust
let changed = ui.widget(&mut rating);
assert!(changed);
```

## Good test targets in SLT

- clipping and viewport behavior for `raw_draw`
- focus order and Tab behavior
- modal and overlay interaction boundaries
- `Response.clicked`, `.changed`, `.hovered`, `.focused`
- widget state persistence across frames
- rendering of wrapped text, markdown, rich output, and charts
- backend contract invariants for `frame()`

## Debugging failing UI tests

When a test fails:

- print `tb.to_string_trimmed()` in the failure path
- compare the expected focus/input state with the actual event sequence
- verify whether the widget depends on previous-frame data (`prev_*` behavior)
- ask whether the failure belongs to widget rendering, runtime contract, or shared-kernel parity

That last question matters.
A surprising amount of confusion disappears when the test is moved to the correct layer.

## Recommended contributor workflow

1. Reproduce the issue with `TestBackend` if possible.
2. Add `EventBuilder` if interaction matters.
3. If the change touches `frame()` or `Backend`, add or update a contract test.
4. If the change touches the frame kernel, consider parity/property coverage too.

This keeps tests small while still protecting the core.

## Related docs

- `docs/DEBUGGING.md` - one-frame delay, F12 overlay, clipping
- `docs/PATTERNS.md` - custom widgets, hooks, overlays, large-app structure
- `docs/BACKENDS.md` - backend mental model and low-level contract
- `src/test_utils.rs` - canonical rustdoc for test helpers
