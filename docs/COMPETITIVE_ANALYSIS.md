# SLT Competitive Analysis & Roadmap

**Date**: 2026-03-19 (v0.13.2)
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
| Dependencies | 2 (crossterm, unicode-width) | 1+ (crossterm, etc.) | Many (Rich, etc.) | Many (React, Yoga) | 0 (pure Go) |

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
| **Testing Suite** | TestBackend + EventBuilder + proptest + criterion + insta | Most comprehensive among all TUI frameworks |
| **Widget Count** | 116+ built-in | Ratatui: ~15, Ink: ~18, Bubbletea: ~12, Textual: ~60 |

---

## Where SLT Lags

### Structural Gaps (Architecture Level)

| Gap | Impact | Competitor Reference |
|---|---|---|
| **Single backend (crossterm only)** | Cannot target WASM, embedded, egui, Bevy | Ratatui: 6 backends (crossterm, termion, termwiz, WASM via ratzilla, embedded via mousefood, Bevy/egui) |
| **No plugin system** | Cannot grow a 3rd-party widget ecosystem | Ratatui: Widget trait enables 50+ community crates |
| **No event bubbling** | Complex widget composition requires manual event forwarding | Textual: DOM-like message propagation |
| **No CSS/declarative styling** | Style reuse limited to code-level composition | Textual: `.tcss` files with hot-reload |
| **O(n) hit testing** | Mouse performance degrades with 1000+ widgets | Should be HashMap for O(1) lookup |

### Missing Features (Implementable)

| Feature | Source | Difficulty | Notes |
|---|---|---|---|
| Gradient text | Ink (ink-gradient) | Easy | `Color::blend()` already exists |
| BigText (ASCII art) | Ink (ink-big-text) | Easy | font8x8 data + `draw()` callback |
| Timer/Stopwatch | Bubbletea (Bubbles) | Easy | Wrapper over animation system |
| QR code | tui-qrcode | Easy | `draw()` + `qrcode` crate |
| Background color query | Bubbletea v2 | Medium | OSC 11 sequence |
| RichLog (log viewer) | Textual | Medium | Essential for AI/CLI apps |
| DirectoryTree | Textual | Medium | Extend existing FilePicker |
| TextArea syntax highlight | Textual (tree-sitter) | Hard | Editable code widget, killer feature |
| Accessibility (screen reader) | **Nobody has this** | Hard | First-mover opportunity |
| SSH server mode | Bubbletea (Wish) | Hard | Remote TUI serving |

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
| Reactive binding | ❌ | ❌ | ✅ | ❌ | ❌ |
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
| Terminal backends | crossterm | crossterm, termion, termwiz | Built-in | Built-in | Built-in |
| WASM/Browser | ❌ | ✅ ratzilla | ✅ textual serve | ❌ | ❌ |
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
| OSC 52 clipboard | ✅ write | ❌ | ❌ | ❌ | ✅ v2 read+write |
| Background color query | ❌ | ❌ | ❌ | ❌ | ✅ v2 |

---

## Prioritized Development Roadmap

### P0 — Ecosystem Foundation (v0.14)

| ID | Feature | Rationale |
|---|---|---|
| P0-1 | **Backend trait abstraction** | crossterm-only → trait separation. Enables WASM/termion/egui backends. Largest structural gap vs ratatui |
| P0-2 | **WASM backend** | Browser demos = adoption multiplier. ratzilla proves it's viable |

### P1 — Competitive Parity (v0.14–0.15)

| ID | Feature | Difficulty | Rationale |
|---|---|---|---|
| P1-1 | Gradient text | Easy | High visual impact, `Color::blend()` exists |
| P1-2 | BigText (ASCII art) | Easy | Dashboard/splash essential. font8x8 + `draw()` |
| P1-3 | Timer/Stopwatch | Easy | Simple wrapper over animation system |
| P1-4 | RichLog widget | Medium | Essential for AI/CLI apps. No alternative |
| P1-5 | Background color query | Medium | Auto dark/light mode via OSC 11 |
| P1-6 | Plugin/extension system | Medium | Prerequisite for 3rd-party ecosystem growth |

### P2 — Differentiation (v0.15+)

| ID | Feature | Difficulty | Rationale |
|---|---|---|---|
| P2-1 | TextArea syntax highlighting | Hard | tree-sitter integration. Editable code widget |
| P2-2 | Accessibility (screen reader) | Hard | Nobody has this. First-mover advantage |
| P2-3 | SSH server mode | Hard | Remote TUI serving |
| P2-4 | Reactive binding | Medium | Cross-widget auto-sync |
| P2-5 | CSS-like style files | Hard | Hot-reload = Textual-tier DX |

### Quick Wins (Implementable Immediately)

| Feature | Approach | Estimated Time |
|---|---|---|
| Gradient text | `Color::blend()` + per-character fg | 1 hour |
| BigText | font8x8 data + `draw()` callback | 2 hours |
| Timer/Stopwatch | Wrapper over `Tween` | 1 hour |
| QR code | `qrcode` crate + `draw()` | 1 hour |
| Background color query | OSC 11 sequence + response parsing | 2 hours |

---

## Key Insight

SLT is the **most widget-rich, easiest-to-start TUI framework** across all languages. But to become a **platform** (like ratatui), it needs Backend trait abstraction — the single change that unlocks WASM, embedded, egui, and 3rd-party ecosystem growth.

**One-line summary**: SLT has the most widgets and the best DX. It needs the backend abstraction to run everywhere.
