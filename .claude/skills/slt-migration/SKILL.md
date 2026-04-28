---
name: slt-migration
description: Migrate Rust TUIs from ratatui (or cursive, Python textual) to SuperLightTUI v0.20. Use when porting an existing TUI codebase to SLT, or when the user asks "how do I do X from ratatui in SLT". Korean triggers "ratatui 마이그레이션", "SLT로 포팅", "이걸 SLT로".
---

# SLT Migration Skill (from ratatui / cursive / textual) — v0.20

This skill complements `.claude/skills/slt/SKILL.md` (authoring). Use this one to **port** an existing codebase. After migration switch to the `slt` skill for day-to-day work.

**Targets SLT v0.20.0** (`Cargo.toml: version = "0.20.0"`). Every API name below has been verified against `src/lib.rs` and `src/context/widgets_*.rs`. The v0.20 API consistency pass removed several v0.19 names — the "v0.20 removed APIs" table below lists every one. Do not migrate to a removed name.

## When to use

Trigger when:
- The user says "migrate from ratatui", "port from cursive", "ratatui equivalent in SLT", or "rewrite this TUI in SLT".
- A file in scope imports `ratatui`, `tui`, or `cursive`, or `Cargo.toml` lists them.
- The user is comparing libraries and wants concrete mappings.
- A Python `textual` project is being rewritten in Rust.

If starting fresh (no existing TUI), use the `slt` skill instead.

## v0.20 removed APIs (do NOT migrate to these)

If you see these in any third-party doc, blog post, or AI training data — they are GONE in v0.20. Use the replacement.

| Removed | Replacement |
|---|---|
| `gauge_w(r, w)` | `ui.gauge(r).width(w)` |
| `gauge_colored(r, c)` | `ui.gauge(r).color(c)` |
| `line_gauge_with(r, opts)` | `ui.line_gauge(r).<chain>` |
| `breadcrumb_sep(b, s)` | `ui.breadcrumb(b).separator(s)` |
| `breadcrumb_response(b)` / `breadcrumb_response_with(b, s)` | `ui.breadcrumb(b).show() -> BreadcrumbResponse` |
| `LineGaugeOpts` | `LineGauge<'_>` builder (chained) |
| `HighlightRange::single(i)` | `HighlightRange::line(i)` |
| `label_owned(s)` | `label(s)` (accepts `impl Into<String>`) |

## Mental model translation

Compact side-by-side — read once before writing any code.

| Aspect | ratatui | SLT |
|---|---|---|
| Loop ownership | You own it: `terminal.draw(\|f\| ...)` | `slt::run(\|ui\| ...)` owns it |
| Layout | `Layout::default().constraints(...).split(area)` → `Vec<Rect>` | `ui.row` / `ui.col` / `ui.bordered(...).col(...)` (flexbox) |
| Widget API | Build a widget value, then `f.render_widget(widget, rect)` | Method on `&mut Context`: `ui.text(...)`, `ui.list(&mut state)` |
| State | App struct outside the closure | Plain Rust variables outside the closure (same idiom) |
| Render mode | Retained mental model — recompute every draw | Immediate — describe every frame |
| Hit testing | Manual `Rect` math | `Response { clicked, right_clicked, hovered, focused, gained_focus, lost_focus, rect }` |
| Setup / teardown | `enable_raw_mode`, `EnterAlternateScreen`, panic hook by you | All handled by `slt::run` (incl. terminal-restoring panic hook) |
| Threading shared state | `&mut App` parameter chain | `ui.provide(value, \|ui\| ...)` + `ui.use_context::<T>()` |

**cursive**: callback-based — `siv.add_global_callback`, layered views, owns its loop. SLT has no callbacks; check inputs and branch in the closure.

**textual** (Python): retained App+Widget classes, CSS-like styling, `compose()` yields widgets, `on_*` event handlers. SLT replaces all of that with one closure and chained method calls.

## ratatui → SLT mapping

### Run loop

ratatui (typical):
```rust
let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
terminal::enable_raw_mode()?;
crossterm::execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
loop {
    terminal.draw(|f| ui(f, &mut app))?;
    if let Event::Key(key) = event::read()? {
        if key.code == KeyCode::Char('q') { break; }
        // dispatch to app
    }
}
crossterm::execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
terminal::disable_raw_mode()?;
```

