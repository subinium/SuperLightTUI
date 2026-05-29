# SLT Naming Conventions (Micro Tier)

> Companion to [`DESIGN_PRINCIPLES.md`](./DESIGN_PRINCIPLES.md). This doc
> is the **Micro tier**: how methods, types, parameters, and modules are
> named.

The principle is **shape-encoding**: a name's shape (verb / noun /
adjective, length, suffix, prefix) tells the reader its category. When a
new contributor or AI assistant sees a name they've never seen, they
should infer category and call shape from the name alone.

---

## Categories

Every public identifier falls into one category. The category drives the
shape.

### 1. Verbs — Actions

Methods that perform a side effect or trigger a state transition.

**Shape**: `<verb>` or `<verb>_<object>`.

**Examples**:
```rust
ui.quit();
ui.notify("saved", ToastLevel::Info);
ui.focus_by_name("search");
ui.register_focusable();
ui.consume_indices(vec![0, 2]);
state.set_ratio(0.5);
```

**Anti-pattern**: noun-as-verb for actions.

```rust
ui.text("hi")    // grandfathered: immediate-mode tradition; "text" treated as a verb here
ui.button("ok")  // grandfathered for the same reason
```

New actions should use real verbs.

### 2. Nouns — Getters

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

### 3. Adjectives — Builder modifiers

Methods on `ContainerBuilder` (Layer 2) that configure the container.

**Shape**: short adjective or terse phrase.

**Examples**:
```rust
ui.container()
  .bordered(Border::Single)
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
.with_padding(2)     // ok if `with_` prefix is the family convention
.p(2)                // good — terse, adjective-shape
.padding(2)          // also good — full word ok
```

### 4. Constructors

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
TabsState::new(["Files", "Settings"]);
```

---

## Suffix Patterns

| Suffix | Meaning | Example |
|--------|---------|---------|
| `_named` | name-keyed variant | `register_focusable_named` |
| `_keyed` | runtime-key variant | `use_state_keyed` |
| `_colored` | color-customized variant (canonical color-override for immediate widgets returning `Response`; blessed in [API_DESIGN.md](API_DESIGN.md) Rule 1) | `text_input_colored` |
| `_lang` | language-aware variant | `code_block_lang` |
| `_styled` | explicit-`Style`/`Color` override variant | `sparkline_styled` |
| `_hd` | high-density / braille-cell rendering variant | `candlestick_hd` |
| `_halfblock` | half-block (▀ / ▄) pixel-doubling render variant | `heatmap_halfblock` |
| `_pct` | percent variant | `w_pct(50)` |
| `_ratio` | ratio variant | `w_ratio(1, 3)` |
| `_minmax` | min/max bounds variant | `w_minmax(10, 30)` |
| `_gap` | gap-supplied variant | `row_gap(g, ...)` |
| `_with` | callback-ish variant | `with_if(cond, f)` |
| `State` | Layer 4 state type | `TextInputState` |
| `Response` | Layer 5 response type | `BreadcrumbResponse` |
| `Opts` | options struct (4+ fields) | `GutterOpts` |
| `Builder` | builder type (rare; usually anon `<Widget><Lifetime>`) | — |

---

## Prefix Patterns

| Prefix | Meaning | Example |
|--------|---------|---------|
| `with_` | builder-style override / conditional | `with_if`, `with_padding` |
| `use_` | hook (state-bound across frames) | `use_state`, `use_effect`, `use_state_keyed` |
| `register_` | frame-level registration | `register_focusable`, `register_focusable_named` |
| `consume_` | mark event/index as handled | `consume_indices` |
| `is_` | boolean getter | `is_quit_requested` |
| `has_` | possession boolean getter | `has_focus` (use sparingly — prefer noun like `focused`) |
| `slt_` | macro/function from SLT internals | `slt_warn!`, `slt_assert!` |

---

## Length Conventions

Length should match category:

- **Adjectives** (Layer 2 modifiers): ≤ 2 syllables typical.
  - ✅ `bg`, `fg`, `p`, `w`, `h`, `gap`, `grow`, `fill`, `border`, `bold`, `dim`
  - ✅ `bordered`, `padding`, `italic`, `inverted`
  - ❌ `with_padding_all_sides` — too long
- **Nouns** (Layer 1/2 getters): full word.
  - ✅ `theme`, `spacing`, `width`, `events`
  - ❌ `tm`, `sp`, `ev`
- **Verbs** (Layer 1 actions): full word + object.
  - ✅ `register_focusable`, `focus_by_name`, `consume_indices`
  - ❌ `reg_foc`, `foc_name`
- **Types**: full word.
  - ✅ `TextInputState`, `SplitPaneState`, `BreadcrumbResponse`
  - ❌ `TIState`, `SPState`, `BCResp`

**Cross-category mismatch is a yellow signal**: when one method in a
family is ≤ 6 chars and another is > 20, ask whether the categories are
correctly assigned.

---

## Abbreviation Policy

### Allowed (universal)

These are recognizable across languages and don't need expansion:

```
bg fg id idx len min max pos pct
w h x y r g b a (RGBA)
```

### Forbidden (domain-specific)

Even common-sounding domain abbreviations are forbidden because they
hide their family from new readers:

| Forbidden | Use instead |
|-----------|-------------|
| `ctx` | `context` (in private code only — public API uses `ui`) |
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

---

## Parameter Naming

### Closures

```rust
fn use_effect<F, D>(&mut self, f: F, deps: &D)              // f for the callback
fn with_if<F>(self, cond: bool, f: F) -> Self               // f for the callback
```

Use single-letter names only when the role is unambiguous from the
function context.

### Indices

- `idx` (preferred) or `i` (in tight loops).
- Never `index_of_thing` unless disambiguation is needed.

### Lengths and sizes

- `n` — count, generic
- `len` — length of a collection
- `w`, `h` — width, height in cells (SLT uses cells, not pixels)
- `cap` — capacity
- `cnt` — forbidden; use `count` or `n`

### References to `Context` / `ui`

- **Always `ui`** in public closures (`|ui| { ui.text("hi") }`).
- **`ctx`** allowed only in private internals.
- **`self`** when defining a method on `Context`.

```rust
// good — public closure
ui.col(|ui| {
    ui.text("hello");
});

