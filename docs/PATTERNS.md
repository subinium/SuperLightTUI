# Common Patterns

This page is for the moment when “hello world” is no longer enough and a real app starts to form.

The goal is not to teach every API again.
It is to show how to keep SLT code readable as screens, widgets, and state grow.

## The default shape of a real SLT app

For most medium-sized apps, the most readable shape is:

- one plain Rust `App` struct for app state
- one top-level `run(...)` or `run_with(...)` closure
- a few `render_*` helper functions for big panels or screens
- occasional hooks for truly local persistent state

That keeps the public grammar small without turning the closure into a 400-line blob.

## Application state lives in normal Rust

```rust
use slt::{Context, KeyCode};

struct App {
    count: i32,
    dark: bool,
}

fn main() -> std::io::Result<()> {
    let mut app = App {
        count: 0,
        dark: false,
    };

    slt::run(|ui: &mut Context| {
        if ui.key('q') || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        if ui.button("+1").clicked {
            app.count += 1;
        }

        ui.checkbox("Dark mode", &mut app.dark);
        ui.text(format!("count: {}", app.count));
    })
}
```

Use plain structs when the state belongs to your app, your domain, or your screens.

## Local persistent state with hooks

```rust
let count = ui.use_state(|| 0i32);
ui.text(format!("{}", count.get(ui)));
if ui.button("+1").clicked {
    *count.get_mut(ui) += 1;
}
```

Use hooks when:

- the state is local to one render subtree
- introducing a top-level field would add more noise than clarity
- you want to prototype quickly

Keep hook call order stable across frames.

## When to use app state vs hooks

Use app state when:

- multiple screens need the value
- the value matters outside rendering
- you want explicit ownership and easier refactoring

Use hooks when:

- the value is local to one widget subtree
- the lifetime is purely UI-local
- you want a lightweight scratchpad for a small interactive fragment

The mistake is not using hooks.
The mistake is using hooks for everything until the screen becomes impossible to reason about.

## Derived state with `use_memo`

```rust
let filtered = ui.use_memo(&(query.clone(), items.len()), |(query, _)| {
    items
        .iter()
        .filter(|item| item.contains(query))
        .cloned()
        .collect::<Vec<_>>()
});
```

Use `use_memo` when the computation is deterministic and should only rerun when the dependency tuple changes.
Like `use_state`, call order must stay stable across frames.

## Reduce repeated builder chains with helpers

Large SLT screens start to look noisy when the same builder chain repeats everywhere.
The fix is usually not a new framework.
It is a helper function.

```rust
use slt::{Border, Context};

fn panel(ui: &mut Context, title: &str, f: impl FnOnce(&mut Context)) {
    let _ = ui
        .bordered(Border::Rounded)
        .title(title)
        .p(1)
        .grow(1)
        .col(f);
}
```

This keeps the chaining style but moves the repetition out of the screen body.

## Split big screens into render helpers

```rust
fn render_sidebar(ui: &mut Context, app: &mut App) {
    ui.text("Navigation").bold();
    let _ = ui.list(&mut app.sidebar);
}

fn render_content(ui: &mut Context, app: &mut App) {
    ui.text(format!("Selected: {}", app.current_title()));
}

fn render_app(ui: &mut Context, app: &mut App) {
    ui.row(|ui| {
        panel(ui, "Sidebar", |ui| render_sidebar(ui, app));
        panel(ui, "Content", |ui| render_content(ui, app));
    });
}
```

If a closure becomes hard to scan, extract render helpers before inventing a new abstraction layer.

## Forms and validation

```rust
let mut email = TextInputState::with_placeholder("you@example.com");
ui.text_input(&mut email);
email.validate(|value| {
    if value.contains('@') {
        Ok(())
    } else {
        Err("Invalid email".into())
    }
});
```

For larger forms, reach for `FormField` and `FormState`.

## Focus and keyboard shortcuts

```rust
if ui.key('q') {
    ui.quit();
}
if ui.key_code(KeyCode::Enter) {
    submit();
}
if ui.raw_key_code(KeyCode::Esc) {
    close_overlay();
}
```

Use `raw_*` shortcuts for keys that must work regardless of modal or overlay state.

## Screen helpers and navigation

```rust
ui.screen("home", &screens, |ui| {
    ui.text("Home").bold();
});

ui.screen("settings", &screens, |ui| {
    ui.text("Settings");
});
```