SLT v0.20:
```rust
fn main() -> std::io::Result<()> {
    let mut app = App::default();
    slt::run(|ui| {
        if ui.key('q') || ui.key_code(KeyCode::Esc) { ui.quit(); }
        render(ui, &mut app);
    })
}
```

`slt::run` enters alternate screen, enables raw mode, installs a panic hook restoring terminal state, and tears down on exit. Variants:
- `slt::run_with(RunConfig::default().mouse(true).theme(Theme::dark()), \|ui\| ...)` — mouse capture, custom theme
- `slt::run_inline(rows, \|ui\| ...)` — render below the prompt, no alt screen
- `slt::run_static(\|ui\| ...)` — append-only scrollback (use with `ui.static_log(...)`)
- `slt::run_async::<Message>(\|ui, messages\| ...)` — tokio integration (`async` feature)

### Widget mapping (top ratatui widgets → v0.20 SLT)

Every SLT method below is verified against `src/context/widgets_*.rs`.

| ratatui | SLT v0.20 |
|---|---|
| `Block::default().borders(Borders::ALL).title("X")` | `ui.bordered(Border::Rounded).title("X").col(\|ui\| ...)` |
| `Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)` | `ui.bordered(Border::Rounded).col(...)` |
| `Block::default().borders(Borders::TOP \| Borders::BOTTOM)` | `ui.bordered(Border::Single).border_sides(BorderSides::vertical()).col(...)` |
| `Paragraph::new("text")` | `ui.text("text")` |
| `Paragraph::new("text").wrap(Wrap { trim: true })` | `ui.text("text").wrap()` |
| `Paragraph::new("text").alignment(Alignment::Center)` | `ui.text("text").text_center()` |
| `List::new(items).highlight_style(...)` | `ui.list(&mut ListState)` — items via `ListState::set_items(...)` |
| `Table::new(rows).widths(&[...])` | `ui.table(&mut TableState)` — rows via `TableState::set_rows(...)`, auto column widths |
| `Tabs::new(titles).select(idx)` | `ui.tabs(&mut TabsState)` — labels via `TabsState::new(["Files", "Settings"])` |
| `Gauge::default().percent(75).label("75%")` | **`ui.gauge(0.75).label("75%").width(24).color(Color::Green)`** (v0.20 builder) |
| `Gauge::default().percent(75)` (no label) | `ui.gauge(0.75)` or `ui.progress(0.75)` |
| `LineGauge::default().ratio(0.6).label("Memory")` | `ui.line_gauge(0.6).label("Memory").filled('━').empty('─').width(48)` |
| `BarChart::default().data(&[("a", 1), ("b", 2)])` | `ui.bar_chart(&[("a", 1.0), ("b", 2.0)], max_width)` (values are `f64`) |
| `Chart::new(datasets)` | `ui.chart(width, height, \|c\| { c.line(&data).color(c).label("X"); })` |
| `Sparkline::default().data(&[1, 2, 3])` | `ui.sparkline(&[1.0, 2.0, 3.0], width)` |
| `Span::styled("x", Style::default().fg(Color::Red))` | `ui.text("x").fg(Color::Red)` |
| `Line::from(vec![span1, span2])` | `ui.row(\|ui\| { ui.text("a"); ui.text("b"); })` or `ui.line(\|ui\| ...)` |
| `Clear` widget | usually unneeded — render at top-level or use `ui.modal(\|ui\| ...)` for overlays |

### v0.20-only widgets (no ratatui equivalent — use these on the migration target)

These are SLT additions; if your ratatui app hand-rolled them, replace with the built-in:

