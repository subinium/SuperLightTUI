<div align="center">

# SuperLightTUI

**Superfast** to write. **Superlight** to run.

[![Crate Badge]][Crate]
[![Docs Badge]][Docs]
[![CI Badge]][CI]
[![MSRV Badge]][Crate]
[![Downloads Badge]][Crate]
[![License Badge]][License]

[Crate] · [Docs] · [Examples] · [Contributing]

**English** · [中文](docs/README.zh-CN.md) · [Español](docs/README.es.md) · [日本語](docs/README.ja.md) · [한국어](docs/README.ko.md)

</div>

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

## Getting Started

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

**Your closure IS the app** — No framework state. No message passing. No trait implementations. You write a function, SLT calls it every frame.

**Everything auto-wires** — Focus cycles with Tab. Scroll works with mouse wheel. Containers report clicks and hovers. Widgets consume their own events.

**Layout like CSS, syntax like Tailwind** — Flexbox with `row()`, `col()`, `grow()`, `gap()`, `spacer()`. Tailwind shorthand: `.p()`, `.px()`, `.py()`, `.m()`, `.mx()`, `.my()`, `.w()`, `.h()`, `.min_w()`, `.max_w()`.

```rust
ui.container()
    .border(Border::Rounded)
    .p(2).mx(1).grow(1).max_w(60)
    .col(|ui| {
        ui.row(|ui| {
            ui.text("left");
            ui.spacer();
            ui.text("right");
        });
    });
```

**Two core dependencies** — `crossterm` for terminal I/O. `unicode-width` for character measurement. Optional: `tokio` for async, `serde` for serialization, `image` for image loading. Zero `unsafe` code.

