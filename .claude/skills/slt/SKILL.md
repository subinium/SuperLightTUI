---
name: slt
description: Build Rust TUI apps with SuperLightTUI v0.20 (immediate-mode terminal UI). Use this skill when the user asks to create, modify, or debug terminal UI code in this repo, or asks "how do I X in SLT / TUI / terminal", or types Korean triggers like "터미널 UI", "TUI 만들어줘", "SLT로", "ratatui 대신". Read REFERENCES.md for feature flags and doc pointers; grep `src/context/` and `src/widgets/` before inventing any API.
---

# SuperLightTUI (SLT) Authoring Skill — v0.20

## Mental model

SLT is **immediate-mode**. Your app is one closure: `slt::run(|ui: &mut Context| { ... })`. The closure runs every frame. State lives in plain Rust variables outside the closure — no `App` trait, no `Model/View/Update`, no retained tree. SLT handles flexbox layout, ANSI diff, and stdout flush.

`Response.rect` reflects the **previous** frame because layout runs after the closure returns. Frame 1 returns a zero `Rect`. Guard measurement-dependent logic with `if ui.tick() > 0 { ... }`. See `docs/PREVIOUS_FRAME_GUIDE.md`.

For larger apps write **components as functions**: `fn render_card(ui: &mut Context, data: &Card)`. Share read-mostly state with `ui.provide(value, |ui| ...)` + `ui.use_context::<T>()` instead of threading `&theme` through every helper.

## The 5 API rules (predictability anchors)

These are non-negotiable in v0.20+. When generating new code, every public widget must match all 5.

1. **Builder for optional config.** Methods on `Context` return a builder when ≥1 option exists. Builders chain `&mut self -> &mut Self`, render on `Drop`, expose `.show()` to capture a `*Response`.
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
   - `&mut Self` — *chainable mutators of the last rendered element.* Use for: `text`, `link`, `styled`, `separator`, `timer_display`, and the style chain (`bold`, `dim`, `italic`, `fg`, `bg`, `wrap`, `truncate`, `align`, `text_center`, `m`, `mx`, `w`, `h`, `grow`, `spacer`, `with_if`, `with`).
   - `Response` — *interaction result of an independently-rendered widget.* Use for every stateful interactive widget (`button`, `checkbox`, `toggle`, `table`, `tabs`, `select`, `radio`, `multi_select`, `text_input`, `list`, `tree`, `file_picker`, `slider`, `calendar`, `command_palette`, `rich_log`).
   - Container helpers split: `col` / `row` / `modal` → `Response`. `line` / `line_wrap` / `screen` → `&mut Self` (these continue an inline-text chain).

   ```rust
   ui.button("Save").bold();           // ❌ Response has no .bold() — compile error
   if ui.button("Save").clicked { … }  // ✅ Response field
   ui.text("Saved").bold().fg(green);  // ✅ &mut Self chain on display element
   ```

## Naming categories (NAMING.md micro tier)

Method names encode their category. When picking a name, match the category shape of nearby methods.

| Category | Shape | Examples |
|---|---|---|
| **Verbs** (actions, side effects) | `<verb>` or `<verb>_<object>` | `quit`, `notify`, `register_focusable`, `focus_by_name`, `consume_indices`, `set_ratio` |
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
| 5 — Response | `Response` or `<Widget>Response: Deref<Response>` | `Response { clicked, hovered, changed, focused, rect, right_clicked, gained_focus, lost_focus }` |

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
ui.toast(&mut ToastState);            ui.spinner(&SpinnerState);              // spinner is &, not &mut

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
```

## Hook ordering — three variants

Hooks must be called in the same order every frame **unless** they are id-keyed.

| Hook | Key | Safe in `if`/`match`? | Use when |
|---|---|---|---|
| `ui.use_state(\|\| init)` | call order | **No** | Top-level state, no conditional placement |
| `ui.use_state_named::<T>("id")` | `&'static str` | **Yes** | Conditional/branching state with compile-time id |
| `ui.use_state_named_with("id", \|\| init)` | `&'static str` | **Yes** | Same, with explicit init fn |
| `ui.use_state_keyed("id-{i}", \|\| init)` | runtime `String` | **Yes** | Per-row state in a list (key from data) |
| `ui.use_state_keyed_default("id-{i}")` | runtime `String` | **Yes** | Same, `T: Default` shortcut |
| `ui.use_memo(&deps, \|d\| compute(d))` | call order + deps | **No** | Cached compute, deps change → recompute |
| `ui.use_effect(\|d\| { ... }, &deps)` | call order + deps | **No** | Side effect on deps change |

