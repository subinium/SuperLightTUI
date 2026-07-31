# AI and Agent Guide

Quick-start for AI coding agents working with SLT.

## Core mental model

- SLT is immediate mode: the closure is the app.
- Application state usually lives in normal Rust variables or structs.
- Widget-local persistent state uses `use_state()` / `use_memo()`.
- Interactive widgets usually return `Response`.
- Layout is mostly `row()`, `col()`, `container()`, `scrollable()`.

## Reading order

1. `README.md`
2. `docs/QUICK_START.md`
3. `docs/WIDGETS.md`
4. `docs/PATTERNS.md`
5. `docs/EXAMPLES.md`
6. `docs/ARCHITECTURE.md` for internals

## Questions agents get wrong

### "What's the difference between text().bold() and styled()?"

`text().bold()` chains after the last text command. `styled()` applies a pre-built `Style` in one call. Use `styled()` when reusing styles across widgets.

### "I called ui.bg() but the container background didn't change"

`ui.bg()` modifies the LAST TEXT element, not the container. For container background, use `container().bg(color).col(...)`.

### "How do I animate something?"

Animation types (`Tween`, `Spring`, `Keyframes`, `Sequence`, `Stagger`) are standalone structs, not Context methods. Compute values with `tick()` and pass them to style/layout methods. See `docs/ANIMATION.md`.

### "Where are the color palettes?"

Use `palette::tailwind::{BLUE, RED, ...}.c500` for Tailwind-style colors. See `docs/THEMING.md`.

### "What's the difference between line() and row()?"

Both are horizontal. `row()` is a full layout container returning `Response`. `line()` is for inline rich text returning `&mut Self` with zero gap between children.

### "How do I use _colored variants?"

Pass `WidgetColors::new().fg(color).bg(color)` as the last argument. See `docs/THEMING.md`.

### "Why does my hook panic?"

`use_state()` and `use_memo()` must be called in the same order every frame. Never put them inside conditionals.

Exception (v0.19.0): `ui.use_state_named(id)` is the id-keyed variant and IS safe inside `if`/`match` branches because it keys by the supplied `&'static str` instead of call order. Reach for it when you genuinely need a hook in a conditional path.

```rust
// Wrong: order-based hook inside a conditional drifts the call order.
if expanded {
    let count = ui.use_state(|| 0); // BAD — frame N has it, frame N+1 may not.
}

// Right: id-keyed variant is safe inside conditionals.
if expanded {
    let count = ui.use_state_named::<i32>("sidebar.count"); // OK
}
```

The original `use_state` / `use_memo` order rule still holds — only the id-keyed `*_named` variants opt out of it.

### "How do I handle keyboard shortcuts?"

Use `key(c)`, `key_code(code)`, or `key_mod(c, mods)`. For modal-aware shortcuts use the regular versions. For global shortcuts that bypass modals, use `raw_key_code()` or `raw_key_mod()`. For multi-key chords that span frames (vi `gg`, leader keys like `<space>ff`) use `key_chord("gg")` — it buffers partial input across frames and times out on inactivity. (`key_seq` is a deprecated alias that now delegates to `key_chord`.)

### "How do I make a custom widget?"

Implement the `Widget` trait with `type Response` and `fn ui(&mut self, ctx: &mut Context) -> Self::Response`. Use `register_focusable()` for keyboard support and `interaction()` for click/hover.

### "Can I use SLT without crossterm?"

Yes, for custom backends with `Backend`, `AppState`, and `frame()`.

### "Should I create an app struct?"

Only if it helps. SLT does not require one.

### "Should every widget return Response?"

No. Built-in interactive widgets usually do. Custom widgets can return `()`, `bool`, or `Response`.

### "Why is hover/focus data weird on the first frame?"

Because layout feedback often uses previous-frame data in immediate-mode UI.
For the frame timeline and prev-frame rect rules, see [Previous Frame Guide](PREVIOUS_FRAME_GUIDE.md).

### "Token streaming re-renders the whole frame every token — how do I speed it up?"

