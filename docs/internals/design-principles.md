# SLT Design Principles

<!-- slt-check-default: illustrative fragments require application context -->

These principles guide every design decision in SLT. Read this before
contributing code. If a decision conflicts with these principles, raise
it in the PR.

This is a **living document**. Every PR that adds, removes, or changes a
public API must:

1. Identify which principles the change touches.
2. Identify which tiers are affected (Macro / Meso / Micro / Detail).
3. Update the [Audit Matrix](#5-audit-matrix-v020) if a cell's status changes.
4. Run `scripts/api_audit.sh` and address any V1 / V2 / V4 flags.

Related docs:
- [Quick start](../guide/01-quick-start.md)
- [Widget guide](../reference/widgets.md) — current widget catalog
- [Patterns guide](../guide/03-patterns.md) — composition patterns
- [Architecture guide](architecture.md) — Macro tier (layers, modules)
- [API review rules](#api-review-rules) (this page) — Meso tier (signatures)
- [Naming conventions](#naming-conventions) (this page) — Micro tier (identifiers)
- [Rustdoc guide](rustdoc-guide.md) — Detail tier (rustdoc)

---

## Document Map

This document is layered. Read top-to-bottom on first pass; jump to
specific sections on later visits.

```
1. The North Star — Predictability         ← why every other rule exists
2. Meta-Principles (P1–P7)                 ← how to evaluate a design choice
3. Concrete Rules (R1–R10)                 ← what the choices have settled into
4. The 4-Tier Convention Stack             ← where rules live
   API review rules                        ← Meso tier: signature rules + PR checklist
   Naming conventions                      ← Micro tier: identifier shapes
5. Audit Matrix (v0.20)                    ← historical design baseline
6. Roadmap (v0.20 → v1.0)                  ← shipped phases + remaining work
7. Failure Mode Catalog                    ← real bugs, classified
8. How to Use This Document                ← reviewer / contributor workflow
```

---

## 1. The North Star — Predictability

**A developer (or AI assistant) reading SLT for the first time should be
able to predict the next method call without checking the docs.** If they
can't, the API is broken — fix the API, not the reader.

ratatui is predictable because every widget is a struct with `default()`
plus chained setters. egui is predictable because every interaction is
`ui.<verb>(args)` and every method returns the same kind of `Response`.
SLT mixes both styles — more **expressive**, but only **predictable** when
the mixing rules are themselves predictable.

Two failure modes to watch for:

- **Surprise on next-call**: a developer who just wrote `ui.gauge(0.6)`
  cannot predict whether the label argument goes inside `gauge(...)` or
  chains as `.label(...)`. (v0.20 answer: chains. We unified to builder
  in #224.)
- **Surprise on next-widget**: a developer who learned
  `ui.text_input(&mut state)` cannot predict that `ui.gauge(0.6).label(...)`
  chains instead. Different widget family, different shape — that's
  allowed, but the *family boundary* must be obvious from the call.

Predictability is what makes SLT viable as a library:
- For humans: lower memory tax → faster iteration → more shipped TUIs.
- For AI assistants: training-data-style guesses succeed → AI-built
  TUIs work on first try.
- For the project: every kept-promise is documentation that doesn't
  need writing.

---

## 2. Meta-Principles (P1–P7)

These seven principles govern *how* design decisions are made. They are
the criteria a reviewer applies when judging a new API. They are not
project-specific (any TUI library could adopt them); the project-specific
results are codified in §3 [Concrete Rules](#3-concrete-rules-r1r10).

### P1 — Predictability over Cleverness

**Rule**: choose the boring shape that matches existing widgets, even if a
clever shorthand exists.

**Why**: every clever shorthand is a memory tax. `Context` alone has
roughly 290 public methods; users remember patterns, not individual calls.

**Apply when**: introducing a new widget, alias, shortcut, or convenience.

**Anti-pattern (caught in v0.20)**: `gauge_w(ratio, w)` and
`gauge_colored(ratio, c)` introduced as shortcuts. Same-release
deprecation: removed entirely, replaced by the builder
`gauge(ratio).width(w).color(c)`. See [F1](#f1--same-release-deprecation-p1--meso).

### P2 — Layer Discipline

**Rule**: every method belongs to exactly one of the 5 layers
(Context / ContainerBuilder / Widget / State / Response). When the layer
is ambiguous, document why and resolve in the next minor version.

**Why**: when the same call exists on multiple layers, callers stop
predicting. Two-path APIs are the single biggest source of
"AI guesses wrong" failures.

**Apply when**: a method shows up in `Context::method` and
`ContainerBuilder::method`. Resolve to one.

**Approved ergonomic exception**: `ui.bordered(B)` (shortcut) and
`ui.container().border(B)` (explicit) both work. The strict V1 API audit rejects
unapproved overlaps; reviewed fluent modifiers and finalizers live in its explicit
allowlist. See [Architecture guide](architecture.md).

### P3 — Naming as Contract

**Rule**: a method's name encodes its category — verbs for actions, nouns
for getters, adjectives for builder modifiers. No abbreviations except the
universal ones listed in [Abbreviation Policy](#abbreviation-policy).

**Why**: naming is a memory-cheap contract. If `register_focusable_named`
is a long verb but `bordered` is a short adjective, the user has to
memorize each one. If both follow the same shape, one call teaches the next.

**Apply when**: introducing a new method or renaming an existing one.
See [Naming conventions](#naming-conventions) for category-by-category specifics.

**Anti-pattern**: mixing verbs and noun-as-verbs in the same family —
`ui.styled(text, style)` (verb) sits next to `ui.text(s)`
(noun-as-verb). Both are valid; not predictable from each other.

### P4 — Immediate-Mode Honesty

**Rule**: state lives in the caller. Widgets that need persistence take
`&mut <Widget>State`. No hidden global state, no implicit
`Rc<RefCell<...>>` inside the library.

**Why**: SLT advertises immediate-mode (see [R5 — State Ownership](#r5--state-ownership)).
Hidden state breaks the mental model and makes testing harder.

**Apply when**: implementing a stateful widget. The state struct must be
public, `Default`-able where possible, and named `<Widget>State`.

**Anti-pattern**: a widget that lazily allocates an internal `static` or
`thread_local` cache. The cache should be either a `&mut` parameter or a
field on `Context::frame_state` with explicit lifecycle.

### P5 — Composability First

**Rule**: containers compose. Every widget that holds children does so
via the standard builder mechanism (`ContainerBuilder::col`, `::row`,
`::line`), not bespoke "parent" parameters.

**Why**: composition is what lets users build their own widgets. Bespoke
parent params force users to learn a per-widget customization model.

**Apply when**: building a new container-like widget.

**Anti-pattern**: a `panel(title, body, footer)` function with three
closures. Replace with `panel(title)` + chained `.body(...)` /
`.footer(...)`, or compose standard containers.

### P6 — Visible Failure Modes

**Rule**: failure must be loud. Wrong types → compile error. Wrong values
→ panic in debug, warn in release. Silent fallbacks are themselves the bug.

**Why**: silent fallbacks are how the v0.20 spacing_scale demo shipped
with a broken syntax-highlight render path: the `code_block_lang(code, "")`
fallback omitted `ui.line(...)` wrapping and stacked tokens vertically.
A panic, warn, or structural lint would have caught this.

**Apply when**: writing a fallback path. Decide explicitly: panic
(debug), `slt_warn!(...)` (release), structural-equivalence test, or
compile error.

**Anti-pattern (caught in v0.20)**: see [F2](#f2--silent-fallback-divergence-p6--macro).

### P7 — AI-Readable Documentation

**Rule**: every public method has a one-paragraph doc + at least one
runnable example. Examples must compile (`cargo doc --no-deps`).

**Why**: SLT users include AI assistants. Method-level rustdoc is what
they read. If `register_focusable_named`'s docstring doesn't show the
exact pattern (register first, render widget after), the AI won't infer it.

**Apply when**: any new `pub fn`, `pub struct`, `pub enum`. Missing
rustdoc is a clippy warning (`-W missing_docs`).

**Anti-pattern**: `#[allow(missing_docs)]` on a public type. Either it's
internal (make it `pub(crate)`) or it deserves a docstring.

---

## 3. Concrete Rules (R1–R10)

These rules are the project-specific outcomes of applying the
meta-principles. They are non-negotiable in current-version SLT — a PR
that violates any of these without explicit waiver is rejected.

### R1 — Ease of Use Above All

SLT exists so that building a TUI is as easy as building a web page.
Every API decision is judged by: **"Can a developer use this correctly
on the first try, without reading the docs?"**

```rust
// 5 lines. No App struct. No Model/Update/View. No event loop.
fn main() -> std::io::Result<()> {
    slt::run(|ui| {
        ui.text("hello, world");
    })
}
```

If an API requires explanation, the API is wrong — not the developer.

(Operationalizes [P1 — Predictability over Cleverness](#p1--predictability-over-cleverness).)

### R2 — Your Closure IS the App

SLT is an **immediate-mode** UI library. There is no framework state to
manage.

- You write a closure. SLT calls it every frame.
- State lives in YOUR code — variables, structs, whatever you want.
- Layout is described every frame, not stored in a retained tree.
- No message passing, no trait implementations, no lifecycle hooks.

```rust
let mut count = 0;
slt::run(|ui| {
    if ui.button("+1").clicked { count += 1; }
    ui.text(format!("Count: {count}"));
});
```

This is the foundational decision. Every other rule flows from it.

(Operationalizes [P4 — Immediate-Mode Honesty](#p4--immediate-mode-honesty).)

### R3 — Widget Contract

Every widget should fit one of a very small number of patterns. Prefer
consistency over cleverness.

#### Interactive widgets

```rust
pub fn widget_name(&mut self, state: &mut WidgetState) -> Response
```

- Return `Response` — contains `clicked`, `hovered`, `changed`,
  `focused`, `rect`.
- Call `register_focusable()` for keyboard navigation.
- Consume handled key events (don't let them bubble).
- Use `self.theme.*` for default colors — never hardcode.

#### Display widgets

```rust
pub fn text(&mut self, s: impl Into<String>) -> Text<'_>    // text builder
pub fn markdown(&mut self, text: &str) -> Response          // plain display widgets
pub fn badge(&mut self, label: &str) -> Badge<'_>           // option-builder widgets
```

- Text elements (`text`, `link`, `styled`, `timer_display`) return the
  scoped `Text<'_>` builder (v0.26): its style, size and margin options
  (`.bold().fg(Color::Cyan).w(10)`) edit that one element, and nothing but
  text options can follow. `.show()` returns its `Response`. `line` and
  `line_wrap` still return `&mut Self`.
- Plain display widgets (`alert`, `markdown`, `big_text`, `empty_state`, …)
  return a `Response` (not `#[must_use]`, so no `let _ =` is needed) so hover,
  right-click, and `on_hover` tooltips work. Display widgets with options
  (`badge`, `stat`, `separator`, `gauge`, `breadcrumb`, `code_block`) return a
  consuming builder; `.show()` returns its response. The two builder recording
  modes are defined in
  [Rule 1](#rule-1-builder-pattern-for-widgets-with-optional-fields).
- No focusable registration.
- No event consumption.

#### Containers

```rust
pub fn col(self, f: impl FnOnce(&mut Context)) -> Response
```

- `ContainerBuilder` uses consuming `self` pattern (builder is done after
  `.col()` / `.row()`).
- Return `Response` for interaction detection.

#### State structs

- Live in `widgets.rs` — e.g. `TextInputState`, `TableState`.
- Named `{Widget}State`, always `pub struct`.
- Implement `Default` when sensible.
- Re-exported from `lib.rs`.
- When a widget needs a state struct at all is
  [Rule 4](#rule-4-stateful-widgets-take-mut-statetype-not-mut-string--mut-bool).

(Operationalizes [P3 — Naming as Contract](#p3--naming-as-contract) and
[P2 — Layer Discipline](#p2--layer-discipline). Detailed signature rules
in [API review rules](#api-review-rules).)

### R4 — Layout = CSS Flexbox, Syntax = Tailwind

Layout uses flexbox semantics: `row()`, `col()`, `gap()`, `grow()`,
`spacer()`. Styling uses Tailwind-inspired shorthand:

| Shorthand | Meaning | Explicit form |
|-----------|---------|---------------|
| `.p(2)` / `.px(2)` / `.py(1)` / `.pt(1)` | padding (all / x / y / top) | `.padding(Padding::all(2))` |
| `.m(1)` / `.mx(1)` / `.my(1)` | margin | `.margin(Margin::all(1))` |
| `.w(20)` / `.min_w(10)` / `.max_w(40)` | width constraints | `.w_pct(50)` |
| `.h(10)` / `.min_h(5)` / `.max_h(20)` | height constraints | `.h_pct(50)` |

The explicit `padding` / `margin` forms take `Padding` / `Margin` values (for
per-side control); there is no `.width()` / `.height()` on `ContainerBuilder`.
The v0.19 long names `pad`, `min_width`, `max_width`, `min_height`, and
`max_height` remain only as deprecated aliases. Shorthand is preferred in
examples.

#### Responsive breakpoints

Prefix with breakpoint: `.md_w(40)`, `.lg_p(2)`, `.xl_gap(3)`.
Breakpoints: Xs (<40), Sm (40–79), Md (80–119), Lg (120–159), Xl (≥160).

#### Builder patterns

| Builder | Pattern | Why |
|---------|---------|-----|
| `ContainerBuilder` | Consuming `self` | Forces explicit finalization (`.col()`, `.row()`, `.draw()`) |
| `Style` | Consuming `mut self` | Chainable, zero-cost |
| `ChartBuilder` | Mutable `&mut self` | Historical — scheduled for unification in v1.0 |

### R5 — State Ownership

| State type | Owner | Example |
|------------|-------|---------|
| Application state | User's closure | `let mut count = 0;` |
| Component-local state | Hook system | `ui.use_state(|| 0)` |
| Widget state | User | `let mut input = TextInputState::new()` |

#### Hook rules (same as React)

- `use_state()` and `use_memo()` must be called in the **same order**
  every frame.
- Never call order-based hooks inside conditionals or loops.
- Hook type mismatches panic with a descriptive message — this is a
  programmer error.
- Id-keyed state, `use_state_keyed(impl Into<String>, init)` and
  `use_state_keyed_default::<T>(id)`, is explicitly safe inside conditional
  branches and loops. It accepts `&'static str` literals and runtime keys
  (e.g. `format!("counter-{i}")`) — use it when conditional placement or
  per-item state is genuinely required.
- History: v0.19.0 added `&'static str`-keyed `use_state_named`, v0.20.0 added
  `use_state_keyed`; v0.26 merged them into one slot map and deprecated
  `use_state_named` (see [Migration guide](../reference/migration.md#v025x--v0260)).

(Operationalizes [P4 — Immediate-Mode Honesty](#p4--immediate-mode-honesty).)

### R6 — Error Handling

SLT uses `std::io::Result` for all fallible operations.
**We intentionally avoid custom error types.**

| Category | Mechanism | When |
|----------|-----------|------|
| Terminal I/O failure | `io::Result` | `run()`, `flush()`, event polling |
| Programmer error | `panic!()` with message | Hook type mismatch, invariant violation |
| Input validation | `Result<(), String>` | User-provided validator closures |

#### Rules

- **No `unwrap()` in Result-returning functions** — enforced by `clippy::unwrap_in_result`.
- **Panics are for programmer errors only** — never for user input or I/O.
- Panic messages must include context: index, expected type, actual value.
- Use `#[track_caller]` on public functions that may panic.

#### Why no custom error type?

SLT's only runtime error path is terminal I/O. Wrapping `io::Error` in
`SltError` would:
- Add API surface that becomes a semver commitment.
- Require `From` conversions with no added information.
- Complicate downstream `?` chains.

When distinct error categories emerge (config parsing, resource loading,
backend initialization), we will introduce a structured error type. Not
before.

(Operationalizes [P6 — Visible Failure Modes](#p6--visible-failure-modes).)

### R7 — Performance Patterns

SLT renders at 60+ FPS on modest hardware. These patterns keep it fast:

| Technique | What it does |
|-----------|-------------|
| `collect_all()` | Single DFS pass replaces 7 separate tree traversals |
| `apply_style_delta()` | Only emits changed ANSI attributes per cell during flush |
| Keyframe pre-sort | Stops sorted at build time, not per-frame |
| Double-buffer diff | Only changed cells are written to the terminal |
| Viewport culling | Off-screen widgets are skipped entirely |

#### Rules

- Performance changes must not break correctness — run the full test suite.
- Measure before optimizing — use the `benchmarks` bench suite (`cargo bench`).
- Minimize per-frame allocations — prefer reuse over allocation.
- Profile before assuming — `cargo flamegraph` for hot path identification.

### R8 — API Stability

SLT follows [Semantic Versioning](https://semver.org/).

| Version range | Compatibility promise |
|---------------|----------------------|
| 0.x.y (patch) | Backward compatible — no breaking changes |
| 0.x → 0.y (minor) | May contain breaking changes (pre-1.0) |
| 1.x (post-1.0) | Strict semver — breaking changes only in major versions |

#### MSRV policy

- Minimum Supported Rust Version is declared in `Cargo.toml`
  (`rust-version`).
- MSRV bumps only happen in **minor** version releases.
- MSRV bumps are documented in CHANGELOG.md.

#### Deprecation

- Deprecate callable methods and named types before removing them: at least one minor version with `#[deprecated]`.
- Deprecated items include a migration path in the deprecation message.
- Removal happens in the next minor version at earliest.
- Before 1.0, invariant-sensitive public fields may be encapsulated in a minor
  release when replacement accessors and explicit migration notes ship together.
  Patch releases never contain breaking changes.
- **Same-release deprecation and removal of callable APIs is forbidden** — see
  [F1 in Failure Mode Catalog](#f1--same-release-deprecation-p1--meso).

### R9 — Dependencies

**Minimal by design.** Four direct required deps; 26 crates resolved with
default features, 10 with `default-features = false` (unique package/version
pairs from normal `cargo tree`, excluding the root package) —
versus 69 for a ratatui + crossterm app. "Light" here means the dependency
tree and public API surface, not stripped binary size or build speed.

| Dependency | Purpose | Required? |
|------------|---------|-----------|
| `crossterm` | Built-in terminal runtime and terminal helpers | Default feature |
| `unicode-width` | Character width measurement | Yes |
| `unicode-segmentation` | Grapheme-cluster segmentation | Yes |
| `smallvec` | Inline small-vector scratch buffers | Yes |
| `compact_str` | String optimization | Yes |
| `tokio` | Async runtime | Optional (`async` feature) |
| `serde` | Serialization | Optional (`serde` feature) |
| `image` | Image loading | Optional (`image` feature) |
| `qrcode` | QR code widget support | Optional (`qrcode` feature) |
| `syntax-*` | Per-language tree-sitter grammar (e.g. `syntax-rust`) | Optional |
| `syntax` | Convenience: enables all `syntax-*` bundles | Optional |
| `kitty-compress` | Compressed Kitty image uploads | Optional |

#### Rules

- Do not add new required dependencies without discussion.
- Optional dependencies go behind feature flags.
- Feature flags must be **additive** — enabling a feature must not remove
  types or change existing behavior.
- Prefer `dep:` syntax in `[features]` to avoid implicit feature names.
- Keep docs explicit about which APIs require `crossterm` and which work
  on the core backend path without it.

### R10 — Safety

- **Zero `unsafe` code** — enforced by `#![forbid(unsafe_code)]`.
- No `unwrap()` in library code where `Result` is returned — enforced by
  `clippy::unwrap_in_result`.
- `dbg!()`, `println!()`, `eprintln!()` are forbidden in library code —
  enforced by clippy lints.
- `missing_docs` tracked via CI (non-blocking) — all new public API
  items should have doc comments.

(Operationalizes [P6 — Visible Failure Modes](#p6--visible-failure-modes)
and [P7 — AI-Readable Documentation](#p7--ai-readable-documentation).)

---

## 4. The 4-Tier Convention Stack

Conventions live at four levels of granularity. The Macro and Detail tiers
have dedicated companion docs; the Meso and Micro tiers live in the next two
sections of this page. Each tier maps to a different kind of lint signal.

| Tier | Owner doc | Sample question | Lint signal |
|------|-----------|-----------------|-------------|
| **Macro** | [Architecture guide](architecture.md) | "Where does mouse handling live?" | module imports cross-layer |
| **Meso** | [API review rules](#api-review-rules) | "Builder or immediate?" | method signature regex |
| **Micro** | [Naming conventions](#naming-conventions) | "Verb or noun?" | identifier-shape lint |
| **Detail** | [Rustdoc guide](rustdoc-guide.md) | "How long is the docstring?" | rustdoc-presence lint |

When a PR violates a rule, the reviewer should be able to point at *which
tier* it broke. If they can't, the rule isn't tier-localized — that's a
problem with the rule, not the PR.

The 4 tiers × 7 meta-principles = 28 audit cells. The
[Audit Matrix](#5-audit-matrix-v020) makes that grid explicit.

---

## API review rules

These are the consistency rules for adding or changing public API in SLT
(the **Meso tier**: signatures). Read this before opening any PR that touches
`Context::*`, widget signatures, or `*Response` types.

### Why these rules exist

In v0.20 we added `gauge`, `line_gauge`, `breadcrumb`, and `scrollable_with_gutter` in roughly the same week,
each through a different agent. The shapes drifted apart, even though the widgets are conceptually the same kind of thing:

```rust
// v0.20 reality — four widgets, four signature shapes
ui.gauge(0.6, "60%");                                    // f32, two positionals
ui.gauge_w(0.6, "60%", 48);                              // f32, three positionals + suffix variant
ui.gauge_colored(0.6, "60%", 48, Color::Green);          // f32, four positionals + another variant
ui.line_gauge(0.6, LineGaugeOpts::default().label("…")); // f32, opts struct (different from above)
ui.breadcrumb(&["a", "b", "c"]);                         // slice, returns BreadcrumbResponse
ui.scrollable_with_gutter(&mut state, total, vp, gf, f); // five positionals, no opts struct
```

When the v0.20 showcase example was first generated by an AI coder, it guessed wrong on every single one
of these widgets. The mistakes were not about logic — they were about which of four plausible shapes
the API actually used. We then audited internal mistakes and external bug reports across v0.18–v0.20:
**roughly 70% of "API guessed wrong" errors traced to inconsistent design between sibling widgets**, not to
actual API complexity.

The lesson: **consistent APIs reduce both human and AI cognitive load.** A consistent surface lets readers
guess correctly on first read; an inconsistent one forces a doc lookup at every call site. The five rules
below codify the consistency we want going forward.

### The Five Rules

#### Rule 1: Builder pattern for widgets with optional fields

SLT builders use the **consuming-builder** shape (`mut self -> Self`), matching
egui's `ui.add(...)` drop-render idiom. The builder borrows `&mut Context`,
chaining methods take `mut self` and return `Self`, and the widget renders when
the builder is **dropped**. Call `.show()` to render eagerly and capture the
response. This is the shape shipped in `gauge.rs` / `status.rs` for `Gauge`,
`LineGauge`, `Breadcrumb`, and `CodeBlock` — new builders must match it.

```rust
// Method on Context returns a builder — required args only
impl Context {
    pub fn gauge(&mut self, ratio: f64) -> Gauge<'_> { ... }
}

// Builder methods are chainable, take `mut self` and return `Self`.
impl<'a> Gauge<'a> {
    pub fn label(mut self, s: impl Into<String>) -> Self { ...; self }
    pub fn width(mut self, w: u32) -> Self { ...; self }
    pub fn color(mut self, c: Color) -> Self { ...; self }
}

// Renders on Drop (so a bare `ui.gauge(0.5).label("CPU");` works).
// Use .show() — which consumes self — when you need the Response back.
impl Drop for Gauge<'_> { ... }
impl Gauge<'_> { pub fn show(self) -> GaugeResponse { ... } }
```

**Why**: matches egui / RataTUI / dioxus mental model. One entry point per widget concept,
optional configuration via chaining. Avoids `gauge_w` / `gauge_with_label` proliferation
where every new optional dimension grows the public surface combinatorially.

**One mechanism per widget (v0.26, #460)**: a widget's options are builder methods on the
value its entry point returns, so knowledge from one widget transfers to the next and options
compose (`ui.button("Delete").variant(ButtonVariant::Danger).colors(&c)`). Builders come in
two recording modes with the same contract — the widget renders exactly where the call is
written, and the builder holds the exclusive `&mut Context` borrow until it is recorded, so
nothing can be recorded in between:

- **Record at creation, restyle in place** (`Text`, `Button`, `Badge`, `Stat`, `Help`,
  `Separator`, `ProgressBar`): used when options only change the look. The `Response` is final
  at creation, so the builder implements `Deref<Target = Response>` (`ui.button("x").clicked`,
  `ui.badge("x").on_hover(ui, ..)`), needs no `Drop`, and `.show()` returns the `Response`.
- **Record on first read** (`Slider`, `LineChart`): used when an option changes input handling
  or rasterization. The widget is recorded when its response is first read (`Deref` or
  `.show()`), or on `Drop` at the end of the statement. The older `Gauge`, `LineGauge`,
  `Breadcrumb` and `CodeBlock` builders render on `Drop` and return their response only from
  `.show()`.

A builder must not allocate beyond what the plain call allocates.

**Known exceptions (tracked for cleanup)**: the stateful widgets that still take colors
through a `<widget>_colored(.., &WidgetColors)` sibling and return their `Response` directly
are listed under [Suffix Patterns](#suffix-patterns); no new `_colored` names may be added
(`scripts/api_audit.sh` V9). `bar_chart_with` / `histogram_with` configure through a closure
(`_with` = extra closure parameter), and `modal_with` pairs `ModalOpts` with the body closure.

**Example** (good):

```rust
ui.gauge(0.5).label("CPU").width(48).color(Color::Green);
let r = ui.gauge(0.7).label("Mem").show(); // explicit Response
```

**Counter-example** (do NOT do this):

```rust
ui.gauge_w(0.5, "CPU", 48);                        // suffix-encoded variant
ui.gauge_colored(0.5, "CPU", Color::Green, 48);    // diverging arg order
ui.gauge_label_color(0.5, "CPU", Color::Green);    // n^2 explosion as opts grow
```

#### Rule 2: Floats are `f64` everywhere

```rust
pub fn gauge(&mut self, ratio: f64) -> Gauge<'_>;
pub fn slider(&mut self, label: &str, value: &mut f64, range: RangeInclusive<f64>) -> Response;
pub fn progress(&mut self, ratio: f64) -> Response; // immediate; `gauge` is the builder form
pub fn chart(&mut self, configure: impl FnOnce(&mut ChartBuilder), width: u32, height: u32) -> Response; // datasets are [(f64, f64)]
```

**Why**:
- Avoids precision-loss cliffs at API boundaries (caller has `f64`, API takes `f32`, silent narrowing).
- Matches Rust's default literal type — `0.5` is `f64`, so `ui.gauge(0.5)` just works without `0.5_f32`.
- Consistent with `Duration::as_secs_f64`, `Instant::elapsed`, and the rest of `std`.

**Example** (good):

```rust
let cpu: f64 = sys.cpu_usage();
ui.gauge(cpu / 100.0).label(format!("{cpu:.1}%"));
```

**Counter-example** (do NOT do this):

```rust
pub fn gauge(&mut self, ratio: f32, label: &str) -> GaugeResponse;
//                            ^^^ caller usually has f64, gets a silent `as f32` cast
ui.gauge(cpu as f32 / 100.0, &format!("{cpu:.1}%"));
```

**Exception**: explicit graphics math where `f32` dominates (color blending in `style/color.rs`,
GPU-style normalized math) may use `f32` internally. Public API still takes/returns `f64` and converts at
the boundary. The last `f32` exceptions (`gradient_stops`, `bg_gradient_stops`,
`ScrollState::progress`, the `f32` `Color` / `Theme` helpers and `AppState::fps`) were removed in
v0.26 (#463).

#### Rule 3: Options struct when public function takes 4+ args

```rust
// 1 arg — positional
pub fn gauge(&mut self, ratio: f64) -> Gauge<'_>;

// 2 args — positional
pub fn alert(&mut self, message: &str, level: AlertLevel) -> Response;

// 3 args — positional, OK
pub fn slider(&mut self, label: &str, value: &mut f64, range: RangeInclusive<f64>) -> Response;

// 4+ args — opts struct (or builder)
pub fn scrollable_with_gutter<G, F>(
    &mut self,
    state: &mut ScrollState,
    opts: GutterOpts<G>,
    body: F,
) -> GutterResponse;
```

**Why**: positional 5-tuples are unmemorable. Every reader (human or AI) has to look up which `u32`
is width vs height vs row count. Named fields on an opts struct are self-documenting and survive future
additions without breaking the signature.

**Counter-example** (the v0.20 mistake we are fixing):

```rust
// 5 positional args — caller has to remember argument order
pub fn scrollable_with_gutter<G, F>(
    &mut self,
    state: &mut ScrollState,
    total: usize,        // is this lines or pixels?
    viewport: u32,       // height? width?
    gutter: G,           // before or after body?
    body: F,
) -> GutterResponse;
```

**Fix**:

```rust
pub struct GutterOpts<G> {
    pub total_lines: usize,
    pub viewport_height: u32,
    pub gutter_fn: G,
}

pub fn scrollable_with_gutter<G, F>(
    &mut self,
    state: &mut ScrollState,
    opts: GutterOpts<G>,
    body: F,
) -> GutterResponse;
```

The arity boundary is intentionally low: 3 positional args is the comfort threshold for most readers.
If you hit 4, add an opts struct or convert the leaf args to a builder. Do not "just live with" 5 positionals.
How the struct is named is under [Options structs](#options-structs).

**Known exceptions (tracked for cleanup)** — existing public methods that predate or slipped past
this rule. Do not use them as precedent for new APIs:

| Method | Positional args (excluding `self`) |
|---|---|
| `kitty_image(rgba, pixel_width, pixel_height, cols, rows)` | 5 |
| `sixel_image(rgba, pixel_width, pixel_height, cols, rows)` | 5 |
| `heatmap(data, width, height, low_color, high_color)` | 5 |
| `heatmap_halfblock(data, width, height, low_color, high_color)` | 5 |
| `overlay_at_offset(anchor, dx, dy, f)` | 4 |
| `modal_at_offset(anchor, dx, dy, f)` (deprecated: `modal_with(ModalOpts)`) | 4 |
| `kitty_image_fit(rgba, src_width, src_height, cols)` | 4 |
| `histogram_with(data, configure, width, height)` | 4 |

#### Rule 4: Stateful widgets take `&mut StateType`, not `&mut String` / `&mut bool`

```rust
ui.text_input(&mut TextInputState);         // not &mut String
ui.scrollable(&mut ScrollState).col(|ui| …); // not &mut usize; returns a ContainerBuilder
ui.tabs(&mut TabsState);                    // not &mut usize; labels live in the state
ui.list(&mut ListState);                    // items live in the state
ui.tree(&mut TreeState);
ui.calendar(&mut CalendarState);
```

**Why**: widgets need to remember more than the visible value. Cursor position, selection range,
scroll offset, drag origin, last-clicked index, in-progress IME composition — none of that is the caller's
business. A newtype wrapper hides it. If we take `&mut String`, we either lose the internal state
between frames or hide it in a side table keyed by interaction id (worse: spooky persistence).

**Acceptable for trivial state**: `slider(value: &mut f64)` is OK because the only thing to remember
is the value itself. The same applies to `checkbox(value: &mut bool)` and `toggle(value: &mut bool)`.
The rule is: **if the widget needs anything beyond the user-visible value, it needs a State newtype.**

**Counter-example**:

```rust
// Caller has to manage cursor + selection manually = leaks internal state
pub fn text_input(&mut self, value: &mut String, cursor: &mut usize) -> Response;
```

**Fix**:

```rust
pub struct TextInputState { value: String, cursor: usize, selection: Option<Range<usize>>, ... }
pub fn text_input(&mut self, state: &mut TextInputState) -> Response;
```

#### Rule 5: Return types — `Response` for one-rect widgets, `XxxResponse` (Deref<Response>) for compound

```rust
// One rect, one set of interactions
pub fn button(&mut self, label: impl Into<String>) -> Response;
pub fn checkbox(&mut self, label: impl Into<String>, checked: &mut bool) -> Response;

// Compound: extra data alongside the standard interaction set
#[must_use = "BreadcrumbResponse contains interaction state — check .clicked_segment, .hovered, or .rect"]
pub struct BreadcrumbResponse {
    pub response: Response,
    pub clicked_segment: Option<usize>,
}

impl Deref for BreadcrumbResponse {
    type Target = Response;
    fn deref(&self) -> &Response { &self.response }
}
```

**Why**: `Response` already covers `clicked / hovered / changed / focused / rect`. A compound widget
adds *more* (which segment clicked, which cell of a grid, current scroll highlight index). Returning a
struct that `Deref`s to `Response` lets callers write `.hovered` and `.rect` exactly like a simple widget,
while still exposing the extra fields. This is the same pattern React uses for components that return
both DOM ref and instance handle.

**Example** (good):

```rust
let r = ui.breadcrumb(&["Home", "Settings", "Profile"]).show(); // builder → BreadcrumbResponse
if r.hovered { /* via Deref */ }
if let Some(idx) = r.clicked_segment { /* compound-specific */ }
```

**Counter-example** (do NOT do this):

```rust
// Returning raw Response loses the extra signal
pub fn breadcrumb(&mut self, segments: &[&str]) -> Response;
//   caller now has to track which segment was clicked via mouse coordinates manually

// Returning a tuple loses naming and Deref ergonomics
pub fn breadcrumb(&mut self, segments: &[&str]) -> (Response, Option<usize>);
//   .0 / .1 access, no `.hovered` shortcut, no `#[must_use]` enforcement
```

The `#[must_use]` attribute on every compound `*Response` is mandatory — it catches the common bug of dropping
a meaningful response without checking it. The plain `Response` is the exception (since v0.26): it is
returned by nearly every layout and display call, so the attribute sits on the individual methods whose
result is their only output instead of on the type: the `Button` builder type (and the deprecated
`button_colored` / `button_with`) is `#[must_use]`.

### v0.20 Retrospective

The rules above are not theoretical. They are direct corrections to mistakes shipped in v0.20.

| Mistake | Symptom | Rule | Fix |
|---|---|---|---|
| `gauge` / `gauge_w` / `gauge_colored` family | Three positional variants for the same widget; AI guessed `gauge_with_label` instead | 1 | Single `gauge(ratio).label().width().color()` builder |
| `f32` for ratios in `gauge`, `progress`, `line_gauge` | Caller had `f64` from `cpu_usage()`, inserted `as f32` casts at every call site | 2 | All public floats are `f64` |
| `scrollable_with_gutter` 5-positional signature | Showcase example used wrong argument order on first attempt | 3 | `GutterOpts` struct |
| `HighlightRange::line` vs `::span` (originally `::single`) | Two constructors with no obvious naming relationship | 1 + naming | Keep `::line` (1-line) and `::span` (n-line); document together |
| Mixed `breadcrumb` / `gauge` return shapes | Some returned `Response`, some returned `(Response, T)`, some returned new types | 5 | All compound widgets return `XxxResponse: Deref<Response>` |

These corrections land in the v0.20 API consistency commit (see `git log --grep="api consistency"` on
`release/v0.20.0`). v0.20 is the last release where the inconsistencies above will appear in the
public surface; v0.21+ enforces all five rules through the PR checklist below. The individual
renames are listed in [Rename History](#rename-history).

### PR Reviewer Checklist

Copy-paste this checklist into any PR that adds or changes a public widget:

```markdown
## API Design Checklist (docs/internals/design-principles.md, API review rules)

- [ ] Floats are `f64` (no `f32` in public signature)
- [ ] Public function takes ≤ 3 positional args (otherwise opts struct or builder)
- [ ] Optional configuration uses the consuming-builder pattern (chainable `mut self -> Self`, renders where written, `.show()` for the response; see Rule 1 for the two recording modes)
- [ ] Stateful widget takes `&mut XxxState` newtype (not `&mut String` / `&mut Vec<…>`)
- [ ] Returns `Response` (or `XxxResponse: Deref<Target = Response>` for compound widgets)
- [ ] `*Response` struct has `#[must_use = "..."]` attribute
- [ ] Doctest shows idiomatic one-line happy path
- [ ] Naming matches existing widgets (`fn gauge` returns `Gauge<'_>`, not `GaugeBuilder`; opts struct is
      `GaugeOpts`, not `GaugeConfig` or `GaugeParams`; tracked exceptions are listed under "Options structs")
- [ ] No `_w` / `_with_label` / `_numbered` combinatoric suffix variants — fold into builder methods instead
- [ ] Color customization is a builder method (`.color(c)`, `.colors(&c)`); no new `_colored` siblings (V9) and no `_with` that takes a value instead of a closure (V8) — see "Suffix Patterns"
```

If a check fails, fix the API before merging. The cost of an inconsistent shape compounds across every
future caller, every future doc reader, and every future AI coder generating SLT code from examples.

### References

- [egui `Widget` pattern](https://docs.rs/egui) — `add(impl Widget)` returns `Response`, builders
  configure widgets before `add`. Direct ancestor of SLT's builder shape.
- [ratatui `Widget` / `StatefulWidget`](https://docs.rs/ratatui) — splits stateless from stateful
  widgets via two traits; SLT collapses both into `Context::*` methods but inherits the
  state-newtype convention.
- React component composition — compound components return both a primary handle and additional refs;
  SLT's `XxxResponse: Deref<Response>` is the closest type-level analog.
- Anthropic API legibility guidance for AI coders — consistent surfaces are predicted correctly on
  first attempt; inconsistent surfaces require disambiguation passes against documentation. See the
  [v0.20 retrospective](#v020-retrospective) above for concrete numbers.
- OpenAI / Cursor coding-agent benchmarks confirm the same trend: per-symbol guess accuracy is
  dominated by sibling-widget consistency, not by surface size.

### When These Rules Conflict

If two rules conflict in a specific case, follow this priority:

1. **Rule 5** (Response shape) — never break the calling convention readers already know
2. **Rule 4** (state newtype) — encapsulation beats convenience
3. **Rule 2** (`f64`) — precision-loss is silent and hard to debug
4. **Rule 3** (opts struct at 4+ args) — readability beats keystroke savings
5. **Rule 1** (builder over suffix variants) — last because the cost is taste, not correctness

If an existing widget violates a rule for legacy reasons, deprecate-and-replace; do not add a new
inconsistency to "match the existing one." The whole point of these rules is that consistency
is the load-bearing property — preserving a bad pattern multiplies its cost.

---

## Naming conventions

This section is the **Micro tier**: how methods, types, parameters, and
modules are named.

The principle is **shape-encoding**: a name's shape (verb / noun /
adjective, length, suffix, prefix) tells the reader its category. When a
new contributor or AI assistant sees a name they've never seen, they
should infer category and call shape from the name alone.

### Categories

Every public identifier falls into one category. The category drives the
shape.

#### 1. Verbs — Actions

Methods that perform a side effect or trigger a state transition.

**Shape**: `<verb>` or `<verb>_<object>`.

**Examples**:
```rust
ui.quit();
ui.notify("saved", ToastLevel::Info);
ui.focus_by_name("search");
ui.register_focusable();
ui.consume_key('q');
state.set_ratio(0.5);
```

**Anti-pattern**: noun-as-verb for actions.

```rust
ui.text("hi")    // grandfathered: immediate-mode tradition; "text" treated as a verb here
ui.button("ok")  // grandfathered for the same reason
```

New actions should use real verbs.

#### 2. Nouns — Getters

Methods that return a value without side effects.

**Shape**: `<noun>` or `<noun>_<modifier>`.

**Examples**:
```rust
ui.theme();
ui.spacing();
ui.width();
ui.focused_name();
ui.events();
state.cursor;
```

**Anti-pattern**: `get_X` prefix.

```rust
fn get_theme(&self) -> Theme { /* ... */ }    // bad — Rust convention drops `get_`
fn theme(&self) -> Theme { /* ... */ }         // good
```

#### 3. Adjectives — Builder modifiers

Methods on `ContainerBuilder` (Layer 2) that configure the container.

**Shape**: short adjective or terse phrase.

**Examples**:
```rust
ui.container()
  .border(Border::Single)
  .p(2)
  .gap(1)
  .grow(1)
  .fill()
  .bold()
  .dim()
  .italic()
  .bg(theme.surface)
  .fg(Color::Cyan)
  .col(|ui| { /* ... */ });
```

**Anti-pattern**: verb form for builder modifiers.

```rust
.apply_padding(2)    // bad — too long, verb-shape
.with_padding(2)     // hypothetical — not an SLT API; acceptable only if `with_` were the family convention
.p(2)                // good — terse, adjective-shape
.padding(Padding::all(2)) // also good — explicit full-value form
```

#### 4. Constructors

Functions that create instances.

**Shape**:
- `Type::default()` — when `Default` is implementable.
- `Type::new(args)` — minimal required config.
- `Type::with_X(arg)` — narrow construction variant.

**Examples**:
```rust
TextInputState::default();
TextInputState::with_placeholder("Search...");
SplitPaneState::new(0.5);
TabsState::new(vec!["Files", "Settings"]);
```

### Suffix Patterns

| Suffix | Meaning | Example |
|--------|---------|---------|
| `_named` | name-keyed variant | `register_focusable_named` |
| `_keyed` | runtime-key variant | `use_state_keyed` |
| `_colored` | the base call plus a trailing `&WidgetColors` (stateful widgets only; frozen set, no new names, see below) | `text_input_colored(&mut state, &colors)` |
| `_styled` | explicit-`Style`/`Color` override variant | `sparkline_styled` |
| `_hd` | high-density / braille-cell rendering variant | `candlestick_hd` |
| `_halfblock` | half-block (▀ / ▄) pixel-doubling render variant | `heatmap_halfblock` |
| `_pct` | percent variant | `w_pct(50)` |
| `_ratio` | ratio variant | `w_ratio(1, 3)` |
| `_minmax` | min/max bounds variant | `w_minmax(10, 30)` |
| `_gap` | gap value on a container builder | `ContainerBuilder::row_gap(g)` |
| `_with` | the base call plus an extra **closure** parameter | `error_boundary_with(f, fallback)`, `table_with(state, cell)`, `bar_chart_with(.., configure)` |
| `State` | Layer 4 state type | `TextInputState` |
| `Response` | Layer 5 response type | `BreadcrumbResponse` |
| `Opts` | options struct (replaces 4+ positional args; [Rule 3](#rule-3-options-struct-when-public-function-takes-4-args)) | `GutterOpts` |
| `Builder` | builder type (rare; usually anon `<Widget><Lifetime>`) | — |

**One meaning per suffix (v0.26, #460).** A value-shaped option (a variant, a
color, a step, an options struct) is a builder method on the widget, never a
suffix ([Rule 1](#rule-1-builder-pattern-for-widgets-with-optional-fields)):
`ui.button("Save").variant(ButtonVariant::Primary).colors(&c)`,
`ui.slider("Vol", &mut v, 0.0..=100.0).step(1.0)`, `ui.badge("new").color(c)`,
`ui.help(&keys).key_color(k).text_color(t)`. `scripts/api_audit.sh --strict`
enforces the two suffixes: V8 fails a non-deprecated `pub fn *_with` without a
`Fn` / `FnMut` / `FnOnce` parameter, V9 fails a `*_colored` that is new or does
not take `&WidgetColors`.

The 0.25 names that broke the rule are deprecated aliases for one release:
`button_with` / `button_colored` → `button(..).variant(..)` / `.colors(..)`,
`slider_with` → `slider(..).step(..)`, `grid_with` → `grid_columns`, and
`separator_colored` / `badge_colored` / `stat_colored` / `progress_bar_colored`
/ `line_chart_colored` / `area_chart_colored` / `help_colored` → `.color(..)` /
`.key_color(..)` / `.text_color(..)` on the builder.

**Known exceptions** (allowlisted in V8 / V9; not a pattern to copy):

- `run_with` / `run_async_with` / `run_inline_with` / `run_static_with` take a
  `RunConfig`, and `EventBuilder::key_with` / `click_with` take modifiers.
- `modal_with(ModalOpts, f)` pairs an options value with the body closure
  (naming tracked by the modal options issue).
- The stateful widgets `text_input`, `list`, `list_reorderable`, `table`,
  `tabs`, `paginator`, `checkbox`, `toggle`, `select`, `radio` and
  `color_picker` keep `<widget>_colored(.., &WidgetColors)` and return their
  `Response` directly until they get a builder; `_colored` means exactly that
  shape.
- `sparkline_styled` / `ContainerBuilder::title_styled` use `_styled` for an
  explicit `Style` / per-point color list.

### Prefix Patterns

| Prefix | Meaning | Example |
|--------|---------|---------|
| `with_` | builder-style override / conditional | `with_if`, `with`, `TextInputState::with_placeholder`, `SplitPaneState::with_min_ratio` |
| `use_` | hook (state-bound across frames) | `use_state`, `use_effect`, `use_state_keyed` |
| `register_` | frame-level registration | `register_focusable`, `register_focusable_named` |
| `consume_` | mark an event as handled | `consume_event` |
| `is_` | boolean getter | `is_dark_mode` |
| `has_` | possession boolean getter | `has_pending_input` (use sparingly — prefer a noun/adjective like `Response.focused`) |

### Length Conventions

Length should match category:

- **Adjectives** (Layer 2 modifiers): ≤ 2 syllables typical.
  - ✅ `bg`, `fg`, `p`, `w`, `h`, `gap`, `grow`, `fill`, `border`, `bold`, `dim`
  - ✅ `bordered`, `padding`, `italic`, `reversed`
  - ❌ `with_padding_all_sides` — too long
- **Nouns** (Layer 1/2 getters): full word.
  - ✅ `theme`, `spacing`, `width`, `events`
  - ❌ `tm`, `sp`, `ev`
- **Verbs** (Layer 1 actions): full word + object.
  - ✅ `register_focusable`, `focus_by_name`, `consume_event`
  - ❌ `reg_foc`, `foc_name`
- **Types**: full word.
  - ✅ `TextInputState`, `SplitPaneState`, `BreadcrumbResponse`
  - ❌ `TIState`, `SPState`, `BCResp`

**Cross-category mismatch is a yellow signal**: when one method in a
family is ≤ 6 chars and another is > 20, ask whether the categories are
correctly assigned.

### Abbreviation Policy

#### Allowed (universal)

These are recognizable across languages and don't need expansion:

```
bg fg id idx len min max pos pct
w h x y r g b a (RGBA)
```

#### Forbidden (domain-specific)

Even common-sounding domain abbreviations are forbidden because they
hide their family from new readers:

| Forbidden | Use instead |
|-----------|-------------|
| `ctx` | `context` (in private code only — public API uses `ui`; see the `Response::on_*` exception below) |
| `btn` | `button` |
| `lbl` | `label` |
| `dbg` | `debug` |
| `cfg` | `config` |
| `req` | `request` |
| `res` | `response` (or just don't shorten — `Response` is the type) |
| `srv` | `server` |
| `db` | `database` (universal-ish, but spell it out in public APIs) |

**Rationale**: AI assistants and new contributors can guess `bg` =
"background" because it's universal CSS/web vocabulary. They can't guess
`lbl` = "label" because it could be "label", "label-id", "labour", etc.
The cost of typing `label` over `lbl` is 2 characters; the cost of a
miscalled API is much higher.

### Parameter Naming

#### Closures

```rust
fn use_effect<F, D>(&mut self, f: F, deps: &D)              // f for the callback
fn with_if<F>(self, cond: bool, f: F) -> Self               // f for the callback
```

Use single-letter names only when the role is unambiguous from the
function context.

#### Indices

- `idx` (preferred) or `i` (in tight loops).
- Never `index_of_thing` unless disambiguation is needed.

#### Lengths and sizes

- `n` — count, generic
- `len` — length of a collection
- `w`, `h` — width, height in cells (SLT uses cells, not pixels)
- `cap` — capacity
- `cnt` — forbidden; use `count` or `n`

#### References to `Context` / `ui`

- **Always `ui`** in public closures (`|ui| { ui.text("hi") }`).
- **`ctx`** allowed only in private internals.
- **`self`** when defining a method on `Context`.
- **Known exception**: the `Response` chain helpers (`on_hover`, `on_hover_ui`,
  `on_click`, `on_changed`, `on_focus`, `on_submit`, `on_double_click` in
  `src/context/state.rs`) name their `&mut Context` parameter `ctx`. The
  rustdoc shows `ctx`; callers still pass `ui`. Do not copy this into new APIs.

```rust
// good — public closure
ui.col(|ui| {
    ui.text("hello");
});

// good — private impl
fn helper(ctx: &mut Context) { /* ... */ }
```

### Method Ordering on `impl` blocks

When defining `impl Type`, order methods to maximize rustdoc readability:

1. **Constructors** — `new`, `default` (via `impl Default`), `with_X`.
2. **Getters** — nouns, return values without side effects.
3. **Builder modifiers** — adjectives, return `Self` or `&mut Self`.
4. **Actions** — verbs, perform side effects.
5. **Trait impls** — separate `impl Trait for Type` blocks.

Generated rustdoc pages then read top-to-bottom: "how to construct, how
to read, how to configure, how to act."

### Type Naming

#### State (Layer 4)

`<Widget>State`, always `pub struct` (placement, `Default` and re-export
rules: [R3 — Widget Contract](#r3--widget-contract)).

"State" means widget state your app owns (`ListState`, `TextInputState`, ...).
The hook handle returned by `use_state` / `use_state_keyed` is a *handle*:
write it as `StateHandle<T>` (an alias of `State<T>`, v0.26) in new code.

```rust
pub struct TextInputState { /* ... */ }
pub struct SplitPaneState { /* ... */ }
pub struct TableState { /* ... */ }
```

#### Response (Layer 5)

`<Widget>Response`. Compound responses must `Deref<Target = Response>`; the
canonical shape and the `#[must_use]` requirement are in
[Rule 5](#rule-5-return-types--response-for-one-rect-widgets-xxxresponse-derefresponse-for-compound).

#### Options structs

`<Widget>Opts` (e.g. `GutterOpts`; when to introduce one is
[Rule 3](#rule-3-options-struct-when-public-function-takes-4-args)), not
`<Widget>Config` or `<Widget>Params`. Known naming exceptions that predate this
rule and are tracked for cleanup: `RunConfig`, `ChartConfig`, `BarChartConfig`
— do not copy them.

#### Enums

Full words for variants, no abbreviations:

```rust
pub enum BarDirection {
    Horizontal,                                 // good
    Vertical,
}

pub enum Border {
    Single, Double, Rounded, Thick, Dashed, DashedThick,   // good
}

// bad
pub enum BD { H, V }
```

#### Builders (anonymous)

Builder structs use `<Widget><Lifetime>` shape (`fn gauge` returns
`Gauge<'_>`, not `GaugeBuilder`):

```rust
pub struct Gauge<'a> { /* borrows &mut Context */ }
pub struct LineGauge<'a> { /* ... */ }
pub struct Breadcrumb<'a> { /* ... */ }
```

The lifetime parameter is required because builders borrow `&mut Context`.

### Module Naming

#### Files

`snake_case.rs`. Group by family, not by widget count.

| Module | Family |
|--------|--------|
| `context/widgets/display/text.rs` | text & inline styling |
| `context/widgets/display/status.rs` | badges, stats, breadcrumbs |
| `context/widgets/text_input.rs` | text input variants |
| `widgets/text_input.rs` | state types in the text-input family |

#### Modules vs files

Rust 2018 style: `filename.rs` + `filename/` directory. Avoid `mod.rs`.

```
src/widgets.rs           # facade re-exports
src/widgets/
    text_input.rs        # state types in text_input family
    selection.rs
    ...
```

### Rename History

For AI assistants: do **not** search for these old names. Rows without a
version were removed in v0.20; later rows name their release.

| Old | New | Reason |
|-----|-----|--------|
| `gauge_w(r, w)` | `gauge(r).width(w)` | Builder consolidation |
| `gauge_colored(r, c)` | `gauge(r).color(c)` | Builder consolidation |
| `line_gauge_with(r, opts)` | `line_gauge(r).<chain>` | Builder consolidation |
| `breadcrumb_sep(b, s)` | `breadcrumb(b).separator(s)` | Builder consolidation |
| `button_with(l, v)` / `button_colored(l, &c)` (v0.26, deprecated) | `button(l).variant(v).colors(&c)` | One mechanism per widget (#460) |
| `slider_with(&mut v, SliderOpts)` (v0.26, deprecated) | `slider(l, &mut v, r).step(s)` | `_with` = closure (#460) |
| `grid_with(cols, f)` (v0.26, deprecated) | `grid_columns(cols, f)` | `_with` = closure (#460) |
| `badge_colored` / `stat_colored` / `separator_colored` / `progress_bar_colored` / `line_chart_colored` / `area_chart_colored` / `help_colored` (v0.26, deprecated) | `.color(c)` / `.key_color(k).text_color(t)` on the builder | `_colored` = `&WidgetColors` (#460) |
| `use_state_named(id, init)` (v0.26, deprecated) | `use_state_keyed(id, init)` | One keyed map (#463) |
| `code_block_lang` / `code_block_numbered*`, `key_seq`, `use_memo_ref`, `col_gap` / `row_gap` on `Context`, `ContainerBuilder::pad` / `min_width` / ... (removed in v0.26) | see [Migration guide](../reference/migration.md#v025x--v0260) | Deprecated since 0.20-0.22 (#463) |
| `LineGaugeOpts` | `LineGauge<'_>` builder | Builder consolidation |
| `HighlightRange::single(...)` | `HighlightRange::line(...)` | Single-line was the only use |
| `label_owned(s)` | `label(s)` | `impl Into<String>` accepts both |

The unversioned removals are **breaking changes**, but they were introduced
*and* removed in the same release (v0.20), so the pre-release window was the
only place they ever existed.

#### Deprecations (v0.21.0, removed in v0.26)

Unlike the v0.20 renames above, these names shipped as
`#[deprecated(since = "0.21.0")]` forwarding aliases through v0.25 and were
removed in v0.26 (#463). Use the new form:

| Deprecated | New | Reason |
|------------|-----|--------|
| `code_block_lang(c, l)` | `code_block(c).lang(l)` | Builder consolidation — `_lang` × `_numbered` cross-product folded into one `CodeBlock` builder |
| `code_block_numbered(c)` | `code_block(c).numbered()` | Builder consolidation |
| `code_block_numbered_lang(c, l)` | `code_block(c).lang(l).numbered()` | Builder consolidation |

> Note: `code_block(code)` now returns a `CodeBlock<'_>` builder instead of
> `Response`. It still renders on `Drop`, so the common
> `ui.code_block(code);` / `let _ = ui.code_block(code);` forms are
> unaffected; callers that bound the old `Response` should add `.show()`.

### Anti-patterns Observed in Review

#### Mixed verbs in the same widget family

```rust
// historic — pre-v0.20 the gauge family had both
ui.gauge(0.6);                    // immediate
ui.gauge_w(0.6, 24);              // immediate-with-arg
LineGaugeOpts::new(...).render()  // builder

// resolved — v0.20 unified to:
ui.gauge(0.6).width(24).color(Color::Cyan);
ui.line_gauge(0.6).label("60%").width(24);
```

#### Long form when a short adjective exists

```rust
// rejected in review
ui.container().apply_border(Border::Single).p_padding_value(2)

// correct
ui.container().border(Border::Single).p(2)
```

#### Abbreviation creep

```rust
// rejected
fn dbg_print_state(...)

// correct
fn debug_print_state(...)
```

#### Type abbreviation

```rust
// rejected
pub struct TIState { /* TextInputState */ }

// correct
pub struct TextInputState { /* ... */ }
```

### When in doubt

1. **Find an existing analogous method/type** in the same family.
2. **Match its shape** — same prefix, same suffix, same length category.
3. **If no analog exists**, document why this is a new pattern in the
   PR description and update the [Audit Matrix](#5-audit-matrix-v020) if the
   new shape changes a cell.

---

## 5. Audit Matrix (v0.20)

This matrix is preserved as the v0.20 design baseline. Current enforcement and
exceptions live in `scripts/api_audit.sh --strict`; current delivery status lives
in the roadmap below. Historical action notes are not claims about current (v0.25) coverage.

Status legend: ✅ enforced & green, ⚠️ documented & ad-hoc, ❌ known gap.

```
                    Macro            Meso             Micro          Detail
                    ─────            ─────            ─────          ──────
P1 Predictability    ⚠️                ✅                ⚠️              ✅
P2 Layer disc.       ⚠️ two-paths     ⚠️                ✅              ✅
P3 Naming            ✅                ✅                ⚠️ mixed       ✅
P4 Immediate         ✅                ✅                ✅              ✅
P5 Composability     ✅                ✅                ✅              ⚠️
P6 Visible fail      ❌                ⚠️                ✅              ⚠️
P7 AI-readable       ⚠️                ✅                ✅              ✅
```

**Total cells: 28. Green: 16. Yellow: 9. Red: 3.**

### Cell-by-cell notes

**P1 × Macro (⚠️)**: 5 layers documented but layer-membership audit is
manual. *Action by v0.21*: every public type marked with which layer it
belongs to (rustdoc tag).

**P1 × Micro (⚠️)**: shortcuts coexist with longer canonical forms in
some families (e.g. `bordered` vs `container().border()`). Not yet a
sweep target.

**P2 × Macro (⚠️ two-paths)**: `ui.bordered` vs `ui.container().border()`,
`text` vs `styled` for plain text. Documented in
[Architecture guide](architecture.md). The v0.23 strict V1 audit now rejects
unapproved overlaps and records reviewed exceptions in an explicit allowlist.

**P2 × Meso (⚠️)**: shortcut and explicit forms coexist for the same
container shape — `ui.bordered(Border::Rounded)` vs
`ui.container().border(Border::Rounded)`, `ui.col(f)` vs
`ui.container().col(f)`. Layer rule (P2) doesn't formalize the
shortcut/explicit split.

**P3 × Micro (⚠️ mixed)**: `register_focusable_named` (long verb)
coexists with `bordered` (terse adjective). Both correct under their
categories, but a new reader doesn't predict the long form when they
only saw the short. *Action by v0.21*: the naming conventions add verb-length
conventions ([Length Conventions](#length-conventions)); lint flags
identifier-shape outliers.

**P5 × Detail (⚠️)**: composition examples in rustdoc are rare; most
show single-widget calls. *Action*: the [Rustdoc guide](rustdoc-guide.md) mandates one
composition example per builder type.

**P6 × Macro (❌)**: silent fallback bugs landed in v0.20
(`code_block_lang` empty-lang path; see
[F2](#f2--silent-fallback-divergence-p6--macro)). No structural lint
exists yet. *Action by v0.21*: clippy custom rule or audit-script flag
for "fallback diverges from primary path."

**P6 × Meso (⚠️)**: `slt_assert!` and `slt_warn!` exist but inconsistent
adoption. Many widgets silently clamp values rather than warning.

**P6 × Detail (⚠️)**: failure section missing from many docstrings.
The [Rustdoc guide](rustdoc-guide.md) now mandates it for non-trivial methods.

**P7 × Macro (⚠️)**: module-level docs (`//!` at file head) inconsistent.
v0.20 added `//!` to all 6 facade files (`lib.rs`, `context.rs`,
`widgets_display.rs`, `widgets_input.rs`, `widgets_interactive.rs`,
`widgets_viz.rs`); ~50 implementation files remain unchanged.
Public item docs are compiler-enforced; module-level implementation docs remain
a review concern rather than a claimed complete sweep.

---

## 6. Roadmap: v0.20 → v1.0

| Version | Phase | Deliverable | Status |
|---------|-------|-------------|--------|
| v0.20 | **Define** | Design, architecture, naming, and rustdoc guides | shipped |
| v0.21 | **Automate** | Compiler lints, test utilities, and API-shape conventions | shipped |
| v0.22 | **Refine** | Edition 2024, f64 surface, terminal/release hardening | shipped |
| v0.23 | **Correct** | Runtime, Unicode, state, docs, and response-contract audit | shipped |
| v0.24 | **Harden runtime** | Owned native input source, recoverable panics, canvas budgets, owned browser (WASM) runtime | shipped |
| v0.25 | **Harden focus & scroll** | Focus-following scroll containers, unique initial focus, modal focus stability, `form_field_response`, layout-rect queries | shipped (current) |
| v0.26 | **Consolidate** | Remove the 0.20–0.22 deprecated aliases (`pad`, `min_width`, `use_state_named_with`, `code_block_*`, f32 color methods); option builders (`Button`, `Badge`, `Stat`, `Help`, `Separator`, `ProgressBar`, `Slider`, `LineChart`); one keyed-state slot map; `slt::prelude` | in progress |
| pre-1.0 | **Stabilize** | Remove the 0.26 deprecations (`button_with`, `button_colored`, display `*_colored` variants, `slider_with`, `grid_with`, `use_state_named`) | not yet scheduled |
| v1.0 | **Freeze** | Public API semver-locked. Matrix all ✅. | not yet scheduled |

Remaining rows are direction, not dated commitments; the 0.26 deprecations
listed above still compile (with a warning) in v0.26. Each stabilization increment should have a

reviewable issue scope and explicit verification. Deprecated API removals follow the schedule in
[R8 — API Stability](#r8--api-stability); invariant-sensitive field changes
follow the pre-1.0 migration exception documented there.

---

## 7. Failure Mode Catalog

Real bugs, classified by which principle they violated. Used to refine
the matrix and as case studies for new contributors.

### F1 — Same-release deprecation (P1 × Meso)

**v0.20 #224, #226**: `gauge_w`, `gauge_colored`, `line_gauge_with`,
`breadcrumb_sep`, `LineGaugeOpts`, `HighlightRange::single`, `label_owned`
all introduced in v0.20 and immediately deprecated by the API consistency
pass. Net effect: zero deprecation tax — the methods existed for a few
unmerged hours.

**Lesson**: builder consolidation must precede shortcut introduction.
When two team members add overlapping APIs in parallel, the gateway
review must catch the overlap before either lands.

**Prevention**: [Rule 1](#rule-1-builder-pattern-for-widgets-with-optional-fields)
(builder for optional fields) and [Rule 3](#rule-3-options-struct-when-public-function-takes-4-args)
(no 4+ positional args) + audit script V2 check.

### F2 — Silent fallback divergence (P6 × Macro)

**v0.20 spacing_scale demo**: `code_block_lang(code, "")` falls back to
a loop calling `render_highlighted_line(ui, line)` directly. The
non-fallback path wraps each line in `ui.line(...)`, but the fallback
path didn't. Result: tokens rendered one-per-row vertically.

```rust
// before fix
} else {
    for line in code.lines() {
        render_highlighted_line(ui, line);  // ← no ui.line(...)
    }
}

// after fix
} else {
    for line in code.lines() {
        ui.line(|ui| render_highlighted_line(ui, line));
    }
}
```

**Lesson**: fallback path must mirror primary path's container nesting.

**Prevention**: structural-equivalence test or lint that checks
"every branch in this `match` / `if` produces the same container shape."

### F3 — Outer-container missing grow (P5 × Macro)

**v0.20 named_focus demo**: outer `bordered().col()` lacked `.grow(1)`,
so the box only filled one row's worth of vertical space. Inside, input
fields lacked grow on the input's row column, so they shrank to 1 cell.

**Lesson**: demo template must explicitly call out grow defaults;
flexbox inheritance is not intuitive even for experienced devs.

**Prevention**: demo lint or visual snapshot test.

### F4 — Em-dash wide-char drift (P6 × Detail)

**v0.20 demo polish**: titles like `"SLT v0.20 — Density presets"` used
U+2014 EM DASH. In some terminals, em-dash counts as 1 column under
`unicode-width` but 2 columns when rendered, causing border misalignment.

**Lesson**: titles restrict to BMP ASCII unless the demo explicitly
tests wide-char handling.

**Prevention**: demo lint that scans for non-ASCII in `.title(...)`
arguments.

### F5 — Race in parallel-agent commits (process, not API)

**v0.20 functional audit**: 4 of 5 audit agents committed directly to
`release/v0.20.0` instead of their assigned worktree branches, racing
each other. One agent's report explicitly noted "다른 agent가 내 unstaged
work를 reset함."

**Lesson**: not an API issue, but parallel-agent isolation is part of
the same predictability principle. Worktrees must be enforced for
multi-agent work.

**Prevention**: `scripts/spawn_agent.sh` that wraps `Task` calls and
forces `isolation: "worktree"` for parallelizable work.

---

## 8. How to Use This Document

### When you write a new public API

1. Read [API review rules](#api-review-rules) for the 5 signature rules and
   copy its [PR Reviewer Checklist](#pr-reviewer-checklist).
2. Read [Naming conventions](#naming-conventions) for the naming categories.
3. Read [Architecture guide](architecture.md) and place the method in
   exactly one layer.
4. Run `scripts/api_audit.sh`. Fix any V1 / V2 / V4 flags.
5. If the new method changes a matrix cell's status, update the matrix.

### When you review a PR

1. Identify which principles the change touches.
2. For each touched principle, check the corresponding tier doc or section.
3. If a violation exists but is documented (yellow cell), note it in the
   PR description and link the matrix row.
4. If a violation is undocumented (would change the matrix), block until
   the matrix is updated.

### When you propose a redesign

1. Quote the principle(s) being violated.
2. Propose the tier change (Macro / Meso / Micro / Detail).
3. Update the milestone roadmap if the change spans multiple versions.

---

## Out of Scope

This document does **not** define:

- **File-level coding conventions** (mod patterns, derive order, attribute
  placement) — those live in `AGENTS.md` because they're project-specific.
- **Release process** — that lives in `AGENTS.md` "Release Workflow Checklist."
- **Test coverage requirements** — that lives in CI config.
- **Performance budgets** — that lives in `tests/alloc_budget.rs` and
  benchmark suites.

Design principles are *what* the API should look like; the project's
release / test / perf docs are *how* we ship it.
