# State and the Previous Frame

Step 2 of the learning path. Where state lives in an SLT app, and the one
timing rule every immediate-mode program has to know: interaction and
geometry answers come from the previous frame's layout.

SLT has three kinds of state:

- **App state** — plain Rust structs you own and pass into the closure.
- **Hook state** — `ui.use_state` / `ui.use_memo` slots that live with one
  render subtree.
- **Widget state** — `ListState`, `TableState`, `TextInputState`, ... structs
  that a widget reads and updates each frame. You own them like app state; the
  [State API reference](../reference/state.md) lists every field and accessor.

## Application state lives in normal Rust

<!-- slt-check: {"name": "state_app_struct", "features": ["crossterm"], "context": "module"} -->
```rust
use slt::{Context, KeyCode};

struct App {
    count: i32,
    dark: bool,
}

fn main() -> std::io::Result<()> {
    let mut app = App {
        count: 0,
        dark: false,
    };

    // The mouse is on by default, so the button and checkbox react to clicks;
    // they also work from the keyboard (Tab to focus, Enter/Space to press).
    slt::run(|ui: &mut Context| {
        if ui.button("+1").clicked {
            app.count += 1;
        }

        ui.checkbox("Dark mode", &mut app.dark);
        ui.text(format!("count: {}", app.count));

        // Global shortcuts can go anywhere; focused text fields keep their keys.
        if ui.key('q') || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
    })
}
```

Use plain structs when the state belongs to your app, your domain, or your screens.

## Local persistent state with hooks

<!-- slt-check: {"name": "state_hooks", "features": [], "context": "ui"} -->
```rust
let count = ui.use_state(|| 0i32);
ui.text(format!("{}", count.get(ui)));
if ui.button("+1").clicked {
    *count.get_mut(ui) += 1;
}
```

Use hooks when:

- the state is local to one render subtree
- introducing a top-level field would add more noise than clarity
- you want to prototype quickly

Keep hook call order stable across frames.

## When to use app state vs hooks

Use app state when:

- multiple screens need the value
- the value matters outside rendering
- you want explicit ownership and easier refactoring

Use hooks when:

- the value is local to one widget subtree
- the lifetime is purely UI-local
- you want a lightweight scratchpad for a small interactive fragment

The mistake is not using hooks.
The mistake is using hooks for everything until the screen becomes impossible to reason about.

## Derived state with `use_memo`

<!-- slt-check: {"name": "state_use_memo", "features": [], "context": "ui", "setup": "let query = String::from(\"ap\"); let items = vec![String::from(\"apple\"), String::from(\"banana\")];"} -->
```rust
let filtered = ui.use_memo(&(query.clone(), items.len()), |(query, _)| {
    items
        .iter()
        .filter(|item| item.contains(query))
        .cloned()
        .collect::<Vec<_>>()
});
ui.text(format!("{} matches", filtered.get(ui).len()));
```

Use `use_memo` when the computation is deterministic and should only rerun when the dependency tuple changes.
Like `use_state`, call order must stay stable across frames.

## The previous-frame rule

This is the canonical explanation; other pages link here.

### The setup

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

### Frame N vs. frame N+1 timeline

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

### When `Response.rect` is meaningful

`Response.rect` is a plain `Rect`, not `Option<Rect>`. On the first frame
there is no previous layout, so you see the default — `Rect { x: 0, y: 0,
width: 0, height: 0 }`. From frame 2 onward the value is the rect that
widget occupied during the previous frame's layout pass.

Use `ui.tick()` (the public helper on `Context`) to branch. The tick
counter is a `u64` that starts at 0 and increments by one per rendered
frame.

<!-- slt-check: {"name": "state_rect_guard", "features": ["crossterm"], "context": "module"} -->
```rust
fn main() -> std::io::Result<()> {
    let mut anim_target = slt::Rect::default();
    slt::run(|ui: &mut slt::Context| {
        // `.show()` returns an owned `Response`, so `ui` is free again below.
        let r = ui.button("Save").show();

        if ui.tick() > 0 && r.rect.width > 0 {
            // Safe to use the visible r.rect for anim targets
            // or tooltip anchors. The rect reflects last frame's layout.
            anim_target = r.rect;
        }
        ui.text(format!("anchor x = {}", anim_target.x));
    })
}
```

`Response.rect` is clipped to ancestor viewports: it can be zero even after
many frames when the widget is offscreen. Do not reconstruct document positions
by adding a scroll offset to this rectangle. Since v0.25, use
`ui.focused_layout_rect()` or `ui.measured_layout_rect(group_name)` for a full
border box before scrolling/clipping. These are previous-frame root/overlay
logical coordinates, including border and padding, not local scroll coordinates.
They return `None` before measurement or after resize. `form_field_response`
also exposes the full field rectangle as optional `layout_rect`.