| Pattern | v0.20 SLT |
|---|---|
| Scrollable code/log with line numbers | `ui.scrollable_with_gutter(&mut scroll, GutterOpts::line_numbers(total, vp_h), \|ui, abs\| { ui.text(lines[abs]); })` |
| Search highlights in scrollable | `ScrollState::set_highlights(&[HighlightRange::line(7), HighlightRange::span(15, 3)])` + `state.highlight_next/previous` |
| Two-pane resizable layout | `ui.split_pane(&mut SplitPaneState::new(0.5), \|ui\| ..., \|ui\| ...)` (horizontal) or `ui.vsplit_pane(...)` (vertical) |
| Breadcrumb with click feedback | `let r = ui.breadcrumb(&segs).separator(" › ").show(); if let Some(i) = r.clicked_segment { ... }` |
| Tooltips | `if ui.button("X").on_hover(ui, "Save").clicked { ... }` |
| Animated value | `let alpha = ui.animate_value("fade", target, 30);` (eased, 1-line) |
| Modal with focus trap (WCAG) | `ui.modal_with(ModalOptions { tab_trap: true }, \|ui\| ...)` (plain `ui.modal` keeps Esc-friendly behavior) |
| Per-subtree theme override | `ui.container().theme(custom_theme).col(\|ui\| ...)` |
| Theme density | `Theme::compact()` / `comfortable()` / `spacious()` instead of manual spacing |

### Custom widgets — Layer 3

ratatui custom `Widget` trait impls don't translate directly. Two options:

**Option A — function** (preferred for one-off widgets):
```rust
fn render_my_widget(ui: &mut Context, data: &MyData) { /* method calls */ }
```

**Option B — implement SLT's `Widget` trait** (for reusable libraries):
```rust
struct Label<'a> { text: &'a str }
impl<'a> slt::Widget for Label<'a> {
    type Response = slt::Response;
    fn ui(&mut self, ui: &mut slt::Context) -> Self::Response {
        ui.register_focusable();
        ui.text(self.text).bold();
        slt::Response::default()
    }
}
ui.add(Label { text: "hello" });
```

Note: SLT's `Widget` trait is different from ratatui's. ratatui's `Widget::render(self, area, buf)` is a stateless paint into a `Buffer`. SLT's `Widget::ui(&mut self, ui)` is an immediate-mode call that returns a `Response`.

### Layout mapping

ratatui:
```rust
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)])
    .split(area);
f.render_widget(header, chunks[0]);
f.render_widget(body, chunks[1]);
f.render_widget(footer, chunks[2]);
```

SLT:
```rust
ui.col(|ui| {
    ui.container().h(3).col(|ui| { /* header */ });
    ui.container().fill().col(|ui| { /* body — fills remaining (v0.20 fill() == grow(1)) */ });
    ui.container().h(1).col(|ui| { /* footer */ });
});
```

Constraint translation:

| ratatui | SLT |
|---|---|
| `Constraint::Length(3)` | `.h(3)` (col) or `.w(3)` (row) |
| `Constraint::Min(0)` | `.fill()` (v0.20) or `.grow(1)` |
| `Constraint::Min(n)` | `.min_h(n).fill()` (col) or `.min_w(n).fill()` (row) |
| `Constraint::Max(n)` | `.max_h(n)` or `.max_w(n)` |
| `Constraint::Percentage(50)` | `.h_pct(50)` or `.w_pct(50)` |
| `Constraint::Ratio(1, 3)` | `.h_ratio(1, 3)` or `.w_ratio(1, 3)` |
| `.margin(1)` on Layout | `.p(1)` on parent container |
| `.spacing(1)` between chunks | `.gap(1)` on parent `row` / `col` |

`Constraints` value type also exposes the v0.20 `WidthSpec` set: `Constraints::default().w_pct(50)`, `.w_ratio(1, 3)`, `.w_minmax(10, 30)` — see `examples/v020_widthspec.rs`.

### State mapping (re-exported via `slt::*`)

