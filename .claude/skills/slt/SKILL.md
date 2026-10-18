---
name: slt
description: Build Rust TUI apps with SuperLightTUI v0.25 (immediate-mode terminal UI). Use this skill when the user asks to create, modify, or debug terminal UI code in this repo, or asks "how do I X in SLT / TUI / terminal", or types Korean triggers like "터미널 UI", "TUI 만들어줘", "SLT로", "ratatui 대신". Read REFERENCES.md for feature flags and doc pointers; grep `src/context/` and `src/widgets/` before inventing any API.
---

# SuperLightTUI (SLT) Authoring Skill

Targets SLT v0.25.0. Verify every API against `src/` before using it.

## Two runtime rules agents get wrong

1. **The mouse is on by default; hover is opt-in.** Since v0.26, `RunConfig::default()` leaves `mouse_mode: None`, which resolves to `MouseMode::Drag` locally and `MouseMode::Click` over SSH/mosh (`src/runtime/config.rs`). Plain `slt::run(...)` therefore delivers `Response.clicked`, `right_clicked`, `double_clicked`, `scroll_delta`, scrollbar and split-pane drags, and the built-in text selection. Pointer moves without a button held are only reported in `MouseMode::Hover`, so any app that relies on `hovered`, `group_hover_*`, `on_hover` or `ui.tooltip` must start with
   ```rust
   slt::run_with(slt::RunConfig::default().mouse_mode(slt::MouseMode::Hover), |ui| { /* ... */ })
   ```
   `MouseMode::Off` (or `.mouse(false)`) disables the mouse and keeps the terminal's native selection; `.mouse(true)` is a shorthand for `Hover`. Headless `TestBackend` tests inject mouse events directly regardless of the mode.
2. **Focused text fields own their keys.** While `text_input` / `textarea` / a form field / an open command palette has focus, `ui.key('q')`, `ui.key_code(..)`, `ui.key_mod(..)` and `ui.shortcut(..)` skip the keys it edits with (printable keys without Ctrl/Super/Meta, Backspace, Delete, arrows, Home, End; Enter/Up/Down for `textarea`), in any call order. Chords, Esc, Tab and F-keys still reach shortcuts; `ui.raw_key(c)` sees every press.
   ```rust
   if ui.shortcut(KeyCode::Char('q'), KeyModifiers::NONE) { ui.quit(); } // safe before the input
   let _ = ui.text_input(&mut search);                                   // typing 'q' inserts it
   ```

## Mental model

SLT is **immediate-mode**. Your app is one closure: `slt::run(|ui: &mut Context| { ... })`. The closure runs every frame. State lives in plain Rust variables outside the closure — no `App` trait, no `Model/View/Update`, no retained tree. SLT handles flexbox layout, ANSI diff, and stdout flush.

`Response.rect` reflects the **previous** frame because layout runs after the closure returns. Frame 1 returns a zero `Rect`. Guard measurement-dependent logic with `if ui.tick() > 0 { ... }`. See `docs/PREVIOUS_FRAME_GUIDE.md`.

For larger apps write **components as functions**: `fn render_card(ui: &mut Context, data: &Card)`. Share read-mostly state with `ui.provide(value, |ui| ...)` + `ui.use_context::<T>()` instead of threading `&theme` through every helper.

## The API rules (predictability anchors)

These are non-negotiable since v0.20. When generating new code, every public widget must match all of them. Known exceptions are listed in `docs/API_DESIGN.md`; do not copy them into new APIs.

1. **Builder for optional config.** Methods on `Context` return a builder when ≥1 option exists. Builders are consuming (`mut self -> Self`), render on `Drop`, and expose `.show()` (consumes the builder) to capture a `*Response`.
   ```rust
   ui.gauge(0.6).label("60%").width(24).color(Color::Cyan);                     // good
   let r = ui.breadcrumb(&segs).separator(" › ").show();                        // capture response
   ```
   **Removed in v0.20**: `gauge_w`, `gauge_colored`, `line_gauge_with`, `breadcrumb_sep`, `LineGaugeOpts`, `HighlightRange::single`, `label_owned`. Do not write these — AI training data may suggest them.