```rust
// WRONG — order-based hook in conditional drifts call order between frames
if expanded { let count = ui.use_state(|| 0); }

// RIGHT — id-keyed variant is safe inside conditionals
if expanded { let count = ui.use_state_named::<i32>("sidebar.count"); }

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

ui.add(Label { text: "hello" });   // or call .ui(ui) directly
```

For mouse hit-testing use `ui.interaction(rect)`. For keyboard use `register_focusable()` + `available_key_presses()`.

## Authoring workflow

1. Confirm the goal — what app? Data table? Dashboard? Form? Game?
2. Check `examples/` for the closest pattern (see Reference Examples below). Start from that file if it fits.
3. Grep `src/context/widgets_*` and `src/widgets/*` for the actual signature before writing `ui.foo(...)`. **Do NOT invent APIs.**
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
cargo check -p superlighttui --no-default-features
cargo check -p slt-wasm --target wasm32-unknown-unknown
cargo hack check -p superlighttui --each-feature --no-dev-deps
cargo audit
cargo deny check
```

## Release workflow (mandatory — do not skip any step)

`CLAUDE.md` has the full 8-step checklist. Short version:

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
- **Stale removed APIs.** `gauge_w`, `gauge_colored`, `line_gauge_with`, `breadcrumb_sep`, `LineGaugeOpts`, `HighlightRange::single`, `label_owned` are GONE in v0.20. Use the builder forms.
- **`Response.rect` on frame 1.** Zero `Rect`. Guard with `ui.tick() > 0`.
- **`use_state()` inside `if`/`match`/`for`.** Use `use_state_named` (`&'static str` id) or `use_state_keyed` (runtime `String`).
- **Forgetting `.show()` on builders that return a response.** Drop renders and discards the response. Capture with `let r = ui.gauge(...).show();`.
- **`'static` on `ContainerBuilder::draw()` closure.** Raw draw is deferred; the closure must be `'static`.
- **Mixing crossterm raw events with `ui.*` helpers.** Prefer `ui.key()`, `ui.key_code()`, `ui.key_mod()`. For modal-aware shortcuts use `ui.raw_key_*`.
- **Hard-coding `Color::Rgb(...)` instead of `ui.theme()`** — themes can't swap.
- **`RichLogState::new()` for unbounded.** New caps at 10000; use `RichLogState::new_unbounded()` if you really want unlimited.
- **First-frame hover/click tests.** Render once to warm the prev-frame hit map, then send the event in a second `tb.render(...)` call.
- **Binding only Ctrl-C as quit.** macOS terminals intercept Ctrl-C as Copy. Always pair `q`, `Esc`, and `Ctrl-Q`.
- **`unsafe` blocks.** `#![forbid(unsafe_code)]`. Hard compile error.
- **Printing to stdout/stderr from a widget.** A library must not write to stdout. Lints catch this.

## Reference examples (skill should reference these by file:line)

