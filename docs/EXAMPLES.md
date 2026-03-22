# Examples Guide

The `examples/` directory is intentionally broad.
Use this index to find the smallest example that matches what you want to build.

## Basics

| Example | Command | Purpose |
|---------|---------|---------|
| `hello` | `cargo run --example hello` | Smallest possible SLT app |
| `counter` | `cargo run --example counter` | State + keyboard input |
| `inline` | `cargo run --example inline` | Inline rendering below the prompt |
| `anim` | `cargo run --example anim` | Tween, spring, keyframes |

## Widget and layout tours

| Example | Command | Purpose |
|---------|---------|---------|
| `demo` | `cargo run --example demo` | Broad widget overview |
| `demo_cli` | `cargo run --example demo_cli` | CLI-style layout |
| `demo_table` | `cargo run --example demo_table` | Table widget focus |
| `demo_wiki` | `cargo run --example demo_wiki` | Text-heavy layout and markdown |
| `demo_website` | `cargo run --example demo_website` | Website-style terminal layout |

## Data and visualization

| Example | Command | Purpose |
|---------|---------|---------|
| `demo_dashboard` | `cargo run --example demo_dashboard` | Dashboard composition |
| `demo_infoviz` | `cargo run --example demo_infoviz` | Charts and visual summaries |
| `demo_trading` | `cargo run --example demo_trading` | Dense market/trading layout |
| `demo_spreadsheet` | `cargo run --example demo_spreadsheet` | Grid and data entry feel |
| `demo_raw_draw` | `cargo run --example demo_raw_draw` | Direct buffer drawing |

## Rich terminal features

| Example | Command | Purpose |
|---------|---------|---------|
| `demo_kitty_image` | `cargo run --example demo_kitty_image` | Kitty graphics protocol |
| `demo_fire` | `cargo run --release --example demo_fire` | Half-block visual effect |
| `demo_ime` | `cargo run --example demo_ime` | IME and CJK input |
| `demo_key_test` | `cargo run --example demo_key_test` | Inspect key events |
| `debug_selection` | `cargo run --example debug_selection` | Selection overlay debugging |

## Interaction-heavy apps

| Example | Command | Purpose |
|---------|---------|---------|
| `demo_game` | `cargo run --example demo_game` | Games in immediate mode |
| `error_boundary_demo` | `cargo run --example error_boundary_demo` | Panic recovery surface |
| `test_mouse` | `cargo run --example test_mouse` | Mouse interaction behavior |

## Async and performance

| Example | Command | Purpose |
|---------|---------|---------|
| `async_demo` | `cargo run --example async_demo --features async` | Background message flow |
| `perf_interactive` | `cargo run --example perf_interactive` | Interactive perf sanity checks |
| `perf_regression` | `cargo run --example perf_regression` | Headless perf regression coverage |

## Suggested path

1. Start with `hello`.
2. Move to `counter`.
3. Open `demo` for breadth.
4. Jump to one domain-specific example that matches your app.
5. Use `docs/PATTERNS.md` when you want composition guidance instead of a single demo.
