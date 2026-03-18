# Contributing to SLT

Before contributing, read:
- **[DESIGN_PRINCIPLES.md](DESIGN_PRINCIPLES.md)** — Why things are the way they are
- **[ARCHITECTURE.md](ARCHITECTURE.md)** — How the code is organized

## Getting Started

```sh
git clone https://github.com/subinium/SuperLightTUI.git
cd superlighttui
cargo test
cargo run --example demo
```

## Development

### Build

```sh
cargo build
cargo build --features async
```

### Test

```sh
cargo test --all-features
cargo clippy --all-features -- -D warnings
```

### Run Examples

```sh
cargo run --example hello
cargo run --example counter
cargo run --example demo
cargo run --example inline
cargo run --example anim
cargo run --example async_demo --features async
```

### Quality Gate (run ALL before submitting)

```sh
cargo fmt -- --check
cargo check --all-features
cargo clippy --all-features -- -D warnings
cargo test --all-features
cargo check --examples --all-features
```

## Pull Requests

- Use [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`, `test:`
- Run the full quality gate above before submitting
- One logical change per PR
- Add examples for new widgets
- The [PR template](.github/PULL_REQUEST_TEMPLATE.md) includes a checklist — complete it

## Code Style

- No `unsafe` code — enforced by `#![forbid(unsafe_code)]`
- No `unwrap()` in functions returning `Result` — enforced by lint
- No `println!()`/`eprintln!()`/`dbg!()` in library code — enforced by lint
- No unnecessary comments — code should be self-documenting
- Use `self.theme.X` for colors, never hardcode

## Adding a Widget

Follow this checklist when adding a new widget:

1. **State struct** in `widgets.rs` — name it `{Widget}State`, implement `Default`
2. **Rendering method** on `Context` in `context/widgets_interactive.rs` (or `widgets_display.rs` for non-interactive)
3. **Re-export** in `lib.rs`
4. **Doc comment** (`///`) on the public method with usage example
5. **Response pattern** — interactive widgets return `Response`, display widgets return `&mut Self`
6. **Focus** — call `register_focusable()` if the widget accepts keyboard input
7. **Events** — consume handled key events so they don't bubble
8. **Theme** — use `self.theme.*` for default colors
9. **Example** — add to an existing example or create a new one

## Error Handling

See [DESIGN_PRINCIPLES.md — Error Handling](DESIGN_PRINCIPLES.md#6-error-handling) for the full policy.

Summary:
- Use `io::Result` for fallible operations
- `panic!()` only for programmer errors (with descriptive messages)
- No custom error types — `io::Error` is sufficient for SLT's error paths

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full module map and data flow.

```
User closure → Context collects Commands → build_tree() → flexbox compute → render to Buffer → diff → flush
```

- **Immediate mode**: Each frame, the closure runs and describes the UI
- **Double buffer**: Previous and current buffers are diffed, only changes are flushed
- **Flexbox**: Row/column layout with gap, grow, shrink
- **One-frame delay**: Layout-computed data (focus count, scroll bounds, hit areas) feeds back to the next frame via `prev_*` fields

## Releasing

Releases are automated via GitHub Actions. To publish a new version:

```sh
# 1. Bump version in Cargo.toml
# 2. Update CHANGELOG.md with new section
# 3. Commit and push
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore: release vX.Y.Z"
git push

# 4. Tag triggers the release pipeline
git tag vX.Y.Z
git push --tags
```

The release workflow (`.github/workflows/release.yml`) will:
1. Run full CI (check, test, clippy, fmt) on stable + MSRV 1.81
2. Verify tag matches `Cargo.toml` version
3. Publish to crates.io
4. Create GitHub Release with notes extracted from CHANGELOG.md

**Do not** run `cargo publish` manually — let the workflow handle it.

## Dependencies

Core: `crossterm`, `unicode-width`, `compact_str`. Optional: `tokio` (async), `serde`, `image`.

Do not add new dependencies without discussion. See [DESIGN_PRINCIPLES.md — Dependencies](DESIGN_PRINCIPLES.md#9-dependencies).
