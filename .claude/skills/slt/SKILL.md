---
name: slt
description: Build Rust TUI apps with SuperLightTUI (immediate-mode terminal UI library). Use this skill when the user asks to create, modify, or debug terminal UI code in this repo, or asks "how do I X in SLT / TUI / terminal". First read docs/COMPLETE_REFERENCE.md for the full API, docs/COOKBOOK.md for app recipes, docs/STATE_APIS.md for state type methods, docs/PREVIOUS_FRAME_GUIDE.md for frame-timing questions.
---

# SuperLightTUI (SLT) Authoring Skill

## Mental model (read every session)

SuperLightTUI is an immediate-mode TUI library. Your app is a single closure:
`slt::run(|ui: &mut Context| { ... })`. That closure runs once per frame.
State lives in normal Rust variables outside the closure — there is no `App` trait,
no `Model/View/Update`, no retained component tree. The library handles layout
(flexbox), rendering to a back buffer, ANSI diffing against the previous frame,
and stdout flush.

Interaction uses *previous-frame* hit rects — when the closure runs for frame N,
layout for N hasn't happened yet, so `Response.rect` reflects frame N-1. On frame 1
it's a zero `Rect`. For measurement-dependent logic, guard with
`if ui.tick() > 0 { /* use rect */ }`. See `docs/PREVIOUS_FRAME_GUIDE.md`.

For larger apps, write "components as functions": `fn render_card(ui: &mut Context, data: &Card)`.
Share read-mostly state via `ui.provide(value, |ui| ...)` + `ui.use_context::<T>()`
(avoids threading `&theme` / `&tick` / `&mut toasts` through every helper fn).
Component-local state lives in `ui.use_state_named(id)` — the id-keyed variant
is safe inside conditionals, unlike order-based `use_state`. Conditional styling:
`.with_if(cond, modifier)` on text and `ContainerBuilder`. See `docs/PATTERNS.md`.

## Authoring workflow

1. Confirm the goal. What app is the user building? Data table? Dashboard? Form? Game?
2. Check `docs/COOKBOOK.md` for a matching recipe (login / data table / modal+toast / dashboard / file picker). Start from that if it fits.
3. Otherwise, read `docs/COMPLETE_REFERENCE.md` (single condensed file) and grep `src/lib.rs` for the needed re-exports.
4. For state types, read `docs/STATE_APIS.md` — every public `*State` struct listed with methods.
5. Stick to the small core grammar: `ui.text / row / col / bordered / button / text_input / table / list / modal / toast / chart / canvas / tabs / select / tree / spinner`. For component composition (v0.19.0+): `ui.provide(...)` / `ui.use_context::<T>()` / `ui.use_state_named(id)` / `.with_if(cond, modifier)`.
6. Before writing `ui.foo(...)`, grep `src/context/` to confirm the method exists. Do NOT invent APIs.
7. Run the quality gate (below) before saying "done".

## Quality gate (mandatory before saying "done")

Core — every commit:
```
cargo fmt -- --check
cargo check --all-features
cargo clippy --all-features -- -D warnings
cargo test --all-features
cargo check --examples --all-features
```

Extended — before PR or release:
```
typos
cargo check -p superlighttui --no-default-features
cargo check -p slt-wasm --target wasm32-unknown-unknown
cargo hack check -p superlighttui --each-feature --no-dev-deps
cargo audit
cargo deny check
```

## Release workflow (mandatory — do not skip any step)

The project-level `CLAUDE.md` has the full 8-step checklist. The short version:

1. Local PRE-CI (Core + Extended gates both green)
2. Bump `Cargo.toml`, update `CHANGELOG.md`
3. Branch `release/vX.Y.Z`, single atomic commit, push
4. `gh pr create`, **wait** for CI green
5. Merge (squash), pull main
6. Tag, push tag, **wait** for `release.yml` green
7. Verify `gh release view`, crates.io, docs.rs
8. Only now announce

