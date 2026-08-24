# SLT Design Principles

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
- [QUICK_START.md](QUICK_START.md)
- [WIDGETS.md](WIDGETS.md)
- [PATTERNS.md](PATTERNS.md)
- [ARCHITECTURE.md](ARCHITECTURE.md) — Macro tier (layers, modules)
- [API_DESIGN.md](API_DESIGN.md) — Meso tier (signatures)
- [NAMING.md](NAMING.md) — Micro tier (identifiers)
- [RUSTDOC_GUIDE.md](RUSTDOC_GUIDE.md) — Detail tier (rustdoc)

---

## Document Map

This document is layered. Read top-to-bottom on first pass; jump to
specific sections on later visits.

```
1. The North Star — Predictability         ← why every other rule exists
2. Meta-Principles (P1–P7)                 ← how to evaluate a design choice
3. Concrete Rules (R1–R10)                 ← what the choices have settled into
4. The 4-Tier Convention Stack             ← where rules live
5. Audit Matrix (v0.20)                    ← historical design baseline
6. Roadmap (v0.20 → v1.0)                  ← planned stabilization sequence
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

**Why**: every clever shorthand is a memory tax. SLT has 70+ public
methods; users remember patterns, not individual calls.

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
allowlist. See [`ARCHITECTURE.md`](./ARCHITECTURE.md).

### P3 — Naming as Contract

**Rule**: a method's name encodes its category — verbs for actions, nouns
for getters, adjectives for builder modifiers. No abbreviations except
universal ones (`bg`, `fg`, `id`, `idx`, `len`, `min`, `max`, `pos`,
`pct`, `w`, `h`, `x`, `y`).

**Why**: naming is a memory-cheap contract. If `register_focusable_named`
is a long verb but `bordered` is a short adjective, the user has to
memorize each one. If both follow the same shape, one call teaches the next.

**Apply when**: introducing a new method or renaming an existing one.
See [`NAMING.md`](./NAMING.md) for category-by-category specifics.

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
pub fn text(&mut self, content: impl Into<String>) -> &mut Self
```

- Return `&mut Self` for chaining (`.bold().fg(Color::Cyan)`).
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
- Named `{Widget}State`.
- Implement `Default` when sensible.
- Re-exported from `lib.rs`.

