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
cargo run --example system_tour --features async
cargo run --example canvas_tour --all-features
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
6. **Response pattern** — interactive and independently framed display widgets return `Response`; style-chain text helpers return `&mut Self`
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

[`AGENTS.md`](AGENTS.md) is the canonical release checklist. Every release,
including patches, uses a `release/vX.Y.Z` branch, a reviewed PR, green CI,
squash merge, and an annotated tag on the merged `main` commit. Never push a
release commit or tag directly from an unreviewed local branch.

The tag-triggered workflow runs the full stable/MSRV/platform/feature/security
gate, publishes the library crate, and creates the GitHub Release. Do not run
`cargo publish` locally. After publication, run
`scripts/smoke_release.sh X.Y.Z` to compile and execute an exact-version
downstream consumer from crates.io.

## Dependencies

Core: `unicode-width`, `unicode-segmentation`, `smallvec`, `compact_str`. Terminal I/O: `crossterm` (default feature). Optional: `tokio` (async), `serde`, `image`, `qrcode`, `flate2` (kitty-compress), tree-sitter syntax features.

Do not add new dependencies without discussion. See [`docs/DESIGN_PRINCIPLES.md` — Dependencies](docs/DESIGN_PRINCIPLES.md#9-dependencies).
