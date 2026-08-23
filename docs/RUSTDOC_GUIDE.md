# SLT Rustdoc Style Guide (Detail Tier)

> Companion to [`DESIGN_PRINCIPLES.md`](./DESIGN_PRINCIPLES.md). This doc
> is the **Detail tier**: how rustdoc is written so both humans and AI
> assistants can predict the next call from the docs alone.

The principle is **predictable docs**: a developer who reads the rustdoc
for one method should be able to anticipate the structure of every other
method's rustdoc. Inconsistent docs force readers to re-orient on every
page.

---

## The 4-part docstring

Every public method has these four sections, in order:

```rust
/// One-sentence purpose. Imperative voice — "Render a gauge" not
/// "This function renders a gauge."
///
/// One paragraph explaining how it fits into the widget family. Mention
/// the layer (Context, Builder, Widget, State, Response). Mention the
/// return shape. Mention any state interaction.
///
/// # Example
///
/// ```no_run
/// // Minimal correct usage. Compiles. Shows the canonical call shape.
/// ```
///
/// # Failure
///
/// What happens on bad input — panic, warn, or silent? If silent, why?
pub fn gauge(&mut self, ratio: f64) -> Gauge<'_> { /* ... */ }
```

### Required sections (every public item)

- **Purpose** — 1 sentence, imperative voice.
- **Family** — 1 paragraph: which layer, what family of widgets it joins.
- **Example** — 1 code block, minimal correct usage.

### Conditional sections

- **`# Failure`** — required when the method can panic or silently fall back.
- **`# State`** — required when the method reads or writes hook state.
- **`# Layout`** — required when the method affects parent or child layout.
- **`# Performance`** — required when the method has surprising cost
  (e.g. allocates per-call, runs O(n²)).

---

## Imperative Voice

✅ "Render a gauge with the given ratio."
❌ "This function renders a gauge with the given ratio."
❌ "Renders a gauge..." — third-person inflection conflicts with Rust
convention (Rust API guidelines use imperative).

The first sentence is the rustdoc summary, picked up by IDE tooltips and
docs.rs index. Make it stand alone.

---

## Code Examples Must Compile

Every `# Example` block must compile via `cargo doc --no-deps` /
`cargo test --doc`. Use:

- **`no_run`** — when the example needs a running terminal but should
  still type-check. **Preferred for SLT**, since most examples need
  `slt::run(...)`.
- **`ignore`** — only when the example must skip both compile and run
  (rare; needs justification).

```rust
/// # Example
///
/// ```no_run
/// # use slt::widgets::TextInputState;
/// # slt::run(|ui: &mut slt::Context| {
/// let mut state = TextInputState::default();
/// ui.text_input(&mut state);
/// # });
/// ```
```

`no_run` is preferred over `ignore` because the compiler still
type-checks — catches API drift in the docs.

---

## Hidden Setup Lines

Lines starting with `# ` are hidden from rendered docs but still
compile-checked. Use them for:

- `use` statements
- `slt::run(...)` wrappers
- helper variable initialization

```rust
/// ```no_run
/// # use slt::Color;
/// # slt::run(|ui: &mut slt::Context| {
/// ui.text("hi").fg(Color::Red);
/// # });
/// ```
```

The reader sees only the meaningful lines; the compiler sees and
type-checks everything.

---

## Cross-references

When mentioning another method or type, **always use intra-doc links**:

```rust
/// See [`Context::register_focusable`] for the unnamed variant.
/// The matching state type is [`TextInputState`].
/// Use [`Self::with_placeholder`] for the placeholder-only constructor.
```

Never write the path as plain text:

```rust
// bad
/// See Context::register_focusable for the unnamed variant.
```

Intra-doc links generate working hyperlinks on docs.rs. Plain text
breaks under refactor.

---

## Module-level Documentation

Every public module file starts with a `//!` doc comment:

```rust
//! Display widgets — text, alerts, badges, code blocks.
//!
//! This module hosts Layer 3 (Widget) primitives that produce visual
//! output without consuming events. For interactive widgets see
//! [`widgets_interactive`](super::widgets_interactive).
//!
//! # Family
//!
//! - [`text`](crate::Context::text) — basic text rendering
//! - [`alert`](crate::Context::alert) — info / warning / error banners
//! - [`badge`](crate::Context::badge) — colored chips
//! - [`code_block`](crate::Context::code_block) — syntax-highlighted code
```

The first line is the rustdoc summary picked up by the parent module's
index. The body explains:

1. What's **in** this module (the family).
2. What's **out** (link to sibling modules with adjacent families).
3. **Layer** the module belongs to.

---

## Length Conventions

| Item | Length |
|------|--------|
| **Trivial getters** (`theme()`, `width()`) | 1 sentence + example |
| **Common widgets** (`text`, `button`, `gauge`) | 1 paragraph + example + optional Layout/State |
| **Complex widgets** (`textarea`, `table`, `chart`) | up to 5 paragraphs, multiple subsections, ≥ 2 examples (minimal + composition) |
| **Hooks** (`use_state`, `use_effect`) | 5+ paragraphs covering rules, re-render semantics, ≥ 1 detailed example |

Hooks are the most surprising part of immediate-mode for newcomers; over-document them.

---

## State and Layout Sections

