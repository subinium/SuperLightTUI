# Coming from ratatui

This guide maps what you already know from ratatui onto SLT: the run loop,
layout, styling, widgets, state, events and tests. It ends with the
`ratatui` feature, which lets you keep existing ratatui widgets inside an SLT
app while you port the rest one region at a time.

## The mental model

Both libraries redraw the whole UI every frame and diff a cell buffer, so the
rendering model is closer than it looks. The difference is who owns what:

| | ratatui | SLT |
|---|---|---|
| Loop | You own it: `terminal.draw(\|f\| ..)` plus your own `event::read()` | `slt::run(\|ui\| ..)` owns it and calls your closure once per frame |
| Setup / teardown | raw mode, alternate screen, mouse capture, panic hook | done by `slt::run`, including a terminal-restoring panic hook |
| Layout | `Layout::split(area)` returns `Rect`s you hand to widgets | nested `ui.row` / `ui.col` containers, laid out by flexbox after your closure returns |
| Widgets | build a value, then `f.render_widget(widget, rect)` | call a method on the context: `ui.text(..)`, `ui.list(&mut state)` |
| Input | match `crossterm` events, hit-test `Rect`s by hand | poll `ui.key(..)`, read the widget's `Response` (`clicked`, `hovered`, `changed`, ...) |
| Focus | your own index | Tab / Shift+Tab focus ring built in; focused widgets handle their own keys |

