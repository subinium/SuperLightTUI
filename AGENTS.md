# SuperLightTUI — Project Instructions

## Language
- Respond in Korean (한글) by default
- All code, commit messages, PR descriptions, and code comments in English

---

## STOP — Release Workflow Checklist (I Keep Forgetting This)

Every release goes through ALL of these steps. No exceptions. No "probably fine". No "CI will catch it".
Patch (x.y.Z), minor (x.Y.0), major (X.0.0) all follow the same flow — only the version bump differs.

### 1. Pre-CI (LOCAL, on release branch)
Run the full Core Gate AND Extended Gate locally BEFORE pushing. The branch must be green on your machine first.
- [ ] `cargo fmt -- --check`
- [ ] `cargo check --all-features`
- [ ] `cargo clippy --all-features -- -D warnings`
- [ ] `cargo clippy --all-features --all-targets -- -D warnings`
- [ ] `cargo test --all-features`
- [ ] `cargo check --examples --all-features`
- [ ] `typos`
- [ ] `bash scripts/api_audit.sh --strict`
- [ ] `cargo check -p superlighttui --no-default-features`
- [ ] `cargo check -p slt-wasm --target wasm32-unknown-unknown` (local only; not in CI while WASM is deferred)
- [ ] `cargo hack check -p superlighttui --each-feature --no-dev-deps`
- [ ] `cargo audit`
- [ ] `cargo deny check`

If ANY fails: fix first. Never push a known-red branch hoping CI will catch it.

### 2. Version bump + changelog
- [ ] Bump `Cargo.toml` version
- [ ] Update `CHANGELOG.md` — group all commits since last tag (Added / Changed / Fixed / Perf / Docs)
- [ ] Update any README / doc badges that show the version

### 3. Branch + commit + push
- [ ] Branch named `release/vX.Y.Z` (already created at start of work, verify)
- [ ] Single atomic commit titled `feat: vX.Y.Z — <short summary>` (or `fix:` for hotfix)
- [ ] `git push -u origin release/vX.Y.Z`

