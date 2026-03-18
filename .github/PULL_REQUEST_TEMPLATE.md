## What

<!-- One-line summary of what this PR does -->

## Why

<!-- Why is this change needed? Link to issue if applicable -->

## Checklist

### Required

- [ ] `cargo fmt -- --check` passes
- [ ] `cargo check --all-features` passes
- [ ] `cargo clippy --all-features -- -D warnings` passes
- [ ] `cargo test --all-features` passes
- [ ] `cargo check --examples --all-features` passes

### Public API Changes

- [ ] New public items have doc comments (`///`)
- [ ] Widget follows the [Response pattern](DESIGN_PRINCIPLES.md#3-widget-contract)
- [ ] State struct in `widgets.rs`, impl on `Context`, re-export in `lib.rs`
- [ ] Breaking change? Update CHANGELOG.md and note it here

### If Adding a Widget

- [ ] Calls `register_focusable()` if interactive
- [ ] Consumes handled key events
- [ ] Uses `self.theme.*` for default colors
- [ ] Example added or existing example updated

### If Applicable

- [ ] Tests added
- [ ] Animation uses build-time pre-sort (not per-frame)
- [ ] No `unwrap()` in Result-returning functions