| ratatui / your code | SLT |
|---|---|
| `ListState` | `slt::ListState` (`set_items`, `set_filter`, `selected_item`, `visible_indices`) |
| `TableState` | `slt::TableState` (`set_rows`, `toggle_sort`, `sort_by`, `set_filter`, `next_page`, `prev_page`) |
| `TabsState` | `slt::TabsState` (`new(["Files", "Settings"])`, `selected_label`) |
| `ScrollbarState` / manual offset | `slt::ScrollState` + `ui.scrollable(&mut state).col(...)` or `ui.scrollable_with_gutter(...)` |
| your own `input: String` | `slt::TextInputState::with_placeholder("…")` (validators via `add_validator`) |
| your own `textarea: Vec<String>` | `slt::TextareaState::default().word_wrap(80)` |
| your own selection set | `slt::SelectState`, `slt::RadioState`, `slt::MultiSelectState` |
| tree view | `slt::TreeState`, `slt::DirectoryTreeState::from_paths(...)` |
| modal flag | `slt::Context::modal(\|ui\| ...)` or `modal_with(ModalOptions{tab_trap:true}, ...)` |
| toast queue | `slt::ToastState::default()` + `ui.notify(level, msg)` |
| log/history view | `slt::RichLogState::new()` (capped at 10000) or `RichLogState::new_unbounded()` |

### Threading state (CRITICAL for ratatui apps)

ratatui apps typically thread `&mut App` (or `&App`) through every render fn:

```rust
fn render(f: &mut Frame, app: &mut App) {
    render_header(f, app);
    render_body(f, app);
    render_footer(f, app);
}
```

In SLT v0.19+ replace **read-only** sharing with `provide`/`use_context`:

```rust
struct AppCtx { theme: slt::Theme, tick: u64, settings: Settings }

slt::run(|ui| {
    let ctx = AppCtx { theme: *ui.theme(), tick: ui.tick(), settings: app.settings.clone() };
    ui.provide(ctx, |ui| {
        render_header(ui);
        render_body(ui, &mut app.doc);   // writes still pass &mut explicitly
        render_footer(ui);
    });
});

fn render_header(ui: &mut slt::Context) {
    let ctx = ui.use_context::<AppCtx>();
    ui.text(format!("tick {}", ctx.tick));
}
```

Reserve explicit `&mut` parameters for **writes** (`&mut MyDocState`).

### Event mapping

| ratatui | SLT |
|---|---|
| `KeyCode::Char('q')` match | `if ui.key('q') { ... }` |
| `KeyCode::Esc` match | `if ui.key_code(KeyCode::Esc) { ... }` |
| `KeyModifiers::CONTROL + Char('c')` | `if ui.key_mod('c', KeyModifiers::CONTROL) { ... }` (Ctrl-C is also auto-handled by `slt::run`) |
| `MouseEventKind::Down(MouseButton::Left)` | `if let Some((x, y)) = ui.mouse_down() { ... }` or `Response.clicked` |
| `MouseEventKind::Down(MouseButton::Right)` | `Response.right_clicked` (v0.20) |
| `MouseEventKind::ScrollUp` | `if ui.scroll_up() { ... }` |
| Manual hit test | `if ui.button("X").clicked { ... }` (`Response.clicked` is a public field) |
| Focus events | `Response.gained_focus` / `Response.lost_focus` (v0.20) |
| `paste` event | `for s in ui.pastes() { ... }` |
| Sequence detection | `if ui.key_seq("gg") { ... }` |

**Modal-aware**: `ui.key()`, `ui.key_code()`, `ui.key_mod()` are filtered when a modal is open. For global shortcuts that bypass modals, use `ui.raw_key_code()` / `ui.raw_key_mod()`.

**Consume**: `ui.consume_key(c)` / `ui.consume_key_code(code)` mark events handled so child widgets don't re-process. Useful for global shortcuts taking precedence over text input.

### Style mapping

| ratatui | SLT |
|---|---|
| `Style::default().fg(Color::Red)` | `Style::new().fg(Color::Red)` |
| `Style::default().add_modifier(Modifier::BOLD)` | `Style::new().bold()` |
| `Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)` | `Style::new().fg(Color::Red).bold()` |
| `Style::default().bg(Color::Blue)` | `Style::new().bg(Color::Blue)` |
| Per-text styling: `Span::styled("x", style)` | Chain on the call: `ui.text("x").fg(Color::Red).bold()` |
| `Modifier::DIM` / `ITALIC` / `UNDERLINED` / `REVERSED` / `CROSSED_OUT` | `.dim()` / `.italic()` / `.underline()` / `.reversed()` / `.strikethrough()` |
| Conditional styling | `ui.text("x").with_if(is_error, \|t\| { t.bold().fg(Color::Red); })` (v0.19+) |