Red flags that mean STOP:
- "Probably fine" — run the gate
- "Just a docs change" — still run `cargo check --examples`
- "CI will catch it" — no, locals first
- "I'll tag now and fix later" — no broken tags

## Common pitfalls (AI-generated SLT code)

- **Inventing method names.** Always grep `src/context/` before writing a `ui.*` call.
- **Using `Response.rect` on frame 1.** Zero Rect. Guard with `ui.tick() > 0` (see `docs/PREVIOUS_FRAME_GUIDE.md`).
- **`.unwrap()` in library paths.** `#![warn(clippy::unwrap_used)]` is on. Use `?` or explicit match.
- **`unsafe` blocks.** `#![forbid(unsafe_code)]` is on at crate root. Hard compile error.
- **Forgetting `'static` on `ContainerBuilder::draw()` closure.** Raw draw is deferred.
- **Mixing crossterm raw events with `ui.*` helpers.** Prefer `ui.key()`, `ui.key_code()`, `ui.key_mod()`. Raw events are for advanced cases only.
- **`use_state()` inside `if` / `match` / `for`.** Order-based hooks misbehave when call order changes between frames. Use `ui.use_state_named(id)` (id-keyed) for state inside conditionals.
- **Threading `&theme`, `&tick`, `&mut state` through every render fn.** v0.19.0+ has `ui.provide(value, |ui| ...)` + `ui.use_context::<T>()` for cross-scope reads. Reserve explicit params for *writes*.
- **Hard-coding `Color::Rgb(...)` in widget code.** Pull from `ui.theme()` (`primary`, `text`, `border`, `selected_bg`, etc.) so themes can swap. v0.19.2 made `ThemeBuilder` `const fn` — themes can be defined at compile time.
- **`RichLogState::new()` for unbounded logs.** v0.19.2 capped `new()` at 10000 entries. Use `RichLogState::new_unbounded()` if you really want unlimited accumulation (tail-style log viewers).
- **Animating without `slt::Tween` / `slt::Spring`.** Don't reinvent — use the animation primitives.
- **Printing to stdout/stderr from a widget.** `#![warn(clippy::print_stdout)]` / `print_stderr`. A library must not write to stdout.

## Reading order when stuck

1. `docs/COMPLETE_REFERENCE.md` — condensed everything, start here.
2. `docs/COOKBOOK.md` — 5 full app recipes.
3. `docs/PATTERNS.md` — component composition (`provide` / `use_context` / `use_state_named` / `with_if`) and state-ownership idioms.
4. `src/lib.rs` — authoritative public re-exports.
5. `examples/` — 32 runnable examples; find the closest pattern. `demo_cjk` for CJK / wide-char rendering, `demo_website` for the canonical `provide` / `use_context` example.
6. If still stuck: ask the user. Korean conventions to honor: "ㄱㄱ" = proceed immediately, "켜줘" = open the file in Cursor (not `cat` to terminal).

## Testing pattern (headless)

```rust
use slt::{TestBackend, EventBuilder};

#[test]
fn my_widget_renders() {
    let mut tb = TestBackend::new(80, 24);
    tb.render(|ui| {
        ui.text("hello");
    });
    assert!(tb.line(0).contains("hello"));
}
```

See `docs/TESTING.md` for event injection, multi-frame scenarios, and snapshot patterns.

## File layout cheat sheet

| Area | Primary files |
|---|---|
| Public API | `src/lib.rs` (re-exports) |
| Run loop / terminal backend | `src/terminal.rs`, `src/lib.rs` (`run`, `run_inline`, `run_async`) |
| Context / widget methods | `src/context/runtime.rs`, `src/context/widgets_*` |
| State types | `src/widgets/*.rs` |
| Layout | `src/layout/` (tree, flexbox, collect, render) |
| Style / theme | `src/style/` |
| Animation | `src/anim.rs` |
| Charts | `src/chart/` |
| Testing helpers | `src/test_utils.rs` |