2. **Floats are `f64`.** Public surface never takes/returns `f32`. `0.5` is `f64` natively, so `ui.gauge(0.5)` just works.

3. **≤3 positional args.** When 4+ args appear, use an **opts struct** (`<Widget>Opts`) or a builder.
   ```rust
   GutterOpts::line_numbers(total, viewport)                                    // Opts struct
   ui.scrollable_with_gutter(&mut scroll, opts, |ui, abs| { ... });
   ```

4. **Stateful widgets take `&mut <Widget>State`.** Never `&mut String`, `&mut Vec<T>`, `&mut usize`. Trivial-value exceptions: `slider(&mut f64)`, `checkbox(&mut bool)`, `toggle(&mut bool)`.
   ```rust
   ui.text_input(&mut TextInputState);   // not &mut String
   ui.tabs(&mut TabsState);              // not &mut usize
   ```

5. **Responses.** Single-rect widgets return `Response`. Compound widgets return `<Widget>Response: Deref<Target = Response>` with `#[must_use]`. Never tuples.
   ```rust
   if ui.button("Save").clicked { ... }                                          // Response
   if let Some(i) = ui.breadcrumb(&segs).show().clicked_segment { ... }          // BreadcrumbResponse
   ```

6. **Return-type pattern.** Methods on `Context` return one of two types — picking the wrong one is the most common AI-generated compile error.
   - `&mut Self` — *chainable mutators of the last rendered element.* Use for: `text`, `link`, `styled`, `timer_display`, and the style chain (`bold`, `dim`, `italic`, `fg`, `bg`, `wrap`, `truncate`, `align`, `text_center`, `m`, `mx`, `w`, `h`, `grow`, `spacer`, `with_if`, `with`).
   - `Response` — *interaction result of an independently-rendered widget.* Use for every stateful interactive widget (`button`, `checkbox`, `toggle`, `table`, `tabs`, `select`, `radio`, `multi_select`, `text_input`, `list`, `tree`, `file_picker`, `slider`, `calendar`, `command_palette`, `rich_log`), plus `separator()`.
   - Container helpers split: `col` / `row` / `modal` → `Response`. `line` / `line_wrap` → `&mut Self` (these continue an inline-text chain). `screen(name, &mut ScreenState, f)` returns `()`.

   ```rust
   ui.button("Save").bold();           // ❌ Response has no .bold() — compile error
   if ui.button("Save").clicked { … }  // ✅ Response field
   ui.text("Saved").bold().fg(green);  // ✅ &mut Self chain on display element
   ```

## Naming categories (NAMING.md micro tier)

Method names encode their category. When picking a name, match the category shape of nearby methods.

| Category | Shape | Examples |
|---|---|---|
| **Verbs** (actions, side effects) | `<verb>` or `<verb>_<object>` | `quit`, `notify`, `register_focusable`, `focus_by_name`, `consume_event`, `set_ratio` |
| **Nouns** (getters, no side effects) | `<noun>` or `<noun>_<modifier>` | `theme`, `width`, `events`, `focused_name`, `state.cursor`. **Never `get_X`.** |
| **Adjectives** (Layer 2 builder modifiers) | short, ≤2 syllables | `bordered`, `bg`, `fg`, `p`, `m`, `w`, `h`, `gap`, `grow`, `fill`, `bold`, `dim` |
| **Constructors** | `Type::default()` / `Type::new(args)` / `Type::with_X(arg)` | `TextInputState::default()`, `SplitPaneState::new(0.5)`, `TextInputState::with_placeholder("…")` |

**Allowed universal abbreviations**: `bg fg id idx len min max pos pct w h x y r g b a`.
**Forbidden**: `ctx btn lbl dbg cfg req res srv db` in public API. Closure params over `&mut Context` are always `ui`, never `ctx`.

## Layer model (5 layers, predictability anchor)

