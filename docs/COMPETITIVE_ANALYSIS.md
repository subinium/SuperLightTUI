# SLT Competitive Analysis & Roadmap

**Date**: 2026-04-27 (v0.19.2)
**Scope**: Feature-level comparison against ratatui, Textual, Ink, Bubbletea + prioritized development roadmap

---

## Framework Overview

| | **SLT** | **Ratatui** | **Textual** | **Ink** | **Bubbletea** |
|---|---|---|---|---|---|
| Language | Rust | Rust | Python | JS (React) | Go |
| GitHub Stars | New | 19K | 34K | 14K | 40K |
| Rendering Model | Immediate (closure) | Immediate (Widget trait) | Retained (Event + CSS) | Component (React) | Elm (MVU) |
| Built-in Widgets | Broad built-in catalog | ~15 | **60+** | 6 (+12 @inkjs/ui) | ~12 (Bubbles) |
| Ecosystem | Small | **2,928 deps, 50+ widget crates** | Moderate | **3.9K deps** | **25K deps** |
| Dependencies | 2 (unicode-width, compact_str; crossterm optional) | 1+ (crossterm, etc.) | Many (Rich, etc.) | Many (React, Yoga) | 0 (pure Go) |

---

Widget counts are not perfectly apples-to-apples across frameworks, so this document avoids treating every public helper method or specialized primitive as a separate widget type.

---

## Where SLT Leads

| Strength | Detail | Competitors |
|---|---|---|
| **AI-Native Widgets** | StreamingText, ToolApproval, ContextBar, StreamingMarkdown | None — SLT is the only TUI framework with built-in AI workflow widgets |
| **Animation System** | 5 types (Tween/Spring/Keyframes/Sequence/Stagger) + 9 easings + 3 loop modes | No other TUI framework has a dedicated animation system |
| **Responsive Breakpoints** | 5 tiers (xs/sm/md/lg/xl) × 35 properties | None — equivalent to CSS media queries for TUI |
| **3-Line Start** | `slt::run(\|ui\| { ui.text("hello"); })` | Ratatui: ~20 lines, Textual: ~10 lines |
| **Error Boundaries** | `error_boundary()` catches panics, app continues | Only React (Ink) has this; no other Rust TUI does |
| **Theme Presets** | 10 built-in + builder + runtime switching + semantic tokens (ThemeColor) + contrast helpers | Ratatui: no theme concept, manual styling only |
| **Chart Widgets** | 10+ types (bar, stacked bar, line, area, scatter, histogram, candlestick, heatmap, sparkline, treemap) | Ratatui: 3 types, others: 0-1 |
| **Image Protocols** | 3 built-in (HalfBlock, Kitty, Sixel) | Ratatui: via ratatui-image (3rd party) |
| **Syntax Highlighting** | Tree-sitter AST-accurate, 15 languages built-in | Textual: tree-sitter. Ratatui/Ink/Bubbletea: none built-in |
| **Testing Suite** | TestBackend + EventBuilder + proptest + criterion + insta | Most comprehensive among all TUI frameworks |
| **Widget Surface** | Broad built-in catalog spanning text, forms, overlays, charts, AI widgets, and terminal-specific primitives | Ratatui: ~15, Ink: ~18, Bubbletea: ~12, Textual: ~60 |

---

## Where SLT Lags

### Structural Gaps (Architecture Level)

| Gap | Impact | Competitor Reference |
|---|---|---|
| **No plugin system** | Cannot grow a 3rd-party widget ecosystem | Ratatui: Widget trait enables 50+ community crates |
| **No event bubbling** | Complex widget composition requires manual event forwarding | Textual: DOM-like message propagation |
| **No CSS/declarative styling** | Style reuse limited to code-level composition | Textual: `.tcss` files with hot-reload |

### Missing Features (Implementable)

| Feature | Source | Difficulty | Notes |
|---|---|---|---|
| TextArea + syntax highlight | Textual (tree-sitter) | Hard | `highlight_code()` exists (v0.14.1), needs TextareaState integration with incremental parsing |
| Accessibility (screen reader) | **Nobody has this** | Hard | First-mover opportunity, terminal accessibility protocol absent |
| SSH server mode | Bubbletea (Wish) | Hard | Backend trait ready, needs `russh` companion crate |

---

## Feature Comparison Matrix

### Layout