### 4. PR + wait for CI
- [ ] `gh pr create` with full release notes in body (closes #issue-numbers)
- [ ] **WAIT** for CI on the PR — `gh pr checks --watch` or `gh run list --branch release/vX.Y.Z`
- [ ] ALL required checks must be `completed success`. No merging yellow-X.

### 5. Merge PR
- [ ] Merge with **squash** — body becomes the squashed commit body
- [ ] `git checkout main && git pull`
- [ ] Verify `git log -1` shows the squashed commit as expected on main

### 6. Tag + push tag
- [ ] `git tag -a vX.Y.Z -m "vX.Y.Z"` on the merged main commit
- [ ] `git push --tags`
- [ ] **WAIT** for `release.yml` workflow — `gh run list --workflow=release.yml --limit 1`
- [ ] Workflow must be `completed success` before proceeding

### 7. Post-release verification
- [ ] `gh release view vX.Y.Z` shows the published release
- [ ] crates.io: `cargo search superlighttui` shows new version (may take a few minutes)
- [ ] docs.rs: `https://docs.rs/superlighttui/X.Y.Z/slt/` built successfully
- [ ] Smoke: `scripts/smoke_release.sh X.Y.Z` resolves the exact crates.io version and runs
- [ ] Homebrew tap (if applicable): formula `url` + `sha256` updated; `brew install --build-from-source` works

### 8. Only now announce
Only after steps 1–7 are all green. Not before.

### Red flags that mean STOP
- "Probably fine" → no, run the gate
- "Just a docs change" → still run `cargo check --examples --all-features` (examples import the library)
- "CI will catch it" → no, locals catch it first, CI is a last line, not a first
- "I'll tag now and fix if anything breaks" → no, no broken tags. A tag is a public artifact.
- "The fmt check is being pedantic" → run `cargo fmt` and move on
- "This test was already flaky" → diagnose root cause, do NOT mark `--skip`

---

## Pre-Release Quality Gate (MANDATORY — NO EXCEPTIONS)

Before ANY commit, PR, or release, run ALL checks below.
**If ANY check fails, DO NOT proceed. Fix first.**

### Core Gate (every commit)
```bash
# 1. Format check (THIS IS THE ONE YOU KEEP FORGETTING)
cargo fmt -- --check

# 2. Compilation
cargo check --all-features

# 3. Clippy (deny all warnings; lib, then tests / examples / benches)
cargo clippy --all-features -- -D warnings
cargo clippy --all-features --all-targets -- -D warnings

# 4. Full test suite
cargo test --all-features

# 5. Examples compile
cargo check --examples --all-features
```

If `cargo fmt -- --check` shows diffs, run `cargo fmt` to fix, then re-run all.

### Extended Gate (before PR or release)
These match CI jobs that block merge. Run after the core gate passes.
```bash
# 6. Typo check
typos

# 7. Public API/documentation contract
bash scripts/api_audit.sh --strict
python3 scripts/check_doc_examples.py   # compiles every maintained doc snippet, warnings denied

# 8. No-default-features (catches missing #[cfg] gates)
cargo check -p superlighttui --no-default-features

# 9. WASM target (local only; not in CI while WASM is deferred)
cargo check -p slt-wasm --target wasm32-unknown-unknown

# 10. Feature combinations (needs cargo-hack: cargo install cargo-hack)
cargo hack check -p superlighttui --each-feature --no-dev-deps

# 11. Dependency audit
cargo audit

# 12. Deny check (licenses, bans, sources)
cargo deny check
```

### CI Job Reference
| Job | Command | Blocks merge? |
|-----|---------|---------------|
| Format | `cargo fmt -- --check` | Yes |
| Check (stable) | `cargo check --all-features` | Yes |
| Check (MSRV 1.88) | `cargo check --features async,serde` (toolchain 1.88) | Yes |
| Clippy | `cargo clippy --all-features -- -D warnings` + `cargo clippy --all-features --all-targets -- -D warnings` | Yes |
| Test | `cargo test --all-features` | Yes |
| Typos | `typos` | Yes |
| API Audit | `bash scripts/api_audit.sh --strict` + `python3 scripts/check_doc_examples.py` | Yes |
| Check (no-default) | `cargo check -p superlighttui --no-default-features` | Yes |
| Check (WASM) | `cargo check -p slt-wasm --target wasm32-unknown-unknown` | No (removed from CI while WASM is deferred; run locally before touching `crates/slt-wasm`) |
| Feature Combinations | `cargo hack check --each-feature --no-dev-deps` | Yes |
| Audit | `cargo audit` | Yes |
| Deny Check | `cargo deny check {advisories,bans,licenses,sources}` | Yes (advisories soft) |
| Doc Coverage | `RUSTFLAGS="-Wmissing_docs" cargo check` | No (soft) |
| Semver Check | `cargo-semver-checks` | No (soft) |

### Pre-PR Additional Gate
Before creating a PR, wait for CI to pass on the pushed branch:
```bash
gh run list --branch $(git branch --show-current) --limit 1
# Must show "completed success" before merging
```

### Pre-Tag Gate
Before tagging a release:
```bash
# Verify the EXACT commit you're tagging passes CI
gh run list --commit $(git rev-parse HEAD) --limit 5
# ALL runs must be "completed success"
```

## Rust Conventions
- Module pattern: `filename.rs` + `filename/` (NOT `mod.rs`) — Rust 2018 style
- Visibility: `pub(super)` for cross-submodule access, `pub(crate)` for crate-wide
- `use super::*;` in submodules to import parent types
- When splitting files, watch for `#[derive]` and `#[cfg_attr]` attributes above type definitions — they MUST stay attached to the type, not get separated by the split boundary

## Architecture
```
src/
├── lib.rs                      # Crate root: docs, mod decls, public re-exports, Backend / AppState / frame()
├── runtime.rs                  # Private run-loop facade; entry points re-exported from lib.rs
├── runtime/
│   ├── config.rs               # RunConfig + builder methods
│   ├── frame_state.rs          # FrameState, FocusState, LayoutFeedbackState, DiagnosticsState, DebugLayer
│   ├── kernel.rs               # run_frame_kernel, run_frame, input_frame, run-loop event handling
│   ├── native.rs               # run / run_with / run_inline* / run_static*, event polling (`crossterm`)
│   ├── async_run.rs            # run_async*, AsyncSender, AsyncRunHandle (`async`)
│   ├── panic.rs                # Panic hook + recoverable-panic scopes
│   ├── suspend.rs              # Ctrl+Z suspend / resume (`crossterm`, unix)
│   └── tests.rs                # Run-loop tests
├── context.rs                  # Facade for core context types + widget impl modules
├── context/
│   ├── state.rs                # State<T>, Response
│   ├── bars.rs                 # BarDirection, Bar, BarChartConfig, BarGroup
│   ├── widget.rs               # Widget trait
│   ├── core.rs                 # Context core types + checkpoint / rollback state
│   ├── container.rs            # ContainerBuilder + CanvasContext
│   ├── runtime.rs              # Core Context methods (hooks, focus, notifications)
│   ├── helpers.rs              # Shared helper functions for widget impls, previous-frame geometry queries
│   ├── async_tasks.rs          # In-frame async task API (spawn/poll, TaskHandle) — `async` feature
│   ├── tests.rs, async_tasks_tests.rs, scheduler_tests.rs  # Context unit tests
│   ├── widgets.rs              # Widget-method facade: one `<family>` module per widget family (see Widget Families)
│   └── widgets/
│       ├── display.rs          # Display family facade
│       ├── display/
│       │   ├── text.rs         # text, link, timer_display, help_from_keymap, style chains, size/margin helpers
│       │   ├── status.rs       # breadcrumb, accordion, definition_list, divider_text, key_hint, stat_trend, empty_state
│       │   ├── status/
│       │   │   └── builders.rs # help, badge, stat (Help / Badge / Stat builders)
│       │   └── tests.rs        # Display widget tests
│       ├── layout.rs           # col/row/line/line_wrap, group, container, scrollable, scrollbar, bordered
│       ├── layout/
│       │   ├── separator.rs    # separator (Separator builder)
│       │   ├── grid.rs         # grid / grid_columns
│       │   ├── gutter.rs       # scrollable_with_gutter, GutterOpts
│       │   ├── split.rs        # split_pane / vsplit_pane
│       │   └── line_wrap_tests.rs
│       ├── overlay.rs          # Anchor, modal*, overlay*, tooltip
│       ├── overlay/
│       │   └── tooltip_wrap.rs # tooltip word wrapping
│       ├── commanding.rs       # screen stack, command_palette, key_chord
│       ├── text_input.rs       # text_input, form / form_field / form_submit
│       ├── text_input/
│       │   ├── textarea.rs     # textarea
│       │   ├── number_input.rs # number_input
│       │   └── tests.rs        # Text-input family tests
│       ├── selection.rs        # tabs, checkbox, toggle, select, radio, multi_select, color_picker
│       ├── selection/
│       │   └── button.rs       # button (Button builder: variant / colors / show)
│       ├── collections.rs      # Collections family facade
│       ├── collections/
│       │   ├── list.rs         # list, list_reorderable, virtual_list(_variable)
│       │   ├── table.rs        # table, paginator
│       │   ├── tree.rs         # tree, directory_tree
│       │   ├── calendar.rs     # calendar
│       │   └── file_picker.rs  # file_picker
│       ├── feedback.rs         # spinner, toast, alert, confirm
│       ├── feedback/
│       │   ├── slider.rs       # slider (Slider builder)
│       │   ├── progress.rs     # progress / progress_bar (ProgressBar builder)
│       │   ├── gauge.rs        # gauge / line_gauge builders
│       │   └── tests.rs        # Feedback family tests
│       ├── rich.rs             # Rich-content family facade
│       ├── rich/
│       │   ├── rich_log.rs     # rich_log
│       │   ├── markdown.rs     # markdown and inline-markdown helpers
│       │   ├── streaming.rs    # streaming_text, streaming_markdown, tool_approval, context_bar
│       │   ├── code_block.rs   # code_block builder
│       │   ├── fallback_highlight.rs # keyword fallback highlighter, tree-sitter line renderer
│       │   └── tests.rs        # Rich-content family tests
│       ├── media.rs            # big_text, image, kitty_image, sixel_image, iterm_image
│       ├── media/
│       │   ├── prepare.rs      # image preparation caches, encoders, half-block / RGBA fallback
│       │   ├── font8x8.rs      # 8x8 bitmap font for big_text
│       │   └── tests.rs        # Media family tests
│       ├── viz.rs              # Viz family facade + shared viz-root helpers
│       ├── viz/
│       │   ├── bars.rs         # bar_chart, grouped and stacked bars
│       │   ├── charts.rs       # sparkline, candlestick(_hd), chart, scatter, histogram
│       │   ├── line_chart.rs   # line_chart / area_chart (LineChart builder)

│       │   ├── heat_tree.rs    # heatmap, heatmap_halfblock, treemap, TreemapItem
│       │   ├── canvas.rs       # canvas / try_canvas / canvas_with
│       │   ├── qr.rs           # qr_code
│       │   └── tests.rs        # Viz family tests
│       └── events.rs           # keyboard, mouse, paste, scroll, theme, size, tick, quit, debug helpers
│
├── widgets.rs                  # Facade: private family submodules below, glob re-exported (`pub use text_input::*;` …)
│                               # so every state type stays at `slt::widgets::*`; `validators` is `pub mod`
├── widgets/
│   ├── display.rs              # Trend, BreadcrumbResponse
│   ├── layout.rs               # ScrollState, HighlightRange, SplitPaneState, GridColumn, SplitPaneResponse, GutterResponse
│   ├── commanding.rs           # ChordState, CommandPaletteState, PaletteCommand, ScreenState, ModeState
│   ├── text_input.rs           # TextInputState, Validator, AsyncValidation, ValidateTrigger, FormField, FormState, FormFieldResponse, TextareaState, NumberInputState
│   ├── selection.rs            # SelectState, RadioState, MultiSelectState, TabsState, ButtonVariant, PickerMode, ColorPickerState
│   ├── collections.rs          # ListState, ListResponse, FilePickerState, FileEntry, TableState, TableColumn, PaginatorState, TreeState, TreeNode, DirectoryTreeState, CalendarState, CalDate
│   ├── feedback.rs             # ToastState, ToastMessage, ToastLevel, AlertLevel, SpinnerState, SpinnerPreset, SliderOpts, GaugeResponse
│   ├── rich.rs                 # RichLogState, RichLogEntry, StreamingTextState, StreamingMarkdownState, ToolApprovalState, ApprovalAction, ContextItem
│   ├── runtime.rs              # Non-widget runtime state: StaticOutput, SchedulerState
│   ├── validators.rs           # Built-in FormField validators (`slt::validators`)
│   └── tests.rs                # Widget state tests (`mod tests`)
│
├── layout.rs                   # Thin facade re-exporting layout kernels
├── layout/
│   ├── command.rs              # Command enum recorded by Context
│   ├── tree.rs                 # LayoutNode, NodeKind, build_tree(), wrap helpers
│   ├── collect.rs              # collect_all(), FrameData, raw-draw collection helpers
│   ├── flexbox.rs              # compute(), layout_row(), layout_column(), gap/grow/shrink resolution
│   ├── focus.rs                # Layout clip, focus/scroll geometry feedback, follow-focus scroll adjustments
│   ├── render.rs               # render(), render_inner(), render_border(), clipping, viewport culling
│   └── tests.rs                # Layout-focused kernel tests
│
├── style.rs                    # Style, Border, Padding, Margin, Constraints, Modifiers, Align, Justify
├── style/
│   ├── color.rs                # Color enum (Named, Indexed, Rgb), ColorDepth, color blending
│   ├── theme.rs                # Theme struct, Spacing, ThemeColor, 11 color presets + 3 density variants, ThemeBuilder, contrast helpers
│   └── theme_io.rs             # TOML ThemeFile loading (`serde`) and ThemeWatcher hot reload (`theme-watch`)
│
├── terminal.rs                 # Terminal backend facade: buffer helpers, Sink, Terminal, InlineTerminal
│   ├── Terminal                # Full-screen mode
│   ├── InlineTerminal          # Inline mode
│   └── submodule re-exports    # Capabilities, ColorScheme, clipboard, bench hooks
├── terminal/
│   ├── kitty.rs                # KittyImageManager: upload dedup, placement diff, compression
│   ├── cell_pixel.rs           # CSI 16 t cell-pixel probe + cache
│   ├── capabilities.rs         # Capabilities, BlitterSupport, Blitter, host / multiplexer / env policy
│   ├── query.rs                # Reply reader, DA1/DA2/DECRPM/XTGETTCAP parsing, sync-output resolution
│   ├── session.rs              # Session registry/guard, enter/exit/cleanup writers, panic cleanup, suspend/resume
│   ├── osc.rs                  # OSC 11 color scheme, OSC 52 clipboard, base64
│   ├── flush.rs                # flush_buffer_diff*, cursor, SGR style delta
│   ├── sprixel.rs              # Graphics emission policy, raw-sequence + sprixel flush
│   ├── bench_hooks.rs          # `#[doc(hidden)]` __bench_* fixtures
│   ├── selection.rs            # SelectionState, text selection overlay rendering
│   ├── tests.rs                # Test facade
│   └── tests/                  # capability / detection / flush / graphics / lifecycle / query / selection tests
│
├── anim.rs                     # Tween, Spring, Keyframes, Sequence, Stagger
├── chart.rs                    # ChartBuilder, ChartConfig, Dataset, Marker
├── chart/
│   ├── render.rs               # Chart rendering, histogram
│   ├── axis.rs                 # Axis struct, label formatting
│   ├── bar.rs                  # Bar chart rendering
│   ├── grid.rs                 # Grid lines
│   └── braille.rs              # Braille dot patterns for line/scatter charts
│
├── buffer.rs                   # Double-buffer with clip stack and diff tracking
├── syntax.rs                   # tree-sitter syntax highlighting
├── sixel.rs                    # Sixel image protocol support
├── iterm.rs                    # iTerm2 OSC 1337 inline-image encoder
├── clock.rs                    # Platform clocks for frame timing, input and scheduling (native + wasm)
├── cell.rs                     # Cell = one grapheme (CompactString symbol) + Style + optional OSC 8 hyperlink
├── rect.rs                     # Rect struct, bounds checking, intersection
├── event.rs                    # Event, KeyCode, KeyModifiers, MouseEvent, MouseButton
├── event/
│   ├── reply.rs                # Terminal query replies; preserves input consumed during probes (`crossterm`)
│   ├── native.rs               # Owned nonblocking Unix input reader (`crossterm`, unix)
│   ├── native/parse.rs         # Escape-sequence parser adapted from Crossterm 0.28.1 (unchanged in 0.29)
│   └── native/lifecycle_tests.rs, reply/review_tests.rs  # Input tests
├── halfblock.rs                # Half-block image rendering
├── keymap.rs                   # KeyMap, Binding structs
├── palette.rs                  # 256-color palette definitions
└── test_utils.rs               # TestBackend, EventBuilder for headless testing
```

## Documentation Map
```
docs/
├── README.md            # Index — must link every maintained page (tests/docs_contract.rs)
├── guide/NN-*.md        # Learning path, read in order: quick start → state (canonical
│                        #   previous-frame rule) → patterns → interaction → overlays →
│                        #   theming → animation → testing → runtime modes → browser →
│                        #   terminal compatibility
├── coming-from/         # web.md, ratatui.md
├── reference/           # widgets, state, cookbook, examples, features, migration,
│                        #   debugging, ai-guide, complete-reference (condensed AI map)
├── internals/           # design-principles (API rules + naming), architecture,
│                        #   custom-backends, performance, rustdoc-guide, demo-guide
├── archive/             # Dated snapshots, not maintained
├── README.{ko,ja,es,zh-CN}.md  # Translated summaries; English is authoritative
└── <OLD_NAME>.md        # One-line "This page moved to ..." stubs for pre-0.26 paths
```
- Each concept has one canonical page; others link to it. The previous-frame rule lives in
  `docs/guide/02-state.md#the-previous-frame-rule`.
- Every ```` ```rust ```` fence in README.md, docs/ and the skills is either compiled
  (`<!-- slt-check: {"name": ..., "features": [...], "context": "module"|"ui"} -->`, built with
  `#![deny(warnings)]`) or marked `<!-- slt-check-skip: <reason> -->`. Only `docs/internals/`
  and `docs/archive/` may use the file-level illustrative default. Run
  `python3 scripts/check_doc_examples.py` after any doc or API change.
- `.agents/skills/` and `.claude/skills/` must stay byte-identical.

## Commit Style
- Conventional Commits: `feat:`, `fix:`, `refactor:`, `perf:`, `test:`, `docs:`, `chore:`
- Release commits: `feat: vX.Y.Z — short summary`
- Hotfix: `fix: description` (no version in message)

## Release Process
1. Run ALL 5 quality gates above
2. Bump version in Cargo.toml
3. Update CHANGELOG.md
4. Branch: `release/vX.Y.Z`
5. Commit + push branch
6. **Wait for CI to pass on the branch** (`gh pr checks`)
7. Create PR + merge (squash)
8. Pull main, tag, push tag
9. **Wait for Release workflow to succeed** before announcing
10. Create GitHub release with structured notes

## Testing
- TestBackend for headless rendering: `TestBackend::new(w, h).render(|ui| { ... })`
- `tb.assert_contains("text")` for content verification
- draw_raw tests must verify clipping, constraints, and multi-region rendering
- VHS gallery job green in CI (regenerates the demo gallery from the `.tape` files; replaces the old manual tmux step)

## Key Patterns
- `ContainerBuilder::draw()` requires `'static` closure (deferred execution)
- `collect_all()` folds 7 former collect-phase walks into one DFS; a frame still runs build → layout → collect → render (four top-level passes), plus a second `collect_all` only when follow-focus scrolling moved a viewport
- `apply_style_delta()` for flush — only emit changed attributes
- `FrameData` struct holds all per-frame collected data
- Keyframes stops pre-sorted at build time, not per-frame
