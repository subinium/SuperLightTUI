# SLT Demo Guide

> Companion to [`DESIGN_PRINCIPLES.md`](./DESIGN_PRINCIPLES.md). Not a tier
> doc — this is a **practitioner manual** for the people writing
> `examples/*.rs`. Every rule here exists because a real demo bug shipped
> when the rule was implicit. The "Why" line cites the bug.

If you're touching `examples/v020_*.rs`, `examples/cookbook_*.rs`, or
adding a new `demo_*.rs`, read this first. The audit checks
(`scripts/api_audit.sh` V5-V7) catch the mechanical violations; the
human-judgment items (sample widget policy, intent labeling) are
reviewer responsibility.

---

## Document map

```
1. The 4 demo archetypes        ← decide which one your demo is
2. Render-function contract     ← signature, state ownership
3. Outer-container policy       ← grow / fill rules
4. Key handling                 ← quit keys, modal-aware paths
5. Composition rules            ← demos that nest into other demos
6. Sample-widget labeling       ← when you stuff helpers in for show
7. Title and visible text       ← character set, length
8. Snapshot-vs-live render      ← when both are needed
9. macOS quirks                 ← Ctrl-C, mouse, scrollback
10. Pre-merge checklist
```

---

## 1. The 4 demo archetypes

Every demo falls into one. Picking the right archetype is the single
biggest layout decision; mismatches cause the bugs §3-§5 prevent.

| Archetype | Owns frame buffer? | Owns scrollback? | Has overlay? | Example |
|-----------|--------------------|--------------------|--------------|---------|
| **Standard** | yes (full canvas) | no | no | `v020_named_focus`, `v020_split_pane` |
| **Overlay-first** | yes | no | yes (centered modal/help) | `v020_modal_trap`, `v020_keymap_help`, `v020_dx_shortcuts` |
| **Scrollback** | partial (inline) | yes (append-only) | no | `v020_static_log` |
| **System** | no (passes through) | no | no | `v020_perf_audit`, `v020_test_utils` (non-interactive reports) |

**Why this matters**: the v0.20 tour combined demos from different
archetypes into a 2x2 grid and broke them all (overlays covered
neighbours, scrollback corrupted the bordered frame, focus widgets
fought for keys). Combining demos works only within the same archetype
— see §5.

---

## 2. Render-function contract

Every demo exposes one of these two signatures and exactly one of them:

```rust
// Stateless — for snapshot tests AND live runs
pub fn render(ui: &mut Context)

// Stateful — caller owns persistent state
pub fn render(ui: &mut Context, state: &mut DemoState)
```

If your demo holds state that must persist across frames (counters,
toggles, modal open/closed, input value), **the second form is
mandatory**. The first form's caller has no place to keep state, so any
mutation gets thrown away on the next call.

**Why**: v0.20's `v020_modal_trap` shipped with the stateless signature
even though it had a `show_modal` flag — every frame reset the modal to
"open", swallowing every Yes/No click. This was caught only when the
tour embedded the demo and tried to use it for real.

**Snapshot fix**: if you also want a one-shot frame for snapshot tests
(e.g. "render the modal-open variant for the test image"), expose a
**second** function:

```rust
// Snapshot-only. Constructs a fresh state internally, renders one frame.
// NEVER call this from a live loop or from another demo — clicks are
// silently dropped because state never persists.
pub fn render_snapshot(ui: &mut Context) {
    let mut state = DemoState { /* deterministic fixture */ };
    body(ui, &mut state);
}
```

The two functions share a private `body(ui, state)` helper. The live
`main()` and any tour-embedding caller use `render`; snapshot tests use
`render_snapshot`.

---

## 3. Outer-container policy

The outermost container (`bordered`, `container().col`, etc.) at the
top of `render()` must have `.grow(1)` or `.fill()` **unless** the demo
is intentionally letting content flow only as wide as it needs.

```rust
// good
let _ = ui
    .bordered(Border::Rounded)
    .title("…")
    .p(pad)
    .grow(1)               // ← this
    .col(|ui| { … });

// bad — outer box only fills its content's natural size, leaving the
// rest of the terminal blank. Inputs inside flex rows shrink to 1 cell.
let _ = ui.bordered(Border::Rounded).title("…").p(pad).col(|ui| { … });
```

**Why**: v0.20 `v020_named_focus` shipped without `.grow(1)`; the input
boxes were 1 cell wide because a flex row with no parent grow gives its
children only their natural minimum width. The bug was visible
instantly on launch.

**Stateful inputs in a row need a wrapping container too**: when you
put a `text_input(&mut s)` inside a `row_gap(...)` next to a label, the
input's natural width is the placeholder length (often 1 cell). Wrap
the input in `container().fill().col(|ui| ui.text_input(&mut s))` so it
claims the row's remaining width.

