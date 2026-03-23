<div align="center">

# SuperLightTUI

**Superfast** to write. **Superlight** to run.

[![Crate Badge]][Crate]
[![Docs Badge]][Docs]
[![CI Badge]][CI]
[![MSRV Badge]][Crate]
[![Downloads Badge]][Crate]
[![License Badge]][License]

[Docs Index] · [Quick Start] · [Widget Guide] · [Patterns Guide] · [Examples Guide] · [Backends Guide] · [Architecture Guide]

**English** · [中文](docs/README.zh-CN.md) · [Español](docs/README.es.md) · [日本語](docs/README.ja.md) · [한국어](docs/README.ko.md)

</div>

SuperLightTUI is an immediate-mode TUI library for Rust with a deliberately small public grammar.
You write one closure, SLT calls it every frame, and the library handles layout, focus, diffing, and rendering.

It is designed for fast product iteration, approachable Rust syntax, and serious backend discipline.
That makes it work equally well for humans prototyping a tool and for coding agents generating UI from docs.

## Showcase

<table>
  <tr>
    <td align="center"><img src="assets/demo.png" alt="Widget Demo" /><br/><b>Widget Demo</b><br/><sub><code>cargo run --example demo</code></sub></td>
    <td align="center"><img src="assets/demo_dashboard.png" alt="Dashboard" /><br/><b>Dashboard</b><br/><sub><code>cargo run --example demo_dashboard</code></sub></td>
    <td align="center"><img src="assets/demo_website.png" alt="Website" /><br/><b>Website Layout</b><br/><sub><code>cargo run --example demo_website</code></sub></td>
  </tr>
  <tr>
    <td align="center"><img src="assets/demo_spreadsheet.png" alt="Spreadsheet" /><br/><b>Spreadsheet</b><br/><sub><code>cargo run --example demo_spreadsheet</code></sub></td>
    <td align="center"><img src="assets/demo_game.gif" alt="Games" /><br/><b>Games</b><br/><sub><code>cargo run --example demo_game</code></sub></td>
    <td align="center"><img src="assets/demo_fire.gif" alt="DOOM Fire" /><br/><b>DOOM Fire Effect</b><br/><sub><code>cargo run --release --example demo_fire</code></sub></td>
  </tr>
</table>

## Quick Start

```sh
cargo add superlighttui
```

```rust
fn main() -> std::io::Result<()> {
    slt::run(|ui: &mut slt::Context| {
        ui.text("hello, world");
    })
}
```

5 lines. No `App` trait. No `Model`/`Update`/`View`. No manual event loop. Ctrl+C just works.

## 60-Second Grammar

There are four ideas most apps start with:

1. State lives in normal Rust variables or structs.
2. Layout is mostly `row()`, `col()`, and `container()`.
3. Styling is method chaining.
4. Interactive widgets usually return `Response`.

```rust
ui.bordered(Border::Rounded).title("Status").p(1).gap(1).col(|ui| {
    ui.text("SLT").bold().fg(Color::Cyan);
    ui.row(|ui| {
        ui.text("mode:");
        ui.text("ready").fg(Color::Green);
        ui.spacer();
        if ui.button("Quit").clicked {
            ui.quit();
        }
    });
});
```

That is the core mental model. Everything else is depth, not a second framework.

## A Real App

```rust
use slt::{Border, Color, Context, KeyCode};

fn main() -> std::io::Result<()> {
    let mut count: i32 = 0;

    slt::run(|ui: &mut Context| {
        if ui.key('q') {
            ui.quit();
        }
        if ui.key('k') || ui.key_code(KeyCode::Up) {
            count += 1;
        }
        if ui.key('j') || ui.key_code(KeyCode::Down) {
            count -= 1;
        }

        ui.bordered(Border::Rounded).title("Counter").p(1).gap(1).col(|ui| {
            ui.text("Counter").bold().fg(Color::Cyan);
            ui.row(|ui| {
                ui.text("Count:");
                let color = if count >= 0 { Color::Green } else { Color::Red };
                ui.text(format!("{count}")).bold().fg(color);
            });
            ui.text("k +1 / j -1 / q quit").dim();
        });
    })
}
```

## Why SLT

- **Small public grammar**. Most screens start with normal Rust state, `row()` / `col()` / `container()`, method chaining, and `Response`.
- **Less framework ceremony**. Many apps do not need an app trait, retained tree, or message enum just to get moving.
- **Batteries included, backend still serious**. Common widgets auto-wire focus, hover, click, and scroll behavior, while the runtime keeps a conservative low-level path through `Backend`, `AppState`, and `frame()`.
- **Conservative internals**. SLT keeps the public surface small, but the internals stay deliberately boring: shared frame kernel, explicit backend contract coverage, zero `unsafe`, feature-gated runtime paths, and validation across `all-features`, `no-default-features`, WASM, clippy, examples, cargo-hack, semver, and deny checks.

For Rust users, that usually means less setup than retained-mode TUI frameworks.
For AI-assisted workflows, it means the public grammar is easy to infer from docs and examples.

SLT fits best when you want to build terminal apps quickly without giving up Rust type safety or backend escape hatches.
If you want a retained component tree or a GUI-first toolkit, another library may be a better fit.

## Common API Surface