| Layer | Object | Examples |
|---|---|---|
| 1 — Context | `&mut Context` (the `ui` parameter) | `ui.text(...)`, `ui.button(...)`, `ui.row(...)`, `ui.theme()`, `ui.use_state(...)` |
| 2 — ContainerBuilder | returned by `ui.container()`, `ui.bordered(...)` | chained adjectives: `.p(2).bg(c).gap(1).col(|ui| ...)` |
| 3 — Widget | `impl Widget for MyType { type Response; fn ui(...) }` | custom widget extension point |
| 4 — State | `pub struct <Widget>State` in `src/widgets/*.rs` | `TextInputState`, `TableState`, `ScrollState` |
| 5 — Response | `Response` or `<Widget>Response: Deref<Response>` | `Response { clicked, right_clicked, double_clicked, hovered, changed, focused, gained_focus, lost_focus, submitted, scroll_delta, rect }` |

When a method belongs to two layers (`ui.bordered(B)` shortcut vs `ui.container().border(B)` explicit), prefer the explicit form in skill output.

## Sibling widget shapes (memorize)

When unsure about a widget signature, find its family and match.

```rust
// Stateful interactive widgets — &mut <Widget>State, returns Response
ui.list(&mut ListState);              ui.tabs(&mut TabsState);
ui.table(&mut TableState);            ui.tree(&mut TreeState);
ui.select(&mut SelectState);          ui.radio(&mut RadioState);
ui.multi_select(&mut MultiSelectState);
ui.file_picker(&mut FilePickerState); ui.calendar(&mut CalendarState);
ui.text_input(&mut TextInputState);   ui.textarea(&mut TextareaState, rows);  // textarea takes rows
ui.rich_log(&mut RichLogState);       ui.command_palette(&mut CommandPaletteState);
ui.toast(&mut ToastState);            ui.spinner(&SpinnerState);              // toast returns &mut Self; spinner is &, not &mut

// Trivial-value siblings — exception to Rule 4
ui.button("Save");                    // Response
ui.checkbox("Done", &mut done);       // Response
ui.toggle("Enabled", &mut on);        // Response
ui.slider("Vol", &mut value, 0.0..=100.0);  // Response

// Builder widgets — chain optional config, render on Drop
ui.gauge(0.6).label("60%").width(24).color(Color::Cyan);
ui.line_gauge(0.6).filled('━').empty('─').width(24).label("60%");
let r = ui.breadcrumb(&segs).separator(" › ").color(Color::Cyan).show();

// Compound responses — Deref<Response> + extra fields
let r = ui.gauge(cpu).label("CPU").show();          // GaugeResponse { ratio: f64 }
let r = ui.breadcrumb(&segs).show();                // BreadcrumbResponse { clicked_segment: Option<usize> }
let r = ui.split_pane(&mut split, l, r);            // SplitPaneResponse { ratio: f64 }
let r = ui.scrollable_with_gutter(&mut scroll, opts, body);  // GutterResponse { current_highlight: Option<usize> }
let r = ui.form_field_response(&mut field);         // FormFieldResponse { response, input, layout_rect } (v0.25)
```

## Hook ordering — three variants

Hooks must be called in the same order every frame **unless** they are id-keyed.

| Hook | Key | Safe in `if`/`match`? | Use when |
|---|---|---|---|
| `ui.use_state(\|\| init)` | call order | **No** | Top-level state, no conditional placement |
| `ui.use_state_named("id", \|\| init)` | `&'static str` | **Yes** | Conditional/branching state with compile-time id and initializer |
| `ui.use_state_named_default::<T>("id")` | `&'static str` | **Yes** | Same, using `T::default()` |
| `ui.use_state_keyed("id-{i}", \|\| init)` | runtime `String` | **Yes** | Per-row state in a list (key from data) |
| `ui.use_state_keyed_default("id-{i}")` | runtime `String` | **Yes** | Same, `T: Default` shortcut |
| `ui.use_memo(&deps, \|d\| compute(d))` | call order + deps | **No** | Cached compute, deps change → recompute |
| `ui.use_effect(\|d\| { ... }, &deps)` | call order + deps | **No** | Side effect on deps change |