> **AI-Assisted Development** — Use the `rust-tui-development-with-slt` skill in [Claude Code](https://docs.anthropic.com/en/docs/claude-code) for full API reference, best patterns, and codegen templates. Or design visually with [tui.builders](https://tui.builders):

[![tui.builders demo](assets/tui-builders-demo.gif)](https://tui.builders)

> Drag widgets, set properties in the inspector, export idiomatic Rust. Free, no signup, open source.

## Widgets

55+ built-in widgets, zero boilerplate:

```rust
ui.text_input(&mut name);                    // single-line input
ui.textarea(&mut notes, 5);                  // multi-line editor
if ui.button("Submit").clicked { /* … */ }    // returns Response
ui.checkbox("Dark mode", &mut dark);         // toggle checkbox
ui.toggle("Notifications", &mut on);         // on/off switch
ui.tabs(&mut tabs);                          // tab navigation
ui.list(&mut items);                         // selectable list
ui.select(&mut sel);                         // dropdown select
ui.radio(&mut radio);                        // radio button group
ui.multi_select(&mut multi);                 // multi-select checkboxes
ui.tree(&mut tree);                          // expandable tree view
ui.virtual_list(&mut list, 20, |ui, i| {}); // virtualized list
ui.table(&mut data);                         // data table
ui.spinner(&spin);                           // loading animation
ui.progress(0.75);                           // progress bar
ui.scrollable(&mut scroll).col(|ui| { });    // scroll container
ui.toast(&mut toasts);                       // notifications
ui.separator();                              // horizontal line
ui.help(&[("q", "quit"), ("Tab", "focus")]); // key hints
ui.link("Docs", "https://docs.rs/superlighttui");      // clickable hyperlink (OSC 8)
ui.modal(|ui| { ui.text("overlay"); });      // modal with dim backdrop
ui.overlay(|ui| { ui.text("floating"); });   // overlay without backdrop
ui.command_palette(&mut palette);            // searchable command palette
ui.markdown("# Hello **world**");            // markdown rendering
ui.form_field(&mut field);                   // labeled input with validation
ui.chart(|c| { c.line(&data); c.grid(true); }, 50, 16); // line/scatter/bar chart
ui.scatter(&points, 50, 16);                 // standalone scatter plot
ui.histogram(&values, 40, 12);               // auto-binned histogram
ui.bar_chart(&data, 24);                     // horizontal bars
ui.sparkline(&values, 16);                   // trend line ▁▂▃▅▇
ui.canvas(40, 10, |cv| { cv.circle(20, 20, 15); }); // braille canvas
ui.grid(3, |ui| { /* 3-column grid */ });    // grid layout
// v0.9 additions
ui.divider_text("Section Title");            // labeled horizontal divider
ui.alert("Saved!", AlertLevel::Success);     // inline alert banner
ui.breadcrumb(&["Home", "Settings", "Profile"]); // navigation breadcrumb
ui.accordion("Details", &mut open, |ui| { }); // collapsible section
ui.badge("New", Color::Green);               // inline status badge
ui.key_hint("q");                             // single key hint chip
ui.stat("Users", "1,234");                   // metric card
ui.stat_trend("Users", "1,234", Trend::Up);  // metric with trend indicator
ui.definition_list(&[("CPU", "4 cores"), ("RAM", "16 GB")]); // term/value pairs
ui.empty_state("No results", "Try a different search"); // empty placeholder
ui.code_block("fn main() {}", "rust");       // syntax-highlighted code
ui.code_block_numbered("let x = 1;");        // code with line numbers
ui.streaming_text(&mut stream);              // AI streaming text with cursor
ui.tool_approval(&mut tool);                 // approve/reject tool call
ui.context_bar(&items);                      // context window token bar
ui.image(&img);                              // half-block image rendering
ui.stat_colored("CPU", "72%", color);        // colored metric card
ui.candlestick(&candles, up_color, down_color); // OHLC candlestick chart
ui.badge_colored("Stable", Color::Green);    // colored status badge
ui.empty_state_action("Empty", "desc", "Add"); // empty state with button
// v0.10 additions
ui.consume_key('x');                         // explicit event consumption
ui.consume_key_code(KeyCode::Enter);         // consume special keys
// v0.13 additions
ui.tooltip("Save the current file");         // hover tooltip popup
ui.calendar(&mut cal);                       // date picker with month nav
ui.screen("home", &screens, |ui| {});        // screen routing stack
ui.sixel_image(&rgba, w, h, cols, rows);     // sixel image (non-Kitty)
ui.confirm("Delete?", &mut yes);             // yes/no with mouse support
```

Every widget handles its own keyboard events, focus state, and mouse interaction.

### Custom Widgets

Implement the `Widget` trait to build your own:

```rust
use slt::{Context, Widget, Color, Style};

struct Rating { value: u8, max: u8 }

impl Widget for Rating {
    type Response = bool;

    fn ui(&mut self, ui: &mut Context) -> bool {
        let focused = ui.register_focusable();
        let mut changed = false;

        if focused {
            if ui.key('+') && self.value < self.max { self.value += 1; changed = true; }
            if ui.key('-') && self.value > 0 { self.value -= 1; changed = true; }
        }

        let stars: String = (0..self.max)
            .map(|i| if i < self.value { '★' } else { '☆' })
            .collect();
        let color = if focused { Color::Yellow } else { Color::White };
        ui.styled(stars, Style::new().fg(color));
        changed
    }
}

// Usage: ui.widget(&mut rating);
```

Focus, events, theming, layout — all accessible through `Context`. One trait, one method.

## Features

<details>
<summary><b>Layout</b></summary>

| Feature | API |
|---------|-----|
| Vertical stack | `ui.col(\|ui\| { })` |
| Horizontal stack | `ui.row(\|ui\| { })` |
| Grid layout | `ui.grid(3, \|ui\| { })` |
| Gap between children | `.gap(1)` |
| Flex grow | `.grow(1)` |
| Push to end | `ui.spacer()` |
| Alignment | `.align(Align::Center)` |
| Padding | `.p(1)`, `.px(2)`, `.py(1)` |
| Margin | `.m(1)`, `.mx(2)`, `.my(1)` |
| Fixed size | `.w(20)`, `.h(10)` |
| Constraints | `.min_w(10)`, `.max_w(60)` |
| Percentage sizing | `.w_pct(50)`, `.h_pct(80)` |
| Justify | `.space_between()`, `.space_around()`, `.space_evenly()` |
| Text wrapping | `ui.text_wrap("long text...")` |
| Borders with titles | `.border(Border::Rounded).title("Panel")` |
| Per-side borders | `.border_top(false)`, `.border_sides(BorderSides::horizontal())` |
| Responsive gap | `.gap_at(Breakpoint::Md, 2)` |

</details>

<details>
<summary><b>Styling</b></summary>

```rust
ui.text("styled").bold().italic().underline().fg(Color::Cyan).bg(Color::Black);
```

16 named colors · 256-color palette · 24-bit RGB · 6 modifiers · 6 border styles

</details>

<details>
<summary><b>Theming</b></summary>

```rust
// 7 built-in presets
slt::run_with(RunConfig { theme: Theme::catppuccin(), ..Default::default() }, |ui| {
    ui.set_theme(Theme::dark()); // switch at runtime
});

// Build custom themes — unfilled fields default to Theme::dark()
let theme = Theme::builder()
    .primary(Color::Rgb(255, 107, 107))
    .accent(Color::Cyan)
    .build();
```

7 presets (dark, light, dracula, catppuccin, nord, solarized_dark, tokyo_night). Custom themes with 15 color slots + `is_dark` flag. All widgets inherit automatically. `Theme::light()` uses high-contrast Tailwind slate-based palette.

</details>

<details>
<summary><b>Style Recipes</b></summary>

```rust
use slt::{ContainerStyle, Border, Color};

// Define reusable styles — const for zero runtime cost
const CARD: ContainerStyle = ContainerStyle::new()
    .border(Border::Rounded).p(1).bg(Color::Indexed(236));

const DANGER: ContainerStyle = ContainerStyle::new()
    .bg(Color::Red);

// Apply one
ui.container().apply(&CARD).col(|ui| { ... });

// Compose multiple — last write wins
ui.container().apply(&CARD).apply(&DANGER).col(|ui| { ... });

// Mix with inline overrides
ui.container().apply(&CARD).grow(1).gap(2).col(|ui| { ... });
```

Define once, apply anywhere. `const` styles have zero runtime cost. Compose by chaining `.apply()` calls — inline methods always override.

</details>

<details>
<summary><b>Dark Mode</b></summary>

```rust
ui.container()
    .bg(Color::White)
    .dark_bg(Color::Rgb(30, 30, 46))  // applied when dark mode active
    .col(|ui| { ... });

ui.set_dark_mode(false); // toggle
```

Container-level style overrides that activate based on dark/light mode.

</details>

<details>
<summary><b>Responsive Layout</b></summary>

```rust
ui.container()
    .w(20).md_w(40).lg_w(60)  // width changes at breakpoints
    .p(1).lg_p(2)
    .col(|ui| { ... });
```

35 breakpoint-conditional methods (`xs_`, `sm_`, `md_`, `lg_`, `xl_` × `w`, `h`, `min_w`, `max_w`, `gap`, `p`, `grow`). Breakpoints: Xs (<40), Sm (40–79), Md (80–119), Lg (120–159), Xl (≥160).

</details>

<details>
<summary><b>Group Hover</b></summary>

```rust
ui.group("card")
    .border(Border::Rounded)
    .group_hover_bg(Color::Indexed(238))
    .col(|ui| {
        ui.text("Hover anywhere on card to highlight");
    });
```

Parent container hover/focus state propagates to children. Like Tailwind's `group-hover:`.

</details>

<details>
<summary><b>Hooks</b></summary>

```rust
let count = ui.use_state(|| 0i32);
ui.text(format!("{}", count.get(ui)));
if ui.button("+1") { *count.get_mut(ui) += 1; }

let doubled = *ui.use_memo(&count_val, |c| c * 2);
```

React-style persistent state in immediate mode. `State<T>` handle pattern. Call in same order every frame.

</details>

<details>
<summary><b>Rendering</b></summary>

- **Double-buffer diff** — only changed cells hit the terminal
- **Synchronized output** — DECSET 2026 prevents tearing on supported terminals
- **u32 coordinates** — no overflow on large terminals
- **Clipping** — content outside container bounds is hidden
- **Viewport culling** — off-screen widgets are skipped entirely
- **FPS cap** — `RunConfig { max_fps: Some(60), .. }` for CPU control
- **Non-TTY safety** — graceful exit when stdout is not a terminal
- **Resize handling** — automatic reflow on terminal resize
- **`collect_all()`** — single DFS pass replaces 7 separate tree traversals (v0.9)
- **Delta flush** — `apply_style_delta()` emits only changed attributes per cell (v0.9)
- **Keyframes pre-sort** — stops sorted at build time, not per-frame (v0.9)

</details>

<details>
<summary><b>Animation</b></summary>

```rust
let mut tween = Tween::new(0.0, 100.0, 60).easing(ease_out_bounce);
let value = tween.value(ui.tick());

let mut spring = Spring::new(0.0, 180.0, 12.0);
spring.set_target(100.0);

let mut kf = Keyframes::new(120)
    .stop(0.0, 0.0).stop(0.5, 100.0).stop(1.0, 50.0)
    .loop_mode(LoopMode::PingPong);

let mut seq = Sequence::new()
    .then(0.0, 50.0, 30, ease_out_quad)
    .then(50.0, 100.0, 30, ease_in_out_cubic);

let mut stagger = Stagger::new(0.0, 1.0, 40).delay(8);
let val = stagger.value(tick, item_index);
```

Tween with 9 easing functions. Spring physics. Keyframe timelines with loop modes. Sequence chains. Stagger for list animations. All support `.on_complete()` callbacks (`.on_settle()` for Spring).

</details>

<details>
<summary><b>Inline Mode</b></summary>

```rust
slt::run_inline(3, |ui| {
    ui.text("Renders below your prompt.");
    ui.text("No alternate screen.").dim();
});
```

Render a fixed-height UI below the cursor without taking over the terminal.

</details>

<details>
<summary><b>Async</b></summary>

```rust
let tx = slt::run_async(|ui, messages: &mut Vec<String>| {
    for msg in messages.drain(..) { ui.text(msg); }
})?;
tx.send("Hello from background!".into()).await?;
```

Optional tokio integration. Enable with `cargo add superlighttui --features async`.

</details>

<details>
<summary><b>Error Boundary</b></summary>

```rust
ui.error_boundary(|ui| {
    ui.text("If this panics, the app keeps running.");
});

ui.error_boundary_with(
    |ui| { /* risky code */ },
    |ui, msg| { ui.text(format!("Recovered: {msg}")); },
);
```

Catch widget panics without crashing the app. Partial commands are rolled back and a fallback is rendered.

</details>

<details>
<summary><b>Input Validation</b></summary>

```rust
let mut email = TextInputState::with_placeholder("you@example.com");
ui.text_input(&mut email);
email.validate(|v| {
    if v.contains('@') { Ok(()) } else { Err("Invalid email".into()) }
});
```

Call `.validate()` after `text_input()` to show inline error messages. Works with `form_field()` for grouped form validation.

</details>

<details>
<summary><b>Modal & Overlay</b></summary>

```rust
ui.modal(|ui| {
    ui.bordered(Border::Rounded).pad(2).col(|ui| {
        ui.text("Confirm?").bold();
        if ui.button("OK") { show = false; }
    });
});

ui.overlay(|ui| {
    ui.row(|ui| {
        ui.spacer();
        ui.text("Status: Online").fg(Color::Green);
    });
});
```

`modal()` dims the background and renders content on top. `overlay()` renders floating content without dimming. Both support full layout and interaction.

</details>

<details>
<summary><b>Hyperlinks</b></summary>

```rust
ui.link("Documentation", "https://docs.rs/superlighttui");
```

Renders clickable OSC 8 hyperlinks. Ctrl/Cmd+click opens in browser on supporting terminals (iTerm2, WezTerm, Ghostty, Windows Terminal).

</details>

<details>
<summary><b>Snapshot Testing</b></summary>

```rust
use slt::TestBackend;

let mut backend = TestBackend::new(40, 10);
backend.render(|ui| {
    ui.bordered(Border::Rounded).pad(1).col(|ui| {
        ui.text("Hello");
    });
});
insta::assert_snapshot!(backend.to_string_trimmed());
```

Use with [insta](https://crates.io/crates/insta) for snapshot-based UI regression tests.

</details>

<details>
<summary><b>Serde</b></summary>

```sh
cargo add superlighttui --features serde
```

Serialize/deserialize `Style`, `Color`, `Theme`, `Border`, `Padding`, `Margin`, `Constraints`, and `Modifiers`.

</details>

<details>
<summary><b>Tooltip</b></summary>

```rust
if ui.button("Save").clicked { save(); }
ui.tooltip("Save the current document to disk");
```

Call `tooltip()` after any widget. Shows a bordered popup when the widget is hovered. Deferred overlay rendering keeps hit detection stable.

</details>

<details>
<summary><b>Calendar</b></summary>

```rust
let mut cal = CalendarState::new();
ui.calendar(&mut cal);
if let Some((y, m, d)) = cal.selected_date() {
    println!("Selected: {y}-{m:02}-{d:02}");
}
```

Month grid with arrow key navigation, Enter to select, h/l for prev/next month. Mouse click on days and navigation arrows.

</details>

<details>
<summary><b>Screens / Routing</b></summary>

```rust
let mut screens = ScreenState::new("home");

ui.screen("home", &screens, |ui| {
    ui.text("Home");
    if ui.button("Settings").clicked { screens.push("settings"); }
});
ui.screen("settings", &screens, |ui| {
    if ui.button("Back").clicked { screens.pop(); }
});
```

Push/pop navigation stack. `ui.screen()` renders content only when the named screen is active.

</details>

<details>
<summary><b>Static Output</b></summary>

```rust
let mut output = StaticOutput::new();
slt::run_static(&mut output, 5, |ui| {
    output.println("Building crate...");
    ui.progress(0.6);
});
```

CLI tool pattern: scrolling logs above + live interactive TUI below. Uses `InlineTerminal` internally.

</details>

<details>
<summary><b>Fuzzy Search</b></summary>

Command palette now uses fuzzy matching — characters match in order but can skip. "sf" matches "Save File", "gc" matches "Git Commit". Falls back to substring matching when no fuzzy results found.

</details>

<details>
<summary><b>Table Zebra</b></summary>

```rust
let mut table = TableState::new(headers, rows);
table.zebra = true;
ui.table(&mut table);
```

Alternating row backgrounds for readability. Even rows use `theme.surface`, odd rows use `theme.surface_hover`.

</details>

<details>
<summary><b>Image Rendering</b></summary>

```sh
cargo add superlighttui --features image
```

```rust
use slt::HalfBlockImage;

let photo = image::open("photo.png").unwrap();
let img = HalfBlockImage::from_dynamic(&photo, 60, 30);
ui.image(&img);
```

Half-block (▀▄) image rendering. Also works without the `image` feature via `HalfBlockImage::from_rgb()`.

**Sixel protocol** (v0.13.2): `ui.sixel_image(&rgba, w, h, cols, rows)` for pixel-perfect images on xterm, foot, mlterm. Falls back to placeholder on unsupported terminals.

</details>

<details>
<summary><b>Feature Flags</b></summary>

| Flag | Description |
|------|-------------|
| `async` | `run_async()` with tokio channel-based message passing |
| `serde` | Serialize/Deserialize for Style, Color, Theme, layout types |
| `image` | `HalfBlockImage::from_dynamic()` with the `image` crate |
| `full` | All of the above |

```toml
[dependencies]
superlighttui = { version = "0.13", features = ["full"] }
```

</details>

<details>
<summary><b>Testing</b></summary>

```rust
use slt::{TestBackend, EventBuilder, KeyCode};

let mut backend = TestBackend::new(80, 24);
let events = EventBuilder::new().key('q').key_code(KeyCode::Enter).build();
backend.run_with_events(events, |ui| {
    ui.text("test content");
});
assert!(backend.to_string().contains("test content"));
```

Headless rendering with `TestBackend` and event simulation with `EventBuilder` for automated testing.

</details>

<details>
<summary><b>Direct Buffer Access</b></summary>

```rust
ui.container().w(40).h(20).draw(|buf, rect| {
    for y in rect.y..rect.bottom() {
        for x in rect.x..rect.right() {
            buf.set_char(x, y, '█', Style::new().fg(Color::Rgb(x as u8, 0, y as u8)));
        }
    }
});
```

`ContainerBuilder::draw()` gives you raw access to the cell buffer for pixel-level rendering. Useful for custom effects, games, and image rendering. The closure receives `(&mut Buffer, Rect)` and runs after layout is resolved.

</details>

<details>
<summary><b>Syntax-Highlighted Code</b></summary>

```rust
ui.code_block("fn greet(name: &str) -> String {\n    format!(\"Hello, {name}!\")\n}", "rust");
```

Renders code with One Dark palette syntax highlighting. Supports Rust keywords, string literals, comments, and numeric literals. Falls back to plain monospace for unknown languages.

</details>

<details>
<summary><b>Debug</b></summary>

Press **F12** in any SLT app to toggle the layout debugger overlay. Shows container bounds, nesting depth, and layout structure.

</details>

## Examples

| Example | Command | What it shows |
|---------|---------|---------------|
| hello | `cargo run --example hello` | Minimal setup |
| counter | `cargo run --example counter` | State + keyboard |
| demo | `cargo run --example demo` | All widgets |
| demo_dashboard | `cargo run --example demo_dashboard` | Live dashboard |
| demo_cli | `cargo run --example demo_cli` | CLI tool layout |
| demo_spreadsheet | `cargo run --example demo_spreadsheet` | Data grid |
| demo_website | `cargo run --example demo_website` | Website in terminal |
| demo_game | `cargo run --example demo_game` | Tetris + Snake + Minesweeper |
| demo_fire | `cargo run --release --example demo_fire` | DOOM fire effect (half-block) |
| demo_ime | `cargo run --example demo_ime` | Korean/CJK IME input |
| inline | `cargo run --example inline` | Inline mode |
| anim | `cargo run --example anim` | Tween + Spring + Keyframes |
| demo_infoviz | `cargo run --example demo_infoviz` | Data visualization |
| demo_trading | `cargo run --example demo_trading` | Exchange-style trading terminal |
| async_demo | `cargo run --example async_demo --features async` | Background tasks |

## Architecture

```
Closure → Context collects Commands → build_tree() → flexbox layout → diff buffer → flush
```

Each frame: your closure runs, SLT collects what you described, computes flexbox layout, diffs against the previous frame, and flushes only the changed cells.

Pure Rust. No macros, no code generation, no build scripts.

### Custom Backends

SLT's rendering is abstracted behind the `Backend` trait, enabling custom rendering targets beyond the terminal:

```rust
use slt::{Backend, AppState, Buffer, Rect, RunConfig, Context, Event};

struct MyBackend { buffer: Buffer }

impl Backend for MyBackend {
    fn size(&self) -> (u32, u32) {
        (self.buffer.area.width, self.buffer.area.height)
    }
    fn buffer_mut(&mut self) -> &mut Buffer { &mut self.buffer }
    fn flush(&mut self) -> std::io::Result<()> {
        // Render self.buffer to your target (canvas, GPU, network, etc.)
        Ok(())
    }
}

fn main() -> std::io::Result<()> {
    let mut backend = MyBackend { buffer: Buffer::empty(Rect::new(0, 0, 80, 24)) };
    let mut state = AppState::new();
    let config = RunConfig::default();

    loop {
        let events: Vec<Event> = vec![];
        if !slt::frame(&mut backend, &mut state, &config, &events, &mut |ui| {
            ui.text("Hello from custom backend!");
        })? { break; }
    }
    Ok(())
}
```

The `Backend` trait has 3 methods: `size()`, `buffer_mut()`, `flush()`. The built-in terminal backend handles ANSI escape codes, double-buffer diffing, and synchronized output internally. Custom backends receive the fully-rendered `Buffer` of `Cell`s and can present them however they choose — WebGL, egui embed, SSH tunnel, test harness, etc.

### AI-Native Widgets

SLT includes purpose-built widgets for AI/LLM workflows:

| Widget | Description |
|--------|-------------|
| `streaming_text()` | Token-by-token text display with blinking cursor |
| `streaming_markdown()` | Streaming markdown with headings, code blocks, inline formatting |
| `tool_approval()` | Human-in-the-loop approve/reject for tool calls |
| `context_bar()` | Token counter bar showing active context sources |
| `markdown()` | Static markdown rendering |
| `code_block()` | Syntax-highlighted code display |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

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
[Docs]: https://docs.rs/superlighttui
[Examples]: https://github.com/subinium/SuperLightTUI/tree/main/examples
[Contributing]: https://github.com/subinium/SuperLightTUI/blob/main/CONTRIBUTING.md
[License]: ./LICENSE