// good — private impl
fn helper(ctx: &mut Context) { /* ... */ }
```

---

## Method Ordering on `impl` blocks

When defining `impl Type`, order methods to maximize rustdoc readability:

1. **Constructors** — `new`, `default` (via `impl Default`), `with_X`.
2. **Getters** — nouns, return values without side effects.
3. **Builder modifiers** — adjectives, return `Self` or `&mut Self`.
4. **Actions** — verbs, perform side effects.
5. **Trait impls** — separate `impl Trait for Type` blocks.

Generated rustdoc pages then read top-to-bottom: "how to construct, how
to read, how to configure, how to act."

---

## Type Naming

### State (Layer 4)

`<Widget>State`. Always `pub struct`.

```rust
pub struct TextInputState { /* ... */ }
pub struct SplitPaneState { /* ... */ }
pub struct TableState { /* ... */ }
```

### Response (Layer 5)

`<Widget>Response`. Compound responses must `Deref<Target = Response>`.

```rust
pub struct BreadcrumbResponse {
    pub response: Response,
    pub clicked_segment: Option<usize>,
}

impl Deref for BreadcrumbResponse {
    type Target = Response;
    fn deref(&self) -> &Response { &self.response }
}
```

### Options structs

`<Widget>Opts`. Used when ≥ 4 optional fields would otherwise inflate
the function signature.

```rust
pub struct GutterOpts<G> {
    pub width: u32,
    pub bg: Color,
    pub label_fn: G,
    pub highlights: Vec<HighlightRange>,
}
```

### Enums

Full words for variants, no abbreviations:

```rust
pub enum BarDirection {
    Horizontal,                                 // good
    Vertical,
}

pub enum Border {
    None, Single, Double, Rounded, Thick,       // good
}

