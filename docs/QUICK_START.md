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
        if ui.key('q') {
            ui.quit();
        }
        if ui.key('k') || ui.key_code(KeyCode::Up) {
            count += 1;
        }
        if ui.key('j') || ui.key_code(KeyCode::Down) {
            count -= 1;
        }

        ui.text(format!("count: {count}"));
    })
}
```

There is no framework state object. Use normal Rust variables and structs.

## 4. Use layout and style chaining

```rust
use slt::{Border, Color, Context};

fn main() -> std::io::Result<()> {
    slt::run(|ui: &mut Context| {
        ui.bordered(Border::Rounded).title("Status").p(1).gap(1).col(|ui| {
            ui.text("SuperLightTUI").bold().fg(Color::Cyan);
            ui.row(|ui| {
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
        ui.col(|ui| {
            ui.text("Name").bold();
            ui.text_input(&mut name);
            ui.text(format!("Value: {}", name.value));
        });
    })
}
```

Interactive widgets usually own a `*State` type that you create once and pass every frame.

## 6. Know the two main return patterns

```rust
// Display widgets return &mut Context for chaining.
ui.text("hello").bold().fg(Color::Cyan);

// Interactive widgets usually return Response.
if ui.button("Save").clicked {
    // handle save
}
```

This is the core contract across the library.

## 7. Next docs

- `docs/WIDGETS.md` - categorized widget map
- `docs/PATTERNS.md` - common composition patterns, including component composition (`provide` / `use_context`)
- `docs/FEATURES.md` - feature flags (`async`, `serde`, `image`, `qrcode`, `syntax-*`)
- `docs/EXAMPLES.md` - runnable examples by use case
- `docs/ARCHITECTURE.md` - internal module map and frame lifecycle
- `docs/DESIGN_PRINCIPLES.md` - why the API is shaped this way