```rust
// Text and layout
ui.text("Hello").bold().fg(Color::Cyan);
ui.row(|ui| {
    ui.text("left");
    ui.spacer();
    ui.text("right");
});

// Inputs and actions
ui.text_input(&mut name);
if ui.button("Save").clicked {}
ui.checkbox("Dark mode", &mut dark);

// Data and navigation
ui.tabs(&mut tabs);
ui.list(&mut items);
ui.table(&mut data);
ui.command_palette(&mut palette);

// Overlays and rich output
ui.toast(&mut toasts);
ui.modal(|ui| {
    ui.text("Confirm?").bold();
});
ui.markdown("# Hello **world**");

// Visualization
ui.chart(|c| {
    c.line(&data);
    c.grid(true);
}, 50, 16);
ui.sparkline(&values, 16);
ui.canvas(40, 10, |cv| {
    cv.circle(20, 20, 15);
});
```

For the categorized widget list, see [Widget Guide].
For composition advice, see [Patterns Guide].

## Learn The Library

| Document | What it covers |
|----------|----------------|
| [Quick Start] | Install, first app, closure mental model, layout, widget state |
| [Widget Guide] | Complete API catalog of widgets, runtime methods, and state types |
| [Patterns Guide] | State placement, screen composition, helper extraction, large-app structure |
| [Examples Guide] | Runnable examples grouped by product shape and feature area |
| [Backends Guide] | `Backend`, `AppState`, `frame()`, inline mode, static output |
| [Testing Guide] | `TestBackend`, `EventBuilder`, multi-frame tests, backend contract tests |
| [Debugging Guide] | F12 overlay, clipping, focus surprises, previous-frame behavior |
| [AI Guide] | Fastest path for AI-assisted builders and coding agents |
| [Architecture Guide] | Module map, frame lifecycle, layout/render pipeline |
| [Features Guide] | Feature flags, optional dependencies, recommended combos |
| [Animation Guide] | Tween, spring, keyframes, sequence, stagger |
| [Theming Guide] | Theme struct, presets, ThemeBuilder, custom themes |
| [Design Principles] | API constraints and design philosophy |

## Representative Examples

| Example | Command | Focus |
|---------|---------|-------|
| `hello` | `cargo run --example hello` | Smallest possible app |
| `counter` | `cargo run --example counter` | State + keyboard input |
| `demo` | `cargo run --example demo` | Broad widget tour |
| `demo_dashboard` | `cargo run --example demo_dashboard` | Dashboard layout |
| `demo_cli` | `cargo run --example demo_cli` | CLI tool layout |
| `demo_infoviz` | `cargo run --example demo_infoviz` | Charts and data viz |
| `demo_game` | `cargo run --example demo_game` | Immediate-mode interaction |
| `inline` | `cargo run --example inline` | Inline rendering below a normal prompt |
| `async_demo` | `cargo run --example async_demo --features async` | Background messages |

The full categorized index lives in [Examples Guide].

## Custom Widgets And Backends

- Implement `Widget` when you want reusable high-level building blocks.
- Implement `Backend` and drive `frame()` when you want a non-terminal target, external event loop, or embedded runtime.
- Use `TestBackend` for headless rendering checks and stable interaction tests.

The public grammar stays small even when you need the escape hatches.

## Contributing

Read [Contributing], then [Design Principles] and [Architecture Guide].
The release process expects format, check, clippy, tests, examples, and backend gates to stay green.

## License

[MIT](LICENSE)

<!-- Badge definitions -->
[Crate Badge]: https://img.shields.io/crates/v/superlighttui?style=flat-square&logo=rust&color=E05D44
[Docs Badge]: https://img.shields.io/docsrs/superlighttui?style=flat-square&logo=docs.rs
[CI Badge]: https://img.shields.io/github/actions/workflow/status/subinium/SuperLightTUI/ci.yml?branch=main&style=flat-square&label=CI
[MSRV Badge]: https://img.shields.io/crates/msrv/superlighttui?style=flat-square&label=MSRV
[Downloads Badge]: https://img.shields.io/crates/d/superlighttui?style=flat-square
[License Badge]: https://img.shields.io/crates/l/superlighttui?style=flat-square&color=1370D3

<!-- Link definitions -->
[CI]: https://github.com/subinium/SuperLightTUI/actions/workflows/ci.yml
[Crate]: https://crates.io/crates/superlighttui
[Docs Index]: docs/README.md
[Docs]: https://docs.rs/superlighttui
[Backends Guide]: docs/BACKENDS.md
[Testing Guide]: docs/TESTING.md
[Debugging Guide]: docs/DEBUGGING.md
[AI Guide]: docs/AI_GUIDE.md
[Quick Start]: docs/QUICK_START.md
[Widget Guide]: docs/WIDGETS.md
[Examples Guide]: docs/EXAMPLES.md
[Patterns Guide]: docs/PATTERNS.md
[Architecture Guide]: docs/ARCHITECTURE.md
[Design Principles]: docs/DESIGN_PRINCIPLES.md
[Animation Guide]: docs/ANIMATION.md
[Theming Guide]: docs/THEMING.md
[Features Guide]: docs/FEATURES.md
[Contributing]: CONTRIBUTING.md
[License]: ./LICENSE
