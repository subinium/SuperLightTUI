# Quick Start

This guide gets you from `cargo add` to a small interactive app without reading the full API reference.

## 1. Install

```sh
cargo add superlighttui
```

## 2. Render text

```rust
fn main() -> std::io::Result<()> {
    slt::run(|ui: &mut slt::Context| {
        ui.text("hello, world");
    })
}
```

That is the whole model: your closure runs every frame and describes the UI.

## 3. Add state in your closure

```rust
use slt::{Context, KeyCode};

fn main() -> std::io::Result<()> {
    let mut count = 0;

    slt::run(|ui: &mut Context| {
        if ui.key('k') || ui.key_code(KeyCode::Up) {
            count += 1;
        }
        if ui.key('j') || ui.key_code(KeyCode::Down) {
            count -= 1;
        }

        ui.text(format!("count: {count}"));

        // Global shortcuts go last — see step 6.
        if ui.key('q') {
            ui.quit();
        }
    })
}
```

There is no framework state object. Use normal Rust variables and structs.

## 4. Use layout and style chaining

```rust
use slt::{Border, Color, Context};

fn main() -> std::io::Result<()> {
    slt::run(|ui: &mut Context| {
        let _ = ui.bordered(Border::Rounded).title("Status").p(1).gap(1).col(|ui| {
            ui.text("SuperLightTUI").bold().fg(Color::Cyan);
            let _ = ui.row(|ui| {
                ui.text("mode:");
                ui.text("ready").fg(Color::Green);
            });
        });
    })
}
```

Use `row()` and `col()` for flexbox-style layout.
Use chainable modifiers like `.p()`, `.m()`, `.w()`, `.grow()`, `.fg()`, and `.bold()`.

## 5. Reach for widget state when a widget needs it

```rust
use slt::{Context, TextInputState};

fn main() -> std::io::Result<()> {
    let mut name = TextInputState::with_placeholder("Type here...");

    slt::run(|ui: &mut Context| {
        let _ = ui.col(|ui| {
            ui.text("Name").bold();
            let _ = ui.text_input(&mut name);
            ui.text(format!("Value: {}", name.value));
        });
    })
}
```

Interactive widgets usually own a `*State` type that you create once and pass every frame.

## 6. Check global shortcuts after your widgets

Widgets consume the keys they handle while they render. `ui.key(...)` only
sees events nobody has consumed *yet*, so the order of calls inside the
closure matters:

```rust
use slt::{Context, KeyCode, TextInputState};

fn main() -> std::io::Result<()> {
    let mut name = TextInputState::with_placeholder("Type here...");

    slt::run(|ui: &mut Context| {
        // 1. Render widgets first: a focused text input consumes the
        //    characters typed into it, including 'q'.
        let _ = ui.text_input(&mut name);

        // 2. Then check global shortcuts. Only unconsumed keys reach here.
        if ui.key('q') || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
    })
}
```

If the `ui.key('q')` check ran *before* `ui.text_input(...)`, typing `q`
into the focused field would insert the letter **and** quit the app. Keys a
text input does not consume (such as Esc and Ctrl+Q) still reach a check placed
after it. Ctrl+C quits by default without any check
(`RunConfig::handle_ctrl_c(false)` hands it to your closure instead).

## 7. Turn on the mouse when you need clicks, hover, or scrolling

Mouse capture is opt-in. `slt::run` uses `RunConfig::default()`, which has
`mouse: false`, so clicks, hover, and the scroll wheel never reach SLT:

```rust
use slt::{Context, RunConfig};

fn main() -> std::io::Result<()> {
    let mut saved = 0;

    slt::run_with(RunConfig::default().mouse(true), |ui: &mut Context| {
        if ui.button("Save").clicked {
            saved += 1;
        }
        ui.text(format!("saved {saved} times"));
    })
}
```

Without `.mouse(true)` the app still works from the keyboard: Tab moves focus
to the button and Enter or Space activates it (`clicked` becomes `true`).
Leaving mouse capture off keeps the terminal's native text selection working,
which is why it is not the default.

## 8. Know the two main return patterns

```rust
// Text and its style chain return &mut Context for chaining.
ui.text("hello").bold().fg(Color::Cyan);

// Most widgets return a #[must_use] Response — interactive ones (button,
// text_input, list) and display ones (badge, stat, alert, separator) alike.
if ui.button("Save").clicked {
    // handle save
}
let _ = ui.badge("beta"); // `let _ =` when you ignore it
```

This is the core contract across the library. `Response` also offers
callback chaining: `ui.button("Save").on_click(ui, |_| save())` and
`.on_hover(ui, "tooltip text")`.

## 9. Layout data is one frame behind

SLT lays out the tree after your closure returns, so anything geometric you
read during the closure — `Response.rect`, hover, click hit-testing — comes
from the **previous** frame. On the very first frame `Response.rect` is all
zeros and nothing can be hovered yet; after a layout change, the first frame
still sees the old geometry. Logic that depends on positions should tolerate
one stale frame. [PREVIOUS_FRAME_GUIDE.md](PREVIOUS_FRAME_GUIDE.md) explains
the model and its edge cases.

## 10. Next docs

- [WIDGETS.md](WIDGETS.md) - categorized widget map
- [INTERACTION.md](INTERACTION.md) - focus, keyboard, mouse, and `Response` signals
- [WEB_DEVELOPERS.md](WEB_DEVELOPERS.md) - SLT explained through React / CSS concepts
- [TERMINAL_COMPATIBILITY.md](TERMINAL_COMPATIBILITY.md) - what differs between terminals and multiplexers
- [PATTERNS.md](PATTERNS.md) - common composition patterns, including component composition (`provide` / `use_context`)
- [FEATURES.md](FEATURES.md) - feature flags (`async`, `serde`, `image`, `qrcode`, `syntax-*`)
- [EXAMPLES.md](EXAMPLES.md) - runnable examples by use case
- [ARCHITECTURE.md](ARCHITECTURE.md) - internal module map and frame lifecycle
- [DESIGN_PRINCIPLES.md](DESIGN_PRINCIPLES.md) - why the API is shaped this way
