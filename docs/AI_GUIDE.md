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

### "How do I handle keyboard shortcuts?"

Use `key(c)`, `key_code(code)`, or `key_mod(c, mods)`. For modal-aware shortcuts use the regular versions. For global shortcuts that bypass modals, use `raw_key_code()` or `raw_key_mod()`. For key sequence detection use `key_seq("gg")`.

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

## Implementation rules for agents

- Prefer `ui.container().p(1).col(...)` over inventing new layout patterns.
- Prefer existing `*State` types over custom ad-hoc input structs.
- Use `Response.changed` / `.clicked` instead of inventing parallel booleans.
- Use `register_focusable()` and `interaction()` in custom widgets.
- Respect hook ordering rules.
- Animation types are standalone structs, not Context methods — compute values separately.
- Use `palette::tailwind` colors instead of hardcoding RGB values.
- Check `docs/FEATURES.md` before using feature-gated APIs.

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
