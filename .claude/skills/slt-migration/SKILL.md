---
name: slt-migration
description: Migrate Rust TUIs from ratatui (or cursive, Python textual) to SuperLightTUI. Use when porting an existing TUI codebase to SLT, or when the user asks "how do I do X from ratatui in SLT".
---

# SLT Migration Skill (from ratatui / cursive / textual)

This skill complements `.claude/skills/slt/SKILL.md` (authoring). Use this one
to *port* an existing codebase. After migration, switch to the `slt` skill for
day-to-day authoring.

Targets SLT v0.19.2. Every API name below has been verified against `src/lib.rs`
and `src/context/widgets_*.rs`. If a future SLT version moves something, the
`slt` skill's "grep before writing" rule still applies.

## When to use

Trigger when any of the following are true:
- The user says "migrate from ratatui", "port from cursive", "ratatui equivalent in SLT", or "rewrite this TUI in SLT".
- A file in scope imports `ratatui`, `tui`, `cursive`, or has a `Cargo.toml` listing them.
- The user is comparing libraries and wants concrete mappings.
- A Python `textual` project is being rewritten in Rust.

If the user is starting fresh (no existing TUI), use the `slt` skill instead.

## Mental model translation

Compact side-by-side. Read once before writing any code.

| Aspect | ratatui | SLT |
|---|---|---|
| Loop ownership | You own the loop, `terminal.draw(\|f\| ...)` | `slt::run(\|ui\| ...)` owns it |
| Layout | `Layout::default().constraints(...).split(area)` returns `Vec<Rect>` | `ui.row \| ui.col \| ui.bordered(...).col(...)` (flexbox) |
| Widget API | Build a widget value, then `f.render_widget(widget, rect)` | Method call on `&mut Context`: `ui.text(...) / ui.list(&mut state) / ui.table(&mut state)` |
| State | App struct outside the closure | Plain Rust variables outside the closure (same idiom) |
| Mode | Retained — recompute every draw | Immediate — describe every frame |
| Hit testing | None built in (you do math on `Rect`) | `Response { clicked, hovered, focused, rect }` returned from each widget |
| Setup / teardown | You do `enable_raw_mode`, `EnterAlternateScreen`, panic hook | `slt::run` handles all of it (including a panic hook restoring terminal state) |

**cursive**: callback-based — `siv.add_global_callback`, views added to layers, runs its own loop. SLT has no callbacks; check inputs and branch in the closure.

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
        // ... dispatch to app
    }
}
crossterm::execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
terminal::disable_raw_mode()?;
```

SLT equivalent:
```rust
fn main() -> std::io::Result<()> {
    let mut app = App::default();
    slt::run(|ui| {
        if ui.key('q') { ui.quit(); }
        render(ui, &mut app);
    })
}
```

`slt::run` enters the alternate screen, enables raw mode, installs a panic hook
that restores terminal state, and tears everything down on exit. Use
`slt::run_with(RunConfig::default().mouse(true), ...)` for mouse capture, or
`slt::run_inline(height, ...)` to render below the prompt without alternate
screen.

### Widget mapping (top ratatui widgets)

Every SLT method below has been confirmed to exist in `src/context/widgets_*.rs`.

| ratatui | SLT |
|---|---|
| `Block::default().borders(Borders::ALL).title("X")` | `ui.bordered(Border::Rounded).title("X").col(\|ui\| { ... })` |
| `Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)` | `ui.bordered(Border::Rounded).col(...)` |
| `Block::default().borders(Borders::TOP \| Borders::BOTTOM)` | `ui.bordered(Border::Single).border_sides(BorderSides::horizontal()).col(...)` |
| `Paragraph::new("text")` | `ui.text("text")` |
| `Paragraph::new("text").wrap(Wrap { trim: true })` | `ui.text("text").wrap()` |
| `Paragraph::new("text").alignment(Alignment::Center)` | `ui.text("text").text_center()` |
| `List::new(items).highlight_style(...)` | `ui.list(&mut state)` — items live in `ListState` (`state.items`, `state.selected`) |
| `Table::new(rows).widths(&[...])` | `ui.table(&mut state)` — rows live in `TableState` (auto column widths) |
| `Tabs::new(titles).select(idx)` | `ui.tabs(&mut state)` — labels live in `TabsState` |
| `Gauge::default().percent(75)` | `ui.progress_bar(0.75, width)` (ratio in `[0.0, 1.0]`, width in cells) |
| `BarChart::default().data(&[("a", 1), ("b", 2)])` | `ui.bar_chart(&[("a", 1.0), ("b", 2.0)], max_width)` (values are `f64`) |
| `Chart::new(datasets)` | `ui.chart(...)` — see `src/context/widgets_viz.rs` line 1342 for full signature, or use `ChartBuilder` |
| `Sparkline::default().data(&[1, 2, 3])` | `ui.sparkline(&[1.0, 2.0, 3.0], width)` (values are `f64`, width required) |
| `Span::styled("x", Style::default().fg(Color::Red))` | `ui.text("x").fg(Color::Red)` |
| `Line::from(vec![span1, span2])` | `ui.row(\|ui\| { ui.text("a"); ui.text("b"); })` (or `ui.line(\|ui\| { ... })`) |
| `Clear` widget (clear background) | `ui.bordered(...).bg(Color::...).col(...)` or just rely on overlay/modal |

Widgets ratatui has but SLT doesn't ship as a built-in:
- ratatui `Canvas` for braille drawing — use `ui.canvas(...)` (different API; takes a closure that gets a `CanvasContext`).
- ratatui `Scrollbar` — use `ui.scrollable(&mut scroll).col(...)` then `ui.scrollbar(&scroll)` for the indicator.
- Custom `Widget` trait impls — these don't translate. Rewrite as a function: `fn render_my_widget(ui: &mut Context, data: &MyData)`.

### Layout mapping

ratatui:
```rust
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)])
    .split(area);