| Feature | SLT | Ratatui | Textual | Ink | Bubbletea |
|---|---|---|---|---|---|
| Flexbox (Row/Col) | ✅ | ✅ Constraint | ✅ | ✅ Yoga | ❌ |
| Grid | ✅ | ❌ | ✅ CSS Grid | ❌ | ❌ |
| Responsive breakpoints | ✅ 5 tiers | ❌ | ❌ | ❌ | ❌ |
| Percentage sizing | ✅ | ✅ | ✅ | ✅ | ❌ |
| Absolute positioning | ❌ | ❌ | ✅ CSS dock | ❌ | ❌ |
| CSS file styling | ❌ | ❌ | ✅ TCSS + hot-reload | ❌ | ❌ |

### State & Architecture

| Feature | SLT | Ratatui | Textual | Ink | Bubbletea |
|---|---|---|---|---|---|
| State model | Closure + hooks | Manual struct | Reactive attrs | React hooks | Elm MVU |
| Hooks | ✅ use_state, use_memo, use_state_named (id-keyed; safe in conditionals), provide / use_context (scoped state injection) | ❌ | ❌ | ✅ Full React | ❌ |
| Event bubbling | ❌ | ❌ | ✅ | ✅ | ❌ |
| Error boundary | ✅ | ❌ | ❌ | ✅ | ❌ |
| Custom widgets | ✅ Widget trait | ✅ Widget trait | ✅ Class inheritance | ✅ React components | ✅ Model interface |

### Animation

| Feature | SLT | Ratatui | Textual | Ink | Bubbletea |
|---|---|---|---|---|---|
| Tween | ✅ | ❌ | ❌ | ❌ | ❌ |
| Spring (physics) | ✅ | ❌ | ❌ | ❌ | ❌ |
| Keyframes | ✅ | ❌ | ❌ | ❌ | ❌ |
| Sequence (chain) | ✅ | ❌ | ❌ | ❌ | ❌ |
| Stagger (list) | ✅ | ❌ | ❌ | ❌ | ❌ |
| CSS transition | ❌ | ❌ | ✅ | ❌ | ❌ |

### Deployment & Backends

| Feature | SLT | Ratatui | Textual | Ink | Bubbletea |
|---|---|---|---|---|---|
| Terminal backends | crossterm (optional) | crossterm, termion, termwiz | Built-in | Built-in | Built-in |
| WASM/Browser | ✅ slt-wasm | ✅ ratzilla | ✅ textual serve | ❌ | ❌ |
| Embedded (no_std) | ❌ | ✅ v0.30 | ❌ | ❌ | ❌ |
| SSH server | ❌ | ❌ | ✅ | ❌ | ✅ Wish |
| Game engine | ❌ | ✅ Bevy, egui | ❌ | ❌ | ❌ |
| Inline mode | ✅ | ❌ | ❌ | ✅ | ✅ |
| Static output | ✅ | ❌ | ✅ | ✅ `<Static>` | ❌ |

### Terminal Protocols

| Protocol | SLT | Ratatui | Textual | Ink | Bubbletea |
|---|---|---|---|---|---|
| Synchronized Output | ✅ | ✅ | ✅ | ❌ | ✅ v2 |
| Kitty Keyboard | ✅ | ✅ | ❌ | ❌ | ✅ v2 |
| Kitty Graphics | ✅ built-in | ✅ ratatui-image | ❌ | ❌ | ❌ |
| Sixel | ✅ built-in | ✅ ratatui-image | ❌ | ❌ | ❌ |
| OSC 8 links | ✅ | ✅ | ✅ | ✅ | ❌ |
| OSC 52 clipboard | ✅ read+write | ❌ | ❌ | ❌ | ✅ v2 read+write |
| OSC 11 color scheme | ✅ | ❌ | ❌ | ❌ | ✅ v2 |

### Syntax Highlighting

| Feature | SLT | Ratatui | Textual | Ink | Bubbletea |
|---|---|---|---|---|---|
| Built-in highlighting | ✅ 15 languages (tree-sitter) | ❌ | ✅ tree-sitter | ❌ | ❌ |
| Language-aware code blocks | ✅ `code_block_lang()` | ❌ | ✅ | ❌ | ❌ |
| Streaming code highlight | ✅ `streaming_markdown()` | ❌ | ❌ | ❌ | ❌ |
| Per-language feature gates | ✅ `syntax-rust`, etc. | N/A | N/A | N/A | N/A |

---

## Completed Roadmap (v0.14.0 – v0.19.2)

### v0.14.0 — Ecosystem Foundation

