# Examples Guide

The `examples/` directory contains both runnable Cargo example targets and
source-only demo modules that are composed into the tour binaries.

`Cargo.toml` sets `autoexamples = false`, so only the explicit `[[example]]`
targets below can be run with `cargo run --example ...`.

## Start Here

| Example | Command | Features | Purpose |
|---------|---------|----------|---------|
| `hello` | `cargo run --example hello` | - | Smallest possible SLT app |
| `counter` | `cargo run --example counter` | - | State + keyboard input |
| `demo` | `cargo run --example demo` | `qrcode`, `syntax` optional | Broad widget overview |
| `color_picker` | `cargo run --example color_picker` | - | Color picker interaction |

## Tour Binaries

These are the canonical way to review feature groups. Each tour pulls several
source-only demos from `examples/` via Rust modules.

| Example | Command | Features | Purpose |
|---------|---------|----------|---------|
| `v020_tour` | `cargo run --example v020_tour` | - | v0.20 API and interaction tour |
| `v0211_tour` | `cargo run --example v0211_tour` | - | v0.21.1 API additions |
| `v0210_widgets` | `cargo run --example v0210_widgets` | - | v0.21.0 widget coverage |
| `cookbook_tour` | `cargo run --example cookbook_tour` | - | Cookbook flows: login, table, modal, file picker, dashboard |
| `showcase_tour` | `cargo run --example showcase_tour` | - | Showcase screens and dense layouts |
| `canvas_tour` | `cargo run --example canvas_tour` | - | Canvas and raw drawing examples |
| `text_tour` | `cargo run --example text_tour` | - | Text, CJK, pretext, and typography demos |
| `system_tour` | `cargo run --example system_tour --features async` | `async` | Async, inline, static, and system-level demos |

## Diagnostics And Development Tools

| Example | Command | Features | Purpose |
|---------|---------|----------|---------|
| `debug_selection` | `cargo run --example debug_selection` | - | Selection overlay debugging |
| `demo_key_test` | `cargo run --example demo_key_test` | - | Inspect key events |
| `test_mouse` | `cargo run --example test_mouse` | - | Mouse interaction behavior |
| `theme_hot_reload` | `cargo run --example theme_hot_reload --features theme-watch` | `theme-watch` | Theme file watcher demo |

## Performance And Reports

| Example | Command | Features | Purpose |
|---------|---------|----------|---------|
| `perf_interactive` | `cargo run --example perf_interactive` | - | Interactive perf sanity checks |
| `perf_regression` | `cargo run --example perf_regression` | - | Headless perf regression coverage |
| `v020_perf_audit` | `cargo run --example v020_perf_audit` | - | Non-interactive v0.20 perf report |
| `v020_test_utils` | `cargo run --example v020_test_utils` | - | Non-interactive test utility report |

## Source-Only Demos

The remaining files in `examples/` are kept as source modules for the tours.
They intentionally do not have standalone `[[example]]` targets. Use the tour
listed below instead of `cargo run --example <source-file-name>`.

| Source demo family | Run this target |
|--------------------|-----------------|
| `inline`, `async_demo`, `v020_static_log`, `error_boundary_demo` | `cargo run --example system_tour --features async` |
| `anim`, `v020_*` API demos | `cargo run --example v020_tour` |
| `cookbook_*` demos | `cargo run --example cookbook_tour` |
| `demo_cli`, `demo_table`, `demo_website`, `demo_design_system`, `demo_pretext` | `cargo run --example showcase_tour` |
| `demo_dashboard`, `demo_infoviz`, `demo_trading`, `demo_spreadsheet`, `demo_raw_draw` | `cargo run --example showcase_tour` |
| `demo_kitty_image`, `demo_fire`, `demo_ime`, `demo_cjk` | `cargo run --example text_tour` |
| `demo_game`, `demo_overlay_anchor` | `cargo run --example showcase_tour` |

## Suggested Path

1. Start with `hello`.
2. Move to `counter`.
3. Open `demo` for breadth.
4. Run the tour closest to your target app.
5. Use `docs/PATTERNS.md` when you want composition guidance instead of a
   single demo.