`Style` is `Copy` in both libraries — no clone needed.

### Color mapping

Both libraries have:
- 16 named colors: `Red`, `Green`, `Blue`, `Yellow`, `Cyan`, `Magenta`, `Black`, `White`, plus `LightRed`, `LightGreen`, etc.
- 256-color: `Color::Indexed(N)` ↔ `Color::Indexed(N)`
- 24-bit: `Color::Rgb(r, g, b)` ↔ `Color::Rgb(r, g, b)`
- `Color::Reset` in both

Differences:
- ratatui has `Color::Gray` and `Color::DarkGray` — SLT only has `Color::DarkGray`. Use `Color::Indexed(8)` (ANSI bright black) or `Color::Rgb(128, 128, 128)` for mid-gray.
- For semantic colors prefer `slt::palette::tailwind::*` (`BLUE.c500`, `RED.c700`) — same 11-shade scale across 22 palettes, identical to Tailwind CSS.

### Border type mapping

| ratatui `BorderType` | SLT `Border` |
|---|---|
| `BorderType::Plain` | `Border::Single` |
| `BorderType::Rounded` | `Border::Rounded` |
| `BorderType::Double` | `Border::Double` |
| `BorderType::Thick` | `Border::Thick` |
| `BorderType::QuadrantInside` / `QuadrantOutside` | no direct equivalent — use a dashed style or custom draw |

| ratatui `Borders` | SLT `BorderSides` |
|---|---|
| `Borders::ALL` | default — `ui.bordered(Border::Rounded)` draws all 4 |
| `Borders::TOP` | `BorderSides::top()` |
| `Borders::BOTTOM` | `BorderSides::bottom()` |
| `Borders::LEFT` / `RIGHT` | `BorderSides::left()` / `right()` |
| `Borders::TOP \| Borders::BOTTOM` | `BorderSides::vertical()` |
| `Borders::LEFT \| Borders::RIGHT` | `BorderSides::horizontal()` |

Use via `.border_sides(...)`: `ui.bordered(Border::Single).border_sides(BorderSides::vertical()).col(...)`.

### Theme

ratatui has no built-in theme. If you have ad-hoc `Color::*` constants, replace with `slt::Theme` and `ui.color(ThemeColor::Primary)` so themes can swap. v0.20 additions:
- `Theme::dark()` / `light()` (base)
- `Theme::compact()` / `comfortable()` / `spacious()` (density variants of dark)
- `Theme::dracula()` / `nord()` / `tokyo_night()` / `gruvbox_dark()` / `one_dark()` / `catppuccin()` / `solarized_dark()` / `solarized_light()`
- `ThemeBuilder::builder_from(Theme::nord())` — extend a preset
- `ContainerBuilder::theme(custom)` — per-subtree override

## cursive → SLT mapping

cursive is callback-driven. SLT replaces both pattern and event loop with the imperative closure model.

| cursive | SLT |
|---|---|
| `Cursive::default().run()` | `slt::run(\|ui\| { ... })` |
| `siv.add_global_callback(Key::Esc, \|s\| s.quit())` | `if ui.key_code(KeyCode::Esc) { ui.quit(); }` |
| `TextView::new("hello")` | `ui.text("hello")` |
| `EditView::new()` | `ui.text_input(&mut TextInputState)` |
| `SelectView::new().item("a", 0).item("b", 1)` | `ui.select(&mut SelectState)` |
| `Dialog::around(view).button("OK", \|s\| ...)` | `ui.modal(\|ui\| { ui.text(...); if ui.button("OK").clicked { ... } })` |
| `LinearLayout::vertical()` | `ui.col(\|ui\| ...)` |
| `LinearLayout::horizontal()` | `ui.row(\|ui\| ...)` |
| `siv.add_layer(view)` | render at top-level; for overlays use `ui.modal(...)` / `ui.overlay_at(anchor, \|ui\| ...)` |
| `Cursive::set_user_data(state)` | plain Rust variable outside the closure, captured by reference |

Mental shift: cursive callbacks fire on input. SLT's closure runs every frame. State updates are visible immediately.