| Domain | Reference file | Key shape |
|---|---|---|
| Hello / minimal | `examples/hello.rs` (21 lines) | `slt::run`, `bordered.title.col`, quit triple |
| Counter (state in closure) | `examples/counter.rs` | move-closure state pattern |
| Inline mode | `examples/inline.rs` (23 lines) | `run_inline(rows, ...)` |
| Tabbed tour | `examples/cookbook_tour.rs` | `TabsState` + child `pub fn render(ui, &mut DemoState)` |
| Form / validators | `examples/cookbook_login.rs` | `TextInputState::with_placeholder`, masked password, validation |
| Searchable+sortable table | `examples/cookbook_table.rs` | `TableState::set_filter`, `toggle_sort`, `consume_key` |
| Modal + Toast | `examples/cookbook_modal_toast.rs` | `ButtonVariant::Danger`, `raw_key_code(Esc)` for modal-aware quit |
| Modal focus trap | `examples/v020_modal_trap.rs` | `ModalOptions { tab_trap: true }` |
| File picker | `examples/cookbook_file_picker.rs` | `FilePickerState::selected_file()` |
| Dashboard (chart+sparkline) | `examples/cookbook_dashboard.rs` | `ui.chart(\|c\| c.line(...).color())`, rolling `VecDeque<f64>` |
| Animation primitives | `examples/anim.rs` | Tween/Spring/Keyframes/Sequence/Stagger |
| `use_state_keyed` | `examples/v020_use_state_keyed.rs` | per-row counter via `format!("counter-{i}")` |
| `use_effect` | `examples/v020_use_effect.rs` | three dep shapes (`&()`, `&i32`, `&bool`) |
| `provide`/`use_context` | `tests/context_provider.rs` (100 lines), `examples/demo_website.rs:139-152` | injection + `try_use_context` |
| Theme subtree | `examples/v020_theme_subtree.rs` | `container().theme(theme)` per-subtree override |
| Theme density | `examples/v020_spacing_scale.rs` | `Theme::compact() / comfortable() / spacious()` |
| Gauge builder | `examples/v020_gauge.rs:104-107` | `ui.gauge(value).label(...).width(24)` |
| Line gauge | `examples/v020_gauge.rs:74-92` | `ui.line_gauge(0.45).filled('#').empty('.').width(24)` |
| Breadcrumb response | `examples/v020_breadcrumb_response.rs:72-77` | `ui.breadcrumb(&segs).separator(" › ").show()` |
| Scrollable + gutter | `examples/v020_gutter_highlights.rs:150-165` | `GutterOpts::line_numbers`, `HighlightRange::line` |
| Split pane | `examples/v020_split_pane.rs` | `split_pane`, `vsplit_pane`, drag handle |
| WidthSpec variants | `examples/v020_widthspec.rs` | `Constraints::default().w_pct(50)`, `.w_ratio(1,3)`, `.w_minmax(10,30)` |
| Named focus | `examples/v020_named_focus.rs:187` | `register_focusable_named` + `focus_by_name` |
| Keymap help overlay | `examples/v020_keymap_help.rs` | `WidgetKeyHelp`, `publish_keymap`, `keymap_help_overlay` |
| DX shortcuts | `examples/v020_dx_shortcuts.rs` | `on_hover`, `animate_bool`, `fill()`, `Rect::center_in` |
| Static log / scrollback | `examples/v020_static_log.rs` | `slt::run_static`, `ui.static_log(...)` |
| Async demo | `examples/async_demo.rs` | `slt::run_async` with tokio |
| All-in-one showcase | `examples/v020_showcase.rs` (277 lines) | every major v0.20 feature |
| Test utilities | `tests/v020_test_utils_demo.rs` | `record_frames`, `sequence().tick().key().type_string()` |

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
    tb.render(|ui| { ui.button("Save"); });                    // warm frame
    tb.run_with_events(vec![EventBuilder::mouse_down(2, 0)],   // event frame
        |ui| { if ui.button("Save").clicked { /* assert */ } });
}

// Sequence builder (most readable)
tb.sequence().tick(5).key(KeyCode::Tab, KeyModifiers::NONE).type_string("hello", &mut state.value).run();
```

See `tests/v020_test_utils_demo.rs` for `record_frames`, `assert_not_contains`, `assert_style_at`.

## File layout cheat sheet

| Area | Primary files |
|---|---|
| Public API | `src/lib.rs` (re-exports) |
| Run loop / backend | `src/terminal.rs`, `src/lib.rs` (`run`, `run_with`, `run_inline`, `run_async`, `run_static`, `frame`, `frame_owned`) |
| Context core | `src/context/{core,runtime,container,helpers,state}.rs` |
| Widget impls | `src/context/widgets_display/*`, `widgets_interactive/*`, `widgets_input/*`, `widgets_viz.rs` |
| Layer 4 state types | `src/widgets/*.rs` |
| Compound responses | `src/widgets/responses.rs` (`BreadcrumbResponse`, `GaugeResponse`, `SplitPaneResponse`, `GutterResponse`) |
| Layout kernels | `src/layout/{tree,flexbox,collect,render,command}.rs` |
| Style / theme | `src/style/{color,theme}.rs`, `src/style.rs` |
| Animation | `src/anim.rs` |
| Charts | `src/chart.rs`, `src/chart/*.rs`, `src/context/widgets_viz.rs` |
| Testing helpers | `src/test_utils.rs` |
| Skill references | `REFERENCES.md` (feature flags, doc pointers) |

## Korean conventions

- "ㄱㄱ" = "go go" → proceed immediately, no clarifying questions
- "켜줘" / "열어줘" → open the file in Cursor (NOT `cat` to terminal)
- "고쳐줘" / "수정해줘" → fix with minimal change, run quality gate
- "리뷰해줘" → audit only, do not modify code unless explicitly asked
