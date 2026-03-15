# SuperLightTUI — Project Instructions

## Language
- Respond in Korean (한글) by default
- All code, commit messages, PR descriptions, and code comments in English

## Pre-Release Quality Gate (MANDATORY — NO EXCEPTIONS)

Before ANY commit, PR, or release, run ALL 5 checks in this exact order.
**If ANY check fails, DO NOT proceed. Fix first.**

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

If `cargo fmt -- --check` shows diffs, run `cargo fmt` to fix, then re-run all 5.

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
├── lib.rs              # Entry points: run(), run_with(), RunConfig (940 lines)
├── context.rs          # Context struct, ContainerBuilder, core methods (2163 lines)
├── context/
│   ├── widgets_display.rs  # impl Context: text, styled, link, button, tabs, etc.
│   ├── widgets_input.rs    # impl Context: text_input, textarea, select, etc.
│   └── widgets_viz.rs      # impl Context: bar_chart, chart, canvas, etc.
├── layout.rs           # Command enum, LayoutNode, build_tree, collect_all (1411 lines)
├── layout/
│   ├── flexbox.rs          # compute(), layout_row(), layout_column()
│   └── render.rs           # render(), render_inner(), render_border()
├── terminal.rs         # Terminal, InlineTerminal, flush (880 lines)
├── terminal/
│   └── selection.rs        # SelectionState, selection overlay
├── style.rs            # Style, Color, Theme, Border, Padding, Margin (1429 lines)
├── widgets.rs          # State types: TextInputState, TableState, etc.
├── anim.rs             # Tween, Spring, Keyframes, Sequence, Stagger
├── chart.rs            # ChartBuilder, histogram
├── buffer.rs           # Double-buffer with clip stack
├── cell.rs, rect.rs, event.rs, halfblock.rs, test_utils.rs
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
- tmux verification for visual demos before release

## Key Patterns
- `ContainerBuilder::draw()` requires `'static` closure (deferred execution)
- `collect_all()` replaces 7 separate tree traversals — single DFS pass
- `apply_style_delta()` for flush — only emit changed attributes
- `FrameData` struct holds all per-frame collected data
- Keyframes stops pre-sorted at build time, not per-frame