## textual (Python) → SLT mapping

textual is class-based with reactive state and CSS. SLT is functional with plain variables.

| textual | SLT |
|---|---|
| `class App(App)` with `compose()` | `slt::run(\|ui\| { ... })` closure |
| `reactive` attributes | plain Rust variables outside the closure |
| CSS-like styling | `ThemeBuilder` + per-widget chains (`.fg(Color::Red).bold()`) |
| `Static("hello")` | `ui.text("hello")` |
| `Button("Click")` + `on_button_pressed` | `if ui.button("Click").clicked { ... }` inline |
| `Input(placeholder="...")` | `ui.text_input(&mut TextInputState::with_placeholder("..."))` |
| `DataTable` | `ui.table(&mut TableState)` |
| `ScrollableContainer` | `ui.scrollable(&mut ScrollState).col(\|ui\| ...)` or `scrollable_with_gutter` |
| `Container(...)` | `ui.bordered(...).col(...)` or `ui.container().col(...)` |
| `compose()` yielding child widgets | the closure body — order is layout order |
| Async event handlers | `slt::run_async` (`async` feature) returns a `tokio::sync::mpsc::Sender` |
| CSS animations | `slt::Tween` / `slt::Spring` / `slt::Keyframes` / `ui.animate_value("id", target, ticks)` |

## Common migration pitfalls

