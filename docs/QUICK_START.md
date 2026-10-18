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

        // Shortcuts are safe anywhere in the closure: a focused text input
        // keeps the keys it edits with (see step 6).
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

    let mut names: Vec<String> = Vec::new();

    slt::run(|ui: &mut Context| {
        let _ = ui.col(|ui| {
            ui.text("Name").bold();
            if ui.text_input(&mut name).submitted && !name.value.is_empty() {
                names.push(name.value.clone());
                name.clear(); // empty the field after Enter
            }
            ui.text(format!("Saved: {}", names.join(", ")));
        });
    })
}
```

Interactive widgets usually own a `*State` type that you create once and pass every frame.
For a password field use `TextInputState::password()` (or `.masked(true)` on any input).

## 6. Global shortcuts and focused text fields

While a text field (`text_input`, `textarea`, a form field, an open command
palette) has keyboard focus, it owns the keys it edits with: printable
characters without Ctrl / Super / Meta, Backspace, Delete, the arrow keys,
Home and End (plus Enter, Up and Down for `textarea`). `ui.key('q')`,
`ui.key_code(...)` and `ui.shortcut(...)` do not report those keys, whether the
check runs before or after the field, so typing `q` into a search box never
quits the app:

```rust
use slt::{Context, KeyCode, KeyModifiers, TextInputState};

fn main() -> std::io::Result<()> {
    let mut name = TextInputState::with_placeholder("Type here...");

    slt::run(|ui: &mut Context| {
        // Order does not matter: while the input is focused, 'q' is text.
        if ui.shortcut(KeyCode::Char('q'), KeyModifiers::NONE) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        // Chords always reach shortcuts, even while typing.
        if ui.shortcut(KeyCode::Char('s'), KeyModifiers::CONTROL) {
            // save
        }
        let _ = ui.text_input(&mut name);
    })
}
```

Keys the field never edits with (Esc, Tab, F-keys, Ctrl/Super/Meta chords)
still reach shortcuts. `ui.raw_key('q')` sees every press, ignoring both text
focus and the modal guard. Focus is decided as the frame starts, so after Tab
or a click moves focus to a button, `q` is a shortcut again. Ctrl+C quits by
default without any check (`RunConfig::handle_ctrl_c(false)` hands it to your
closure instead).

## 7. The mouse is on by default; hover is opt-in

`slt::run` uses `RunConfig::default()`, which turns the mouse on: clicks, the
scroll wheel, drag handles, and text selection reach SLT with no setup (over
SSH the default drops to clicks and the wheel only):

```rust
use slt::Context;

fn main() -> std::io::Result<()> {
    let mut saved = 0;

    slt::run(|ui: &mut Context| {
        if ui.button("Save").clicked {
            saved += 1;
        }
        ui.text(format!("saved {saved} times"));
    })
}
```

The app also works from the keyboard: Tab moves focus to the button and Enter
or Space activates it (`clicked` becomes `true`).

Hover styling (`hovered`, `group_hover_bg`) and tooltips need every pointer
move, which is heavier, so opt in:

```rust
use slt::{Context, MouseMode, RunConfig};

fn main() -> std::io::Result<()> {
    slt::run_with(RunConfig::default().mouse_mode(MouseMode::Hover), |ui: &mut Context| {
        let _ = ui.button("Deploy").on_hover(ui, "Push to staging");
    })
}
```

While the mouse is captured, SLT's own selection replaces the terminal's: drag
across text and it is copied on release. Most terminals still select natively
with Shift-drag, and `RunConfig::default().mouse_mode(MouseMode::Off)` turns
the mouse off entirely. See [INTERACTION.md](INTERACTION.md) for every mode.

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
