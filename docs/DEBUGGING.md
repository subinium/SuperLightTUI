# Debugging Guide

This guide covers the runtime behaviors that are easy to misread if you only look at widget code.

## The big idea: interaction is often previous-frame data

SLT is immediate mode.

1. your closure records commands
2. layout runs later
3. hit areas and focus rectangles are known after layout
4. the next frame reads that data back through `prev_*`

That means hover/click/focus-sensitive behavior can feel one frame delayed in the implementation even though it looks instant at runtime.

## F12 layout debugger

Press `F12` in a running terminal app to toggle the debug overlay.

Use it when you need to inspect:

- container bounds
- nesting depth
- widget count
- frame timing / FPS
- current terminal dimensions

This is the fastest way to debug clipping, spacing, and invisible layout issues.

## Common failure modes

### 1. Hover or click seems "off"

Check:

- was the widget laid out in the previous frame?
- are you reading a child response when the container owns the interaction area?
- are you conditionally rendering the widget in a way that changes call order?

### 2. Hook panic or state mismatch

`use_state()` and `use_memo()` must be called in the same order every frame.

Bad:

```rust
if show_sidebar {
    let state = ui.use_state(|| 0);
}
```

Good:

```rust
let state = ui.use_state(|| 0);
if show_sidebar {
    ui.text(format!("{}", state.get(ui)));
}
```

If you genuinely need a hook inside an `if` or `match` arm, use the id-keyed variant `ui.use_state_named::<T>(id)` (v0.19.0). It keys by the supplied `&'static str` instead of call order, so it is safe inside conditional branches. The original order-based `use_state` rule still applies — only `*_named` variants opt out of it.

### 3. `Response.rect` is empty

`Response.rect` is meaningful only after the widget has participated in layout.
If the widget was not laid out or the response comes from a helper path that has no visible rect yet, you may see a zero-sized rect.

### 4. `focused` is not what you expected

`Response.focused` is for the widget that actually registered focus.
Wrapping a focused child inside `row()` or `container()` does not automatically mean the container response is focused.

### 5. Raw draw output is clipped strangely

`raw_draw` is clipped by the same layout and viewport rules as other content.
Verify:

- parent container bounds
- scrollable viewport
- draw rect size
- child margin/padding

## Layout debugging checklist

1. Toggle F12.
2. Reduce the case to one container and one child.
3. Replace dynamic content with `ui.text("X")` to prove layout is correct first.
4. Check whether you want `row()` or `line()`.
5. If using overlays or modals, verify whether background interaction should be blocked.

## Focus and modal debugging

Focus handling changes inside modal layers.

- `register_focusable()` inside a modal participates in modal-local focus.
- widgets outside the modal are effectively inert while the modal is active.
- `interaction()` also short-circuits outside the active modal layer.

If a custom widget behaves strangely in overlays, inspect both `focused` and whether it is inside the active overlay depth.

## Clipboard and terminal caveats

- `read_clipboard()` depends on OSC 52 support in the terminal.
- `copy_to_clipboard(...)` writes OSC 52 escape sequences through the active terminal backend.
- Kitty keyboard support is best-effort: unsupported terminals silently ignore it.

## Related docs

- `docs/ARCHITECTURE.md` - frame lifecycle and module map
- `docs/PATTERNS.md` - hooks, overlays, custom widgets
- `docs/TESTING.md` - headless testing patterns
