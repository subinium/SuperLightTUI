# SLT Competitive Analysis & Roadmap

**Date**: 2026-03-19 (v0.14.1)
**Scope**: Feature-level comparison against ratatui, Textual, Ink, Bubbletea + prioritized development roadmap

---

## Framework Overview

| | **SLT** | **Ratatui** | **Textual** | **Ink** | **Bubbletea** |
|---|---|---|---|---|---|
| Language | Rust | Rust | Python | JS (React) | Go |
| GitHub Stars | New | 19K | 34K | 14K | 40K |
| Rendering Model | Immediate (closure) | Immediate (Widget trait) | Retained (Event + CSS) | Component (React) | Elm (MVU) |
| Built-in Widgets | **116+** | ~15 | **60+** | 6 (+12 @inkjs/ui) | ~12 (Bubbles) |
| Ecosystem | Small | **2,928 deps, 50+ widget crates** | Moderate | **3.9K deps** | **25K deps** |
| Dependencies | 2 (crossterm optional, unicode-width) | 1+ (crossterm, etc.) | Many (Rich, etc.) | Many (React, Yoga) | 0 (pure Go) |

---

## Where SLT Leads

| Strength | Detail | Competitors |
|---|---|---|
| **AI-Native Widgets** | StreamingText, ToolApproval, ContextBar, StreamingMarkdown | None — SLT is the only TUI framework with built-in AI workflow widgets |
| **Animation System** | 5 types (Tween/Spring/Keyframes/Sequence/Stagger) + 9 easings + 3 loop modes | No other TUI framework has a dedicated animation system |
| **Responsive Breakpoints** | 5 tiers (xs/sm/md/lg/xl) × 35 properties | None — equivalent to CSS media queries for TUI |
| **3-Line Start** | `slt::run(\|ui\| { ui.text("hello"); })` | Ratatui: ~20 lines, Textual: ~10 lines |
| **Error Boundaries** | `error_boundary()` catches panics, app continues | Only React (Ink) has this; no other Rust TUI does |
| **Theme Presets** | 7 built-in + builder + runtime switching | Ratatui: no theme concept, manual styling only |
| **Chart Widgets** | 8 types (bar, line, area, scatter, histogram, candlestick, heatmap, sparkline) | Ratatui: 3 types, others: 0-1 |
| **Image Protocols** | 3 built-in (HalfBlock, Kitty, Sixel) | Ratatui: via ratatui-image (3rd party) |
| **Syntax Highlighting** | Tree-sitter AST-accurate, 15 languages built-in | Textual: tree-sitter. Ratatui/Ink/Bubbletea: none built-in |
| **Testing Suite** | TestBackend + EventBuilder + proptest + criterion + insta | Most comprehensive among all TUI frameworks |
| **Widget Count** | 116+ built-in | Ratatui: ~15, Ink: ~18, Bubbletea: ~12, Textual: ~60 |

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
| Hooks | ✅ use_state, use_memo | ❌ | ❌ | ✅ Full React | ❌ |
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

## Completed Roadmap (v0.14.0–v0.14.1)

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

---

## Remaining Roadmap

### P1 — Ecosystem Growth (v0.15)

| ID | Feature | Difficulty | Rationale |
|---|---|---|---|
| P1-6 | Plugin/extension system | Medium | pub API surface decision. Prerequisite for 3rd-party ecosystem |

### P2 — Differentiation (v0.15+)

| ID | Feature | Difficulty | Rationale |
|---|---|---|---|
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

SLT is the **most widget-rich, easiest-to-start TUI framework** across all languages, now with **multi-backend support** (crossterm, WASM) and **AST-accurate syntax highlighting**. The remaining gap to becoming a **platform** is the plugin/extension system — enabling 3rd-party widget crates to grow the ecosystem organically.

**One-line summary**: SLT has the most widgets, the best DX, multi-backend, and tree-sitter highlighting. It needs the plugin system to unlock ecosystem growth.