```rust
// WRONG — order-based hook in conditional drifts call order between frames
if expanded { let count = ui.use_state(|| 0); }

// RIGHT — id-keyed variant is safe inside conditionals
if expanded { let count = ui.use_state_named_default::<i32>("sidebar.count"); }

// Per-list-item runtime keys
for i in 0..items.len() {
    let count = ui.use_state_keyed_default::<i32>(format!("counter-{i}"));
}
```

## Context injection (`provide` / `use_context`)

Stop threading `&theme`, `&tick`, `&mut toasts` through every render fn. `provide` injects a typed value scoped to a closure; nested code reads it back with `use_context`.

```rust
struct AppCtx { theme: slt::Theme, tick: u64, user: &'static str }

slt::run(|ui| {
    let ctx = AppCtx { theme: *ui.theme(), tick: ui.tick(), user: "subin" };
    ui.provide(ctx, |ui| {
        render_header(ui);
        render_card(ui);
    });
});

fn render_card(ui: &mut slt::Context) {
    let ctx = ui.use_context::<AppCtx>();           // panics if missing
    // let maybe = ui.try_use_context::<AppCtx>(); // returns Option<&T>
    ui.text(format!("hi {} (tick {})", ctx.user, ctx.tick));
}
```

Reserve explicit parameters for **writes** (`&mut MyDocState`). Bound is `T: 'static` — use `&'static str` for literals, `String` for runtime values.

## Conditional styling (`with_if` / `with`)

`with_if(cond, modifier)` and `with(modifier)` collapse conditional branches into a single chain. Available on text and `ContainerBuilder`. Beware: text uses `&mut self -> &mut Self`, ContainerBuilder uses consuming `Self -> Self`.

```rust
// text — closure receives &mut Self
ui.text("Status").with_if(is_error, |t| { t.bold().fg(Color::Red); });

// ContainerBuilder — closure receives Self by value
ui.container().with_if(is_focused, |c| c.bg(theme.surface_hover)).col(|ui| ...);
```

## Custom widget pattern (Layer 3)

**When to use which pattern**:
- *Function* (`fn render_card(ui: &mut Context, data: &CardData)`): 90% of cases. Use for screens, sections, reusable layouts. Cheaper to write, no trait bounds, easier to test. Built-in widgets follow this shape internally (`impl Context` direct methods).
- *`impl Widget`*: when the component (a) has its own state struct that the *caller* owns, and (b) you want `ui.widget(&mut w)` ergonomics matching built-ins. Required for third-party crates that export widgets through trait-bound APIs.

```rust
struct Label<'a> { text: &'a str }

impl<'a> slt::Widget for Label<'a> {
    type Response = slt::Response;
    fn ui(&mut self, ui: &mut slt::Context) -> Self::Response {
        ui.register_focusable();
        ui.text(self.text).bold();
        slt::Response::default()
    }
}

let mut label = Label { text: "hello" };
ui.widget(&mut label);              // or call `label.ui(ui)` with `Widget` in scope
```

For mouse hit-testing call `ui.interaction()` (no arguments) immediately before the element that should own the hit rect; it returns a `Response`. For keyboard, `let focused = ui.register_focusable();`, iterate `ui.key_presses_when(focused)` (yields `(index, &KeyEvent)`), and mark handled keys with `ui.consume_event(index)` so later widgets and shortcuts ignore them.

## Focus, scrolling and geometry (v0.24 / v0.25)

