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

## Components

A "component" in SLT is not a framework concept. There is no `Component` trait, no virtual DOM, no lifecycle hooks beyond the few state primitives you already know. A component is whatever helper function you find yourself extracting when the same shape repeats.

This section walks the four building blocks that make components ergonomic:

1. Functions for the shape itself
2. `use_state_named` for state that survives across frames inside the component
3. `provide` / `use_context` for values that propagate through deep trees
4. `.with` / `.with_if` for conditional styling

The order matters. Reach for the simpler tool first.

### Components as Functions (the canonical pattern)

The simplest reusable component is a free function that takes `&mut Context` and any explicit state it needs. No framework magic, no registration step.

```rust
use slt::{Border, Context, Trend};

fn metric_card(ui: &mut Context, label: &str, value: f64, trend: Trend) {
    let _ = ui.bordered(Border::Single).pad(1).col(|ui| {
        ui.text(label).dim();
        ui.text(format!("{:.1}", value)).bold();
        let arrow = match trend {
            Trend::Up => "▲",
            Trend::Down => "▼",
            Trend::Flat => "—",
        };
        ui.text(arrow);
    });
}
```

Usage from a screen:

```rust
fn render_dashboard(ui: &mut Context, app: &App) {
    ui.row(|ui| {
        metric_card(ui, "Revenue", app.revenue, Trend::Up);
        metric_card(ui, "Latency p99", app.p99_ms, Trend::Down);
        metric_card(ui, "Active users", app.users, Trend::Flat);
    });
}
```

Trade-offs:

- Fully explicit. Rust's ownership tells you exactly who reads and who mutates what.
- Type-safe. The compiler enforces parameter shapes at every call site.
- No hidden state. The function only renders; it does not remember anything between frames.

The cost is parameter count. When a component grows past four or five arguments, group related values into a small struct and pass `&Props` instead of repeating five-arg signatures across the codebase.

```rust
struct MetricCardProps<'a> {
    label: &'a str,
    value: f64,
    trend: Trend,
    accent: Color,
}

fn metric_card(ui: &mut Context, p: &MetricCardProps<'_>) {
    // ...
}
```

This is still "just a function." No ceremony.

### Component-local State with `use_state_named`

Sometimes a component needs state that must survive across frames but does not belong to the caller. A collapsible panel knows whether it is expanded. A pagination control knows the current page. Threading those values up through every call site pollutes the caller's API.

`use_state_named` is the answer. It is `use_state`, but keyed by an explicit `&'static str` instead of by hook call order:

```rust
use slt::{Border, Context};

fn expandable_card(
    ui: &mut Context,
    id: &'static str,
    title: &str,
    body: impl FnOnce(&mut Context),
) {
    let expanded = ui.use_state_named::<bool>(id);
    let _ = ui.bordered(Border::Single).col(|ui| {
        let label = if *expanded.get(ui) { "▼" } else { "▶" };
        if ui.button(label).clicked {
            let v = *expanded.get(ui);
            *expanded.get_mut(ui) = !v;
        }
        ui.text(title).bold();
        if *expanded.get(ui) {
            body(ui);
        }
    });
}
```

Usage:

```rust
expandable_card(ui, "card.networking", "Networking", |ui| {
    ui.text("eth0  192.168.1.42");
    ui.text("wlan0 10.0.0.7");
});

expandable_card(ui, "card.disks", "Disks", |ui| {
    ui.text("nvme0n1  931 GiB");
});
```

When you need a default different from `Default::default()`, use `use_state_named_with`:

```rust
let page = ui.use_state_named_with::<usize>("pager", || 1);
```

Rules of the road:

- IDs are `&'static str`. Pick something descriptive — `"card.networking"`, `"pager.users"`, not `"x"`.
- Two calls with the same id at the same scope share state. This is sometimes what you want (siblings agreeing on a value) and sometimes a bug (two unrelated cards collapsing together). When in doubt, suffix with the data key: `"card.disks"` vs `"card.networking"`.
- Unlike positional `use_state`, named state does not depend on call order. You can branch around a named-state read without losing the value next frame.
- For the full method surface on the returned `State<T>`, see `STATE_APIS.md`.

### Context Injection with `ui.provide` / `ui.use_context`

Some values want to propagate through a deep tree without being passed to every function. The classic examples are theme, the current user, and feature flags. Threading them through ten render helpers is parameter-drilling, and it makes refactoring painful.

`provide` makes a value available inside a closure. Anything inside that closure can read it via `use_context`:

```rust
use slt::{Color, Context};

struct AppContext {
    username: String,
    show_debug: bool,
}

fn main() -> std::io::Result<()> {
    slt::run(|ui| {
        let app = AppContext {
            username: "sb".into(),
            show_debug: true,
        };
        ui.provide(app, |ui| {
            render_home(ui);
        });
    })
}

fn render_home(ui: &mut Context) {
    let app = ui.use_context::<AppContext>();
    ui.text(format!("Hello, {}", app.username));
    if app.show_debug {
        ui.text("debug mode on").dim();
    }
}
```

