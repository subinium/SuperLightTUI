# Examples Guide

The `examples/` directory contains both runnable Cargo example targets and
source-only demo modules that are composed into the tour binaries.

`Cargo.toml` sets `autoexamples = false`, so only the 20 explicit
`[[example]]` targets below can be run with `cargo run --example ...`.
Running `cargo run --example <source-file-name>` for any other file in
`examples/` fails with "no example target named ...".

Interactive examples that react to clicks, hover, or the scroll wheel start
with `slt::run_with(RunConfig::default().mouse(true), ...)`. Mouse capture is
off by default in your own apps too — see [QUICK_START.md](QUICK_START.md).

## Recommended learning order

1. `cargo run --example hello` — the smallest possible app.
2. `cargo run --example counter` — state plus keyboard input.
3. `cargo run --example cookbook_tour` — the [COOKBOOK.md](COOKBOOK.md)
   recipes (login, table, modal + toast, dashboard, file picker) as tabs.
4. `cargo run --example v0211_tour` — click, double-click, and scroll-wheel
   `Response` signals shown live, plus focus traversal and `KeyMap`.
5. `cargo run --example system_tour --features async` — error boundaries,
   overlay anchors, and the async / inline runtime modes.
6. `cargo run --example showcase_tour` — dense real-world screens
   (dashboard, spreadsheet, trading, website, design system).
7. Then pick by topic: `text_tour` (CJK, IME, CLI text), `canvas_tour`
   (raw drawing, images, animation, games), `v020_tour` (hooks, theming,
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

## Tour Binaries

These are the canonical way to review feature groups. A tour either
**includes** a source file verbatim through `#[path = "..."] mod ...;` (the
tab calls that file's `pub fn render(...)`), or **re-implements** a demo
inside the tour file because the original owns its own `fn main()` loop.

| Example | Command | Features | Purpose |
|---------|---------|----------|---------|
| `cookbook_tour` | `cargo run --example cookbook_tour` | - | Cookbook flows: login, table, modal, file picker, dashboard |
| `v0211_tour` | `cargo run --example v0211_tour` | - | v0.21.1 API additions: `Response` click / double-click / scroll signals, `KeyMap`, focus traversal, color helpers |
| `system_tour` | `cargo run --example system_tour --features async` | `async` | Error boundary, overlay anchor, async and inline runtime modes |
| `showcase_tour` | `cargo run --example showcase_tour` | - | Showcase screens and dense layouts |
| `text_tour` | `cargo run --example text_tour` | - | Text, CJK, IME, pretext, and CLI-style demos |
| `canvas_tour` | `cargo run --example canvas_tour` | - | Fire effect, games, raw drawing, Kitty images, animation |
| `v020_tour` | `cargo run --example v020_tour` | - | v0.20 hooks, theming, modal, layout, and utility demos |
| `v0210_widgets` | `cargo run --example v0210_widgets` | - | v0.21.0 widgets: paginator, number input, variable-height virtual list, scheduler, devtools inspector |

### Which source files each tour uses

Derived from the `#[path = ...]` includes in each tour file.

| Tour target | Included verbatim (`#[path]`) | Re-implemented or described in the tour |
|-------------|-------------------------------|------------------------------------------|
| `cookbook_tour` | `cookbook_dashboard`, `cookbook_file_picker`, `cookbook_login`, `cookbook_modal_toast`, `cookbook_table` | - |
| `showcase_tour` | `demo_dashboard`, `demo_design_system`, `demo_infoviz`, `demo_spreadsheet`, `demo_table`, `demo_trading`, `demo_website` | - |
| `text_tour` | `demo_cjk`, `demo_cli`, `demo_ime`, `demo_pretext` | - |
| `system_tour` | `demo_overlay_anchor` | `error_boundary_demo` (live re-implementation); `async_demo` and `inline` (description-only tabs — they need `run_async` / `run_inline` entry points) |
| `canvas_tour` | - | `demo_fire`, `demo_game`, `demo_raw_draw`, `demo_kitty_image`, `anim` (all re-implemented inside `canvas_tour.rs`) |
| `v020_tour` | `v020_breadcrumb_response`, `v020_ctrl_c_passthrough`, `v020_dx_shortcuts`, `v020_gauge`, `v020_gutter_highlights`, `v020_keymap_help`, `v020_modal_trap`, `v020_named_focus`, `v020_progress_response`, `v020_spacing_scale`, `v020_split_pane`, `v020_static_log`, `v020_theme_subtree`, `v020_use_effect`, `v020_use_state_keyed`, `v020_widthspec` | - |

`v0211_tour` and `v0210_widgets` are single files with no includes.

## Diagnostics And Development Tools

| Example | Command | Features | Purpose |
|---------|---------|----------|---------|
| `debug_selection` | `cargo run --example debug_selection` | - | Selection overlay debugging |
| `demo_key_test` | `cargo run --example demo_key_test` | - | Inspect key events |
| `test_mouse` | `cargo run --example test_mouse` | - | Check whether mouse events reach the app at all |
| `theme_hot_reload` | `cargo run --example theme_hot_reload --features theme-watch` | `theme-watch` | Theme file watcher demo |

## Performance And Reports

| Example | Command | Features | Purpose |
|---------|---------|----------|---------|
| `perf_interactive` | `cargo run --example perf_interactive` | - | Interactive perf sanity checks |
| `perf_regression` | `cargo run --example perf_regression` | - | Headless perf regression coverage |
| `v020_perf_audit` | `cargo run --example v020_perf_audit` | - | Non-interactive v0.20 perf report |
| `v020_test_utils` | `cargo run --example v020_test_utils` | - | Non-interactive test utility report |

## Source-only (not built)

These files are not `[[example]]` targets and are not included by any tour,
so no `cargo run --example` command compiles them. They are kept as reference
source; the closest runnable equivalent is listed.

| Source file | Closest runnable target |
|-------------|-------------------------|
| `anim.rs` | `canvas_tour` (Anim tab, re-implemented) |
| `async_demo.rs` | `system_tour --features async` (Async tab, description only) |
| `demo_fire.rs` | `canvas_tour` (Fire tab, re-implemented) |
| `demo_game.rs` | `canvas_tour` (Game tab, re-implemented) |
| `demo_kitty_image.rs` | `canvas_tour` (Kitty Image tab, re-implemented) |
| `demo_raw_draw.rs` | `canvas_tour` (Raw Draw tab, re-implemented) |
| `error_boundary_demo.rs` | `system_tour --features async` (Error Boundary tab, re-implemented) |
| `inline.rs` | `system_tour --features async` (Inline tab, description only) |
| `v020_showcase.rs` | `v020_tour` covers the same v0.20 features |

`v020_regression_panel.rs` is also not a runnable target, but it is compiled
by `tests/v020_regression_panel_demo.rs`.

Because these files are not compiled, they can drift from the current API.
Treat the tour versions as authoritative.