- **Scroll containers follow focus.** `ui.scrollable(&mut scroll)` reveals the newly focused child (Tab into an off-screen field scrolls it into view). `ScrollState::follow_focus` is a public `bool`, default `true`; set `scroll.follow_focus = false` for application-controlled scrolling.
- **Forms with geometry.** `ui.form_field(&mut field)` stays chainable (`&mut Self`). `ui.form_field_response(&mut field) -> FormFieldResponse` returns `response` (field-level flags, also via `Deref`), `input` (the input's own response) and `layout_rect: Option<Rect>` (previous-frame full border box).
- **Full vs clipped rects.** `Response.rect` is the clipped hit rect. `ui.focused_layout_rect()` and `ui.measured_layout_rect("name")` return the previous frame's full layout box (before scroll offset and clipping), `None` on frame 1 or after resize.
- **Headless time.** `TestBackend` ticks advance per frame; use `tb.advance_time(Duration::ZERO)` for a fixed clock. `has_pending_input()` reports input queued behind a Tab group (custom loops on `AppState` too).
- **Bounded canvas.** `ui.try_canvas(w, h, |cv| ..) -> Result<Response, CanvasError>`; `ui.canvas_with(&mut canvas, |cv| ..)` reuses storage across frames.

## Authoring workflow

1. Confirm the goal — what app? Data table? Dashboard? Form? Game?
2. Check `examples/` for the closest pattern (see Reference Examples below). Start from that file if it fits.
3. Grep `src/context/widgets/` and `src/widgets/` for the actual signature before writing `ui.foo(...)`. **Do NOT invent APIs.**
4. Keep `Cargo.toml` features minimal — see REFERENCES.md.
5. Run the quality gate before saying "done".

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
bash scripts/api_audit.sh --strict
cargo check -p superlighttui --no-default-features
cargo check -p slt-wasm --target wasm32-unknown-unknown
cargo hack check -p superlighttui --each-feature --no-dev-deps
cargo audit
cargo deny check
```

## Release workflow (mandatory — do not skip any step)

`AGENTS.md` has the full 8-step checklist. Short version:

1. Local PRE-CI (Core + Extended both green)
2. Bump `Cargo.toml`, update `CHANGELOG.md`
3. Branch `release/vX.Y.Z`, single atomic commit, push
4. `gh pr create`, **wait** for CI green
5. Merge (squash), pull main
6. Tag, push tag, **wait** for `release.yml` green
7. Verify `gh release view`, crates.io, docs.rs
8. Only now announce

Red flags that mean STOP: "Probably fine", "Just a docs change", "CI will catch it", "I'll tag now and fix later". Run the gate locally first.

## Common pitfalls (AI-generated SLT code)

- **Inventing method names.** Always grep `src/context/` and `src/widgets/` first.
- **Expecting hover without `MouseMode::Hover`.** Clicks, wheel and drags work by default, but plain pointer moves are not reported; see "Two runtime rules" above. Do not add `.mouse(true)` just to "turn the mouse on".
- **Shortcut checked before a text input renders.** Typing `q` quits the app. Render inputs first, then check `ui.key(...)`.
- **Assuming `ui.modal` traps focus.** Plain `modal()` keeps legacy `tab_trap: false`. Use `ui.modal_with(slt::context::ModalOptions { tab_trap: true }, |ui| ...)` (also `ModalOptions::default()`).
- **Calling `ui.interaction(rect)` or `available_key_presses()`.** `interaction()` takes no arguments; `available_key_presses` is crate-private. Use `key_presses_when(focused)` + `consume_event(i)`.
- **Stale removed APIs.** `gauge_w`, `gauge_colored`, `line_gauge_with`, `breadcrumb_sep`, `LineGaugeOpts`, `HighlightRange::single`, `label_owned` are GONE in v0.20. Use the builder forms.
- **`Response.rect` on frame 1.** Zero `Rect`. Guard with `ui.tick() > 0`.
- **`use_state()` inside `if`/`match`/`for`.** Use `use_state_named` (`&'static str` id) or `use_state_keyed` (runtime `String`).
- **Forgetting `.show()` on builders that return a response.** Drop renders and discards the response. Capture with `let r = ui.gauge(...).show();`.
- **`'static` on deferred raw draw.** Use `draw_with` for owned data, or `draw_precomputed` when the closure must borrow local data and fixed source dimensions are acceptable.
- **Mixing crossterm raw events with `ui.*` helpers.** Prefer `ui.key()`, `ui.key_code()`, `ui.key_mod()`. For modal-aware shortcuts use `ui.raw_key_*`.
- **Hard-coding `Color::Rgb(...)` instead of `ui.theme()`** — themes can't swap.
- **`RichLogState::new()` for unbounded.** New caps at 10000; use `RichLogState::new_unbounded()` if you really want unlimited.
- **First-frame hover/click tests.** Render once to warm the prev-frame hit map, then send the event in a second `tb.render(...)` call.
- **Binding only Ctrl-C as quit.** macOS terminals intercept Ctrl-C as Copy. Always pair `q`, `Esc`, and `Ctrl-Q`.
- **`unsafe` blocks.** `#![forbid(unsafe_code)]`. Hard compile error.
- **Printing to stdout/stderr from a widget.** A library must not write to stdout. Lints catch this.

## Reference examples (skill should reference these by file:line)

Only Cargo-listed targets are runnable (`cargo run --example <name>`; 29 targets, auto-discovery is off). Every other file under `examples/` is compiled as a module of a tour target or test (`#[path = ...] mod`), so all of them are type-checked by `cargo check --all-targets --all-features`; `tests/docs_contract.rs` enforces that no example source is left uncompiled.

| Domain | Reference file | Compiled via | Key shape |
|---|---|---|---|
| Hello / minimal | `examples/hello.rs` (21 lines) | target `hello` | `slt::run`, `bordered.title.col`, quit triple |
| Counter (state in closure) | `examples/counter.rs` | target `counter` | move-closure state pattern |
| Inline mode | `examples/inline.rs` (23 lines) | target `inline` | `run_inline(rows, ...)` |
| Tabbed tour | `examples/cookbook_tour.rs` | target `cookbook_tour` | `TabsState` + child `pub fn render(ui, &mut DemoState)` |
| Form / validators | `examples/cookbook_login.rs` | `cookbook_tour` | `TextInputState::with_placeholder`, masked password, validation |
| Searchable+sortable table | `examples/cookbook_table.rs` | `cookbook_tour` | `TableState::set_filter`, `toggle_sort`, `consume_key` |
| Modal + Toast | `examples/cookbook_modal_toast.rs` | `cookbook_tour` | `ButtonVariant::Danger`, `raw_key_code(Esc)` for modal-aware quit |
| Modal focus trap | `examples/modal_trap.rs` | `widgets_tour` | `ModalOptions { tab_trap: true }` |
| File picker | `examples/cookbook_file_picker.rs` | `cookbook_tour` | `FilePickerState::selected_file()` |
| Dashboard (chart+sparkline) | `examples/cookbook_dashboard.rs` | `cookbook_tour` | `ui.chart(\|c\| c.line(...).color(..), w, h)`, rolling `VecDeque<f64>` |
| Animation primitives | `examples/canvas_tour.rs` (Anim tab) | target `canvas_tour` | Tween/Spring/Keyframes/Sequence/Stagger; standalone version `examples/anim.rs` (target `anim`) |
| `use_state_keyed` | `examples/use_state_keyed.rs` | `widgets_tour` | per-row counter via `format!("counter-{i}")` |
| `use_effect` | `examples/use_effect.rs` | `widgets_tour` | three dep shapes (`&()`, `&i32`, `&bool`) |
| `provide`/`use_context` | `tests/context_provider.rs`, `examples/demo_website.rs:139-152` | test / `showcase_tour` | injection + `try_use_context` |
| Theme subtree | `examples/theme_subtree.rs` | `widgets_tour` | `container().theme(theme)` per-subtree override |
| Theme density | `examples/spacing_scale.rs` | `widgets_tour` | `Theme::compact() / comfortable() / spacious()` |
| Gauge builder | `examples/gauge.rs:104-107` | `widgets_tour` | `ui.gauge(value).label(...).width(24)` |
| Line gauge | `examples/gauge.rs:74-92` | `widgets_tour` | `ui.line_gauge(0.45).filled('#').empty('.').width(24)` |
| Breadcrumb response | `examples/breadcrumb_response.rs:72-77` | `widgets_tour` | `ui.breadcrumb(&segs).separator(" › ").show()` |
| Scrollable + gutter | `examples/gutter_highlights.rs:150-165` | `widgets_tour` | `GutterOpts::line_numbers`, `HighlightRange::line` |
| Split pane | `examples/split_pane.rs` | `widgets_tour` | `split_pane`, `vsplit_pane`, drag handle |
| WidthSpec variants | `examples/widthspec.rs` | `widgets_tour` | `Constraints::default().w_pct(50)`, `.w_ratio(1,3)`, `.w_minmax(10,30)` |
| Named focus | `examples/named_focus.rs:187` | `widgets_tour` | `register_focusable_named` + `focus_by_name` |
| Keymap help overlay | `examples/keymap_help.rs` | `widgets_tour` | `WidgetKeyHelp`, `publish_keymap`, `keymap_help_overlay(open)` |
| DX shortcuts | `examples/dx_shortcuts.rs` | `widgets_tour` | `on_hover`, `animate_bool`, `fill()`, `Rect::center_in` |
| Static log / scrollback | `examples/static_log.rs` | `widgets_tour` | `slt::run_static`, `ui.static_log(...)` |
| Async | `examples/async_demo.rs` | target `async_demo` (needs `--features async`) | `slt::run_async` with tokio, `.sender()`, `.join()` |
| All-in-one tour | `examples/widgets_tour.rs` | target `widgets_tour` | tabs over every single-feature demo; `examples/feature_showcase.rs` (target `feature_showcase`) composes them on one screen |
| Recent widgets | `examples/widgets_extra.rs`, `examples/interaction_tour.rs` | targets `widgets_extra`, `interaction_tour` | paginator / number_input / scheduler / inspector; `Response` interaction signals, `KeyMap`, focus traversal |
| Test utilities | `tests/example_snapshots/testing_report.rs` | test | `record_frames`, `sequence().tick(..).key(..).type_string(..)` |

## Testing pattern (headless)

```rust
use slt::{TestBackend, EventBuilder};

#[test]
fn my_widget_renders() {
    let mut tb = TestBackend::new(80, 24);
    tb.render(|ui| { ui.text("hello"); });
    tb.assert_contains("hello");
}

// Multi-frame with events (warm prev-frame hit map first)
#[test]
fn click_triggers() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| { ui.button("Save"); });                            // warm frame
    tb.run_with_events(EventBuilder::new().click(2, 0).build(),        // event frame
        |ui| { if ui.button("Save").clicked { /* assert */ } });
}

// Sequence builder (most readable): each step takes the render closure
tb.sequence()
    .tick(|ui| { ui.text("ready"); })
    .key(KeyCode::Esc, |ui| { ui.text("after esc"); })
    .run();
tb.assert_contains("after esc");
// Steps are FnOnce closures held together; for shared mutable state use
// tb.type_string("hello", |ui| { ... }) or successive tb.render(...) calls.
```

See `tests/example_snapshots/testing_report.rs` for `record_frames`, `assert_not_contains`, `assert_style_at`.

## File layout cheat sheet

| Area | Primary files |
|---|---|
| Public API | `src/lib.rs` (re-exports) |
| Run loop / backend | `src/terminal.rs`, `src/lib.rs` (`run`, `run_with`, `run_inline`, `run_async`, `run_static`, `frame`, `frame_owned`) |
| Context core | `src/context/{core,runtime,container,helpers,state}.rs` |
| Widget impls | `src/context/widgets/<family>.rs` (+ `<family>/`), same family names as the state files |
| Layer 4 state types | `src/widgets/<family>.rs` |
| Compound responses | next to their family's state: `BreadcrumbResponse` (`display.rs`), `GaugeResponse` (`feedback.rs`), `SplitPaneResponse` / `GutterResponse` (`layout.rs`), `FormFieldResponse` (`text_input.rs`) |
| Layout kernels | `src/layout/{tree,flexbox,collect,render,command}.rs` |
| Style / theme | `src/style/{color,theme}.rs`, `src/style.rs` |
| Animation | `src/anim.rs` |
| Charts | `src/chart.rs`, `src/chart/*.rs`, `src/context/widgets/viz/*.rs` |
| Testing helpers | `src/test_utils.rs` |
| Skill references | `REFERENCES.md` (feature flags, doc pointers) |

## Korean conventions

- "ㄱㄱ" = "go go" → proceed immediately, no clarifying questions
- "켜줘" / "열어줘" → open the file in Cursor (NOT `cat` to terminal)
- "고쳐줘" / "수정해줘" → fix with minimal change, run quality gate
- "리뷰해줘" → audit only, do not modify code unless explicitly asked
