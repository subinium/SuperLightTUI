# Previous Frame Guide

Why interactive widgets in SLT use last-frame layout data, when `Response.rect`
is zero vs. meaningful, and how to write animations, measurements, and focus
logic that stay correct on frame 1.

## The setup

SLT is immediate mode. Your closure runs once per frame. Every call you make
(`ui.text(...)`, `ui.button(...)`, `ui.row(|ui| ...)`) records a `Command`
into a per-frame buffer. Layout runs **after** the closure returns, walks the
recorded tree, and computes positions and sizes. Rendering follows, and then
events are collected for the next frame.

That ordering has one consequence you must internalize:

> When you call `ui.button("Save")` on frame N, layout has not run yet for
> frame N. The `Response` returned inside the closure uses frame **N-1**'s
> layout — positions, sizes, and hit-test rectangles — to answer `.clicked`,
> `.hovered`, and `.rect`.

This is what the README calls "synchronous feedback". You get interaction
results on the same line where you describe the widget, even though the rect
you actually see returned was measured one frame ago.

## Frame N vs. frame N+1 timeline

```
Frame N                                Frame N+1
-------------------------------------  -------------------------------------
1. Events arrive (mouse, keys)
2. Context::new() swaps prev_*
   maps in from last frame's layout
3. Your closure runs
   - records commands
   - reads prev_hit_map to decide
     .clicked / .hovered / .focused
   - Response.rect = last frame's rect
4. build_tree() walks commands
5. flexbox::compute() assigns rects
6. collect_all() produces FrameData:
   hit_areas, focus_rects,
   scroll_rects, group_rects, ...
7. render() draws the buffer diff
8. FrameData saved into FrameState
   as prev_* for the NEXT frame        1. Events arrive
                                        2. Context::new() pulls the
                                           FrameData saved at the end of
                                           frame N; the hit map and focus
                                           rects now describe the tree the
                                           user actually saw
                                        3. Your closure runs again; this
                                           time Response.rect is valid
                                           because layout has run at least
                                           once
```

The key variables live in `src/context/core.rs` (`prev_hit_map`,
`prev_focus_count`, `_prev_focus_rects`, `prev_scroll_rects`, ...) and get
populated at the end of each frame from `collect_all()` in
`src/layout/collect.rs`. `Context::new()` in `src/context/runtime.rs` reads
those same fields at the start of the next frame.

## When `Response.rect` is meaningful

`Response.rect` is a plain `Rect`, not `Option<Rect>`. On the first frame
there is no previous layout, so you see the default — `Rect { x: 0, y: 0,
width: 0, height: 0 }`. From frame 2 onward the value is the rect that
widget occupied during the previous frame's layout pass.

Use `ui.tick()` (the public helper on `Context`) to branch. The tick
counter is a `u64` that starts at 0 and increments by one per rendered
frame.

```rust
slt::run(|ui: &mut slt::Context| {
    let r = ui.button("Save");

    if ui.tick() > 0 && r.rect.width > 0 {
        // Safe to use the visible r.rect for anim targets
        // or tooltip anchors. The rect reflects last frame's layout.
        anim_target = r.rect;
    }
});
```

`Response.rect` is clipped to ancestor viewports: it can be zero even after
many frames when the widget is offscreen. Do not reconstruct document positions
by adding a scroll offset to this rectangle. In v0.25, use
`ui.focused_layout_rect()` or `ui.measured_layout_rect(group_name)` for a full
border box before scrolling/clipping. These are previous-frame root/overlay
logical coordinates, including border and padding, not local scroll coordinates.
They return `None` before measurement or after resize. `form_field_response`
also exposes the full field rectangle as optional `layout_rect`.

Focus-following scroll corrections render with the completed layout and update
the bound `ScrollState` on its next use. Application offset edits between those
frames override the pending state update. No UI closure is executed twice.

Equivalent guards that also work:

```rust
if r.rect.area() > 0 { /* rect has real dimensions */ }
if !r.rect.is_empty() { /* same idea */ }
```