f.render_widget(header_widget, chunks[0]);
f.render_widget(body_widget, chunks[1]);
f.render_widget(footer_widget, chunks[2]);
```

SLT:
```rust
ui.col(|ui| {
    ui.container().h(3).col(|ui| { /* header */ });
    ui.container().grow(1).col(|ui| { /* body — fills remaining */ });
    ui.container().h(1).col(|ui| { /* footer */ });
});
```

Constraint translation:

| ratatui | SLT |
|---|---|
| `Constraint::Length(3)` | `.h(3)` (col) or `.w(3)` (row) |
| `Constraint::Min(0)` | `.grow(1)` |
| `Constraint::Min(n)` | `.min_h(n).grow(1)` (col) or `.min_w(n).grow(1)` (row) |
| `Constraint::Max(n)` | `.max_h(n)` (col) or `.max_w(n)` (row) |
| `Constraint::Percentage(50)` | `.h_pct(50)` (col) or `.w_pct(50)` (row) — takes `u8` |
| `Constraint::Ratio(1, 3)` | `.grow(1)` on each child (use equal grow weights), or `.w_pct(33)` |
| `.margin(1)` on Layout | `.pad(1)` on the parent container, or `.m(1)` on individual children |
| `.spacing(1)` between chunks | `.gap(1)` on the parent `row`/`col` |

**Important**: SLT does NOT have a `.shrink()` builder. To prevent a child from
expanding, just leave `.grow` unset (default 0) and set explicit `.h`/`.w`.
For pure flexbox, only `.grow(u16)` is exposed.

### State mapping

| ratatui | SLT (re-exported via `slt::*`) |
|---|---|
| `ListState` | `slt::ListState` (`state.items: Vec<String>`, `state.selected: usize`) |
| `TableState` | `slt::TableState` (`state.headers`, `state.rows`, `state.selected`, plus `set_filter`, `toggle_sort`, `next_page`) |
| `TabsState` | `slt::TabsState` (`state.labels`, `state.selected`) |
| `ScrollbarState` / manual offset | `slt::ScrollState` (`state.offset`, paired with `ui.scrollable(&mut state).col(...)`) |
| your own `input: String` | `slt::TextInputState` (`state.value`, `state.errors()`, `add_validator`) |
| your own `textarea: Vec<String>` | `slt::TextareaState` (multi-line) |

All state types are re-exported at crate root. Confirmed lines 146–153 of `src/lib.rs`.

### Event mapping

ratatui reads crossterm events directly. SLT exposes a higher-level event query
API that handles edge-detect, focus routing, and key consumption:

| ratatui | SLT |
|---|---|
| `KeyCode::Char('q')` match | `if ui.key('q') { ... }` |
| `KeyCode::Esc` match | `if ui.key_code(KeyCode::Esc) { ... }` |
| `KeyModifiers::CONTROL + Char('c')` | `if ui.key_mod('c', KeyModifiers::CONTROL) { ... }` (Ctrl-C is also auto-handled by `slt::run`) |
| `MouseEventKind::Down(MouseButton::Left)` | `if let Some((x, y)) = ui.mouse_down() { ... }` (or `ui.mouse_down_button(MouseButton::Left)`) |
| Manual hit test: `if click in rect { ... }` | `if ui.button("X").clicked { ... }` (`Response.clicked` is a public field) |
| `MouseEventKind::ScrollUp` | `if ui.scroll_up() { ... }` |

Key consumption: `ui.consume_key(c)` and `ui.consume_key_code(code)` mark the
event as handled so child widgets don't see it. Useful when you want global
shortcuts to take precedence over widget input.

For the rare case you need raw events, use `ui.events()` to iterate. Prefer the
helpers — they handle modal stacking, focus, and previous-frame state correctly.

### Style mapping

| ratatui | SLT |
|---|---|
| `Style::default().fg(Color::Red)` | `Style::new().fg(Color::Red)` |
| `Style::default().add_modifier(Modifier::BOLD)` | `Style::new().bold()` |
| `Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)` | `Style::new().fg(Color::Red).bold()` |
| `Style::default().bg(Color::Blue)` | `Style::new().bg(Color::Blue)` |
| Per-text styling: `Span::styled("x", style)` | Chain on the call: `ui.text("x").fg(Color::Red).bold()` |
| `Modifier::DIM` | `.dim()` |
| `Modifier::ITALIC` | `.italic()` |
| `Modifier::UNDERLINED` | `.underline()` |
| `Modifier::REVERSED` | `.reversed()` |
| `Modifier::CROSSED_OUT` | `.strikethrough()` |

`Style` is `Copy` in both libraries — no need to clone.

### Color mapping

Colors are nearly identical. Both libraries have:
- 16 named colors: `Color::Red`, `Color::Green`, `Color::Blue`, `Color::Yellow`, `Color::Cyan`, `Color::Magenta`, `Color::Black`, `Color::White`, plus `LightRed`, `LightGreen`, etc.
- 256-color palette: ratatui `Color::Indexed(N)` ↔ SLT `Color::Indexed(N)`.
- 24-bit: ratatui `Color::Rgb(r, g, b)` ↔ SLT `Color::Rgb(r, g, b)`.
- Reset: `Color::Reset` in both.

Differences:
- ratatui has `Color::Gray` and `Color::DarkGray` — SLT only has `Color::DarkGray`. Use `Color::Indexed(8)` or `Color::Rgb(128, 128, 128)` for mid-gray.
- SLT's bright white is `Color::LightWhite` (ratatui calls it `White` since their `White` is 7 and `Gray` is the not-bright version). Audit any color-dependent comparisons during port.

### Theme

ratatui has no built-in theme. If you have ad-hoc color constants, replace them
with `slt::Theme` and call `ui.color(ThemeColor::Primary)` etc. so the same
code respects light/dark mode and theme swaps. See `slt::ThemeBuilder` (a
`const fn` since v0.19.2 — themes can be defined at compile time).

## cursive → SLT mapping

cursive is callback-driven and runs its own event loop. SLT replaces both
patterns with the imperative closure model.

| cursive | SLT |
|---|---|
| `Cursive::default().run()` | `slt::run(\|ui\| { ... })` |
| `siv.add_global_callback(Key::Esc, \|s\| s.quit())` | `if ui.key_code(KeyCode::Esc) { ui.quit(); }` |
| `TextView::new("hello")` | `ui.text("hello")` |
| `EditView::new()` | `ui.text_input(&mut state)` (state is `TextInputState`) |
| `SelectView::new().item("a", 0).item("b", 1)` | `ui.select(&mut state)` (state is `SelectState`) |
| `Dialog::around(view).button("OK", \|s\| ...)` | `ui.modal(\|ui\| { ui.text(...); if ui.button("OK").clicked { ... } })` |
| `LinearLayout::vertical().child(...).child(...)` | `ui.col(\|ui\| { ... })` |
| `LinearLayout::horizontal()` | `ui.row(\|ui\| { ... })` |
| `siv.add_layer(view)` | Render at top-level in the closure (no layering needed unless using `ui.modal` / `ui.overlay`) |
| `Cursive::set_user_data(state)` | Plain Rust variable outside the closure, captured by reference |

The biggest mental shift: cursive callbacks fire on user input. SLT's closure
runs every frame. State updates are visible immediately because you re-render
each frame.

## textual (Python) → SLT mapping

textual is class-based with reactive state and CSS. SLT is functional with
plain variables.

| textual | SLT |
|---|---|
| `class App(App)` with `compose()` yielding widgets | A `slt::run(\|ui\| { ... })` closure with imperative method calls |
| `reactive` attributes | Plain Rust variables (`let mut count: i32 = 0;`) outside the closure |
| CSS-like styling | `ThemeBuilder` + per-widget chains (`.fg(Color::Red).bold()`) |
| `Static("hello")` | `ui.text("hello")` |
| `Button("Click")` + `on_button_pressed` | `if ui.button("Click").clicked { ... }` inline in the closure |
| `Input(placeholder="...")` | `ui.text_input(&mut TextInputState::with_placeholder("..."))` |
| `DataTable` | `ui.table(&mut TableState::new(...))` |
| `ScrollableContainer` | `ui.scrollable(&mut ScrollState::new()).col(\|ui\| ...)` |
| `Container(...)` | `ui.bordered(...).col(...)` or `ui.container().col(...)` |
| `compose()` yielding child widgets | The closure body — order is the layout order |
| Async event handlers | Either keep them async-side (read state in the SLT closure), or use `slt::run_async` (requires `async` feature, returns a `tokio::sync::mpsc::Sender`) |
| CSS animations | `slt::Tween` / `slt::Spring` / `slt::Keyframes` (see `slt::anim::*`) |

## Common migration pitfalls

- **"I have a struct that implements `Widget` trait."**
  Drop the trait. SLT widgets are method calls, not types. Rewrite as
  `fn render_my_widget(ui: &mut Context, data: &MyData)` and call it
  from inside the `slt::run` closure.

- **"My App has a `draw(&mut self, frame: &mut Frame)` method."**
  Convert to `fn render(ui: &mut Context, app: &mut App)` and call from
  the closure: `slt::run(|ui| render(ui, &mut app))`. Same data flow,
  different parameter shape. State stays outside the closure.

- **"ratatui `ListState` lives across frames."**
  Same in SLT — `slt::ListState` lives outside the closure. Pass `&mut state`
  to `ui.list(&mut state)` each frame. Up/Down arrow handling is built in.

- **"I want raw crossterm events directly."**
  Prefer `ui.key()`, `ui.key_code()`, `ui.key_mod()`, `ui.mouse_down()`,
  `ui.mouse_pos()`, `ui.scroll_up()/down()`. Raw `ui.events()` is for advanced
  cases (key release detection, paste handling beyond the helpers, custom
  modifier matching).

- **"I have heavy custom layout math (`.split()` arithmetic on `Rect`)."**
  Try `ui.row` / `ui.col` + `.grow(n)` / `.h(n)` / `.w(n)` / `.h_pct(50)` /
  `.align(...)` / `.justify(...)` first. Flexbox handles 95% of cases.
  Drop down to `ui.container().draw(|buf, rect| { ... })` only when
  flexbox can't express it. The `draw` closure must be `'static` (deferred
  execution).

