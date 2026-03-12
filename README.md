# superlighttui

[![Crates.io](https://img.shields.io/crates/v/superlighttui.svg)](https://crates.io/crates/superlighttui)
[![docs.rs](https://docs.rs/superlighttui/badge.svg)](https://docs.rs/superlighttui)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Build terminal UIs in Rust. Fast.**

Immediate-mode. Two dependencies. Zero `unsafe`. ~5k lines of code.

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
        if ui.key('q') { ui.quit(); }
        if ui.key('k') || ui.key_code(KeyCode::Up) { count += 1; }
        if ui.key('j') || ui.key_code(KeyCode::Down) { count -= 1; }

        ui.bordered(Border::Rounded).title("Counter").pad(1).gap(1).col(|ui| {
            ui.text("Counter").bold().fg(Color::Cyan);
            ui.row(|ui| {
                ui.text("Count:");
                let c = if count >= 0 { Color::Green } else { Color::Red };
                ui.text(format!("{count}")).bold().fg(c);
            });
            ui.text("k +1 / j -1 / q quit").dim();
        });
    })
}
```

State lives in your closure. Layout is `row()` and `col()`. Styling chains. That's it.

## Why SLT

### Your closure IS the app

No framework state. No message passing. No trait implementations. You write a function, SLT calls it every frame. Variables in your closure are your state.

### Everything auto-wires

- **Focus** &mdash; Tab/Shift+Tab cycles through widgets. You never call `register_focus()`.
- **Scroll** &mdash; Mouse wheel and drag just work. You pass `&mut ScrollState`, done.
- **Click & Hover** &mdash; `col()` and `row()` return `Response { clicked, hovered }`. No event plumbing.
- **Events** &mdash; Widgets consume their own keypresses. No manual event routing.

### Layout like CSS, syntax like Tailwind

```rust
ui.container()
    .border(Border::Rounded)
    .p(2)            // padding: 2
    .mx(1)           // margin-x: 1
    .grow(1)         // flex-grow: 1
    .max_w(60)       // max-width: 60
    .col(|ui| {
        ui.row(|ui| {
            ui.text("left");
            ui.spacer();
            ui.text("right");
        });
    });
```

Flexbox with `row()`, `col()`, `grow()`, `gap()`, `spacer()`. Tailwind shorthand: `.p()`, `.px()`, `.py()`, `.m()`, `.mx()`, `.my()`, `.w()`, `.h()`, `.min_w()`, `.max_w()`.

### 14 widgets, zero boilerplate

```rust
ui.text_input(&mut name);                    // single-line input
ui.textarea(&mut notes, 5);                  // multi-line editor
if ui.button("Submit") { /* clicked */ }     // button returns bool
ui.checkbox("Dark mode", &mut dark);         // toggle checkbox
ui.toggle("Notifications", &mut on);         // on/off switch
ui.tabs(&mut tabs);                          // tab navigation
ui.list(&mut items);                         // selectable list
ui.table(&mut data);                         // data table
ui.spinner(&spin);                           // loading animation
ui.progress(0.75);                           // progress bar
ui.scrollable(&mut scroll).col(|ui| { });    // scroll container
ui.toast(&mut toasts);                       // notifications
ui.separator();                              // horizontal line
ui.help(&[("q", "quit"), ("Tab", "focus")]); // key hints
```

Every widget handles its own keyboard events, focus state, and mouse interaction.

### Two dependencies

`crossterm` for terminal I/O. `unicode-width` for character measurement. That's the entire dependency tree. You can audit the supply chain in minutes.

### Small enough to read

~5,800 lines of Rust. 11 source files. No macros, no code generation, no build scripts. If something behaves unexpectedly, you can read the source and understand why.

## Showcase

| | |
|:---:|:---:|
| ![Widget Demo](assets/demo.png) | ![System Dashboard](assets/demo_dashboard.png) |
| **Widget Demo** — All 14 widgets | **Dashboard** — Live metrics & logs |
| `cargo run --example demo` | `cargo run --example demo_dashboard` |
| ![Website Layout](assets/demo_website.png) | ![Tetris](assets/demo_tetris.png) |
| **Website** — Full page layout | **Tetris** — Playable game |
| `cargo run --example demo_website` | `cargo run --example demo_tetris` |

> More examples: `demo_cli` (package manager), `demo_spreadsheet` (data grid), `inline` (inline mode), `anim` (animations)

## Features

### Layout

| Feature | API |
|---------|-----|
| Vertical stack | `ui.col(\|ui\| { })` |
| Horizontal stack | `ui.row(\|ui\| { })` |
| Gap between children | `ui.col_gap(1, \|ui\| { })` or `.gap(1)` |
| Flex grow | `.grow(1)` |
| Push to end | `ui.spacer()` |
| Alignment | `.align(Align::Center)` |
| Padding | `.pad(1)`, `.p(1)`, `.px(2)`, `.py(1)` |
| Margin | `.m(1)`, `.mx(2)`, `.my(1)` |
| Fixed size | `.w(20)`, `.h(10)` |
| Constraints | `.min_w(10)`, `.max_w(60)`, `.min_h(5)`, `.max_h(20)` |
| Text wrapping | `ui.text_wrap("long text...")` |
| Borders with titles | `.border(Border::Rounded).title("Panel")` |

### Styling

```rust
ui.text("styled").bold().italic().underline().fg(Color::Cyan).bg(Color::Black);
```

- 16 named colors, 256-color palette, 24-bit RGB
- 6 modifiers: bold, dim, italic, underline, reversed, strikethrough
- 4 border styles: Single, Double, Rounded, Thick

### Theming

```rust
slt::run_with(RunConfig { theme: Theme::light(), ..Default::default() }, |ui| {
    ui.set_theme(Theme::dark()); // switch at runtime
});
```

Dark and light presets. Custom themes with 13 color slots. All widgets inherit the theme automatically.

### Rendering

- **Double-buffer diff** &mdash; only changed cells hit the terminal
- **u32 coordinates** &mdash; no overflow on large terminals
- **Clipping** &mdash; content outside container bounds is hidden
- **Resize handling** &mdash; automatic reflow on terminal resize

### Animation

```rust
use slt::{Tween, Spring, anim::ease_out_bounce};

let mut tween = Tween::new(0.0, 100.0, 60).easing(ease_out_bounce);
let value = tween.value(ui.tick());

let mut spring = Spring::new(0.0, 180.0, 12.0);
spring.set_target(100.0);
```

Tween with 9 easing functions. Spring with configurable stiffness and damping. Both advance with the frame tick automatically.

### Inline Mode

```rust
slt::run_inline(3, |ui| {
    ui.text("Renders below your prompt.");
    ui.text("No alternate screen.").dim();
});
```

Render a fixed-height UI below the cursor. No alternate screen, no full takeover. For CLI tools that need a small interactive widget inline.

### Async

```rust
let tx = slt::run_async(|ui, messages: &mut Vec<String>| {
    for msg in messages.drain(..) {
        ui.text(msg);
    }
})?;
tx.send("Hello from background!".into()).await?;
```

Optional tokio integration. Background tasks send messages to the UI through a channel. Enable with `cargo add superlighttui --features async`.

### Debug

Press **F12** in any SLT app to toggle the layout debugger overlay. Shows container bounds, nesting depth, and layout structure.

## Install

```sh
cargo add superlighttui
```

Then `use slt::*;` &mdash; the crate name is descriptive, the import is short.

## Examples

| Example | Command | What it shows |
|---------|---------|---------------|
| hello | `cargo run --example hello` | Minimal setup |
| counter | `cargo run --example counter` | State + keyboard |
| demo | `cargo run --example demo` | All 14 widgets |
| demo_dashboard | `cargo run --example demo_dashboard` | Live dashboard |
| demo_cli | `cargo run --example demo_cli` | CLI tool layout |
| demo_spreadsheet | `cargo run --example demo_spreadsheet` | Data grid |
| demo_website | `cargo run --example demo_website` | Website in terminal |
| inline | `cargo run --example inline` | Inline mode |
| anim | `cargo run --example anim` | Tween + Spring |
| async_demo | `cargo run --example async_demo --features async` | Background tasks |

## Architecture

```
Closure -> Context collects Commands -> build_tree() -> flexbox layout -> diff buffer -> flush
```

Each frame: your closure runs, SLT collects what you described, computes flexbox layout, diffs against the previous frame, and flushes only the changed cells.

## License

MIT