Prefer the `tick() > 0` test when the rect could legitimately be zero-sized
(an empty column, a hidden tab). Prefer the area check when you want to
skip the widget regardless of why the rect is empty.

## Common pitfalls

### Pitfall 1: first-frame flicker from premature measuring

```rust
// BROKEN: on frame 0 this reads zero, so you get a one-frame flash
let r = ui.container().col(|ui| { ui.text(long_label); });
let inset = r.rect.width / 2;
ui.text(format!("centered under width {inset}"));
```

Fix: branch on `tick()`, or smooth with a `use_state` cache that only
updates when width is non-zero.

```rust
let cached = ui.use_state(|| 0u32);
let r = ui.container().col(|ui| { ui.text(long_label); });
if r.rect.width > 0 {
    *cached.get_mut(ui) = r.rect.width;
}
let inset = *cached.get(ui) / 2;
```

### Pitfall 2: focus lost because the hit map was empty

A widget added on frame 1 has no entry in `prev_hit_map`, so a click that
lands on it during frame 1 is ignored — `prev_focus_rects` is still empty.
On the first frame, `ui.register_focusable()` matches only the requested slot
(0 by default), rather than granting focus to every new widget. Call
`ui.set_focus_index(desired)` before declaring the widgets for deterministic
startup selection instead of relying on mouse hit-testing. Responses already
returned before a later programmatic focus request keep their original flags;
the next frame reflects that request and reports its gain/loss edges.

### Pitfall 3: animating toward a previous-frame rect

`Tween`, `Spring`, `Keyframes`, `Sequence`, and `Stagger` from
`src/anim.rs` all take the current tick via `value(ui.tick())`. When the
animation target is another widget's rect, thread the previous-frame rect
through `use_state` so the animation keeps a stable target across
re-layouts:

```rust
let target = ui.use_state(|| Rect::default());
let row = ui.row(|ui| { ui.text("follow me"); });
if row.rect.width > 0 {
    *target.get_mut(ui) = row.rect;
}

let t = tween.value(ui.tick());
let dest = *target.get(ui);
let x = (dest.x as f64) * t;
// ... place a marker at x using ui.draw_with or absolute positioning
```

See `examples/anim.rs` for the canonical pattern: start the animation with
`tween.reset(ui.tick())`, read the current value with
`tween.value(ui.tick())`, and always use `ui.tick()` — never a local
frame counter that may drift under `run_inline` or `frame()`-based
backends.

## Design rationale

Immediate-mode UIs face a chicken-and-egg problem: to tell you whether a
widget was clicked, the framework has to know the widget's rect, but the
rect only exists after you've finished describing the tree. Two options
exist. A two-pass approach would run your closure once to measure and once
to commit — double the work every frame. The one-pass approach SLT takes
instead answers interaction questions using the **previous** frame's
layout. Typical TUI apps run at 30–60 FPS with sub-cell sized widgets, so
a one-frame lag on hit-testing is imperceptible to a human and buys you
one closure execution per frame instead of two.

The tradeoff pays off again during scrolling and resizing. When the
layout changes between frames, the hit map the user aimed at is the hit
map from the frame they saw on screen, which is exactly the previous
frame's map. Using the in-flight frame's rects would mean clicks could
miss a button that had moved under the cursor between input capture and
rendering. Borrowing egui's phrasing: "the rectangle they clicked is the
one they saw, not the one we're about to draw."

## See also

- `docs/AI_GUIDE.md` — general rules for AI agents working with SLT
- `docs/PATTERNS.md` — idiomatic immediate-mode patterns
- `docs/ANIMATION.md` — full reference for `Tween`, `Spring`, `Keyframes`
- `examples/anim.rs` — all five animation primitives driven by `ui.tick()`
- `src/context/runtime.rs` — where `tick`, `prev_hit_map`, and focus state
  are wired up at the start of each frame
- `src/layout/collect.rs` — `collect_all()` DFS that produces the
  `FrameData` snapshot saved for the next frame