- **"ratatui `Style` is `Copy`."**
  Same in SLT — `Style` derives `Copy`, no `.clone()` needed.

- **"`Constraint::Percentage(50)` is everywhere in my layout."**
  Map to `.w_pct(50)` (row child) or `.h_pct(50)` (col child). Both take `u8`.
  There is no `.width_pct` / `.height_pct` — those names don't exist.

- **"I use `Layout::default().margin(1).split(area)`."**
  Use `.pad(1)` on the parent container. `pad` adds inside-the-border padding,
  which is what `Layout::margin` effectively does in ratatui.

- **"I check `Response.rect` immediately."**
  SLT layout runs *after* the closure. On frame 1, `Response.rect` is a
  zero `Rect`. Guard with `if ui.tick() > 0 { ... }` for measurement-dependent
  logic. (See `docs/PREVIOUS_FRAME_GUIDE.md`.)

- **"`Borders::ALL`."**
  SLT does not have a `Borders::ALL` constant. `ui.bordered(Border::Rounded)`
  draws all four sides by default. To draw a subset, pass a
  `BorderSides` via `.border_sides(BorderSides::horizontal())` etc.

- **"I want `Color::Gray`."**
  Doesn't exist in SLT. Use `Color::Indexed(8)` (ANSI dim gray) or
  `Color::Rgb(128, 128, 128)`.

