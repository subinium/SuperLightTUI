# Contributing to SLT

## Getting Started

```sh
git clone https://github.com/[owner]/superlighttui.git
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
cargo test
cargo clippy
cargo clippy --features async
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

## Pull Requests

- Use [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`, `test:`
- Run `cargo test` and `cargo clippy` before submitting
- One logical change per PR
- Add examples for new widgets

## Code Style

- No `unsafe` code
- No unnecessary comments — code should be self-documenting
- Use `self.theme.X` for colors, never hardcode
- Follow existing patterns in `context.rs` for new widgets:
  1. State struct in `widgets.rs`
  2. Rendering method on `Context` in `context.rs`
  3. Re-export in `lib.rs`
- Widget methods should:
  - Call `register_focusable()` if interactive
  - Consume handled key events
  - Use theme colors as defaults

## Architecture

```
User closure -> Context collects Commands -> build_tree() -> flexbox compute -> render to Buffer -> diff -> flush
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
1. Run full CI (check, test, clippy, fmt) on stable + MSRV 1.74
2. Verify tag matches `Cargo.toml` version
3. Publish to crates.io
4. Create GitHub Release with notes extracted from CHANGELOG.md

**Do not** run `cargo publish` manually — let the workflow handle it.

## Dependencies

Only `crossterm` and `unicode-width`. `tokio` is optional behind the `async` feature flag. Do not add new dependencies without discussion.
