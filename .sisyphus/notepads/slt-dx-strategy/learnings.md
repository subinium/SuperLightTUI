# Learnings

## [2026-03-15] Session ses_30f1110cfffeHLJwhBf9HHmFmD

### Architecture
- Context is a command collector (not executor) — layout/terminal fully decoupled
- `consumed: Vec<bool>` parallel to `events: Vec<Event>` — already exists, pub(crate)
- `slt_warn!()` already exists at context.rs:26 — function, NOT macro
- `slt_assert!()` exists at context.rs:20
- Both marked `#[allow(dead_code)]`

### Key File Locations
- Panic hook: src/lib.rs:83-126 (install_panic_hook fn)
- error_boundary_with: src/context.rs:1857-1886
- Context struct fields: src/context.rs:230-263
- slt_warn: src/context.rs:26
- Theme: src/style/theme.rs
- F12 debug: src/layout/render.rs:24-224 + src/lib.rs debug_mode bool
- draw_debug_padding_markers: src/layout/render.rs (dead_code)
- TestBackend: src/test_utils.rs:132-319

### Constraints
- MUST NOT add external deps
- MUST NOT change key()/key_code() signatures (&self)
- All debug features behind #[cfg(debug_assertions)]
- RunConfig::default() behavior must be identical
- MUST NOT add #[must_use] to Response

## [2026-03-15] Task 1.1 — error_boundary panic hook conflict

### Panic Recovery Strategy
- Added `Context::is_real_terminal` gate to distinguish interactive terminal runs from headless tests.
- In `error_boundary_with()`, on `catch_unwind` error and real terminal mode, re-enter with `enable_raw_mode()` + `EnterAlternateScreen` before rendering fallback.
- This keeps panic hook behavior untouched while recovering terminal state for boundary-contained panics.

### Call Site Convention
- `run_frame()` now marks `ctx.is_real_terminal = true`, covering both `run_with()` and `run_inline_with()` paths.
- `TestBackend` render paths explicitly keep `ctx.is_real_terminal = false` to preserve headless behavior.

### Validation
- LSP diagnostics clean on changed files.
- All 5 quality gates passed in required order (`fmt`, `check`, `clippy -D warnings`, `test`, `check --examples`).
- Added `examples/error_boundary_demo.rs` to demonstrate panic recovery with fallback rendering.

## [2026-03-15] Task 1.2 — ContextSnapshot rollback scope

### Error Boundary State Rollback
- Added private ContextSnapshot in src/context.rs with 13 mutable per-frame fields that can be corrupted during panic unwind.
- error_boundary_with() now captures snapshot before user closure and restores it before running fallback.
- Restore strategy uses truncate for vector-backed state (commands, group_stack, hook_states, deferred_draws) and direct counter/flag reset for scalar fields.

### Regression Coverage
- Added panic-recovery tests in tests/widgets.rs for focus, hook cursor/state slots, modal state, and group stack scenarios.
- Hook cursor test validates cross-frame hook type integrity after panic rollback.

### Validation
- LSP diagnostics clean on changed files.
- Full 5-step quality gate passed in required order (fmt --check, check --all-features, clippy -D warnings, test --all-features, check --examples --all-features).