When a method reads/writes hook state, document **what it touches** and
**when it triggers re-render**:

```rust
/// Reset the focus to the first focusable in the next frame.
///
/// # State
///
/// Writes [`Context::focus_index`]. The reset takes effect on the next
/// frame; the current frame still reflects the previous focus.
///
/// # Example
///
/// ```no_run
/// # slt::run(|ui: &mut slt::Context| {
/// if ui.button("Reset").clicked {
///     ui.focus_first();
/// }
/// # });
/// ```
pub fn focus_first(&mut self) { /* ... */ }
```

When a method affects layout, document **what container shape it produces**:

```rust
/// Wrap children in a flex column.
///
/// # Layout
///
/// Produces a column container with `gap = 0`. For non-zero gap use
/// [`Self::col_gap`]. The column claims its parent's full cross-axis
/// (width) by default; pass `.grow(0)` to opt out.
pub fn col<F: FnOnce(&mut Context)>(&mut self, f: F) -> Response { /* ... */ }
```

---

## Failure Section

Required for any method that can panic, warn, or silently fall back. Use
this template:

```rust
/// # Failure
///
/// Panics in debug if `ratio < 0.0` or `ratio > 1.0`. In release, the
/// value is clamped silently. To detect bad input at the call site,
/// validate before calling.
```

For panics, include the **exact assertion text** if possible. AI
assistants searching for an error message benefit from finding it in the
docs.

---

## AI-Readable Formatting

Two patterns make AI consumption easier:

### 1. Each parameter named in prose

Don't rely on the signature alone:

```rust
/// Render a block-fill gauge.
///
/// `ratio` is clamped to `[0.0, 1.0]`. Values outside this range are
/// silently clamped — no warning, no panic. To detect bad input,
/// validate before calling.
///
/// `label`, set via the builder method, is rendered inside the bar
/// when present.
```

### 2. Each related method linked

If `gauge` is the entry point, link siblings:

```rust
/// Render a block-fill gauge.
///
/// For a single-line variant, see [`Context::line_gauge`].
/// For an animated indeterminate variant, see [`Context::progress`].
/// For the underlying state-free flush, see [`Context::progress_bar`].
```

The AI sees:
- the bounds (clamping)
- the visual model (block fill)
- the alternatives (line_gauge, progress, progress_bar)

That's enough to suggest the right call without checking three other
docs.

---

## Forbidden

- **Marketing language**: "blazing fast", "ergonomic", "modern", "powerful".
  The user is reading this to learn how to call the method, not be sold
  the library. Marketing belongs in `README.md`.
- **Internal jargon without expansion**: "the hot path", "the rollback",
  "the hit map" without context. First mention in any doc must clarify.
- **TODO/FIXME in public docs**: leave those as inline comments.
  Public docstrings represent the committed API.
- **Empty examples**: `# Example` with `// see source` is worse than no
  example. Either write a real example or omit the section.

---

## Real Example: a complete docstring

```rust
/// Render a block-fill gauge with optional label and color override.
///
/// The gauge is a Layer-3 widget. It fills a horizontal bar in 8 sub-cell
/// steps using Unicode block characters; the bar's width is set via the
/// builder's `.width(n)` method (default: parent's full width).
///
/// `ratio` is clamped to `[0.0, 1.0]`. Color tiers are automatic by
/// default — green below 50%, yellow 50–80%, red ≥ 80% — and can be
/// overridden via [`Gauge::color`].
///
/// # Example
///
/// ```no_run
/// # use slt::Color;
/// # slt::run(|ui: &mut slt::Context| {
/// ui.gauge(0.42).label("42%").width(24);
/// ui.gauge(0.95).color(Color::Red);
/// # });
/// ```
///
/// # Layout
///
/// The gauge claims one row. With no `.width(n)` call, it fills the
/// parent's cross-axis (typically the parent column's full width).
///
/// # Failure
///
/// `ratio` outside `[0.0, 1.0]` is silently clamped — no warning, no
/// panic. To detect bad input, validate before calling.
///
/// # See Also
///
/// - [`Context::line_gauge`] — single-line variant with custom characters
/// - [`Context::progress`] — animated indeterminate variant
pub fn gauge(&mut self, ratio: f64) -> Gauge<'_> { /* ... */ }
```

This docstring:
- ✅ Imperative voice ("Render a...")
- ✅ Layer noted (Layer-3)
- ✅ Example compiles (`no_run` + hidden setup)
- ✅ Cross-references via intra-doc links
- ✅ Failure section explicit (silent clamping)
- ✅ Layout section explicit (claims one row, fills cross-axis)
- ✅ See Also links siblings

---

## Lint Enforcement

In v0.23:
- `#![deny(missing_docs)]` enabled at the crate root for `pub` items.
- `cargo doc --no-deps` runs in CI; broken intra-doc links fail.
- `cargo test --doc` runs; broken examples fail.
- `scripts/api_audit.sh --strict` blocks CI on public rustdoc and API-shape
  regressions; parser fixtures cover multiline attributes.

Example and panic-section completeness remain review requirements; compiler
rustdoc and the API audit are the automated source-of-truth checks.

---

## When in doubt

1. **Find a similar method's docstring** in the same family.
2. **Copy its structure** — sections, length, link style.
3. **Adapt the content**.

Consistency across the codebase beats local optimization.