- **"My ratatui app calls `terminal.clear()` between frames."**
  Don't. SLT diffs the buffer each frame and only emits changed cells.
  Manually clearing breaks the diff and causes flicker.

- **"My panic hook restores raw mode."**
  Drop it. `slt::run` installs a panic hook on first call that disables
  raw mode and prints a clean panic header.

## Migration workflow

1. **Inventory ratatui widgets used.** From the project root:
   ```sh
   grep -rn "render_widget\|f\.render_widget" src/
   grep -rn "Block::\|Paragraph::\|List::\|Table::\|Tabs::\|Gauge::\|BarChart::\|Chart::\|Sparkline::" src/
   ```
   Map each to an SLT method via the table above.

2. **Convert the run loop.** Replace `Terminal::new` setup + draw loop +
   `disable_raw_mode` teardown with one of:
   - `slt::run(|ui| { ... })` — full-screen alt-screen mode (most apps).
   - `slt::run_with(RunConfig::default().mouse(true).theme(Theme::dark()), |ui| { ... })` — when you need mouse, custom theme, etc.
   - `slt::run_inline(height, |ui| { ... })` — render below the prompt (CLI tools, no alt screen).
   - `slt::run_async::<Message>(|ui, messages| { ... })` — tokio integration (requires `async` feature).