```rust
// good — input fills row's remaining width
let _ = ui.row_gap(gap, |ui| {
    ui.text("Name: ");
    let _ = ui.register_focusable_named("name");
    let r = ui.container().fill().col(|ui| {
        let _ = ui.text_input(&mut state.name);
    });
    if r.clicked {
        let _ = ui.focus_by_name("name");
    }
});
```

The wrapping container is also where you read `r.clicked` if you want
the whole input area to focus on click — the parent row's `clicked`
gets shadowed by the wrapping container's hit area (this surprised the
v0.20 author and produced the second iteration of the named_focus bug).

---

## 4. Key handling

### Quit keys

Every demo binds quit to the **same triple**: `q`, `Esc`, `Ctrl-Q`.
**Never bind `Ctrl-C`** — it's bound to "Copy" by default in Ghostty,
iTerm2, and Terminal.app on macOS, so the keystroke never reaches the
app reliably.

```rust
if ui.key('q') || ui.key_code(KeyCode::Esc) || ui.key_mod('q', KeyModifiers::CONTROL) {
    ui.quit();
}
```

### Modal-aware paths

When a demo opens a modal, the modal's Esc-to-dismiss must take
precedence over Esc-to-quit. Two options:

```rust
// Option A: gate the quit on !show_modal
if !state.show_modal && (ui.key('q') || ui.key_code(KeyCode::Esc) || …) {
    ui.quit();
}
if state.show_modal && ui.raw_key_code(KeyCode::Esc) {
    state.show_modal = false;
}
```

```rust
// Option B: use raw_key_code in both branches and let the modal
// consume Esc first (raw_key_code bypasses the focus-filter modal
// guard so it works even when a modal button has focus)
```

**Why**: v0.20 tour ate Esc at the top-level dispatcher, leaving
`v020_modal_trap`'s confirm modal undismissable.

### Conflict avoidance

If your demo binds keys outside the standard quit triple
(e.g. `Space`, digits, `?`, letters), document them at the top of
the file in the `//!` doc comment and **don't pick keys that other
demos in the same archetype use** — Tab is reserved for focus
cycling, Left/Right are reserved for tabs widgets, `?` is the
keymap-help convention.

---

## 5. Composition rules

A "composition" is a demo that embeds other demos — the prime example
is `v020_tour.rs`, which dispatches to one of N feature demos by tab.

### Rule C1: one archetype per tab/cell

Don't combine an Overlay-first demo with a Scrollback demo with a
Standard demo into one screen. Each archetype claims a different
resource (overlay z-order, scrollback rows, frame buffer
geometry); collisions are visible immediately and not always graceful.

The v0.20 tour originally had a 2x2 "Util" grid combining
`keymap_help` (Overlay-first), `static_log` (Scrollback),
`ctrl_c_passthrough` (Standard, fullscreen-keyed),
`dx_shortcuts` (Overlay-first). Result: overlays covered each other,
the log appended unboundedly into the bordered frame, key handlers
fought. The fix was 4 separate tabs.

### Rule C2: scrollback demos can't be re-embedded

`v020_static_log` calls `ui.static_log(line)` which writes to the
terminal scrollback. In a single-binary live run that's fine — the
line lands above the inline buffer. In a composing demo (tour), every
frame now appends a new line, scrolling the bordered frame off the top
of the screen.

If you need to surface a scrollback demo inside a tour, render a
**description page** with a code snippet and a "run standalone"
pointer instead of calling the actual scrollback API. See
`v020_tour.rs::render_log` for the pattern.

### Rule C3: state passes through, not state resets

A composing demo holds the embedded demo's state in its own state
struct:

```rust
struct TourState {
    tabs: TabsState,
    modal: modal_trap::State,        // owned by tour, passed to embedded render
    use_state_keyed: use_state_keyed::DemoState,
    // …
}
```

Never construct embedded demo state inside the composing demo's render
closure — it gets reset every frame. (See §2 "Snapshot fix" for the
underlying reason.)

---

## 6. Sample-widget labeling

When a demo embeds widgets purely for visual showcase (a `help` bar
showing keybinds, a button labeled "Click me", a code block of
`fn main()`), label each one with the issue number it demonstrates.
Otherwise viewers can't tell which widget is the *point* and which is
the *backdrop*.

```rust
// good — viewer maps each visible element to a v0.20 issue
ui.text("(buttons here demo #209 on_hover)").dim().fg(Color::Cyan);
let _ = ui.button("Save").on_hover(ui, "…");

ui.text(format!("#210 panel_alpha = {alpha:.2}")).dim();
```