How it behaves:

- **Scoped.** The value is alive only inside the body closure passed to `provide`. Outside that closure, `use_context::<AppContext>()` panics — there is no value.
- **Shadowing.** Nested `provide` calls of the same type shadow the outer one for the duration of the inner closure. Think of it as a LIFO stack, one stack per `TypeId`.
- **Optional reads.** `try_use_context::<T>() -> Option<&T>` returns `None` instead of panicking. Use it when a component should work both inside and outside the provider.

```rust
fn render_footer(ui: &mut Context) {
    if let Some(app) = ui.try_use_context::<AppContext>() {
        ui.text(format!("logged in as {}", app.username)).dim();
    } else {
        ui.text("anonymous").dim();
    }
}
```

When *not* to use context:

- The value lives in your top-level app struct and you already pass `&mut App` around. Do not duplicate it into context.
- The value is needed by exactly one render helper that is two scope levels away. A parameter is clearer.

A good heuristic: reach for `provide` when the same value is needed by three or more render helpers across two or more scope levels.

### Conditional Styling with `.with_if`

Conditional styling is everywhere — a row that highlights when selected, a label that turns red when invalid, a button that dims when disabled. Without help, this clutters the call site:

```rust
let mut t = ui.text("Status");
if is_error {
    t = t.bold().fg(Color::Red);
}
if is_selected {
    t = t.bg(Color::DarkGray);
}
```

`.with_if(cond, modifier)` compresses that into a chain that reads top-to-bottom:

```rust
ui.text("Status")
    .with_if(is_error, |t| {
        t.bold().fg(Color::Red);
    })
    .with_if(is_selected, |t| {
        t.bg(Color::DarkGray);
    });
```

The closure runs only when the condition is true. The modifier receives a mutable handle to the same builder, so you can chain any styling method inside.

For unconditional grouping — factoring shared modifier blocks out of multiple call sites — use `.with(modifier)`:

```rust
fn dim_label(t: &mut TextBuilder<'_>) {
    t.dim().italic();
}

ui.text("uptime: 3d 4h").with(dim_label);
ui.text("region: us-west-2").with(dim_label);
```

Both `.with` and `.with_if` are available on text and on container builders, so the same idiom works for `bordered`, `row`, `col`, etc.:

```rust
ui.bordered(Border::Single)
    .with_if(panel_focused, |c| {
        c.title("(focused)").pad(2);
    })
    .col(|ui| {
        // ...
    });
```

### When to use which

| Pattern | Use when | Example scenarios |
|---|---|---|
| Function + explicit args | Small components, fewer than 3-4 params | `metric_card`, `header_row`, `kv_pair` |
| `use_state_named` | Component has LOCAL state that should survive across frames | collapsible panel, pagination cursor, sort direction |
| `provide` / `use_context` | Values cross 3+ scope levels | theme, logged-in user, feature flags, request id |
| `.with_if` | Styling depends on a runtime condition | selected row, error text, disabled state, focused panel |
| `.with` | Factor shared style blocks out of multiple call sites | "dim label", "code-style text", "subtle border" |

### Anti-patterns

These look tempting but make code harder, not easier:

- **Reaching for context when a parameter is clearer.** If a value is used in one helper, just pass it. Context is for values that propagate, not for "I do not feel like typing the parameter."
- **Using `use_state_named` for app-level state.** Page-level state, selected tab, current user — these belong in your top-level `App` struct so that tests, persistence, and refactors stay sane. Named state is for state that is genuinely local to a component instance.
- **Reusing the same id by accident.** `expandable_card(ui, "card", ...)` called five times shares one boolean across all five cards. Pick ids that name the *instance*, not the *kind*.
- **Nesting `provide` purely to "override defaults."** If you find yourself wrapping every render helper in a fresh `provide`, the value should probably be a parameter or a method argument instead.

For full working apps that combine these patterns, see `COOKBOOK.md`. For the per-method reference on `State<T>` and friends, see `STATE_APIS.md`. For the single-file AI-oriented reference, see `COMPLETE_REFERENCE.md`.

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
ui.screen("home", &mut screens, |ui| {
    ui.text("Home").bold();
});

ui.screen("settings", &mut screens, |ui| {
    ui.text("Settings");
});
```

Use `screen(name, &mut screens, ...)` when you want declarative rendering that only runs for the active screen. Each screen gets isolated hook state and focus. Navigate from **inside** a `screen(...)` closure with `ui.push_screen(name)`,
`ui.pop_screen()`, or `ui.reset_screen()` when you need explicit navigation transitions. The state changes when the active closure returns, but the destination renders on the next frame so source and destination content never share one terminal buffer.

## Modal, overlay, and screen composition

```rust
ui.screen("home", &mut screens, |ui| {
    if ui.button("Settings").clicked {
        ui.push_screen("settings");
    }
});

ui.screen("settings", &mut screens, |ui| {
    if ui.button("Back").clicked {
        ui.pop_screen();
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
