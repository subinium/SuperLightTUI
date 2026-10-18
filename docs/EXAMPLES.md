# Examples Guide

The `examples/` directory contains both runnable Cargo example targets and
source-only demo modules that are composed into the tour binaries.

`Cargo.toml` sets `autoexamples = false`, so only the 29 explicit
`[[example]]` targets below can be run with `cargo run --example ...`.
Running `cargo run --example <source-file-name>` for any other file in
`examples/` fails with "no example target named ...". Every other file is a
tour part or test fixture included through `#[path = "..."]`, so every
`examples/*.rs` source is compiled by some target;
`tests/docs_contract.rs` fails if one is not.

Interactive examples that react to clicks, hover, or the scroll wheel start
with `slt::run_with(RunConfig::default().mouse(true), ...)`. Mouse capture is
off by default in your own apps too — see [QUICK_START.md](QUICK_START.md).

## Recommended learning order

1. `cargo run --example hello` — the smallest possible app.
2. `cargo run --example counter` — state plus keyboard input.
3. `cargo run --example cookbook_tour` — the [COOKBOOK.md](COOKBOOK.md)
   recipes (login, table, modal + toast, dashboard, file picker) as tabs.
4. `cargo run --example interaction_tour` — click, double-click, and scroll-wheel
   `Response` signals shown live, plus focus traversal and `KeyMap`.
5. `cargo run --example system_tour --features async` — error boundaries,
   overlay anchors, and the async / inline runtime modes.
6. `cargo run --example showcase_tour` — dense real-world screens
   (dashboard, spreadsheet, trading, website, design system).
7. Then pick by topic: `text_tour` (CJK, IME, CLI text), `canvas_tour`
   (raw drawing, images, animation, games), `widgets_tour` (hooks, theming,
   modal focus traps, layout), `demo` (one screen with most widgets).

Use [PATTERNS.md](PATTERNS.md) when you want composition guidance instead of
a single demo.

## Start Here

| Example | Command | Features | Purpose |
|---------|---------|----------|---------|
| `hello` | `cargo run --example hello` | - | Smallest possible SLT app |
| `counter` | `cargo run --example counter` | - | State + keyboard input |
| `demo` | `cargo run --example demo` | `qrcode` optional (`cargo run --example demo --features qrcode`) | Broad widget overview |
| `color_picker` | `cargo run --example color_picker` | - | Color picker interaction |
| `inline` | `cargo run --example inline` | - | Inline mode: a fixed-height TUI below the shell prompt (`run_inline`) |
| `async_demo` | `cargo run --example async_demo --features async` | `async` | Background tokio task feeding messages into the UI (`run_async`) |
| `error_boundary_demo` | `cargo run --example error_boundary_demo` | - | Catch a panicking child closure with `error_boundary` |

## Tour Binaries