That is the immediate-mode contract: `stream.push(delta)` then the whole
closure runs again. Use `ui.container().cached(version_key, f)` to wrap the
*static chrome* (chat history, sidebar, status bar) keyed off a value you
already own — e.g. a hash of the non-streaming inputs, or
`StreamingTextState::version()` of the *other* panes. Leave the stream itself
uncached; it changes every token. Note the current semantics are honest: `f`
still runs every frame (output is byte-identical to `.col(f)`), and `cached`
records a hit/miss stability signal (`Context::region_cache_hits()` /
`region_cache_misses()`) rather than skipping the body — this is an
author-controlled cache, **not** reactive binding. See
[Performance — Pattern 7](PERFORMANCE.md) for the full rationale and the
Phase-0 streaming benchmark.

## Implementation rules for agents

- Prefer `ui.container().p(1).col(...)` over inventing new layout patterns.
- Prefer existing `*State` types over custom ad-hoc input structs.
- Use `Response.changed` / `.clicked` instead of inventing parallel booleans.
- Use `register_focusable()` and `interaction()` in custom widgets.
- Respect hook ordering rules.
- Animation types are standalone structs, not Context methods — compute values separately.
- Use `palette::tailwind` colors instead of hardcoding RGB values.
- Check `docs/FEATURES.md` before using feature-gated APIs.

### Context injection: stop threading shared state through every render fn

When a value is *read* by many nested render functions (theme, current tick, current user, toast bus), do not thread `&theme`, `&tick`, `&mut toasts` parameters through every signature. Use `ui.provide(value, |ui| ...)` once near the root, then have nested code read it back with `ui.use_context::<T>()` (panics if missing) or `ui.try_use_context::<T>()` (returns `Option<&T>`). Added in v0.19.0.

```rust
// `provide` boxes the value as `dyn Any`, so the type must satisfy `T: 'static`.
// `Theme` is `Copy`, so deref-copy from `ui.theme()`. Use `&'static str` for
// string literals; switch to `String` if the value comes from runtime input.
#[derive(Clone, Copy)]
struct AppCtx {
    theme: slt::Theme,
    tick: u64,
    user: &'static str,
}

slt::run(|ui: &mut slt::Context| {
    let ctx = AppCtx { theme: *ui.theme(), tick: ui.tick(), user: "subin" };
    ui.provide(ctx, |ui| {
        render_header(ui);
        render_card(ui);
    });
});

fn render_card(ui: &mut slt::Context) {
    // End the immutable borrow before the next `&mut ui` widget call.
    let ctx = *ui.use_context::<AppCtx>();
    ui.text(format!("hi {} (tick {})", ctx.user, ctx.tick));
}
```

`use_context` and `try_use_context` return references tied to `ui`. Never keep
one across a widget call: copy or clone the whole value (or just the fields the
component needs) first.

Reserve explicit parameters for **writes** (e.g. `&mut MyDocState`) — those should still be passed in, not read out of the context bag. See [PATTERNS.md](PATTERNS.md) for the full pattern.

### Conditional styling without re-chaining

`with_if(cond, modifier)` and `with(modifier)` (v0.19.0) let you fold conditional styling into a single fluent chain on text and `ContainerBuilder`. Replace `if cond { t.bold(); t.fg(Color::Red); }` style branching with `ui.text(...).with_if(is_error, |t| t.bold().fg(Color::Red))`. Cleaner diffs, no broken chains.

## Internal widget rules for agents

When editing SLT built-in widgets inside `src/context/*`, prefer the internal interaction helpers instead of hand-rolling event scans again.
These are internal `pub(crate)` helpers, not stable public API for external widgets.

- Use `begin_widget_interaction(focused)` for marker + `Response` setup.
- Use `available_key_presses()` / `available_pastes()` to iterate unconsumed inputs.
- Use `left_clicks_for_interaction()` or `mouse_events_in_rect()` for hit-tested mouse handling.
- Finish by calling `consume_indices(...)` instead of mutating `self.consumed[...]` across the widget body.

Public custom widgets should still use the stable public surface (`register_focusable()`, `interaction()`, `Response`).

## Keep docs and implementation aligned

When an agent changes a public API, it should update:

- crate rustdoc in `src/lib.rs` or the defining file
- the most relevant guide in `docs/`
- example coverage if the new API needs a runnable reference

If the behavior is hard to explain cleanly, the API may still need refinement.