3. **Move state out of the draw closure.** ratatui apps usually already do this
   (`App` struct outside, `terminal.draw(|f| ui(f, &mut app))` inside). Keep
   the same shape — your `App` struct now feeds into a single SLT closure.

4. **Replace layout splitters.** Convert each
   `Layout::default().constraints(...).split(area)` to nested `ui.row` /
   `ui.col` + `.grow / .h / .w / .h_pct / .w_pct / .align / .justify`.
   Use `.gap(n)` instead of `.spacing(n)`, `.pad(n)` instead of `.margin(n)`.

5. **Replace each `f.render_widget(...)` with the SLT method.** Convert
   widget by widget in the order they're rendered. Use the widget mapping
   table; verify any uncertain method against `src/context/widgets_*.rs`
   (or use the `slt` skill's grep workflow).

6. **Replace event handling.** Convert raw `crossterm::event::read()`
   matches to `ui.key()`, `ui.key_code()`, `ui.key_mod()`, `ui.mouse_down()`,
   `ui.scroll_up/down()`. Drop your manual hit-testing in favour of
   `Response.clicked` returned from each widget.

7. **Run `cargo check` and fix one widget at a time.** Add tests with
   `slt::TestBackend::new(80, 24).render(|ui| { ... })` plus
   `tb.assert_contains("text")` once a section compiles. See
   `docs/TESTING.md` for event injection and multi-frame scenarios.

After everything compiles, run the full quality gate from project `CLAUDE.md`:
`cargo fmt -- --check`, `cargo check --all-features`,
`cargo clippy --all-features -- -D warnings`, `cargo test --all-features`,
`cargo check --examples --all-features`.

## Things SLT 0.19.2 doesn't have a direct equivalent for

Be honest with the user — these need workarounds:

- **ratatui `Canvas` braille drawing primitive.** SLT has `ui.canvas(...)` but
  the API takes a `CanvasContext` closure, not a value-typed widget. Custom
  point/line drawing logic needs a small rewrite. See
  `src/context/widgets_viz.rs` line 1289.
- **ratatui `Wrap { trim: true }` exact semantics.** SLT wraps via container
  width and `.wrap()` on text; the trim-leading-whitespace behaviour isn't
  identical. Test wrap-heavy text manually.
- **ratatui custom `Widget` trait impls.** No equivalent — wrap as a function
  taking `&mut Context`.
- **cursive's deep view layering / multiple modal stacks.** SLT supports
  `ui.modal(...)` and `ui.overlay(...)` but not arbitrary nested view
  managers. Most uses fold into a single `if state.show_modal { ui.modal(...) }`.
- **textual's CSS hot reload.** Themes are Rust values (no hot reload). For
  fast iteration use `cargo watch -x run`.

If a feature genuinely doesn't map, tell the user — don't fake it.

## References

- `.claude/skills/slt/SKILL.md` — SLT authoring skill (use after migration is done).
- `docs/COMPLETE_REFERENCE.md` — full SLT API in one file.
- `docs/COOKBOOK.md` — 5+ working SLT app recipes (login, data table, modal+toast, dashboard, file picker).
- `docs/PATTERNS.md` — component composition (`provide` / `use_context` / `use_state_named` / `with_if`).
- `docs/STATE_APIS.md` — every public `*State` struct's methods listed.
- `docs/PREVIOUS_FRAME_GUIDE.md` — when `Response.rect` is meaningful (frame 2+, not frame 1).
- ratatui repo: <https://github.com/ratatui-org/ratatui>
- cursive repo: <https://github.com/gyscos/cursive>
- textual repo: <https://github.com/Textualize/textual>