// bad
pub enum BD { H, V }
```

### Builders (anonymous)

Builder structs use `<Widget><Lifetime>` shape:

```rust
pub struct Gauge<'a> { /* borrows &mut Context */ }
pub struct LineGauge<'a> { /* ... */ }
pub struct Breadcrumb<'a> { /* ... */ }
```

The lifetime parameter is required because builders borrow `&mut Context`.

---

## Module Naming

### Files

`snake_case.rs`. Group by family, not by widget count.

| Module | Family |
|--------|--------|
| `widgets_display/text.rs` | text & inline styling |
| `widgets_display/status.rs` | alerts, badges, code blocks |
| `widgets_input/text_input.rs` | text input variants |
| `widgets/responses.rs` | Layer 5 compound response types |

### Modules vs files

Rust 2018 style: `filename.rs` + `filename/` directory. Avoid `mod.rs`.

```
src/widgets.rs           # facade re-exports
src/widgets/
    input.rs             # state types in input family
    selection.rs
    ...
```

---

## Rename History

For AI assistants: do **not** search for these old names. They were
removed in v0.20.

| Old | New | Reason |
|-----|-----|--------|
| `gauge_w(r, w)` | `gauge(r).width(w)` | Builder consolidation |
| `gauge_colored(r, c)` | `gauge(r).color(c)` | Builder consolidation |
| `line_gauge_with(r, opts)` | `line_gauge(r).<chain>` | Builder consolidation |
| `breadcrumb_sep(b, s)` | `breadcrumb(b).separator(s)` | Builder consolidation |
| `LineGaugeOpts` | `LineGauge<'_>` builder | Builder consolidation |
| `HighlightRange::single(...)` | `HighlightRange::line(...)` | Single-line was the only use |
| `label_owned(s)` | `label(s)` | `impl Into<String>` accepts both |

These removals are **breaking changes**, but they were introduced *and*
removed in the same release (v0.20), so the pre-release window was the
only place they ever existed.

### Deprecations (v0.21.0)

Unlike the v0.20 renames above, these names **still exist** as
`#[deprecated(since = "0.21.0")]` forwarding aliases for the whole v0.21.x
line. Prefer the new form; the old names compile (with a warning) and are
scheduled for removal in the next major.

| Deprecated | New | Reason |
|------------|-----|--------|
| `code_block_lang(c, l)` | `code_block(c).lang(l)` | Builder consolidation — `_lang` × `_numbered` cross-product folded into one `CodeBlock` builder |
| `code_block_numbered(c)` | `code_block(c).numbered()` | Builder consolidation |
| `code_block_numbered_lang(c, l)` | `code_block(c).lang(l).numbered()` | Builder consolidation |

> Note: `code_block(code)` now returns a `CodeBlock<'_>` builder instead of
> `Response`. It still renders on `Drop`, so the common
> `ui.code_block(code);` / `let _ = ui.code_block(code);` forms are
> unaffected; callers that bound the old `Response` should add `.show()`.

The `_colored` suffix is **not** deprecated: it is the blessed canonical
color-override variant for immediate widgets (see the suffix table and
[API_DESIGN.md](API_DESIGN.md) Rule 1).

---

## Anti-patterns Observed in Review

### Mixed verbs in the same widget family

```rust
// historic — pre-v0.20 the gauge family had both
ui.gauge(0.6);                    // immediate
ui.gauge_w(0.6, 24);              // immediate-with-arg
LineGaugeOpts::new(...).render()  // builder

// resolved — v0.20 unified to:
ui.gauge(0.6).width(24).color(Color::Cyan);
ui.line_gauge(0.6).label("60%").width(24);
```

### Long form when a short adjective exists

```rust
// rejected in review
ui.container().apply_border(Border::Single).p_padding_value(2)

// correct
ui.container().border(Border::Single).p(2)
```

### Abbreviation creep

```rust
// rejected
fn dbg_print_state(...)

// correct
fn debug_print_state(...) // or, better: slt_debug!(...)
```

### Type abbreviation

```rust
// rejected
pub struct TIState { /* TextInputState */ }

// correct
pub struct TextInputState { /* ... */ }
```

---

## When in doubt

1. **Find an existing analogous method/type** in the same family.
2. **Match its shape** — same prefix, same suffix, same length category.
3. **If no analog exists**, document why this is a new pattern in the
   PR description and update [`DESIGN_PRINCIPLES.md`](./DESIGN_PRINCIPLES.md)
   matrix if the new shape changes a cell.
