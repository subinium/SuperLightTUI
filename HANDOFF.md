# Handoff — SuperLightTUI v0.13.1 (2026-03-19)

## Current Status

- Project: `superfastlighttui`
- Latest: v0.13.1 (releasing now)
- Previous: v0.13.0 (API consistency), v0.12.13 (docs.rs fix), v0.12.12 (code improvements), v0.12.11 (governance)

## Next Version: v0.14.0 (Features)

Prioritized feature additions with demos. All items researched and specced.

### P1 — Should Have

| ID | Feature | Description | Depends On |
|----|---------|-------------|------------|
| P1-1 | Tooltip | Mouse hover popup via overlay | Response.hovered ✅ |
| P1-2 | Table colspan + zebra | Column spanning, striped rows, fixed headers | TableState ✅ |
| P1-3 | Fuzzy matching | command_palette fuzzy search | CommandPaletteState ✅ |
| P1-4 | Screens/routing | push_screen/pop_screen navigation stack | modal/overlay ✅ |

### P2 — Nice to Have

| ID | Feature | Description |
|----|---------|-------------|
| P2-1 | Calendar widget | Date picker via grid() |
| P2-2 | Static output | Fixed top + dynamic bottom (CLI tool pattern) |
| P2-3 | Benchmark suite | criterion benches, CI integration |
| P2-4 | cargo-generate template | Starter project |
| P2-5 | Sixel image | Non-Kitty terminal image support |
| P2-6 | Snapshot testing | insta-based visual regression |

### Demo Requirements
Each new feature must have a demo section in examples/demo.rs.

## Competitive Analysis Summary

SLT strengths vs ratatui/Ink/Textual:
- AI-native widgets (streaming_text, tool_approval, context_bar)
- Built-in focus system, animation, Kitty image, OSC 52 clipboard
- Zero-config entry (`slt::run(|ui| {})`)
- Error boundaries, panic recovery

Key gaps to close:
- Tooltip, Table enhancements, Fuzzy search, Screen routing
- Calendar, Static output, Sixel
- Benchmarks, templates, snapshot testing
- Target release: `v0.12.12`
- State: Code changes are prepared and previously passed quality gates; release execution is pending.
- Latest blocker reported by user: intermittent Cargo lock/doc check issue on local machine.

## What Was Completed

### Governance setup (v0.12.11) — completed and released

- Added governance docs and community templates:
  - `DESIGN_PRINCIPLES.md`
  - `ARCHITECTURE.md`
  - `SECURITY.md`
  - `.github/PULL_REQUEST_TEMPLATE.md`
  - `.github/CODEOWNERS`
  - `.github/ISSUE_TEMPLATE/bug_report.md`
  - `.github/ISSUE_TEMPLATE/feature_request.md`
- Updated lint/CI/contribution workflow:
  - `src/lib.rs` crate-level lint policies
  - `.github/workflows/ci.yml` docs + semver check jobs
  - `CONTRIBUTING.md` checklist improvements
  - `Cargo.toml` cleanup (`exclude` updates)

### Code improvements for v0.12.12 — implemented

- `E1`: Added `Default` support for 8 state types in `src/widgets.rs`.
- `E5`: Updated architecture section in `CLAUDE.md`.
- `E6`: Improved `use_memo` panic message with type info in `src/context.rs`.
- `M5`: Replaced repeated breakpoint methods with macro in `src/context.rs`.
- `M2`: Split long functions in:
  - `src/context/widgets_interactive.rs`
  - `src/context/widgets_viz.rs`
- `M6`: Added/trimmed doc comments in:
  - `src/style.rs`
  - `src/style/theme.rs`
  - `src/palette.rs`
- `M7`: Added tests across style/theme/widgets areas.
- Version/changelog staged for release:
  - `Cargo.toml` -> `0.12.12`
  - `CHANGELOG.md` updated

## Known Decisions and Notes

- Custom `SltError` introduction was intentionally dropped (over-engineering risk for current immediate-mode patterns).
- `#![warn(missing_docs)]` should not be hard-enforced in the blocking clippy gate (`-D warnings`); docs checks are better isolated as non-blocking CI docs job.
- `#![forbid(unsafe_code)]` policy was added.
- `doc_auto_cfg` behavior is docs.rs/nightly-context sensitive.

## Remaining Tasks (Do These Next)

1. Resolve local Cargo lock/doc check issue.
2. Re-run the mandatory 5 quality gates in order.
3. Create/update release branch and push.
4. Open PR and wait for CI green.
5. Merge PR, tag `v0.12.12`, push tag.
6. Confirm GitHub release workflow/crates.io publication.

## Exact Local Runbook

Run from repo root:

```bash
cd /Users/subinium/Desktop/github/superfastlighttui

# A) Clean potential stale lock/processes
pkill -f cargo || true
pkill -f rustc || true
pkill -f rustdoc || true
rm -f target/.rustc_info.json.lock

# B) Fresh build cache
cargo clean

# C) Re-check docs (for recent lock symptom)
cargo doc --all-features --no-deps

# D) Mandatory quality gate (project rule)
cargo fmt -- --check
cargo check --all-features
cargo clippy --all-features -- -D warnings
cargo test --all-features
cargo check --examples --all-features
```

If all pass, proceed:

```bash
# Verify release version/status
git status
grep '^version' Cargo.toml

# Branch/push/PR
git branch --show-current
git push -u origin HEAD
gh pr create

# CI gate before merge
gh run list --branch $(git branch --show-current) --limit 1
```

Expected CI status before merge: `completed success`.

## Open Item for Next Version Discussion

- `M4` (string allocation optimization) is still pending by choice/scope; decide whether to include in `v0.12.12` follow-up patch or defer to `v0.12.13`.

## Files Most Relevant Right Now

- `Cargo.toml`
- `CHANGELOG.md`
- `src/widgets.rs`
- `src/context.rs`
- `src/context/widgets_interactive.rs`
- `src/context/widgets_viz.rs`
- `src/style.rs`
- `src/style/theme.rs`
- `src/palette.rs`
- `CLAUDE.md`
