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
