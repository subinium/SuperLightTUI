# Contributing to SLT

Before contributing, read:
- **[`docs/DESIGN_PRINCIPLES.md`](docs/DESIGN_PRINCIPLES.md)** — Why things are the way they are
- **[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)** — How the code is organized
- **[`docs/WIDGETS.md`](docs/WIDGETS.md)** — Which APIs and state types live where
- **[`docs/TESTING.md`](docs/TESTING.md)** — How to verify widget and layout behavior
- **[`docs/BACKENDS.md`](docs/BACKENDS.md)** — Low-level backend and run-loop contracts

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
2. **State placement** in the matching `src/widgets/*.rs` group file, then surfaced through `src/widgets.rs`
3. **Rendering method** on `Context` in the matching `src/context/widgets_*/` subfile (`widgets_input/`, `widgets_display/`, `widgets_interactive/`, or `widgets_viz.rs`)
4. **Re-export** in `lib.rs`
5. **Doc comment** (`///`) on the public method with usage example
6. **Response pattern** — interactive widgets return `Response`, display widgets return `&mut Self`
7. **Focus** — call `register_focusable()` if the widget accepts keyboard input
8. **Events** — consume handled key events so they don't bubble
9. **Theme** — use `self.theme.*` for default colors
10. **Example** — add to an existing example or create a new one

## Error Handling

See [`docs/DESIGN_PRINCIPLES.md` — Error Handling](docs/DESIGN_PRINCIPLES.md#6-error-handling) for the full policy.

Summary:
- Use `io::Result` for fallible operations
- `panic!()` only for programmer errors (with descriptive messages)
- No custom error types — `io::Error` is sufficient for SLT's error paths

## Architecture

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the full module map and data flow.

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
1. Run full CI (check, test, clippy, fmt) on stable + MSRV 1.88
2. Verify tag matches `Cargo.toml` version
3. Publish to crates.io
4. Create GitHub Release with notes extracted from CHANGELOG.md

**Do not** run `cargo publish` manually — let the workflow handle it.

## Dependencies

Core: `unicode-width`, `compact_str`. Terminal I/O: `crossterm` (default feature). Optional: `tokio` (async), `serde`, `image`, `qrcode`, `flate2` (kitty-compress), tree-sitter syntax features.

Do not add new dependencies without discussion. See [`docs/DESIGN_PRINCIPLES.md` — Dependencies](docs/DESIGN_PRINCIPLES.md#9-dependencies).
