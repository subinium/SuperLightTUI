# Documentation Guide

The root [README](../README.md) shows the shape of the library, this index
tells you where to go next, and [docs.rs](https://docs.rs/superlighttui) is the
source of truth for API signatures.

The docs are split into four parts:

- **[Learning path](#learning-path)** — read in order; each guide assumes the ones before it.
- **[Coming from](#coming-from)** — maps from another stack you already know.
- **[Reference](#reference)** — look things up while you build.
- **[Internals](#internals)** — for contributors to the library itself.

## Learning path

1. [Quick start](guide/01-quick-start.md) — install, first app, counter, layout, input, return patterns
2. [State and the previous frame](guide/02-state.md) — app state vs hooks vs widget state, `use_memo`, and the [previous-frame rule](guide/02-state.md#the-previous-frame-rule)
3. [Patterns](guide/03-patterns.md) — components, helpers, forms, screens, modals, async messages, custom widgets
4. [Interaction](guide/04-interaction.md) — mouse (on by default, `MouseMode`), keyboard, focus, shortcuts, hover, drag, wheel and scrolling, menus
5. [Overlays and positioning](guide/05-overlays.md) — `Anchor`, `overlay_at`, tooltips; CSS/Flutter/React Native positioning mapped to SLT
6. [Theming](guide/06-theming.md) — theme struct, presets, `ThemeBuilder`, custom themes
7. [Animation](guide/07-animation.md) — tween, spring, keyframes, sequence, stagger
8. [Testing](guide/08-testing.md) — `TestBackend`, `EventBuilder`, input simulation, snapshots
9. [Runtime modes](guide/09-runtime-modes.md) — `run` / `run_inline` / `run_static`, `RunConfig`, built-in terminal guarantees
10. [Browser runtime](guide/10-browser.md) — the `slt-wasm` backend: mounting, input, lifecycle, capability boundary
11. [Terminal compatibility](guide/11-terminal-compatibility.md) — terminal emulator, multiplexer, SSH and protocol support

## Coming from

- [The web](coming-from/web.md) — HTML/CSS, React and DOM concepts mapped to SLT
- [ratatui](coming-from/ratatui.md) — widgets, layout, state and the incremental `ratatui` bridge

## Reference

- [Widgets](reference/widgets.md) — categorized widget catalog, runtime methods and state types
- [Widget state](reference/state.md) — public state fields, accessors, and mutation APIs
- [Cookbook](reference/cookbook.md) — copy-paste recipes for common TUI apps
- [Examples](reference/examples.md) — runnable example index
- [Feature flags](reference/features.md) — feature flags and runtime capability matrix
- [Migration](reference/migration.md) — version-by-version upgrade notes and replacements
- [Debugging](reference/debugging.md) — F12 overlay, clipping, focus and modal debugging, hot reload
- [AI guide](reference/ai-guide.md) — fastest path for AI-assisted builders and coding agents
- [Complete reference](reference/complete-reference.md) — condensed single-file API map for coding agents
- [CHANGELOG](../CHANGELOG.md) — release history

## Internals

- [Design principles](internals/design-principles.md) — API rules, review checklist, naming conventions
- [Architecture](internals/architecture.md) — module map and frame lifecycle
- [Custom backends](internals/custom-backends.md) — `Backend`, `AppState`, driving `frame()` yourself
- [Performance](internals/performance.md) — frame budgets, benchmarks, and regression checks
- [Rustdoc guide](internals/rustdoc-guide.md) — public documentation standards
- [Demo guide](internals/demo-guide.md) — demo authoring and gallery workflow

## Archive

Dated snapshots kept for history; they are not updated.

- [Competitive analysis (v0.25.0)](archive/competitive-analysis-v0.25.0.md) — 2026-10-18 snapshot against ratatui, Textual, Bubble Tea, Ink, OpenTUI and others

## Translations

Summaries of the root README; the English guides above are authoritative.

- [한국어](README.ko.md) · [日本語](README.ja.md) · [Español](README.es.md) · [中文](README.zh-CN.md)

## Old paths

Pages that lived directly in `docs/` before v0.26 (for example `docs/QUICK_START.md`)
are one-line stubs that link to their new location, so published links keep working.
