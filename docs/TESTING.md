# Testing Guide

This guide covers the test path that AI-assisted contributors should reach for first.

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

`render_with_events()` is the lowest-level test helper. Use it when you need to control `focus_index` or `prev_focus_count` directly.

## Assertions that scale well

```rust
tb.assert_contains("Saved");
tb.assert_line(0, "Header");
tb.assert_line_contains(3, "status");
let snapshot = tb.to_string_trimmed();
```

The most AI-friendly pattern is:

1. render one frame
2. inspect one or two lines
3. assert a small, stable substring

Avoid asserting giant whole-screen strings unless the UI is intentionally snapshot-tested.

## Testing custom widgets

For custom widgets:

- call `register_focusable()` if keyboard input matters
- use `interaction()` if you need click/hover without a wrapping container
- verify both rendering and return value semantics

```rust
let changed = ui.widget(&mut rating);
assert!(changed);
```

## Good test targets in SLT

- clipping and viewport behavior for `raw_draw`
- focus order and Tab behavior
- modal/overlay interaction boundaries
- `Response.clicked`, `.changed`, `.hovered`, `.focused`
- widget state persistence across frames
- rendering of wrapped text, markdown, and charts

## Debugging failing UI tests

When a test fails:

- print `tb.to_string_trimmed()` in the failure path
- compare the expected focus/input state with the actual event sequence
- verify whether the widget depends on previous-frame data (`prev_*` behavior)

That last point matters: some interaction data only becomes visible on the next frame in immediate-mode UI.

## Related docs

- `docs/DEBUGGING.md` - one-frame delay, F12 overlay, clipping
- `docs/PATTERNS.md` - custom widgets, hooks, overlays
- `src/test_utils.rs` - canonical rustdoc for test helpers