Use `screen(name, &screens, ...)` when you want declarative rendering that only runs for the active screen.
Use manual `push()` / `pop()` logic on `ScreenState` when you need explicit navigation transitions.

## Modal, overlay, and screen composition

```rust
ui.screen("home", &screens, |ui| {
    if ui.button("Settings").clicked {
        screens.push("settings");
    }
});

ui.screen("settings", &screens, |ui| {
    if ui.button("Back").clicked {
        screens.pop();
    }
});

if show_modal {
    ui.modal(|ui| {
        ui.text("Confirm?").bold();
    });
}
```

Use screens for view-level navigation and modal/overlay for transient UI layers.

## Error boundaries and recovery

```rust
ui.error_boundary(|ui| {
    ui.text("Protected subtree");
});
```

Use `error_boundary` or `error_boundary_with` when you want one subtree to fail without taking down the whole app.
This is especially useful for experimental widgets, user-generated content, or plugins.

## Custom widgets: focus and interaction

```rust
let focused = ui.register_focusable();
let response = ui.interaction();

if response.hovered {
    ui.tooltip("Hovered");
}
```

Use `register_focusable()` when the widget needs keyboard participation.
Use `interaction()` when the widget needs click/hover without wrapping everything in a container.

## Async background messages

```rust
let tx = slt::run_async(|ui, messages: &mut Vec<String>| {
    for message in messages.drain(..) {
        ui.text(message);
    }
})?;

tx.send("Background work done".into()).await?;
```

Enable with the `async` feature.

## Animation patterns

```rust
// Tween: smooth transition over N ticks
let mut fade = Tween::new(0.0, 1.0, 30);
let opacity = fade.value(ui.tick());

// Spring: physics-based, responds to target changes
let mut spring = Spring::new(0.0, 0.2, 0.85);
if hovered {
    spring.set_target(1.0);
} else {
    spring.set_target(0.0);
}
spring.tick();
let scale = spring.value();

// Stagger: offset animation across list items
let mut stagger = Stagger::new(0.0, 1.0, 20).delay(3).items(items.len());
for (i, item) in items.iter().enumerate() {
    let alpha = stagger.value(ui.tick(), i);
    ui.text(item)
        .fg(Color::Rgb(255, 255, (alpha * 255.0) as u8));
}
```

Animation types are standalone structs that compute values from `ui.tick()`.
Pass computed values to style and layout methods.
See `docs/ANIMATION.md` for the full API.

## Responsive layout

```rust
match ui.breakpoint() {
    Breakpoint::Xs | Breakpoint::Sm => {
        ui.col(|ui| { /* stacked layout */ });
    }
    _ => {
        ui.row(|ui| { /* side-by-side layout */ });
    }
};
```

Use `breakpoint()` for width-dependent layout decisions.
ContainerBuilder also supports responsive methods like `.gap_sm(1).gap_lg(2)`.

## Custom widgets

```rust
use slt::{Color, Context, Style, Widget};

struct Rating {
    value: u8,
    max: u8,
}

impl Widget for Rating {
    type Response = bool;

    fn ui(&mut self, ui: &mut Context) -> bool {
        let focused = ui.register_focusable();
        let mut changed = false;

        if focused {
            if ui.key('+') && self.value < self.max {
                self.value += 1;
                changed = true;
            }
            if ui.key('-') && self.value > 0 {
                self.value -= 1;
                changed = true;
            }
        }

        let stars: String = (0..self.max)
            .map(|i| if i < self.value { '★' } else { '☆' })
            .collect();
        ui.styled(
            stars,
            Style::new().fg(if focused {
                Color::Yellow
            } else {
                Color::White
            }),
        );
        changed
    }
}
```

If you add a new built-in widget to the library itself, also follow the checklist in `CONTRIBUTING.md`.

## Testing and verification

```rust
use slt::TestBackend;

let mut backend = TestBackend::new(40, 10);
backend.render(|ui| {
    ui.text("Hello");
});

backend.assert_contains("Hello");
```

Use `TestBackend` for headless rendering checks and snapshot-style assertions.
For runtime-contract work, add a custom `Backend` + `frame()` test too.

## Heuristics that keep SLT readable

- If a builder chain repeats three times, extract a helper.
- If a closure becomes hard to scan, split it into `render_*` functions.
- If state is shared across screens, move it into your app struct.
- If state is local to one subtree, hooks are fine.
- If behavior depends on previous-frame data, test at least two frames.

SLT stays pleasant when you keep the public grammar small even as the codebase grows.