There is no `Terminal`, `Frame` or backend type to set up. Your closure
describes the screen, and SLT records it, lays it out, diffs the result and
writes only the changed cells. Because layout runs after the closure,
`Response.rect` and hit-testing use the last frame's geometry; see the
[previous-frame rule](../guide/02-state.md#the-previous-frame-rule).

## The app skeleton

A typical ratatui app:

<!-- slt-check-skip: ratatui code shown for comparison -->
```rust
fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut state = ListState::default().with_selected(Some(0));
    let items = ["Inbox", "Drafts", "Sent"];
    loop {
        terminal.draw(|f| {
            let list = List::new(items)
                .block(Block::bordered().title("Mail"))
                .highlight_style(Style::new().reversed());
            f.render_stateful_widget(list, f.area(), &mut state);
        })?;
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Down => state.select_next(),
                KeyCode::Up => state.select_previous(),
                _ => {}
            }
        }
    }
    ratatui::restore();
    Ok(())
}
```

The same app in SLT. The list handles Up/Down (and `j`/`k`) itself while it
has focus, and mouse clicks select rows with no configuration:

<!-- slt-check: {"name": "ratatui_skeleton", "features": ["crossterm"], "context": "module"} -->
```rust
use slt::prelude::*;

fn main() -> std::io::Result<()> {
    let mut mail = ListState::new(vec!["Inbox", "Drafts", "Sent"]);

    slt::run(|ui: &mut Context| {
        if ui.key('q') {
            ui.quit();
        }
        ui.bordered(Border::Rounded).title("Mail").grow(1).col(|ui| {
            ui.list(&mut mail);
        });
        ui.text(format!("selected: {}", mail.selected_item().unwrap_or("-")))
            .dim();
    })
}
```

`use slt::prelude::*;` brings in `Context`, the run entry points, the style
types, `KeyCode` and the common state structs. It does not re-export
`crossterm`, so it never shadows your own crossterm dependency during a port.
Other entry points: `slt::run_with(RunConfig, ..)` for configuration,
`slt::run_inline(rows, ..)` to render below the prompt without the alternate
screen, and `slt::run_async` with the `async` feature. See
[runtime modes](../guide/09-runtime-modes.md).

## Layout

Where ratatui splits a `Rect` with
`Layout::vertical([Length(3), Min(0), Length(1)])` and renders into the
pieces, SLT nests containers and lets flexbox size them:

<!-- slt-check: {"name": "ratatui_layout", "features": [], "context": "ui"} -->
```rust
ui.container().grow(1).col(|ui| {
    ui.container().h(3).col(|ui| {
        ui.text("header");
    });
    ui.container().grow(1).gap(1).row(|ui| {
        ui.container().w_pct(30).col(|ui| {
            ui.text("sidebar");
        });
        ui.container().fill().col(|ui| {
            ui.text("main");
        });
    });
    ui.container().h(1).col(|ui| {
        ui.text("footer");
    });
});
```

| ratatui | SLT |
|---|---|
| `Layout::vertical` / `horizontal` | `ui.col(..)` / `ui.row(..)`, or `ui.container().<chain>.col(..)` |
| `Constraint::Length(n)` | `.h(n)` in a column, `.w(n)` in a row |
| `Constraint::Fill(1)` / `Min(0)` | `.fill()` (same as `.grow(1)`); `.grow(n)` for weights |
| `Constraint::Min(n)` | `.min_h(n).fill()` / `.min_w(n).fill()` |
| `Constraint::Max(n)` | `.max_h(n)` / `.max_w(n)` |
| `Constraint::Percentage(p)` | `.h_pct(p)` / `.w_pct(p)` |
| `Constraint::Ratio(a, b)` | `.h_ratio(a, b)` / `.w_ratio(a, b)` |
| `.margin(n)` | `.p(n)` on the parent container |
| `.spacing(n)` | `.gap(n)` on the parent container |
| `.spacing(-1)` (collapse shared borders) | `.gap_overlap(1)` |
| `Flex::Center` / `Flex::SpaceBetween` | `.justify(Justify::Center)` / `.space_between()` (main axis; `.center()` centers on the cross axis) |
| `Borders::TOP \| Borders::BOTTOM` | `.border_sides(BorderSides::horizontal())` |
| `BorderType::Plain` / `Rounded` / `Double` / `Thick` | `Border::Single` / `Rounded` / `Double` / `Thick` |

One difference to watch for: a fixed-size SLT child keeps its size and the
overflow is clipped unless you mark it `.shrink()`, rather than being
squeezed by the solver.

## Styling

ratatui composes `Style`, `Span`, `Line` and `Text` values. In SLT you chain
style onto the call that draws the text, and `ui.line(..)` lays out several
pieces on one row, the way a `Line` of `Span`s does:

<!-- slt-check: {"name": "ratatui_styling", "features": [], "context": "ui"} -->
```rust
// Span::styled("error", Style::new().fg(Color::Red).bold())
ui.text("error").fg(Color::Red).bold();

// Line::from(vec![Span::raw("status: "), Span::styled("ok", green)])
ui.line(|ui| {
    ui.text("status: ");
    ui.text("ok").fg(Color::Green);
});

// A reusable Style value works the same way as in ratatui.
let warn = Style::new().fg(Color::Yellow).italic();
ui.styled("disk almost full", warn);

// Paragraph::new(..).wrap(..).alignment(Alignment::Center)
ui.text("a long paragraph that wraps").wrap().text_center();
```

Modifiers map one to one: `.bold()`, `.dim()`, `.italic()`, `.underline()`,
`.reversed()`, `.strikethrough()`; `.with_if(cond, |t| ..)` applies a chain
conditionally. Colors are named the same, with one shift:
ratatui's `Color::Gray` is ANSI 7, which SLT calls `Color::White`, and
ratatui's `Color::White` (ANSI 15) is SLT's `Color::LightWhite`. ratatui has
no theme; prefer theme tokens such as `ui.color(ThemeColor::Primary)` over
hard-coded colors so `Theme::light()`, `Theme::nord()` and the other presets
apply. See [theming](../guide/06-theming.md).

## Widget mapping

| ratatui | SLT |
|---|---|
| `Paragraph::new(text)` | `ui.text(text)`, `.wrap()` for wrapping |
| `Block` | `ui.bordered(Border::Rounded).title(..).col(..)` |
| `List` + `ListState` | `ui.list(&mut ListState)` |
| `Table` + `TableState` | `ui.table(&mut TableState)`: column widths from content, sorting, filtering, paging |
| `Tabs` | `ui.tabs(&mut TabsState)` |
| `Gauge` | `ui.gauge(ratio).label(..)`, or `ui.progress(ratio)` without a label |
| `LineGauge` | `ui.line_gauge(ratio).label(..).filled('━').empty('─')` |
| `Sparkline` | `ui.sparkline(&[f64], width)` |
| `BarChart` | `ui.bar_chart(&[(&str, f64)], max_width)` |
| `Chart` + `Dataset` | `ui.chart(\|c\| { c.line(&points).label(..); }, width, height)` |
| `Canvas` | `ui.canvas(width, height, \|cv\| ..)` |
| `Scrollbar` + manual offset | `ui.scrollable(&mut ScrollState).col(..)` and `ui.scrollbar(&mut state)` |
| `Clear` + a popup `Rect` | `ui.modal(\|ui\| ..)`, `ui.overlay_at(anchor, \|ui\| ..)` |
| custom `Widget` impl | a plain `fn panel(ui: &mut Context, ..)`, or `impl slt::Widget` |

SLT also ships widgets most ratatui apps hand-roll: `text_input`, `textarea`,
`button`, `checkbox`, `select`, `tree`, `split_pane`, `command_palette`,
toasts and more. The [widget catalog](../reference/widgets.md) lists them all.

<!-- slt-check: {"name": "ratatui_widgets", "features": [], "context": "module"} -->
```rust
use slt::*;

fn dashboard(ui: &mut Context, tabs: &mut TabsState, procs: &mut TableState) {
    ui.tabs(tabs);
    ui.gauge(0.75).label("CPU 75%").color(Color::Green);
    ui.line_gauge(0.6).label("Memory").filled('━').empty('─');
    ui.sparkline(&[1.0, 4.0, 2.0, 6.0, 3.0], 20);
    ui.bar_chart(&[("api", 12.0), ("db", 30.0)], 24);
    let points = [(0.0, 1.0), (1.0, 3.0), (2.0, 2.0)];
    ui.chart(|c| { c.line(&points).color(Color::Cyan).label("ms"); }, 40, 10);
    ui.table(procs);
}
```

Gauges render when the builder is dropped; call `.show()` when you need the
response. The states are built once, outside the closure:
`TabsState::new(vec!["Overview", "Processes"])`,
`TableState::new(vec!["PID", "Name"], rows)`.

## State

Widget state lives outside the closure, exactly as `ListState` lives outside
`terminal.draw` in ratatui. Pass `&mut state` each frame.

| ratatui / your code | SLT |
|---|---|
| `ListState` + your item `Vec` | `ListState::new(items)`: holds the items and selection (`selected_item`, `selected_index`, `set_items`, `set_filter`) |
| `TableState` + your rows | `TableState::new(headers, rows)` (`selected_row`, `set_rows`, `toggle_sort`, `set_filter`, `next_page`) |
| tab index | `TabsState::new(labels)` (`selected_label`) |
| `ScrollbarState` / scroll offset | `ScrollState` |
| your own `input: String` | `TextInputState` |
| `&mut App` threaded through every render fn | keep `&mut` for writes; share read-only data with `ui.provide(value, \|ui\| ..)` and `ui.use_context::<T>()` |

For small UI-local state SLT also has hooks (`ui.use_state(|| init)`), which
ratatui does not. The [state guide](../guide/02-state.md) covers
when to use each, and [state APIs](../reference/state.md) lists every state
struct.

## Events

There is no event loop to write. Keys and mouse events are polled inside the
closure, and each widget reports what happened to it in its `Response`:

<!-- slt-check: {"name": "ratatui_events", "features": [], "context": "module"} -->
```rust
use slt::*;

fn toolbar(ui: &mut Context, saved: &mut u32) {
    // match key.code { KeyCode::Char('q') => .., KeyCode::Esc => .. }
    if ui.key('q') || ui.key_code(KeyCode::Esc) {
        ui.quit();
    }
    // KeyModifiers::CONTROL + Char('s')
    if ui.shortcut(KeyCode::Char('s'), KeyModifiers::CONTROL) {
        *saved += 1;
    }
    // Manual Rect hit-testing becomes a Response field.
    if ui.button("Save").variant(ButtonVariant::Primary).clicked {
        *saved += 1;
    }
    // Button is #[must_use]: read a field or call .show().
    let help = ui.button("Help").show();
    if help.right_clicked || help.gained_focus {
        ui.text("Help: press q to quit").dim();
    }
}
```

`ui.key_mod('c', KeyModifiers::CONTROL)`, `ui.mouse_down()`,
`ui.scroll_up()`, `ui.paste()` and `ui.key_chord("gg")` cover the rest of a
typical crossterm `match`; `ui.events()` is there for anything unusual.

Rules that differ from a hand-written crossterm loop:

- **The mouse is on by default.** `slt::run` enables clicks, the wheel and
  drags (`MouseMode::Drag` locally, `MouseMode::Click` over SSH). ratatui's
  `EnableMouseCapture` also reports every pointer move; the SLT equivalent is
  `RunConfig::default().mouse_mode(MouseMode::Hover)`, which `hovered` and
  tooltips need. `MouseMode::Off` keeps native terminal selection.
- **Focused inputs keep their keys.** `ui.key('q')` stays `false` while a
  `text_input` has focus, in any call order, so typing `q` into a search box
  does not quit. `ui.raw_key('q')` fires regardless.
- **Ctrl+C quits** by default, and the terminal is restored on exit or panic.
- **Focus is built in.** Tab and Shift+Tab move between focusable widgets;
  Enter and Space activate buttons. `ui.focus_by_name(..)` moves focus from
  code.

[Interaction](../guide/04-interaction.md) covers the `Response` fields,
hover, drag and click-through rules.

## Custom widgets

A ratatui `Widget` paints into a `Buffer` at a `Rect`. In SLT a custom widget
is usually just a function that calls other widgets. For a reusable type,
implement `slt::Widget`, whose `ui` method runs in immediate mode and returns
a response:

<!-- slt-check: {"name": "ratatui_custom_widget", "features": [], "context": "module"} -->
```rust
use slt::*;

struct StatusBadge {
    ok: bool,
}

impl Widget for StatusBadge {
    type Response = Response;

    fn ui(&mut self, ui: &mut Context) -> Response {
        let color = if self.ok { Color::Green } else { Color::Red };
        ui.bordered(Border::Rounded).border_fg(color).px(1).col(|ui| {
            ui.text("api").fg(color).bold();
        })
    }
}

fn header(ui: &mut Context) {
    if ui.widget(&mut StatusBadge { ok: true }).clicked {
        ui.text("api is healthy");
    }
}
```

When a region really needs cell-level drawing, `ui.container().draw(..)`
hands you SLT's `Buffer` and the laid-out `Rect`. The closure is deferred
until after layout, so it must be `'static`.

## Testing

ratatui's `TestBackend` becomes SLT's `TestBackend`, which also drives input,
focus and multi-frame interactions:

<!-- slt-check: {"name": "ratatui_test", "features": [], "context": "module"} -->
```rust
use slt::{Context, KeyCode, ListState, TestBackend};

fn list_moves_on_down() {
    let mut tb = TestBackend::new(30, 6);
    let mut mail = ListState::new(vec!["Inbox", "Drafts", "Sent"]);
    let mut app = |ui: &mut Context| {
        ui.list(&mut mail);
    };
    tb.step(vec![], &mut app).press(KeyCode::Down, &mut app);
    tb.assert_contains("Drafts");
    assert_eq!(mail.selected_item(), Some("Drafts"));
}
```

`EventBuilder` builds key, click, drag, scroll and paste events for
`step(..)`. See the [testing guide](../guide/08-testing.md).

## Incremental migration with the `ratatui` feature

A large app does not have to move in one step. The opt-in `ratatui` feature
renders ratatui widgets inside SLT containers:

```toml
[dependencies]
superlighttui = { version = "0.26", features = ["ratatui"] }
ratatui = "0.30" # or `ratatui-widgets = "0.3"` for just the stock widgets
```

The feature depends only on `ratatui-core` 0.1 (the widget and buffer half
of ratatui 0.30), not on a ratatui backend, and is not part of `full`. SLT
owns layout, input, focus and scrolling; the ratatui widget paints into the
region SLT laid out:

<!-- slt-check-skip: needs the ratatui-widgets (or ratatui) crate as a dependency -->
```rust
use ratatui::widgets::{Block, Paragraph};

let log = format!("frame {}", ui.tick());
ui.container().h(5).ratatui_widget(Paragraph::new(log).block(Block::bordered()));
```

Anything that implements ratatui's `Widget` works, including ratatui's own
`Line` and `Text`, which `slt::ratatui_compat` re-exports through
`ratatui_core`. `.ratatui(|area, buf| ..)` gives you the raw ratatui `Rect`
and `Buffer` for code that calls `render` itself:

<!-- slt-check: {"name": "ratatui_bridge", "features": ["ratatui"], "context": "module"} -->
```rust
use slt::Context;
use slt::ratatui_compat::ratatui_core::style::{Color, Style};
use slt::ratatui_compat::ratatui_core::text::{Line, Span};

fn legacy_status(ui: &mut Context) {
    // A ratatui `Line` rendered as a widget.
    let line = Line::from(vec![
        Span::raw("status: "),
        Span::styled("ok", Style::new().fg(Color::Green)),
    ]);
    ui.container().h(1).ratatui_widget(line);

    // The closure is deferred until after layout: move owned data in.
    let text = format!("frame {}", ui.tick());
    ui.container().h(1).ratatui(move |area, buf| {
        buf.set_string(area.x, area.y, &text, Style::new().fg(Color::Cyan));
    });
}
```

The bridge is render-only: input, focus and `StatefulWidget` state stay in
SLT code. `slt::Color`, `Modifiers` and `Style` convert to and from the
ratatui types with `From`; blink variants, `HIDDEN` and `OVERLINE` are lossy
(see the `ratatui_compat` module docs). `cargo run --example ratatui_bridge --features ratatui`
shows a ratatui `Gauge`, `Sparkline` and a scrolled `Paragraph` inside an SLT
layout. Port each bridged region to native SLT widgets when convenient.

A practical order for a port:

1. Replace the terminal setup, draw loop and teardown with `slt::run`.
2. Wrap each stateless `render_widget` call in a bridged container so those
   regions keep rendering unchanged inside SLT layout.
3. Convert `Layout::split` code to `row` / `col` containers.
4. Replace widgets one at a time, starting with the ones that need input
   (lists, tables, text fields), since the bridge cannot give them focus.
5. Delete the crossterm event match once `ui.key(..)` and `Response` fields
   cover it.

## Performance

On the matched comparison harness, which drives both libraries through their
real per-frame path with the same visible output, SLT renders faster per frame
than ratatui. Methodology, workloads and current numbers are in
[performance](../internals/performance.md) and the
[ratatui comparison bench](../../bench/ratatui-compare/README.md).

## Next

- [Quick start](../guide/01-quick-start.md): the shortest path to a running app
- [State guide](../guide/02-state.md): app state, hooks and layout timing
- [Widget catalog](../reference/widgets.md): every widget with its signature
- [State APIs](../reference/state.md): every state struct and helper
- [Migration notes](../reference/migration.md): changes between SLT versions