These are the canonical way to review feature groups. A tour either
**includes** a source file verbatim through `#[path = "..."] mod ...;` (the
tab calls that file's `pub fn render(...)`), or **re-implements** a demo
inside the tour file because the original owns its own `fn main()` loop.

| Example | Command | Features | Purpose |
|---------|---------|----------|---------|
| `cookbook_tour` | `cargo run --example cookbook_tour` | - | Cookbook flows: login, table, modal, file picker, dashboard |
| `interaction_tour` | `cargo run --example interaction_tour` | - | `Response` click / double-click / scroll signals, `KeyMap`, focus traversal, color helpers |
| `system_tour` | `cargo run --example system_tour --features async` | `async` | Error boundary, overlay anchor, async and inline runtime modes |
| `showcase_tour` | `cargo run --example showcase_tour` | - | Showcase screens and dense layouts |
| `text_tour` | `cargo run --example text_tour` | - | Text, CJK, IME, pretext, and CLI-style demos |
| `canvas_tour` | `cargo run --example canvas_tour` | - | Fire effect, games, raw drawing, Kitty images, animation |
| `widgets_tour` | `cargo run --example widgets_tour` | - | Hooks, theming, modal, layout, and utility demos |
| `widgets_extra` | `cargo run --example widgets_extra` | - | Paginator, number input, variable-height virtual list, scheduler, devtools inspector |

### Which source files each tour uses

Derived from the `#[path = ...]` includes in each tour file.

| Tour target | Included verbatim (`#[path]`) | Re-implemented or described in the tour |
|-------------|-------------------------------|------------------------------------------|
| `cookbook_tour` | `cookbook_dashboard`, `cookbook_file_picker`, `cookbook_login`, `cookbook_modal_toast`, `cookbook_table` | - |
| `showcase_tour` | `demo_dashboard`, `demo_design_system`, `demo_infoviz`, `demo_spreadsheet`, `demo_table`, `demo_trading`, `demo_website` | - |
| `text_tour` | `demo_cjk`, `demo_cli`, `demo_ime`, `demo_pretext` | - |
| `system_tour` | `demo_overlay_anchor` | `error_boundary_demo` (live re-implementation); `async_demo` and `inline` (description-only tabs — they need `run_async` / `run_inline` entry points; both are also standalone targets) |
| `canvas_tour` | - | `demo_fire`, `demo_game`, `demo_raw_draw`, `demo_kitty_image`, `anim` (all re-implemented inside `canvas_tour.rs`; each is also a standalone target) |
| `widgets_tour` | `breadcrumb_response`, `ctrl_c_passthrough`, `dx_shortcuts`, `gauge`, `gutter_highlights`, `keymap_help`, `modal_trap`, `named_focus`, `progress_response`, `spacing_scale`, `split_pane`, `static_log`, `theme_subtree`, `use_effect`, `use_state_keyed`, `widthspec` | - |

`interaction_tour` and `widgets_extra` are single files with no includes.

## Standalone Feature Demos

| Example | Command | Features | Purpose |
|---------|---------|----------|---------|
| `feature_showcase` | `cargo run --example feature_showcase` | - | Builder widgets (gauge, breadcrumb, gutter), WidthSpec, theme subtree, `animate_bool`, `on_hover`, and named focus on one screen |
| `anim` | `cargo run --example anim` | - | Tween, Spring, Keyframes, Sequence, and Stagger |
| `demo_fire` | `cargo run --example demo_fire` | - | Half-block fire effect |
| `demo_game` | `cargo run --example demo_game` | - | Tetris, snake, and minesweeper |
| `demo_kitty_image` | `cargo run --example demo_kitty_image` | - | Kitty graphics image gallery in a scroll view |
| `demo_raw_draw` | `cargo run --example demo_raw_draw` | - | `ContainerBuilder::draw` raw buffer primitives |

## Diagnostics And Development Tools

| Example | Command | Features | Purpose |
|---------|---------|----------|---------|
| `debug_selection` | `cargo run --example debug_selection` | - | Selection overlay debugging |
| `demo_key_test` | `cargo run --example demo_key_test` | - | Inspect key events |
| `mouse_probe` | `cargo run --example mouse_probe` | - | Check whether mouse events reach the app at all |
| `theme_hot_reload` | `cargo run --example theme_hot_reload --features theme-watch` | `theme-watch` | Theme file watcher demo |

## Performance And Reports

| Example | Command | Features | Purpose |
|---------|---------|----------|---------|
| `perf_interactive` | `cargo run --example perf_interactive` | - | Interactive perf sanity checks |
| `perf_regression` | `cargo run --example perf_regression` | - | Headless perf regression coverage |
| `perf_audit` | `cargo run --example perf_audit` | - | Non-interactive hot-path perf report |
| `testing_report` | `cargo run --example testing_report` | - | Non-interactive test utility report |

## Test-only sources

`regression_panel.rs` is not a runnable target. It is compiled by
`tests/example_snapshots/regression_panel.rs`, which renders it headlessly.
