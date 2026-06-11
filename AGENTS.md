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
- [ ] `cargo test --all-features`
- [ ] `cargo check --examples --all-features`
- [ ] `typos`
- [ ] `cargo check -p superlighttui --no-default-features`
- [ ] `cargo check -p slt-wasm --target wasm32-unknown-unknown`
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
- [ ] `gh release view vX.Y.Z` shows artifacts
- [ ] crates.io: `cargo search superlighttui` shows new version (may take a few minutes)
- [ ] docs.rs: `https://docs.rs/superlighttui/X.Y.Z` built successfully
- [ ] Smoke: `cargo install superlighttui --version X.Y.Z` succeeds; run a trivial example
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

# 3. Clippy (deny all warnings)
cargo clippy --all-features -- -D warnings

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

# 7. No-default-features (catches missing #[cfg] gates)
cargo check -p superlighttui --no-default-features

# 8. WASM target
cargo check -p slt-wasm --target wasm32-unknown-unknown

# 9. Feature combinations (needs cargo-hack: cargo install cargo-hack)
cargo hack check -p superlighttui --each-feature --no-dev-deps

# 10. Dependency audit
cargo audit

# 11. Deny check (licenses, bans, sources)
cargo deny check
```

### CI Job Reference
| Job | Command | Blocks merge? |
|-----|---------|---------------|
| Format | `cargo fmt -- --check` | Yes |
| Check (stable) | `cargo check --all-features` | Yes |
| Check (MSRV 1.88) | `cargo check --features async,serde` (toolchain 1.88) | Yes |
| Clippy | `cargo clippy --all-features -- -D warnings` | Yes |
| Test | `cargo test --all-features` | Yes |
| Typos | `typos` | Yes |
| Check (no-default) | `cargo check -p superlighttui --no-default-features` | Yes |
| Check (WASM) | `cargo check -p slt-wasm --target wasm32-unknown-unknown` | Yes |
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
├── lib.rs                      # Crate root, public re-exports, run()/frame() entry points
├── context.rs                  # Facade for core context types + widget impl modules
├── context/
│   ├── state.rs                # State<T>, Response
│   ├── bars.rs                 # BarDirection, Bar, BarChartConfig, BarGroup
│   ├── widget.rs               # Widget trait
│   ├── core.rs                 # Context core types + checkpoint / rollback state
│   ├── container.rs            # ContainerBuilder + CanvasContext
│   ├── runtime.rs              # Core Context methods (hooks, focus, notifications)
│   ├── helpers.rs              # Shared helper functions for widget impls
│   ├── widgets_display.rs      # Display/layout facade
│   ├── widgets_display/
│   │   ├── text.rs             # text, style chains, size/margin helpers
│   │   ├── rich_output.rs      # big_text, image, streaming, tool approval, context bar
│   │   ├── status.rs           # alert, breadcrumb, badge, stat, code_block, empty_state
│   │   └── layout.rs           # screen, row/col, modal, tooltip, container, scrollable, form helpers
│   ├── widgets_interactive.rs  # Interactive facade
│   ├── widgets_interactive/
│   │   ├── collections.rs      # grid, list, calendar, file picker
│   │   ├── selection.rs        # table, tabs, button, checkbox, toggle, select, radio, multi_select
│   │   ├── rich_markdown.rs    # rich_log, virtual_list, command palette, markdown, key_seq
│   │   ├── events.rs           # keyboard, mouse, theme, size, quit helpers
│   │   └── tree_widgets.rs     # tree widget internals
│   ├── widgets_input.rs        # Input facade
│   ├── widgets_input/
│   │   ├── text_input.rs       # text input widget
│   │   ├── feedback.rs         # spinner, toast, slider
│   │   └── textarea_progress.rs # textarea and progress widgets
│   └── widgets_viz.rs          # charts, sparklines, heatmap, treemap, candlestick, stacked bar, canvas, QR
│
├── widgets.rs                  # Facade for widget state types
├── widgets/
│   ├── input.rs                # StaticOutput, TextInputState, FormField, FormState, TextareaState, SpinnerState
│   ├── collections.rs          # ListState, FilePickerState, TabsState, TableState, ScrollState
│   ├── feedback.rs             # RichLogState, CalendarState, Toast types, ButtonVariant, Trend
│   ├── selection.rs            # SelectState, RadioState, MultiSelectState, TreeState, DirectoryTreeState, PaletteCommand
│   └── commanding.rs           # CommandPaletteState, streaming states, ScreenState, ModeState, tool approval types, ContextItem
│
├── layout.rs                   # Thin facade re-exporting layout kernels
├── layout/
│   ├── command.rs              # Command enum recorded by Context
│   ├── tree.rs                 # LayoutNode, NodeKind, build_tree(), wrap helpers
│   ├── collect.rs              # collect_all(), FrameData, raw-draw collection helpers
│   ├── flexbox.rs              # compute(), layout_row(), layout_column(), gap/grow/shrink resolution
│   ├── render.rs               # render(), render_inner(), render_border(), clipping, viewport culling
│   └── tests.rs                # Layout-focused kernel tests
│
├── style.rs                    # Style, Border, Padding, Margin, Constraints, Modifiers, Align, Justify
├── style/
│   ├── color.rs                # Color enum (Named, Indexed, Rgb), ColorDepth, color blending
│   └── theme.rs                # Theme struct, Spacing, ThemeColor, 10 presets, ThemeBuilder, contrast helpers
│
├── terminal.rs                 # Terminal backend
│   ├── Terminal                # Full-screen mode
│   ├── InlineTerminal          # Inline mode
│   └── ANSI diff output, synchronized output, event polling
├── terminal/
│   └── selection.rs            # SelectionState, text selection overlay rendering
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
├── cell.rs                     # Cell = char + Style + optional URL
├── rect.rs                     # Rect struct, bounds checking, intersection
├── event.rs                    # Event, KeyCode, KeyModifiers, MouseEvent, MouseButton
├── halfblock.rs                # Half-block image rendering
├── keymap.rs                   # KeyMap, Binding structs
├── palette.rs                  # 256-color palette definitions
└── test_utils.rs               # TestBackend, EventBuilder for headless testing
```

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
- `collect_all()` replaces 7 separate tree traversals — single DFS pass
- `apply_style_delta()` for flush — only emit changed attributes
- `FrameData` struct holds all per-frame collected data
- Keyframes stops pre-sorted at build time, not per-frame