(Operationalizes [P3 — Naming as Contract](#p3--naming-as-contract) and
[P2 — Layer Discipline](#p2--layer-discipline). Detailed signature rules
in [API_DESIGN.md](./API_DESIGN.md).)

### R4 — Layout = CSS Flexbox, Syntax = Tailwind

Layout uses flexbox semantics: `row()`, `col()`, `gap()`, `grow()`,
`spacer()`. Styling uses Tailwind-inspired shorthand:

| Full name | Shorthand |
|-----------|-----------|
| `.padding(2)` | `.p(2)` |
| `.margin(1)` | `.m(1)` |
| `.width(20)` | `.w(20)` |
| `.height(10)` | `.h(10)` |

Both forms are always available. Shorthand is preferred in examples.

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
- v0.19.0 added id-keyed state, now exposed as `use_state_named(id, init)`
  and `use_state_named_default::<T>(id)`, keyed by `&'static str` and explicitly
  safe inside conditional branches — use them when conditional placement
  is genuinely required.
- v0.20.0 added `use_state_keyed(impl Into<String>, init)` that accepts
  runtime-computed keys (e.g. `format!("counter-{i}")`). Use it for
  list-item state where the key isn't `&'static str`.

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

Conventions live at four levels of granularity. Each tier has a dedicated
companion doc, and each tier maps to a different kind of lint signal.

| Tier | Owner doc | Sample question | Lint signal |
|------|-----------|-----------------|-------------|
| **Macro** | [`ARCHITECTURE.md`](./ARCHITECTURE.md) | "Where does mouse handling live?" | module imports cross-layer |
| **Meso** | [`API_DESIGN.md`](./API_DESIGN.md) | "Builder or immediate?" | method signature regex |
| **Micro** | [`NAMING.md`](./NAMING.md) | "Verb or noun?" | identifier-shape lint |
| **Detail** | [`RUSTDOC_GUIDE.md`](./RUSTDOC_GUIDE.md) | "How long is the docstring?" | rustdoc-presence lint |

When a PR violates a rule, the reviewer should be able to point at *which
tier* it broke. If they can't, the rule isn't tier-localized — that's a
problem with the rule, not the PR.

The 4 tiers × 7 meta-principles = 28 audit cells. The next section makes
that grid explicit.

---

## 5. Audit Matrix (v0.20)

This matrix is preserved as the v0.20 design baseline. Current enforcement and
exceptions live in `scripts/api_audit.sh --strict`; current delivery status lives
in the roadmap below. Historical action notes are not claims about v0.23 coverage.

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
[`ARCHITECTURE.md`](./ARCHITECTURE.md). The v0.23 strict V1 audit now rejects
unapproved overlaps and records reviewed exceptions in an explicit allowlist.

**P2 × Meso (⚠️)**: `Context::text` and `ContainerBuilder::text` both
exist. The latter is the inner-of-builder form, the former is the
unbordered shortcut. Layer rule (P2) doesn't formalize the
shortcut/explicit split.

**P3 × Micro (⚠️ mixed)**: `register_focusable_named` (long verb)
coexists with `bordered` (terse adjective). Both correct under their
categories, but a new reader doesn't predict the long form when they
only saw the short. *Action by v0.21*: NAMING.md adds verb-length
conventions; lint flags identifier-shape outliers.

**P5 × Detail (⚠️)**: composition examples in rustdoc are rare; most
show single-widget calls. *Action*: RUSTDOC_GUIDE.md mandates one
composition example per builder type.

**P6 × Macro (❌)**: silent fallback bugs landed in v0.20
(`code_block_lang` empty-lang path; see
[F2](#f2--silent-fallback-divergence-p6--macro)). No structural lint
exists yet. *Action by v0.21*: clippy custom rule or audit-script flag
for "fallback diverges from primary path."

**P6 × Meso (⚠️)**: `slt_assert!` and `slt_warn!` exist but inconsistent
adoption. Many widgets silently clamp values rather than warning.

**P6 × Detail (⚠️)**: failure section missing from many docstrings.
RUSTDOC_GUIDE.md now mandates it for non-trivial methods.

**P7 × Macro (⚠️)**: module-level docs (`//!` at file head) inconsistent.
v0.20 added `//!` to all 6 facade files (`lib.rs`, `context.rs`,
`widgets_display.rs`, `widgets_input.rs`, `widgets_interactive.rs`,
`widgets_viz.rs`); ~50 implementation files remain unchanged.
Public item docs are compiler-enforced; module-level implementation docs remain
a review concern rather than a claimed complete sweep.

---

## 6. Roadmap: v0.23 → v1.0

| Version | Phase | Deliverable | Status |
|---------|-------|-------------|--------|
| v0.20 | **Define** | Design, architecture, naming, and rustdoc guides | complete |
| v0.21 | **Automate** | Compiler lints, test utilities, and API-shape conventions | complete |
| v0.22 | **Refine** | Edition 2024, f64 surface, terminal/release hardening | complete |
| v0.23 | **Correct** | Runtime, Unicode, state, docs, and response-contract audit | complete |
| v0.24–0.30 | **Stabilize** | Deprecate-and-remove sweep per principle | planned |
| v1.0 | **Freeze** | Public API semver-locked. Matrix all ✅. | planned |

Each stabilization increment should have a reviewable issue scope and explicit
verification. Deprecated API removals follow the schedule in
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

**Prevention**: API_DESIGN.md rule 1 (builder when 4+ optional fields)
+ audit script V2 check.

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

1. Read [`API_DESIGN.md`](./API_DESIGN.md) for the 5 signature rules.
2. Read [`NAMING.md`](./NAMING.md) for the naming categories.
3. Read [`ARCHITECTURE.md`](./ARCHITECTURE.md) and place the method in
   exactly one layer.
4. Run `scripts/api_audit.sh`. Fix any V1 / V2 / V4 flags.
5. If the new method changes a matrix cell's status, update the matrix.

### When you review a PR

1. Identify which principles the change touches.
2. For each touched principle, check the corresponding tier doc.
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
- **Performance budgets** — that lives in `tests/v020_perf_alloc.rs` and
  benchmark suites.

Design principles are *what* the API should look like; the project's
release / test / perf docs are *how* we ship it.