Focus-following scroll corrections render with the completed layout and update
the bound `ScrollState` on its next use. Application offset edits between those
frames override the pending state update. No UI closure is executed twice.

Equivalent guards that also work:

<!-- slt-check: {"name": "state_rect_guards", "features": [], "context": "ui", "setup": "let r = ui.button(\"Save\").show();"} -->
```rust
if r.rect.area() > 0 { /* rect has real dimensions */ }
if !r.rect.is_empty() { /* same idea */ }
```

Prefer the `tick() > 0` test when the rect could legitimately be zero-sized
(an empty column, a hidden tab). Prefer the area check when you want to
skip the widget regardless of why the rect is empty.

### Common pitfalls

#### Pitfall 1: first-frame flicker from premature measuring

<!-- slt-check-skip: anti-pattern shown for contrast; BROKEN on purpose, reads r.rect before the first layout -->
```rust
// BROKEN: on frame 0 this reads zero, so you get a one-frame flash
let r = ui.container().col(|ui| { ui.text(long_label); });
let inset = r.rect.width / 2;
ui.text(format!("centered under width {inset}"));
```

Fix: branch on `tick()`, or smooth with a `use_state` cache that only
updates when width is non-zero.

<!-- slt-check: {"name": "state_cached_width", "features": [], "context": "ui", "setup": "let long_label = \"a long status label\";"} -->
```rust
let cached = ui.use_state(|| 0u32);
let r = ui.container().col(|ui| { ui.text(long_label); });
if r.rect.width > 0 {
    *cached.get_mut(ui) = r.rect.width;
}
let inset = *cached.get(ui) / 2;
ui.text(format!("centered under width {inset}"));
```

#### Pitfall 2: focus lost because the hit map was empty

A widget added on frame 1 has no entry in `prev_hit_map`, so a click that
lands on it during frame 1 is ignored — `prev_focus_rects` is still empty.
On the first frame, `ui.register_focusable()` matches only the requested slot
(0 by default), rather than granting focus to every new widget. Call
`ui.set_focus_index(desired)` before declaring the widgets for deterministic
startup selection instead of relying on mouse hit-testing. Responses already
returned before a later programmatic focus request keep their original flags;
the next frame reflects that request and reports its gain/loss edges.

`ui.focus_count()` follows the same rule: it reads the settled count from the
last completed frame, so it is 0 on frame 0.

#### Pitfall 3: animating toward a previous-frame rect

`Tween`, `Spring`, `Keyframes`, `Sequence`, and `Stagger` from
`src/anim.rs` all take the current tick via `value(ui.tick())`. When the
animation target is another widget's rect, thread the previous-frame rect
through `use_state` so the animation keeps a stable target across
re-layouts:

<!-- slt-check: {"name": "state_anim_target", "features": [], "context": "ui", "setup": "let mut tween = Tween::new(0.0, 1.0, 30);"} -->
```rust
let target = ui.use_state(|| Rect::default());
let row = ui.row(|ui| { ui.text("follow me"); });
if row.rect.width > 0 {
    *target.get_mut(ui) = row.rect;
}

let t = tween.value(ui.tick());
let dest = *target.get(ui);
let x = (dest.x as f64) * t;
// Place a marker at x using ui.draw_with or absolute positioning.
ui.text(format!("marker x = {x:.0}"));
```

See `examples/anim.rs` for the canonical pattern: start the animation with
`tween.reset(ui.tick())`, read the current value with
`tween.value(ui.tick())`, and always use `ui.tick()` — never a local
frame counter that may drift under `run_inline` or `frame()`-based
backends.

### Design rationale

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

- [AI guide](../reference/ai-guide.md) — general rules for AI agents working with SLT
- [Patterns guide](03-patterns.md) — idiomatic immediate-mode patterns
- [Animation guide](07-animation.md) — full reference for `Tween`, `Spring`, `Keyframes`
- [`examples/anim.rs`](../../examples/anim.rs) — all five animation primitives driven by `ui.tick()`
- `src/context/runtime.rs` — where `tick`, `prev_hit_map`, and focus state
  are wired up at the start of each frame
- `src/layout/collect.rs` — `collect_all()` DFS that produces the
  `FrameData` snapshot saved for the next frame
- [State API reference](../reference/state.md) — every widget state type, field and accessor