| ID | Feature | Status |
|---|---|---|
| P0-1 | Backend trait abstraction (crossterm optional) | ✅ |
| P0-2 | WASM backend (slt-wasm companion crate) | ✅ |
| P1-1 | Gradient text | ✅ |
| P1-2 | BigText (ASCII art) | ✅ |
| P1-3 | Timer display | ✅ |
| P1-4 | RichLog widget | ✅ |
| P1-5 | Background color query (OSC 11) | ✅ |
| — | QR code widget | ✅ |
| — | DirectoryTree widget | ✅ |
| — | Event constructors (crossterm-free) | ✅ |
| — | OSC 52 clipboard read | ✅ |

### v0.14.1 — Syntax Highlighting

| ID | Feature | Status |
|---|---|---|
| P2-1 | Tree-sitter syntax highlighting (15 languages) | ✅ |
| — | `code_block_lang()` / `code_block_numbered_lang()` APIs | ✅ |
| — | `streaming_markdown()` code block highlighting upgrade | ✅ |
| — | `markdown()` fenced code block fix | ✅ |

### v0.16.0 — Core Consolidation

| ID | Feature | Status |
|---|---|---|
| P1-6 | Single frame kernel + `TestBackend` parity | ✅ |
| P1-7 | Context/session hardening | ✅ |
| P1-8 | Layout kernel split + `collect_all()` sole collector | ✅ |
| P1-9 | Runner/terminal flush dedup | ✅ |
| — | Backend contract tests | ✅ |
| — | Kernel parity + proptest coverage | ✅ |
| — | Terminal session hardening | ✅ |
| — | Interaction allocator unification | ✅ |

### v0.18.x — Performance + Hardening

| ID | Feature | Status |
|---|---|---|
| #62 | Flush coalescing — single ANSI emit per frame | ✅ |
| #64 | Command enum boxing — shrinks the per-frame command stream | ✅ |
| #67 | Flexbox U32Stack scratch — no per-frame allocations in layout | ✅ |
| — | NO_COLOR env var support | ✅ |
| — | `scroll_col` + `draw_with` helpers | ✅ |
| — | `try_get` for safer state access | ✅ |

### v0.19.0 — Component DX

| ID | Feature | Status |
|---|---|---|
| — | `provide` / `use_context::<T>()` — scoped state injection (no parameter threading) | ✅ |
| — | `use_state_named(id)` — id-keyed local state, safe inside conditionals | ✅ |
| — | `with_if(cond, modifier)` / `with(modifier)` — fluent conditional styling on text and ContainerBuilder | ✅ |

### v0.19.1 — Output + Image Perf

| ID | Feature | Status |
|---|---|---|
| — | BufWriter stdout (1 flush/frame) | ✅ |
| — | `image()` reduced to 1 RawDraw/frame (was 841) | ✅ |
| — | Sixel exact-match detection | ✅ |
| #131 | EventBuilder chain wrappers (`mouse_up`, `drag`, `key_release`, `focus_gained`, `focus_lost`) | ✅ |
| #160 | CJK title clamp regression coverage | ✅ |

### v0.19.2 — Theme + Status DX

| ID | Feature | Status |
|---|---|---|
| #108 | `mx` / `my` margin shorthands | ✅ |
| — | `ThemeBuilder` const fn — compile-time theme construction | ✅ |
| #182 | `breadcrumb_response` / `breadcrumb_response_with` returning `(Response, Option<usize>)` | ✅ |
| — | `RichLogState` bounded default (no unbounded growth) | ✅ |

---

## Remaining Roadmap

### P2 — Ecosystem Growth (post-0.16)

| ID | Feature | Difficulty | Rationale |
|---|---|---|---|
| P2-1 | Plugin/extension system | Medium | Better postponed until internal contracts stop moving |
| P2-2 | Accessibility (screen reader) | Hard | Nobody has this. First-mover advantage. Terminal protocol absent |
| P2-3 | SSH server mode | Hard | Backend trait ready. `russh` + slt-ssh companion crate |
| P2-5 | CSS-like style files | Hard | Hot-reload DX. Conflicts with immediate mode philosophy |

### Removed from Roadmap

| ID | Feature | Reason |
|---|---|---|
| ~~P2-4~~ | ~~Reactive binding~~ | Violates Principle 2 ("Your Closure IS the App"). Permanently removed |
| ~~O(n) hit testing~~ | ~~HashMap conversion~~ | Already O(1) — original analysis was incorrect |

---

## Key Insight

SLT is already one of the broadest and easiest-to-start TUI libraries in Rust, now with **multi-backend support** (crossterm, WASM) and **AST-accurate syntax highlighting**. The next gap is no longer raw feature count; it is making the frame kernel and state boundaries boring, explicit, and hard to regress.

**One-line summary**: SLT now has the core consolidation it needed. The next priority is ecosystem growth without losing the small public grammar or backend discipline.
