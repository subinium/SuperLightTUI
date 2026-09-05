# Documentation Guide

Use this directory by depth, not by file-name guessing.
The idea is simple:

- the root `README.md` sells the shape of the library
- this directory tells you where to go next
- docs.rs is the API signature source of truth

## Start here

- [QUICK_START.md](QUICK_START.md) - install, first app, counter, layout, input
- [WIDGETS.md](WIDGETS.md) - categorized widget map
- [PATTERNS.md](PATTERNS.md) - common composition patterns
- [EXAMPLES.md](EXAMPLES.md) - runnable example index

## Recommended paths

### I want to build an app quickly

1. [QUICK_START.md](QUICK_START.md)
2. [WIDGETS.md](WIDGETS.md)
3. [PATTERNS.md](PATTERNS.md)
4. [EXAMPLES.md](EXAMPLES.md)

### I want to trust the runtime and backend path

1. [BACKENDS.md](BACKENDS.md)
2. [TESTING.md](TESTING.md)
3. [DEBUGGING.md](DEBUGGING.md)
4. [ARCHITECTURE.md](ARCHITECTURE.md)

### I want to contribute to the library itself

1. [Design Principles](DESIGN_PRINCIPLES.md)
2. [ARCHITECTURE.md](ARCHITECTURE.md)
3. [TESTING.md](TESTING.md)
4. [CHANGELOG.md](../CHANGELOG.md)

## By task

- [AI_GUIDE.md](AI_GUIDE.md) - fastest path for AI-assisted builders and coding agents
- [ANIMATION.md](ANIMATION.md) - tween, spring, keyframes, sequence, stagger
- [BACKENDS.md](BACKENDS.md) - custom backends, inline mode, static output, `frame()`
- [COOKBOOK.md](COOKBOOK.md) - copy-paste recipes for five common TUI apps
- [COMPLETE_REFERENCE.md](COMPLETE_REFERENCE.md) - broad single-file API reference
- [DEBUGGING.md](DEBUGGING.md) - F12 overlay, clipping, focus, previous-frame behavior
- [FEATURES.md](FEATURES.md) - feature flags and runtime capability matrix
- [MIGRATION.md](MIGRATION.md) - version-by-version upgrade notes and replacements
- [PREVIOUS_FRAME_GUIDE.md](PREVIOUS_FRAME_GUIDE.md) - response geometry and previous-frame timing
- [STATE_APIS.md](STATE_APIS.md) - public state fields, accessors, and mutation APIs
- [TESTING.md](TESTING.md) - `TestBackend`, `EventBuilder`, input simulation, snapshots
- [THEMING.md](THEMING.md) - theme struct, presets, ThemeBuilder, custom themes
- [WASM.md](WASM.md) - browser runtime, input, embedding, and publication boundaries
- [CHANGELOG.md](../CHANGELOG.md) - release history and migration notes

## Contributor docs

- [Design Principles](DESIGN_PRINCIPLES.md) - API rules and design constraints
- [API_DESIGN.md](API_DESIGN.md) - widget/API review rules
- [ARCHITECTURE.md](ARCHITECTURE.md) - module map and frame lifecycle
- [DEMO_GUIDE.md](DEMO_GUIDE.md) - demo authoring and gallery workflow
- [NAMING.md](NAMING.md) - public API naming conventions
- [PERFORMANCE.md](PERFORMANCE.md) - frame budgets, benchmarks, and regression checks
- [POSITIONING.md](POSITIONING.md) - product scope and audience
- [RUSTDOC_GUIDE.md](RUSTDOC_GUIDE.md) - public documentation standards

## Historical snapshots

- [COMPETITIVE_ANALYSIS.md](COMPETITIVE_ANALYSIS.md) - v0.19.2 competitive and roadmap snapshot; refreshed dependency counts are labeled in place

## Translations

- [README.ko.md](README.ko.md)
- [README.ja.md](README.ja.md)
- [README.es.md](README.es.md)
- [README.zh-CN.md](README.zh-CN.md)

The root `README.md` is the landing page.
This directory holds the deeper guides.
Use `docs.rs` for API signatures; use the files here for architecture, patterns, examples, and contributor-facing contracts.