**Why**: v0.20 `v020_dx_shortcuts` packed four DX helpers (#209, #210,
#220, #221) onto one screen with no per-helper labels. Reviewers asked
"what's this demo showing?" — answer required reading the source.

**Anti-pattern**: a `ui.help(&[("Tab", "next"), ("Enter", "ok"),
("Esc", "cancel")])` row stuffed into a layout demo to "fill space".
The viewer reasonably interprets the help bar as the demo's actual
keybindings, even when none of those keys do anything in that demo.
Drop it or label it `"(help bar — sample widget, keys are decorative)"`.

---

## 7. Title and visible text

### Title character set

`.title("...")` strings on bordered containers must be **BMP ASCII
only**. No em-dash, en-dash, ideographic chars, smart quotes, or any
codepoint above U+007F unless the demo is *specifically* exercising
wide-character handling.

```rust
// good
.title("SLT v0.20: Density presets")

// bad — em-dash counts as 1 col in `unicode-width` but 2 cols when
// rendered in some terminals, breaking border alignment
.title("SLT v0.20 — Density presets")
```

**Why**: v0.20 demo polish caught this for ~10 demos. The
`scripts/api_audit.sh` V7 check now flags it automatically.

### Body text length

Help banners and instructional text inside the demo body should fit
the demo's intended minimum width (typically 80 cols). Long sentences
that wrap mid-instruction look like artifacts. If you need a long
explanation, use `.line_wrap(...)` so the wrap is intentional.

---

## 8. Snapshot-vs-live render

If your demo has both a snapshot test (`tests/v020_*_demo.rs`) and a
live binary, use the §2 split:

```rust
fn body(ui: &mut Context, state: &mut DemoState) { /* shared */ }

pub fn render(ui: &mut Context, state: &mut DemoState) {
    handle_input(ui, state);
    body(ui, state);
}

pub fn render_snapshot(ui: &mut Context) {
    let mut state = deterministic_fixture();
    body(ui, &mut state);
}

fn main() -> std::io::Result<()> {
    let mut state = DemoState::default();
    slt::run_with(RunConfig::default().mouse(true), move |ui| {
        if ui.key('q') || ui.key_code(KeyCode::Esc) || ui.key_mod('q', KeyModifiers::CONTROL) {
            ui.quit();
        }
        render(ui, &mut state);
    })
}
```

`render_snapshot` constructs the fixture and calls `body` directly,
bypassing input handling. Snapshot tests call `render_snapshot`; live
binary uses `render`.

---

## 9. macOS quirks

- **Ctrl-C is bound to "Copy"** in Ghostty, iTerm2, and Terminal.app
  by default. Don't rely on it as a quit key. See §4.
- **Mouse capture**: every interactive demo uses
  `slt::run_with(RunConfig::default().mouse(true), …)`. Without this,
  click events never reach the app and demos appear "broken".
- **Scrollback in alternate screen**: full-screen demos (default
  `slt::run`) use the alternate screen, where scrollback is isolated
  from the user's shell. `static_log` only makes sense in inline mode
  (`InlineTerminal`); don't add it to alternate-screen demos.

---

## 10. Pre-merge checklist

Before submitting a demo (new or modified):

- [ ] Archetype declared (one of §1) — comment in the file header.
- [ ] Render signature matches §2.
- [ ] Outer container has `.grow(1)` or `.fill()` (or §3 explains why
      not).
- [ ] Quit keys are exactly `q` / `Esc` / `Ctrl-Q` (§4).
- [ ] Title characters are BMP ASCII (§7).
- [ ] If composing other demos, follows §5 rules C1-C3.
- [ ] If sample widgets are present, each is labeled or dim-captioned
      (§6).
- [ ] `cargo run --example <name>` actually exercises the intended
      interaction without manual prodding.
- [ ] `scripts/api_audit.sh --strict` exits 0.
- [ ] `cargo test --all-features` passes.
- [ ] (When the v0.21 visual-regression fixtures land:) at least one
      assertion in `tests/v020_interaction_regression.rs` covers the
      demo's headline interaction.

---

## Appendix: bug → rule mapping

Every rule in this document was added because a real bug shipped or
nearly shipped. Use this table to navigate from "I'm hitting bug X"
back to "the rule that prevents X".

| Bug | Section | Rule |
|-----|---------|------|
| Input shrinks to 1 cell | §3 | Outer container needs `.grow(1)` |
| Click on row doesn't focus input | §3 | Wrap input in `container().fill()`, read `clicked` from THAT |
| Modal Yes/No reset every frame | §2 | Stateful demos need `(ui, &mut state)` signature |
| Esc closes app instead of modal | §4 | Modal-aware paths use `raw_key_code` + `!show_modal` gate |
| Tokens stack vertically | (audit V6) | Fallback path must mirror primary's container nesting |
| Border misaligned around title | §7 | BMP ASCII titles only |
| `register_focusable_named("X")` doesn't focus the next widget | (lib fix) | Library now uses deferred-attach; demo pattern unchanged |
| Tour overlays cover other cells | §5 C1 | One archetype per cell |
| Tour log appends unboundedly | §5 C2 | Scrollback demos render description page when embedded |
| Demo helpers are visually indistinguishable | §6 | Label sample widgets with `dim().fg(Cyan)` captions |
