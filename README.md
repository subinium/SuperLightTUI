<div align="center">

# SuperLightTUI

**Superfast** to write. **Superlight** to run.

[![Crate Badge]][Crate]
[![Docs Badge]][Docs]
[![CI Badge]][CI]
[![MSRV Badge]][Crate]
[![Downloads Badge]][Crate]
[![License Badge]][License]

[Docs Index] · [Quick Start] · [Widget Guide] · [Examples Guide] · [Patterns Guide] · [Architecture Guide] · [Contributing]

**English** · [中文](docs/README.zh-CN.md) · [Español](docs/README.es.md) · [日本語](docs/README.ja.md) · [한국어](docs/README.ko.md)

</div>

SuperLightTUI is an immediate-mode TUI library for Rust.
You write one closure, SLT calls it every frame, and the library handles layout, diffing, focus, and rendering.

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

5 lines. No `App` struct. No `Model`/`Update`/`View`. No event loop. Ctrl+C just works.

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

        ui.bordered(Border::Rounded).title("Counter").pad(1).gap(1).col(|ui| {
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

State lives in your closure. Layout is `row()` and `col()`. Styling chains. That is the model.

## Why SLT

- **Your closure is the app** - no framework state, no trait implementation boilerplate, no message loop API.
- **Layout like CSS, syntax like Tailwind** - `row()`, `col()`, `gap()`, `grow()`, `spacer()`, plus shorthand like `.p()`, `.px()`, `.m()`, `.w()`, `.max_w()`.
- **Widgets auto-wire the boring parts** - focus order, hover, click handling, scroll, and common keyboard behavior are built in.
- **Small core, optional extras** - core dependencies are `unicode-width` and `compact_str`; terminal I/O comes from optional `crossterm`; async, serde, image, qrcode, and syntax highlighting stay behind feature flags.
- **Library hygiene matters** - zero `unsafe`, explicit feature flags, docs, examples, tests, and semver-aware release discipline.

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

## Learn the Library

| Document | What it covers |
|----------|----------------|
| [Docs Index] | Guide to the documentation layers |
| [Quick Start] | Install, first app, counter, layout, input, next steps |
| [Widget Guide] | Categorized widget map and the main APIs to reach for |
| [Patterns Guide] | State, forms, overlays, async, custom widgets, testing |
| [Examples Guide] | Curated example index with commands and use cases |
| [Architecture Guide] | Module map, frame lifecycle, dependency flow |
| [`docs/DESIGN_PRINCIPLES.md`](docs/DESIGN_PRINCIPLES.md) | Why the API is shaped this way |

## Example Highlights

| Example | Command | Focus |
|---------|---------|-------|
| hello | `cargo run --example hello` | Smallest possible app |
| counter | `cargo run --example counter` | State + keyboard input |
| demo | `cargo run --example demo` | Broad widget tour |
| demo_dashboard | `cargo run --example demo_dashboard` | Dashboard layout |
| demo_cli | `cargo run --example demo_cli` | CLI tool layout |
| demo_infoviz | `cargo run --example demo_infoviz` | Charts and data viz |
| demo_game | `cargo run --example demo_game` | Immediate-mode interaction |
| async_demo | `cargo run --example async_demo --features async` | Background messages |

The full categorized index lives in [Examples Guide].

## Custom Widgets and Backends

- Implement `Widget` to build reusable widgets with full access to focus, layout, events, and theming.
- Implement `Backend` + drive `frame()` if you want a non-terminal renderer, test harness, or embedded target.
- Use `TestBackend` for headless rendering and snapshot-style assertions.

See [Patterns Guide] and [Architecture Guide] for the deeper paths.

## Contributing

Read [Contributing], then `docs/DESIGN_PRINCIPLES.md` and [Architecture Guide].
The release and CI process expects formatting, check, clippy, tests, and examples compile to stay green.

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
[Quick Start]: docs/QUICK_START.md
[Widget Guide]: docs/WIDGETS.md
[Examples Guide]: docs/EXAMPLES.md
[Patterns Guide]: docs/PATTERNS.md
[Architecture Guide]: docs/ARCHITECTURE.md
[Contributing]: CONTRIBUTING.md
[License]: ./LICENSE