- **"I have a struct that implements `Widget` trait."** — drop ratatui's. Either rewrite as `fn render_my_widget(ui: &mut Context, data: &MyData)`, or implement `slt::Widget` (different shape — see Custom widgets section).
- **"My App has a `draw(&mut self, frame: &mut Frame)` method."** — convert to `fn render(ui: &mut Context, app: &mut App)` and call from `slt::run(\|ui\| render(ui, &mut app))`.
- **"ratatui `ListState` lives across frames."** — same in SLT. `let mut list = ListState::new();` outside the closure; pass `&mut list` to `ui.list(&mut list)` each frame.
- **"I want raw crossterm events."** — prefer `ui.key()` / `ui.key_code()` / `ui.key_mod()` / `ui.mouse_down()` / `ui.scroll_up()`. Raw `ui.events()` is for advanced cases (key release, paste handling, custom modifier matching). For modal-aware globals use `ui.raw_key_code()` / `ui.raw_key_mod()`.
- **"I have heavy custom layout math (`.split()` arithmetic on `Rect`)."** — try `ui.row` / `ui.col` + `.fill / .h / .w / .h_pct / .w_pct / .align / .justify` first. Flexbox handles 95% of cases. Drop to `ui.container().draw(\|buf, rect\| { ... })` only when flexbox can't express it. The `draw` closure must be `'static` (deferred execution).
- **"`Constraint::Percentage(50)` is everywhere."** — `.w_pct(50)` (row child) or `.h_pct(50)` (col child). Both take `u8`.
- **"I use `Layout::default().margin(1).split(area)`."** — `.p(1)` on the parent container.
- **"I check `Response.rect` immediately."** — SLT layout runs *after* the closure. Frame 1 returns zero `Rect`. Guard with `if ui.tick() > 0 { ... }`.
- **"`Borders::ALL`."** — SLT default. `ui.bordered(Border::Rounded)` draws all 4 sides. Subset via `.border_sides(BorderSides::vertical())`.
- **"I want `Color::Gray`."** — doesn't exist. Use `Color::Indexed(8)` or `Color::Rgb(128, 128, 128)`. Or pull from `palette::tailwind::SLATE.c500`.
- **"My ratatui app calls `terminal.clear()` between frames."** — don't. SLT diffs the buffer and only emits changed cells. Manual clear breaks the diff and causes flicker.
- **"My panic hook restores raw mode."** — drop it. `slt::run` installs one on first call.
- **"I'm threading `&App` through every render fn."** — replace read-only sharing with `ui.provide(ctx, \|ui\| ...)` + `ui.use_context::<AppCtx>()`. Keep `&mut` for writes.
- **"My ratatui Gauge has no label."** — use `ui.gauge(0.75)` (no label) or `ui.progress(0.75)` (display widget, returns `&mut Self`).

## Migration workflow

1. **Inventory ratatui widgets used.**
   ```sh
   grep -rn "render_widget\|f\.render_widget" src/
   grep -rn "Block::\|Paragraph::\|List::\|Table::\|Tabs::\|Gauge::\|BarChart::\|Chart::\|Sparkline::" src/
   ```
   Map each to an SLT method via the tables above.

2. **Convert the run loop.** Replace `Terminal::new` setup + draw loop + `disable_raw_mode` teardown with one of `slt::run`, `slt::run_with`, `slt::run_inline`, or `slt::run_async`.

3. **Move state out of the draw closure.** Most ratatui apps already do this. Keep the same shape — your `App` struct now feeds into one SLT closure.

4. **Replace layout splitters.** Each `Layout::default().constraints(...).split(area)` becomes nested `ui.row` / `ui.col` + `.fill / .h / .w / .h_pct / .w_pct / .align / .justify`. `.gap(n)` instead of `.spacing(n)`, `.p(n)` instead of `.margin(n)`.

5. **Replace each `f.render_widget(...)` with the SLT method.** Convert widget by widget. Verify any uncertain method via the v0.20 mapping table or grep `src/context/widgets_*.rs`.

6. **Adopt v0.20 builders.** Where the old code hand-rolled gauges, breadcrumbs, scrollable-with-line-numbers, or split panes, use the v0.20 builders/opts directly. They handle hit-testing and accessibility.

7. **Replace event handling.** Convert raw `crossterm::event::read()` matches to `ui.key()`, `ui.key_code()`, `ui.key_mod()`, `ui.mouse_down()`, `ui.scroll_up()`. Drop manual hit-testing in favor of `Response.clicked` / `right_clicked` / `hovered` / `gained_focus` / `lost_focus`.

8. **Replace `&App` threading with `provide`/`use_context`** for read-only shared state.

9. **Run `cargo check` and fix one widget at a time.** Add tests with `slt::TestBackend::new(80, 24).render(\|ui\| ...)` once a section compiles.

After everything compiles, run the full quality gate (project `CLAUDE.md`):
`cargo fmt -- --check`, `cargo check --all-features`, `cargo clippy --all-features -- -D warnings`, `cargo test --all-features`, `cargo check --examples --all-features`.

## Things SLT v0.20 doesn't have a direct equivalent for

Be honest with the user — these need workarounds:

- **ratatui `Canvas` braille drawing primitive.** SLT has `ui.canvas(width, height, \|cv\| { cv.line(...); cv.circle(...); })` but the API takes a `CanvasContext` closure, not a value-typed widget. Custom point/line drawing logic needs a small rewrite. See `src/context/widgets_viz.rs:1289`.
- **ratatui `Wrap { trim: true }` exact semantics.** SLT wraps via container width and `.wrap()` on text; trim-leading-whitespace behavior isn't identical. Test wrap-heavy text manually.
- **cursive's deep view layering / multiple modal stacks.** SLT supports `ui.modal(...)` / `ui.modal_with(...)` / `ui.overlay_at(anchor, \|ui\| ...)` but not arbitrary nested view managers. Most uses fold into `if state.show_modal { ui.modal(\|ui\| ...) }`.
- **textual's CSS hot reload.** Themes are Rust values (no hot reload). Use `cargo watch -x run` for fast iteration.

If a feature genuinely doesn't map, tell the user — don't fake it.

## References

- `.claude/skills/slt/SKILL.md` — SLT authoring skill (use after migration is done).
- `.claude/skills/slt/REFERENCES.md` — feature flags, v0.20 surface, doc pointers.
- `examples/v020_*.rs` — runnable v0.20 demos (gauge, breadcrumb, scrollable_with_gutter, split_pane, etc.).
- `tests/v020_*.rs` — regression tests showing canonical TestBackend + sequence patterns.
- `src/lib.rs` — authoritative public re-exports.
- `src/context/widgets_*.rs` — authoritative widget signatures.
- ratatui repo: <https://github.com/ratatui-org/ratatui>
- cursive repo: <https://github.com/gyscos/cursive>
- textual repo: <https://github.com/Textualize/textual>
