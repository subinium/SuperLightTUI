# SLT Competitive Analysis

**Snapshot**: as of 2026-10-18, SLT v0.25.0
**Scope**: SLT against ratatui, Textual, Bubble Tea, Ink, and OpenTUI, with short
notes on other TUI frameworks. This is a dated snapshot, not a living spec:
competitor versions, star counts, and download figures go stale quickly.

> **How this was checked.** SLT rows were verified against `src/` at v0.25.0.
> Competitor facts come from the release notes and repositories listed under
> [Sources](#sources), gathered for this audit. Cells marked "n/v" were not
> verified for this snapshot; do not read them as "no".

---

## TL;DR

- **Where SLT is strong**: one Rust crate ships the interaction layer that
  ratatui leaves to apps or ecosystem crates: hit-tested `Response` on any
  container, a focus manager with modal trap and follow-focus scrolling, scroll
  containers with a draggable scrollbar, forms with sync/async validation, a
  command palette, an F12 layout inspector, and headless snapshot testing. The
  default dependency tree is small (26 crates vs 67 for ratatui + crossterm).
- **Where SLT loses**: adoption and ecosystem, by orders of magnitude. ratatui
  has 23.0K stars and 6,469 reverse dependencies; SLT has 193 and 5. SLT also
  lacks several features that competitors ship today: text selection inside
  inputs, an editable syntax-highlighted textarea, anchored popovers and
  menus, a diff viewer, hover/focus state styles on containers, an ARIA-style
  accessibility tree (v0.26 adds only a linear prompt mode), terminal
  title/progress/cursor-shape APIs, and SSH serving.
- **Performance is not an SLT advantage.** As of v0.25.0 SLT is slower per
  frame than ratatui 0.30 (2–3.5x on matched workloads); optimization is
  tracked. Its stripped hello-world binary is 1.92x larger and its cold release
  build ~1.6x slower. SLT does emit 20–40% fewer bytes on a full redraw (see
  [Performance and footprint](#performance-and-footprint)).

---

## Framework overview

| | **SLT** | **ratatui** | **Textual** | **Bubble Tea** | **Ink** | **OpenTUI** |
|---|---|---|---|---|---|---|
| Version (2026-10) | 0.25.0 | 0.30.2 | 8.2.8 | v2.1.0 | 8.0.0 | v0.6.6 |
| Language | Rust | Rust | Python | Go | JS/TS (React) | Zig core + TS (React/Solid) |
| GitHub stars | 193 | 23.0K | 37.5K | 45.4K | 40.1K | 13.6K |
| Model | Immediate (closure + hooks) | Immediate (widgets render to a buffer) | Retained (DOM + TCSS) | Elm (MVU) | React components | React/Solid components |
| Layout | Flexbox + grid | Constraint splits | TCSS (dock, grid) | Lip Gloss joins/layers | Yoga flexbox | Yoga flexbox |
| Notable users | n/v | Large crate ecosystem | n/v | Charm tooling | Claude Code, Gemini CLI | opencode |

crates.io (2026-10): ratatui 60.96M total downloads and 6,469 reverse
dependencies; SLT 4,993 downloads and 5 reverse dependencies.

State notes:

- **ratatui 0.30** split into `ratatui-core` and `ratatui-widgets`. It still has
  no built-in focus, hover, or scroll containers; apps hand-roll them or use
  ecosystem crates (`ratatui-textarea` 0.9.3, `tachyonfx` 0.25, `ratatui-image`,
  `tui-scrollview`, `tui-tree-widget`, `tui-logger`).
- **Textual**: the Textualize company wound down in 2025; the open-source
  project continues to ship releases.
- **Bubble Tea v2** moved to a declarative `View` that carries window title,
  cursor shape, and progress state. **Lip Gloss v2** added layers and
  hyperlinks; **Huh v2** has an accessible mode.
- **OpenTUI** (`anomalyco/opentui`) is the newest serious entrant, with a Zig
  rendering core and React/Solid bindings.

---

## Where SLT leads

Calibrated: "leads" means SLT ships it in-crate with less glue, not that nobody
else has it.

| Strength | SLT (verified in `src/`) | Comparison |
|---|---|---|
| Interaction on any container | `Response { clicked, right_clicked, double_clicked, hovered, scroll_delta, rect, .. }` from rows, columns, and widgets | ratatui: manual `Rect` hit-testing. Textual/Ink/OpenTUI have component events; Ink has no mouse API |
| Focus management | Tab/Shift+Tab focus manager, named focus, modal focus trap (on by default, `ModalOpts::tab_trap`), follow-focus scroll (`ScrollState::follow_focus`, v0.25) | ratatui: none built-in. Textual has a full focus system |
| Scrolling | `scrollable` containers, `scrollbar` with click-to-jump and thumb drag, `virtual_list`, `scrollable_with_gutter` | ratatui: `tui-scrollview` (ecosystem). Ink 8: `contentOffset` scroll views. OpenTUI: `ScrollBox` |
| Forms | `FormState`/`FormField` with sync validators and async validation, `FormFieldResponse` (v0.25) | ratatui: app-built. Bubble Tea: Huh |
| Built-in catalog | accordion, tree, draggable `split_pane`, command palette, table with sort/filter/paging, file picker, calendar, toasts | ratatui: ~15 core widgets plus crates |
| Editing | `TextareaState` undo/redo (Ctrl+Z / Ctrl+Y) | `ratatui-textarea`, Textual `TextArea`, OpenTUI `Textarea` also have editing |
| Devtools and testing | F12 layout inspector, `TestBackend`, `EventBuilder`, `snapshot()` / `assert_snapshot_eq` | Textual: devtools + Pilot + SVG snapshots (stronger). OpenTUI: snapshot renderer |
| Animation | `Tween`, `Spring`, `Keyframes`, `Sequence`, `Stagger` | Not unique: tachyonfx (ratatui), Textual animations, OpenTUI `Timeline`, FTXUI |
| Images | 4 protocols: half-block, Kitty, Sixel, iTerm2 (`src/halfblock.rs`, `src/sixel.rs`, `src/iterm.rs`) | ratatui: `ratatui-image` (ecosystem) |
| Text | UAX #9 bidi on by default (`bidi` default feature); tree-sitter highlighted code blocks (15 opt-in grammars) | Textual TextArea and OpenTUI `Code` also use tree-sitter |
| Run modes | full-screen, `run_inline`, `run_static`, `run_async` (tokio), WASM browser backend | ratatui inline viewport with `insert_before`; Ink `<Static>` |
| Resilience | `error_boundary` rolls back a panicking subtree, including focus metadata (v0.25) | Ink: React error boundaries |
| Theming | 11 color presets, `ThemeBuilder`, semantic `ThemeColor`, per-subtree `.theme()`, breakpoints (xs–xl) | Textual TCSS is more expressive; ratatui has no theme concept |
| AI-agent UI pieces | `streaming_text`, `streaming_markdown`, `tool_approval`, `context_bar` | OpenTUI ships `Markdown`, `Code`, `Diff` for the same space; Ink powers the largest agent CLIs |

---

## Where SLT lags

### Adoption and ecosystem (biggest gap)

| Metric (2026-10) | SLT | ratatui |
|---|---|---|
| GitHub stars | 193 | 23.0K |
| crates.io total downloads | 4,993 | 60.96M |
| Reverse dependencies | 5 | 6,469 |

With 5 reverse dependencies there is effectively no third-party SLT widget
ecosystem yet. Every competitor here has a flagship app or a large dependent
base that SLT does not.

### Missing features (verified absent in `src/`)

| Feature | Who has it |
|---|---|
| Text selection inside text inputs | Textual (cross-container selection), FTXUI 7 (selection) |
| Syntax-highlighted **editable** textarea | Textual `TextArea` (tree-sitter), OpenTUI `Textarea` + `Code` |
| Anchored popover / context menu / menubar | n/v per framework; SLT only has `tooltip` and anchored overlays |
| Diff viewer | OpenTUI `Diff` |
| Hover/focus/active state styles on containers | Textual TCSS `:hover`, `:focus`, `:focus-within`. SLT has `Response.hovered` and `group_hover_*` only |
| Mouse cursor shape | Textual `pointer` rule, Bubble Tea v2 `View` cursor shape |
| Window title / progress (OSC 9;4) / status APIs | Bubble Tea v2 declarative `View` (title, progress) |
| SSH serving | Bubble Tea via Wish, OpenTUI `@opentui/ssh` |
| Live style reload of layout/styling code | Textual `--dev` live CSS. SLT only hot-reloads TOML themes (`theme-watch`) |

### Structural

| Gap | Impact | Reference |
|---|---|---|
| No declarative style sheets | Style reuse is code-level only | Textual TCSS |
| No event bubbling | Composite widgets forward events manually | Textual message propagation, Ink/OpenTUI React events |

---

## Feature matrix

Legend: ✅ built-in · 🧩 ecosystem crate/package · ◐ partial · ❌ absent ·
n/v not verified for this snapshot.

### Layout and interaction

| Feature | SLT | ratatui | Textual | Bubble Tea | Ink | OpenTUI |
|---|---|---|---|---|---|---|
| Flexbox | ✅ | ◐ constraints | ✅ | ◐ Lip Gloss | ✅ Yoga | ✅ Yoga |
| Grid | ✅ | n/v | ✅ | n/v | n/v | n/v |
| Absolute positioning | ◐ anchored overlays (`overlay_at_offset`, `modal_at_offset`) | n/v | ✅ dock/offset | ◐ Lip Gloss layers | n/v | n/v |
| Hover/focus state styles | ◐ `Response.hovered`, `group_hover_*` | ❌ | ✅ TCSS pseudo-classes | n/v | n/v | n/v |
| Built-in focus manager | ✅ incl. modal trap | ❌ | ✅ | n/v | ✅ `useFocus` | n/v |
| Scroll containers | ✅ + draggable scrollbar | 🧩 tui-scrollview | ✅ | 🧩 Bubbles viewport | ✅ v8 `contentOffset` | ✅ ScrollBox |
| Mouse API | ✅ | ✅ (raw events) | ✅ | ✅ | ❌ | n/v |
| Text selection in inputs | ❌ | n/v | ✅ | n/v | n/v | n/v |

### Widgets

| Feature | SLT | ratatui | Textual | Bubble Tea | Ink | OpenTUI |
|---|---|---|---|---|---|---|
| Editable textarea | ✅ (undo/redo, no highlighting) | 🧩 ratatui-textarea | ✅ tree-sitter | 🧩 Bubbles | n/v | ✅ |
| Highlighted code (display) | ✅ 15 grammars, opt-in | n/v | ✅ | n/v | n/v | ✅ Code |
| Markdown | ✅ incl. streaming | n/v | ✅ | 🧩 Glamour | n/v | ✅ |
| Diff viewer | ❌ | n/v | n/v | n/v | n/v | ✅ |
| Tree | ✅ | 🧩 tui-tree-widget | ✅ | n/v | n/v | n/v |
| Split pane (draggable) | ✅ | n/v | n/v | n/v | n/v | n/v |
| Forms + validation | ✅ sync + async | n/v | ✅ | 🧩 Huh | n/v | n/v |
| Images | ✅ half-block, Kitty, Sixel, iTerm2 | 🧩 ratatui-image | n/v | n/v | n/v | n/v |
| Animation | ✅ | 🧩 tachyonfx | ✅ | n/v | n/v | ✅ Timeline |

### Runtime, tooling, deployment

| Feature | SLT | ratatui | Textual | Bubble Tea | Ink | OpenTUI |
|---|---|---|---|---|---|---|
| Inline mode | ✅ `run_inline` | ✅ inline viewport + `insert_before` | n/v | ✅ | ✅ | n/v |
| Static/scrollback output | ✅ `run_static` | ✅ `insert_before` | n/v | n/v | ✅ `<Static>` | n/v |
| Browser/WASM | ✅ `slt-wasm` (see [WASM.md](WASM.md)) | 🧩 ratzilla | ✅ textual-serve (web, not SSH) | n/v | n/v | n/v |
| SSH serving | ❌ | n/v | ❌ (textual-serve is web) | 🧩 Wish | n/v | ✅ `@opentui/ssh` |
| Devtools / inspector | ✅ F12 overlay | n/v | ✅ devtools + `--dev` live CSS | n/v | n/v | n/v |
| Headless snapshot tests | ✅ `TestBackend` | ✅ `TestBackend` | ✅ Pilot + SVG snapshots | n/v | 🧩 ink-testing-library | ✅ snapshot renderer |
| Accessibility mode | ◐ `RunConfig::accessible` linear prompts (v0.26; no ARIA tree) | n/v | n/v | 🧩 Huh accessible mode | ✅ ARIA / screen reader | n/v |
| Background workers | ✅ `run_async` (tokio) | n/v (app-owned loop) | ✅ workers | ✅ commands | ✅ (JS async) | ✅ (JS async) |

### Terminal protocols

| Protocol | SLT | ratatui | Textual | Bubble Tea v2 | Ink 8 |
|---|---|---|---|---|---|
| Synchronized output | ✅ | ✅ | ✅ | ✅ default | ✅ since 6.7 |
| Kitty keyboard | ✅ | ✅ | n/v | ✅ | ✅ since 6.7 |
| OSC 8 hyperlinks | ✅ | ✅ | ✅ | ✅ Lip Gloss v2 | ✅ |
| OSC 52 clipboard | ✅ read + write | n/v | n/v | ✅ | n/v |
| OSC 11 color-scheme query | ✅ | n/v | n/v | ✅ | n/v |
| Window title / progress (OSC 9;4) | ❌ | n/v | n/v | ✅ declarative `View` | n/v |
| Mouse cursor shape | ❌ | n/v | ✅ `pointer` | ✅ | n/v |

---

## Other frameworks (2026-10)

| Framework | State |
|---|---|
| iocraft (Rust) | 0.9.3; React-like declarative components |
| tui-realm (Rust) | 4.1; component framework on top of ratatui |
| rxtui (Rust) | archived |
| Cursive (Rust) | 0.21.1; no release since 2024 |
| FTXUI (C++) | 7.0; added `Collapsible`, `ResizableSplit`, and text selection |

More: [awesome-tuis](https://github.com/rothgar/awesome-tuis).

---

## Performance and footprint

As of v0.25.0 SLT is slower per frame than ratatui 0.30 (2–3.5x on matched
workloads); optimization is tracked.

**Method**: release builds on an Apple M3 Pro, rustc 1.98.0, SLT at commit
`15fd63a` (v0.25.0) versus ratatui 0.30.2 + crossterm 0.29, both rendering into a
matched ANSI-sink harness (bytes are written to a sink instead of a terminal).
The harness lives outside this repository.

### Frame time (median, µs; lower is better)

| Workload | SLT | ratatui | SLT / ratatui |
|---|---|---|---|
| Dashboard, 200x60 | 215 | 107 | 2.0x |
| 10k-item list, `ui.list` | 1832 | 648 | 2.8x |
| 10k-item list, SLT `virtual_list` | 258 | — | — |
| Table 1000x6 | 1032 | 591 | 1.7x |
| Table 1000x6, SLT paged | 643 | — | — |
| 60 nested bordered boxes, 240x80 | 1255 | 358 | 3.5x |
| Idle frame | 219 | 108 | 2.0x |

Profiling puts the cost in buffer cell writes (grapheme normalization) and
flush row hashing, not in layout. For large collections, `virtual_list` and
paged tables close most of the gap; that is a workaround, not parity.

### Output bytes

SLT emits 20–40% fewer bytes than ratatui on a full redraw, and 16 vs 25 bytes
on an idle frame. This matters over slow links (SSH, tmux), but it does not
offset the CPU cost per frame.

### Build and dependencies

| Measurement | SLT | ratatui + crossterm |
|---|---|---|
| Unique normal dependencies | **26** | 67 |
| Stripped hello-world binary | 1,063,952 B (1039 KiB); 0.26: 1,164,360 B (#491) | 553,384 B (540 KiB) — SLT 1.92x larger (0.26: 2.10x; see PERFORMANCE.md "Binary size") |
| Cold `cargo build --release` (two runs) | ~11.0 s / 9.9 s | ~7.1 s / 6.2 s — SLT ~1.6x slower |

The SLT dependency count was re-run for this snapshot with
`cargo tree -p superlighttui -e normal --prefix none` (unique package/version
lines, excluding the root, default features `crossterm` + `bidi`).

SLT's "light" claim is about the dependency tree and the size of the public
grammar. Do not cite frame time, binary size, or build speed as SLT
advantages.

---

## Corrections to the v0.19.2 snapshot

The previous version of this document (2026-04-27, v0.19.2) contained claims
that are now false or were overstated:

| Old claim | Correction |
|---|---|
| "No other TUI framework has a dedicated animation system" | False: tachyonfx (ratatui), Textual animations, OpenTUI `Timeline`, FTXUI |
| Accessibility: "Nobody has this" | False: Ink has screen-reader/ARIA support since 6.2; Huh v2 has an accessible mode |
| Ink ❌ synchronized output / Kitty keyboard | False since Ink 6.7 |
| Bubble Tea ❌ OSC 8 | Lip Gloss v2 has hyperlinks |
| ratatui ❌ inline mode | False: ratatui has an inline viewport with `insert_before` |
| Textual ✅ SSH server | `textual-serve` serves to a web browser, not SSH |
| SLT image protocols: 3 | Now 4: half-block, Kitty, Sixel, iTerm2 |
| SLT absolute positioning ❌ | Partial: anchored overlays with `(dx, dy)` insets (`overlay_at_offset`, `modal_at_offset`) |
| SLT testing "most comprehensive among all TUI frameworks" | Overclaim. Textual's Pilot + SVG snapshots and devtools are at least as strong |
| "SLT is the only TUI framework with built-in AI workflow widgets" | Overclaim. OpenTUI ships Markdown/Code/Diff and powers opencode; Ink powers Claude Code and Gemini CLI |
| Dependency tree "26 vs 69" for ratatui + crossterm | 26 vs 67 in the 2026-10 measurement |
| Binary "~1.45x larger" (preliminary) | 1.92x larger in the matched hello-world measurement |
| Star counts (ratatui 19K, Textual 34K, Ink 14K, Bubble Tea 40K) | See the 2026-10 overview table |

---

## Candidate priorities

Derived from the gaps above. These are not commitments.

1. **Adoption**: real apps, third-party widget crates, and docs for people coming
   from ratatui and the web (the gap no feature closes).
2. **Editing**: selection inside text inputs, then a tree-sitter highlighted
   editable textarea.
3. **Overlays**: anchored popover, context menu, menubar.
4. **State styles**: hover/focus/active styling on containers without
   hand-written `Response.hovered` branches.
5. **Terminal integration**: window title, OSC 9;4 progress, cursor shape.
6. **Accessibility**: v0.26 ships a linear prompt mode modelled on Huh's
   accessible forms (`RunConfig::accessible`, `SLT_ACCESSIBLE=1`). Next: an
   ARIA-style role/label tree for arbitrary layouts (Ink-style), and recorded
   VoiceOver / Orca walkthroughs.
7. **SSH serving** through the `Backend` trait, as a companion crate.

---

## Sources

- ratatui 0.30 highlights: <https://ratatui.rs/highlights/v030/>
- Textual releases: <https://github.com/Textualize/textual/releases>
- Bubble Tea v2.0.0: <https://github.com/charmbracelet/bubbletea/releases/tag/v2.0.0>
- Bubble Tea v2.1.0: <https://github.com/charmbracelet/bubbletea/releases/tag/v2.1.0>
- Lip Gloss v2.0.0: <https://github.com/charmbracelet/lipgloss/releases/tag/v2.0.0>
- Huh v2.0.0: <https://github.com/charmbracelet/huh/releases/tag/v2.0.0>
- Ink releases: <https://github.com/vadimdemedes/ink/releases>
- OpenTUI: <https://github.com/anomalyco/opentui>
- FTXUI v7.0.0: <https://github.com/ArthurSonzogni/FTXUI/releases/tag/v7.0.0>
- awesome-tuis: <https://github.com/rothgar/awesome-tuis>
- Star counts and crates.io figures: GitHub and crates.io, read 2026-10.
- Performance figures: matched ANSI-sink benchmark harness (outside this repository), run 2026-10.
