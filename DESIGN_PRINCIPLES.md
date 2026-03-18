# SLT Design Principles

These principles guide every design decision in SLT.
Read this before contributing code. If a decision conflicts with these principles, raise it in the PR.

---

## 1. Ease of Use Above All

SLT exists so that building a TUI is as easy as building a web page.
Every API decision is judged by: **"Can a developer use this correctly on the first try, without reading the docs?"**

```rust
// 5 lines. No App struct. No Model/Update/View. No event loop.
fn main() -> std::io::Result<()> {
    slt::run(|ui| {
        ui.text("hello, world");
    })
}
```

If an API requires explanation, the API is wrong — not the developer.

---

## 2. Your Closure IS the App

SLT is an **immediate-mode** UI library. There is no framework state to manage.

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

This is the foundational decision. Every other principle flows from it.

---

## 3. Widget Contract

Every widget follows the same pattern. No exceptions.

### Interactive Widgets

```rust
pub fn widget_name(&mut self, state: &mut WidgetState) -> Response
```

- Return `Response` — contains `clicked`, `hovered`, `changed`, `focused`, `rect`
- Call `register_focusable()` for keyboard navigation
- Consume handled key events (don't let them bubble)
- Use `self.theme.*` for default colors — never hardcode

### Display Widgets

```rust
pub fn text(&mut self, content: impl Into<String>) -> &mut Self
```

- Return `&mut Self` for chaining (`.bold().fg(Color::Cyan)`)
- No focusable registration
- No event consumption

### Containers

```rust
pub fn col(self, f: impl FnOnce(&mut Context)) -> Response
```

- `ContainerBuilder` uses consuming `self` pattern (builder is done after `.col()`/`.row()`)
- Return `Response` for interaction detection

### State Structs

- Live in `widgets.rs` — e.g., `TextInputState`, `TableState`
- Named `{Widget}State`
- Implement `Default` when sensible
- Re-exported from `lib.rs`

---

## 4. Layout = CSS Flexbox, Syntax = Tailwind

Layout uses flexbox semantics: `row()`, `col()`, `gap()`, `grow()`, `spacer()`.
Styling uses Tailwind-inspired shorthand:

| Full name | Shorthand |
|-----------|-----------|
| `.padding(2)` | `.p(2)` |
| `.margin(1)` | `.m(1)` |
| `.width(20)` | `.w(20)` |
| `.height(10)` | `.h(10)` |

Both forms are always available. Shorthand is preferred in examples.

### Responsive Breakpoints

Prefix with breakpoint: `.md_w(40)`, `.lg_p(2)`, `.xl_gap(3)`.
Breakpoints: Xs (<40), Sm (40-79), Md (80-119), Lg (120-159), Xl (>=160).

### Builder Patterns

| Builder | Pattern | Why |
|---------|---------|-----|
| `ContainerBuilder` | Consuming `self` | Forces explicit finalization (`.col()`, `.row()`, `.draw()`) |
| `Style` | Consuming `mut self` | Chainable, zero-cost |
| `ChartBuilder` | Mutable `&mut self` | Historical — scheduled for unification in v1.0 |

---

## 5. State Ownership

| State type | Owner | Example |
|------------|-------|---------|
| Application state | User's closure | `let mut count = 0;` |
| Component-local state | Hook system | `ui.use_state(|| 0)` |
| Widget state | User | `let mut input = TextInputState::new()` |

### Hook Rules (same as React)

- `use_state()` and `use_memo()` must be called in the **same order** every frame
- Never call hooks inside conditionals or loops
- Hook type mismatches panic with a descriptive message — this is a programmer error

---

## 6. Error Handling

SLT uses `std::io::Result` for all fallible operations.
**We intentionally avoid custom error types.**

| Category | Mechanism | When |
|----------|-----------|------|
| Terminal I/O failure | `io::Result` | `run()`, `flush()`, event polling |
| Programmer error | `panic!()` with message | Hook type mismatch, invariant violation |
| Input validation | `Result<(), String>` | User-provided validator closures |

### Rules

- **No `unwrap()` in Result-returning functions** — enforced by `clippy::unwrap_in_result`
- **Panics are for programmer errors only** — never for user input or I/O
- Panic messages must include context: index, expected type, actual value
- Use `#[track_caller]` on public functions that may panic

### Why No Custom Error Type?

SLT's only runtime error path is terminal I/O. Wrapping `io::Error` in `SltError` would:
- Add API surface that becomes a semver commitment
- Require `From` conversions with no added information
- Complicate downstream `?` chains

When distinct error categories emerge (config parsing, resource loading, backend initialization), we will introduce a structured error type. Not before.

---

## 7. Performance Principles

SLT renders at 60+ FPS on modest hardware. These patterns keep it fast:

| Technique | What it does |
|-----------|-------------|
| `collect_all()` | Single DFS pass replaces 7 separate tree traversals |
| `apply_style_delta()` | Only emits changed ANSI attributes per cell during flush |
| Keyframe pre-sort | Stops sorted at build time, not per-frame |
| Double-buffer diff | Only changed cells are written to the terminal |
| Viewport culling | Off-screen widgets are skipped entirely |

### Rules

- Performance changes must not break correctness — run the full test suite
- Measure before optimizing — use the `benchmarks` bench suite (`cargo bench`)
- Minimize per-frame allocations — prefer reuse over allocation
- Profile before assuming — `cargo flamegraph` for hot path identification

---

## 8. API Stability

SLT follows [Semantic Versioning](https://semver.org/).

| Version range | Compatibility promise |
|---------------|----------------------|
| 0.12.x (patch) | Backward compatible — no breaking changes |
| 0.x → 0.y (minor) | May contain breaking changes (pre-1.0) |
| 1.x (post-1.0) | Strict semver — breaking changes only in major versions |

### MSRV Policy

- Minimum Supported Rust Version is declared in `Cargo.toml` (`rust-version`)
- MSRV bumps only happen in **minor** version releases
- MSRV bumps are documented in CHANGELOG.md

### Deprecation

- Deprecate before removing: at least one minor version with `#[deprecated]`
- Deprecated items include a migration path in the deprecation message
- Removal happens in the next minor version at earliest

---

## 9. Dependencies

**Minimal by design.**

| Dependency | Purpose | Required? |
|------------|---------|-----------|
| `crossterm` | Terminal I/O, event polling | Yes |
| `unicode-width` | Character width measurement | Yes |
| `compact_str` | String optimization | Yes |
| `tokio` | Async runtime | Optional (`async` feature) |
| `serde` | Serialization | Optional (`serde` feature) |
| `image` | Image loading | Optional (`image` feature) |

### Rules

- Do not add new required dependencies without discussion
- Optional dependencies go behind feature flags
- Feature flags must be **additive** — enabling a feature must not remove types or change existing behavior
- Prefer `dep:` syntax in `[features]` to avoid implicit feature names

---

## 10. Safety

- **Zero `unsafe` code** — enforced by `#![forbid(unsafe_code)]`
- No `unwrap()` in library code where `Result` is returned — enforced by `clippy::unwrap_in_result`
- `dbg!()`, `println!()`, `eprintln!()` are forbidden in library code — enforced by clippy lints
- `missing_docs` tracked via CI (non-blocking) — all new public API items should have doc comments
