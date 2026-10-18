# SuperLightTUI — Complete Reference (LLM-optimized)

> This document condenses the full SLT API and common patterns into one file. Source signatures and compile-tested examples remain authoritative when this guide and rustdoc disagree.

### Example classification

Every Rust fence below is a maintained `slt-check` compile contract. Run
`python3 scripts/check_doc_examples.py docs/reference/complete-reference.md`
to compile the actual Markdown source with the declared features, with warnings
denied. The checker does not execute terminal runtimes. `context: ui` supplies
`&mut Context` (plus any short `setup` the marker declares for app variables);
`context: module` checks a complete program or helper items as a module.
API tables and text diagrams are not executable. New recipes need a unique
marker name.

---

## 1. Mental Model

SLT is **immediate-mode**. `slt::run(|ui| { ... })` calls your closure every frame. The closure describes what the UI looks like right now. The library records commands, builds a layout tree, runs flexbox, diffs against the last rendered buffer, and writes only the changed cells.

State lives in **normal Rust** variables, structs, or — for widget-local persistent state — `ui.use_state(|| ...)` hooks. There is no App trait, no Model/Update/View, no message enum, no retained component tree. Control flow is plain `if` / `for`. Tab cycles focus automatically. Ctrl+C quits by default. Interactive widgets return a `Response` with click, hover, change, focus, submit, scroll, and rect signals. Hit-testing uses the last completed frame's layout, so `Response.rect` is only meaningful from frame 2 onward ([previous-frame rule](../guide/02-state.md#the-previous-frame-rule)).

---

## 2. 60-second grammar

<!-- slt-check: {"name":"counter", "features":["crossterm"], "context":"module"} -->
```rust
use slt::{Border, Color, Context, KeyCode};

fn main() -> std::io::Result<()> {
    let mut count: i32 = 0;
    slt::run(|ui: &mut Context| {
        if ui.key('q') { ui.quit(); }
        if ui.key_code(KeyCode::Up) { count += 1; }

        ui.bordered(Border::Rounded).title("Counter").p(1).gap(1).col(|ui| {
            ui.text("Count").bold().fg(Color::Cyan);
            ui.text(format!("{count}"));
            ui.row(|ui| {
                if ui.button("+1").clicked { count += 1; }
                if ui.button("-1").clicked { count -= 1; }
            });
        });
    })
}
```

Rules you will use every file:

- Text and style: `ui.text("x").bold().italic().dim().underline().fg(Color::Cyan).bg(Color::Reset)`
- Layout: `ui.row(|ui| {...})`, `ui.col(|ui| {...})`, `ui.bordered(Border::Rounded).title("T").p(1).gap(1).col(|ui| {...})`
- Containers return `Response`; chain style on `ContainerBuilder` then finalize with `.col(f)`, `.row(f)`, or `.draw(f)`.
- Inputs return `Response` with `.clicked`, `.hovered`, `.changed`, `.focused`, `.rect`.
- Widget state: create once outside the closure (`let mut tabs = TabsState::new(vec!["A","B"])`), pass `&mut` every frame.
- Exit: `ui.quit()`. Ctrl+C also exits. F12 toggles the layout debug overlay.

---

## 3. Entry points and configuration

| Call | Purpose |
|---|---|
| `slt::run(|ui| { ... })` | Full-screen alternate-screen runtime. Default 60 FPS. Needs `crossterm` (default feature). Uses `RunConfig::default()`, so the **mouse is on** in `MouseMode::Drag` (clicks, wheel, drags, text selection; `MouseMode::Click` over SSH/mosh). Hover styling and tooltips need `run_with(RunConfig::default().mouse_mode(MouseMode::Hover), f)`. |
| `slt::run_with(config, |ui| { ... })` | Same plus a `RunConfig`. |
| `slt::run_inline(height, |ui| { ... })` | Render a fixed `height` below the current cursor. No alt-screen. |
| `slt::run_inline_with(height, config, f)` | Inline + config. |
| `slt::run_static(&mut out, height, f)` | Static scrollback lines above a fixed inline UI (log-style). |
| `slt::run_static_with(&mut out, height, config, f)` | Same plus config. |
| `slt::run_async(|ui, msgs: &mut Vec<M>| { ... })` | Requires `async`. Returns `AsyncRunHandle<M>`; clone its wake-aware sender and join/cancel the owned loop. |
| `slt::run_async_with(config, f)` | Async + config. |
| `slt::frame(&mut backend, &mut state, &config, &events, &mut f)` | Low-level per-frame driver for custom backends. Returns `Ok(true)` to keep going, `Ok(false)` when quit. |
| `slt::frame_owned(&mut backend, &mut state, &config, events, &mut f)` | Same as `frame()` but takes `Vec<Event>` by value (zero-copy when callers already own a vector — no slice→Vec clone). |

`RunConfig` (builder, all setters consume `self`):

<!-- slt-check: {"name": "run_config_builder", "features": [], "context": "module"} -->
```rust
use std::time::Duration;

fn config() -> slt::RunConfig {
    slt::RunConfig::default()
        .tick_rate(Duration::from_millis(16))
        .mouse_mode(MouseMode::Hover) // default None = Drag locally, Click over SSH
        .kitty_keyboard(true)
        .theme(Theme::dracula())
        .color_depth(ColorDepth::TrueColor)
        .max_fps(60)
        .no_fps_cap()             // disable the FPS limit (sets max_fps = None)
        .scroll_speed(3)
        .title("My App")
        .widget_theme(WidgetTheme::new().button(WidgetColors::new().accent(Color::Cyan)))
}
```

`Backend` trait (for non-terminal targets): implement `fn size(&self) -> (u32, u32)`, `fn buffer_mut(&mut self) -> &mut Buffer`, `fn flush(&mut self) -> io::Result<()>`. Drive with `slt::frame`.

Embedding with your own event loop (issue #278): the built-in crossterm backends `slt::Terminal` / `slt::InlineTerminal` are public — construct one, render into `buffer_mut()`, call `flush()`, and feed input you read yourself through `slt::event::from_crossterm`. The `crossterm` crate is re-exported as `slt::crossterm` so you never pin a mismatched version.

`AppState::new()` holds frame-to-frame state; reuse the same instance across frames. `AppState::tick()`, `::fps_f64()`, `::set_debug(bool)`.

---

## 4. Core types (from `src/lib.rs` re-exports)

### Runtime and backend
| Type | One-line |
|---|---|
| `Context` | Per-frame handle you call widgets on (`ui`). |
| `Response` | `{ clicked: bool, hovered: bool, changed: bool, focused: bool, rect: Rect }`. `Response::none()` for a zero struct. |
| `State<T>` / `StateHandle<T>` | Handle returned from `use_state` / `use_state_keyed`; read with `.get(ui)`, mutate with `.get_mut(ui)`. `StateHandle<T>` is an alias that reads clearer next to widget state structs. |
| `Widget` | Trait for custom widgets (`type Response; fn ui(&mut self, ui: &mut Context) -> Self::Response`). |
| `ContainerBuilder` | Fluent builder returned by `container()`, `bordered()`, `scrollable()`, `group()`. Finalize with `.col(f)`, `.row(f)`, or a raw-draw method. |
| `CanvasContext` | Braille canvas passed to `ui.canvas(w, h, |cv| { ... })`. |
| `Backend` | Trait for custom render targets. |
| `AppState` | Opaque session state passed to `frame()`. |
| `RunConfig` | Per-run configuration. |
| `MouseMode` | `Off` / `Click` / `Drag` / `Hover` mouse reporting level for `RunConfig::mouse_mode`. Default (`None`) resolves to `Drag` locally, `Click` over SSH/mosh; `RunConfig::effective_mouse_mode()` returns the resolved mode. `RunConfig::mouse(true)` = `Hover`, `mouse(false)` = `Off`. |
| `TestBackend` | Headless backend for tests: `TestBackend::new(w, h).render(|ui| ...)`, then `assert_contains`/`line`/`to_string_trimmed`. |
| `EventBuilder` | `EventBuilder::new().key('a').key_code(KeyCode::Enter).click(5, 2).mouse_up(x, y).drag(x, y).key_release('c').focus_gained().focus_lost().scroll_up(x, y).paste("txt").resize(w, h).build()`. |

### Style and layout
| Type | Notes |
|---|---|
| `Style` | `Style::new().fg(c).bg(c).bold().italic().dim().underline().reversed().strikethrough()`. |
| `Color` | `Color::Rgb(r, g, b)`, `Color::Indexed(u8)`, `Color::Named` (all 17 ANSI: `Black`, `Red`, `Green`, `Yellow`, `Blue`, `Magenta`, `Cyan`, `White`, `DarkGray`, `LightRed`, `LightGreen`, `LightYellow`, `LightBlue`, `LightMagenta`, `LightCyan`, `LightWhite`), `Color::Reset`. Helpers: `.blend_f64(o, a)`, `.lighten_f64(f)`, `.darken_f64(f)`, `.luminance_f64()`, `.contrast_ratio_f64(a, b)`, `.meets_contrast_aa(a, b)`, `.contrast_fg(bg)`, `.downsampled(depth)`. |
| `ColorDepth` | `TrueColor`, `EightBit`, `Basic`, `NoColor`. `ColorDepth::detect()` auto-detects from `$NO_COLOR`/`$COLORTERM`/`$TERM`. |
| `Modifiers` | Bitflags: `BOLD`, `DIM`, `ITALIC`, `UNDERLINE`, `REVERSED`, `STRIKETHROUGH`. Methods: `.contains(o)`, `.is_empty()`, `.remove(o)` (clears bits in-place). |
| `Border` | `None`, `Single`, `Rounded`, `Double`, `Heavy`, `Thick`, `Dashed`, `Dotted`, `Ascii`. |
| `BorderSides` | bitflags: `TOP`, `RIGHT`, `BOTTOM`, `LEFT`, or combos. |
| `Padding` | `{ top, right, bottom, left: u32 }`. |
| `Margin` | Same shape as `Padding`. |
| `Constraints` | `{ min_width, max_width, min_height, max_height: Option<u32>, width_pct, height_pct: Option<u8> }`. |
| `Align` | `Start`, `Center`, `End`. Cross-axis alignment. |
| `Justify` | `Start`, `Center`, `End`, `SpaceBetween`, `SpaceAround`, `SpaceEvenly`. Main-axis. |
| `Breakpoint` | `Xs` (<40), `Sm` (40–79), `Md` (80–119), `Lg` (120–159), `Xl` (>=160). |
| `ContainerStyle` | Const-buildable recipe with `ContainerStyle::new().border(..).p(..).bg(..).theme_bg(..)` etc. Apply with `.apply(&style)`. Compose with `ContainerStyle::extending(&base).theme_bg(..)`. |
| `Direction` | `Row`, `Column`. |
| `Rect` | `{ x, y, width, height: u32 }`. `Rect::new(x,y,w,h)`, `.right()`, `.bottom()`, `.contains(x,y)`. |
| `Buffer` | Double-buffer cells. `Buffer::empty(rect)`, `.get(x,y)`, `.try_get(x,y) -> Option<&Cell>`, `.set_char(x,y,ch,style)`, `.set_string(x,y,s,style)`, `.push_clip(rect)`, `.pop_clip()`. |
| `Cell` | `{ symbol: String, style: Style, url: Option<String> }`. |

### Theming
| Type | Notes |
|---|---|
| `Theme` | 17-field struct. `#[non_exhaustive]`. Use presets or `Theme::builder()`. Presets: `Theme::dark()` (default), `light()`, `dracula()`, `catppuccin()`, `nord()`, `solarized_dark()`, `solarized_light()`, `tokyo_night()`, `gruvbox_dark()`, `one_dark()`. Builder entry points: `Theme::builder()` (empty), `Theme::builder_from(base: Theme) -> ThemeBuilder` (pre-fill from any theme), `Theme::light_builder() -> ThemeBuilder` (shortcut for `builder_from(Theme::light())`). |
| `ThemeBuilder` | `.primary(c).secondary(c).accent(c).text(c).text_dim(c).border(c).bg(c).success(c).warning(c).error(c).selected_bg(c).selected_fg(c).surface(c).surface_hover(c).surface_text(c).is_dark(b).spacing(sp).build()`. All setters and `build()` are `const fn` — themes can be defined in `const` context. |
| `ThemeColor` | Semantic tokens: `Primary`, `Secondary`, `Accent`, `Text`, `TextDim`, `Border`, `Bg`, `Success`, `Warning`, `Error`, `SelectedBg`, `SelectedFg`, `Surface`, `SurfaceHover`, `SurfaceText`, `Info`, `Link`, `FocusRing`, `Custom(Color)`. Resolve via `ui.color(ThemeColor::X)` or `ui.theme().resolve(t)`. |
| `Spacing` | `{ base: u32 }`. Methods: `none()`, `xs()`, `sm()`, `md()`, `lg()`, `xl()`, `xxl()`. `Spacing::new(2)` doubles everything. `ui.spacing()` returns the current one. |
| `WidgetColors` | `WidgetColors::new().fg(c).bg(c).border(c).accent(c).theme_fg(t).theme_bg(t).theme_border(t).theme_accent(t)`. Pass to `_colored` variants. |
| `WidgetTheme` | Default colors for all instances of a widget type. `WidgetTheme::new().button(colors).table(colors).list(...).select(...)` etc. Set via `RunConfig::widget_theme`. |

### Events, input, keymap
| Type | Notes |
|---|---|
| `Event` | `Key(KeyEvent)`, `Mouse(MouseEvent)`, `Resize(w, h)`, `Paste(String)`, `FocusLost`, `FocusGained`. |
| `KeyEvent` | `{ code: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind }`. |
| `KeyCode` | `Char(char)`, `Enter`, `Esc`, `Tab`, `BackTab`, `Backspace`, `Delete`, `Home`, `End`, `PageUp`, `PageDown`, `Up`, `Down`, `Left`, `Right`, `Insert`, `F(u8)`, `Null`, `CapsLock`, `ScrollLock`, `NumLock`, `PrintScreen`, `Pause`, `Menu`, `KeypadBegin`, `Media(m)`, `Modifier(m)`. |
| `KeyModifiers` | Bitflags: `NONE`, `SHIFT`, `CONTROL`, `ALT`, `SUPER`, `HYPER`, `META`. |
| `KeyEventKind` | `Press`, `Repeat`, `Release`. |
| `MouseEvent` | `{ kind: MouseKind, x: u32, y: u32, modifiers: KeyModifiers, pixel_x: Option<u32>, pixel_y: Option<u32> }`. |
| `MouseKind` | `Down(MouseButton)`, `Up(MouseButton)`, `Drag(MouseButton)`, `Moved`, `ScrollUp`, `ScrollDown`, `ScrollLeft`, `ScrollRight`. |
| `MouseButton` | `Left`, `Right`, `Middle`. |
| `KeyMap` | Declarative keymap used to drive `help_from_keymap(&km)`. Register via `.bind(char, desc)`, `.bind_code(KeyCode, desc)`, `.bind_mod(char, mods, desc)`, `.bind_code_mod(KeyCode, mods, desc)` (any KeyCode + modifiers, e.g. Ctrl+Enter), `.bind_hidden(char, desc)`. Input matching is still your responsibility via `ui.key(...)`. |
| `Binding` | Entry in a `KeyMap` (`{ key, modifiers, display, description, visible }`). |

### Animation
| Type | Notes |
|---|---|
| `Tween` | `Tween::new(from, to, duration_ticks).easing(ease_out_quad).on_complete(|| {})`. `.reset(tick)`, `.value(tick) -> f64`, `.is_done() -> bool`. |
| `Spring` | `Spring::new(initial, stiffness, damping)`. `.set_target(v)`, `.tick()`, `.value() -> f64`. |
| `Keyframes` | Timeline with stops. `Keyframes::new(duration).stop(0.0, 0.0).stop(0.5, 100.0).stop(1.0, 0.0).loop_mode(LoopMode::Repeat)`. |
| `Sequence` | Chained tween segments in order. |
| `Stagger` | `Stagger::new(from, to, duration).delay(ticks).items(count)`. `.value(tick, item_index)`. `.is_done()` reports completion of the last sampled item; `.is_all_done(tick, item_count) -> bool` reports completion of every item in the batch. |
| `LoopMode` | `Once`, `Repeat`, `PingPong`. |
| Easings (under `slt::anim::*`) | `ease_linear`, `ease_in_quad`, `ease_out_quad`, `ease_in_out_quad`, `ease_in_cubic`, `ease_out_cubic`, `ease_in_out_cubic`, `ease_out_elastic`, `ease_out_bounce`. |

### Charts and visualization
| Type | Notes |
|---|---|
| `ChartBuilder` | Passed to `ui.chart(|c| {...}, w, h)`. `.line(&[(f64, f64)])`, `.area(...)`, `.scatter(...)`, `.bar(...)`, `.grid(bool)`, `.xlim(min, max)`, `.ylim(min, max)`, `.legend(pos)`. Series methods return `DatasetEntry` with `.color(..)`, `.label(..)`, `.marker(..)`. |
| `ChartConfig` | Config struct for chart builder internals. |
| `Dataset` | Single series description. |
| `Marker` | Marker shapes: `Dot`, `Block`, `HalfBlock`, `Braille`, `Cross`, `Circle`. |
| `Candle` | `{ open, high, low, close: f64 }`. |
| `LegendPosition` | `TopLeft`, `TopRight`, `BottomLeft`, `BottomRight`, `None`. |
| `Bar` | `Bar::new(label, value).color(c).text_value(s)`. |
| `BarGroup` | `BarGroup::new(label, bars)`; spacing between groups is `BarChartConfig::group_gap(n)`. |
| `BarDirection` | `Horizontal`, `Vertical`. |
| `BarChartConfig` | Bar chart configuration for `bar_chart_with` / `bar_chart_grouped_with`. |
| `TreemapItem` | `TreemapItem::new(label, value, color)`. |
| `HalfBlockImage` | Half-block 2-px-per-cell image. |

### Widget state types (all re-exported at `slt::`)

| State | Widget it drives |
|---|---|
| `TextInputState` | `text_input`, `text_input_colored` |
| `TextareaState` | `textarea` |
| `CodeEditorState` | `code_editor` |
| `SelectState` | `select`, `select_colored` |
| `RadioState` | `radio`, `radio_colored` |
| `MultiSelectState` | `multi_select` |
| `TabsState` | `tabs`, `tabs_colored` |
| `ListState` | `list`, `list_colored`, `virtual_list` |
| `TableState` | `table`, `table_colored` |
| `TreeState` | `tree` |
| `DirectoryTreeState` | `directory_tree` |
| `TreeNode` | Item in `TreeState.nodes`. `TreeNode::new(label).expanded().children(vec![...])`. |
| `CalendarState` | `calendar` |
| `FilePickerState`, `FileEntry` | `file_picker` |
| `CommandPaletteState`, `PaletteCommand` | `command_palette` |
| `ScrollState` | `scrollable`, `scrollbar`, `scroll_col`, `scroll_row` |
| `SpinnerState` | `spinner` |
| `ToastState`, `ToastMessage`, `ToastLevel` | `toast` |
| `AlertLevel` | `alert` severity |
| `ButtonVariant` | `ui.button(label).variant(v)` (`Default`, `Primary`, `Danger`, `Outline`) |
| `Trend` | `stat_trend` (`Up`, `Down`) |
| `FormState`, `FormField` | `form`, `form_field`, `form_submit` |
| `ScreenState` | `screen(name, &mut screens, f)` |
| `ModeState` | Multi-mode wrapper; each mode owns its own `ScreenState` |
| `StreamingTextState`, `StreamingMarkdownState` | `streaming_text`, `streaming_markdown` |
| `ToolApprovalState`, `ApprovalAction` | `tool_approval` |
| `DiffState`, `DiffResponse`, `HunkId`, `HunkAction`, `HunkDecision`, `DiffLayout`, `DiffKeys`, `DiffColors`, `DiffParseError` (model: `slt::widgets::{DiffFile, DiffHunk, DiffLine, DiffLineKind, DiffFileStatus}`) | `diff` |
| `ContextItem` | `context_bar(items)` |
| `RichLogState`, `RichLogEntry` | `rich_log` |
| `GridColumn` | Per-column spec for `grid_columns` |
| `StaticOutput` | Scrollback buffer for `run_static` |

### Palette

`slt::Palette` plus `slt::palette::tailwind::{SLATE, GRAY, ZINC, NEUTRAL, STONE, RED, ORANGE, AMBER, YELLOW, LIME, GREEN, EMERALD, TEAL, CYAN, SKY, BLUE, INDIGO, VIOLET, PURPLE, FUCHSIA, PINK, ROSE}` — each palette exposes `.c50 .c100 .c200 .c300 .c400 .c500 .c600 .c700 .c800 .c900 .c950`.

---

## 5. Widget catalog (dense)

The full catalog lives in the [Widget Guide](widgets.md). Every widget call,
return type, builder option and caveat is listed there once. One line per
family:

- Return shapes, `Response` fields, builder borrow rules: [Return type conventions](widgets.md#return-type-conventions).
- Text, rich text (`markdown`, `code_block`, `big_text`): [Text & Display](widgets.md#text--display).
- `alert`, `badge`, `stat`, `breadcrumb`, `help`: [Status & Info](widgets.md#status--info).
- Containers, scrolling, overlays, menus, screens, forms: [Layout & Containers](widgets.md#layout--containers).
- `ContainerBuilder` chain and finalizers: [ContainerBuilder Quick Reference](widgets.md#containerbuilder-quick-reference).
- `button`, inputs, `code_editor`, `select`, `list`, `table`, `tree`, pickers: [Interactive Widgets](widgets.md#interactive-widgets).
- `progress`, `spinner`, `toast`, `notify`: [Feedback & Progress](widgets.md#feedback--progress).
- Charts, canvas, heatmap, treemap, QR: [Visualization](widgets.md#visualization) and [CanvasContext](widgets.md#canvascontext-quick-reference).
- Streaming text, `tool_approval`, `diff`, images, `rich_log`: [AI-Native & Rich Terminal](widgets.md#ai-native--rich-terminal).
- Hooks, focus, keyboard, mouse, clipboard, theme: [Runtime & Hooks](widgets.md#runtime--hooks).

---

## 6. Common patterns (compressed)

### 6.1 App state in plain Rust
<!-- slt-check: {"name": "app_state_plain_rust", "features": ["crossterm"], "context": "module"} -->
```rust
struct App { count: i32, dark: bool, tabs: TabsState, list: ListState }
fn main() -> std::io::Result<()> {
    let mut app = App {
        count: 0,
        dark: false,
        tabs: TabsState::new(vec!["Overview", "Logs", "Settings"]),
        list: ListState::new(vec!["one", "two", "three"]),
    };
    slt::run(|ui: &mut Context| {
        if ui.key('q') { ui.quit(); }
        ui.checkbox("Dark", &mut app.dark);
        ui.tabs(&mut app.tabs);
        ui.list(&mut app.list);
    })
}
```

### 6.2 Hook-based local state
<!-- slt-check: {"name":"hook_state", "features":[], "context":"ui"} -->
```rust
let count = ui.use_state(|| 0i32);
ui.text(format!("{}", count.get(ui)));
if ui.button("+1").clicked { *count.get_mut(ui) += 1; }
```
Hook calls must run in the same order every frame. Do not put them inside `if`/`match` branches.

### 6.3 Derived state
<!-- slt-check: {"name": "derived_state_memo", "features": [], "context": "ui", "setup": "let query = String::from(\"ru\"); let items = vec![String::from(\"rust\"), String::from(\"go\")];"} -->
```rust
let filtered = ui.use_memo(&(query.clone(), items.len()), |(q, _)| {
    items.iter().filter(|i| i.contains(q.as_str())).cloned().collect::<Vec<_>>()
});
let count = filtered.get(ui).len();
ui.text(format!("{count} matches"));
```

### 6.4 Helper extraction
When a builder chain repeats, extract a helper:
<!-- slt-check: {"name": "helper_extraction", "features": [], "context": "module"} -->
```rust
fn panel(ui: &mut Context, title: &str, f: impl FnOnce(&mut Context)) {
    ui.bordered(Border::Rounded).title(title).p(1).grow(1).col(f);
}
```

### 6.5 Screen navigation
<!-- slt-check: {"name": "screen_navigation", "features": ["crossterm"], "context": "module"} -->
```rust
fn main() -> std::io::Result<()> {
    let mut screens = ScreenState::new("home");
    slt::run(|ui| {
        ui.screen("home", &mut screens, |ui| {
            if ui.button("Settings").clicked { ui.push_screen("settings"); }
        });
        ui.screen("settings", &mut screens, |ui| {
            if ui.button("Back").clicked { ui.pop_screen(); }
        });
    })
}
```
Each `screen(...)` call isolates hook state and focus.

Navigate from **inside** a `screen(...)` closure with `ui.push_screen(name)`,
`ui.pop_screen()`, or `ui.reset_screen()`. These record a deferred navigation
that is applied to your `ScreenState` the moment the closure returns — calling
`screens.push(...)` directly inside the closure does **not** compile, because
`screen(...)` already holds a `&mut ScreenState` borrow for the duration of the
closure (issue #279). Outside any `screen(...)` closure you can still mutate the
`ScreenState` directly (`screens.push("x")`, `screens.pop()`).

The transition frame finishes rendering only the source screen. The destination
starts on the next frame, preventing both views from being composed into one
terminal buffer.

### 6.6 Multi-mode app
<!-- slt-check: {"name": "multi_mode_app", "features": ["crossterm"], "context": "module"} -->
```rust
fn main() -> std::io::Result<()> {
    let mut modes = ModeState::new("app", "home");
    modes.add_mode("settings", "general");
    slt::run(|ui| {
        if ui.key('1') { modes.try_switch_mode("app"); }
        if ui.key('2') { modes.try_switch_mode("settings"); }
        let screens = modes.screens_mut();
        ui.screen("home", screens, |ui| { ui.text("Home"); });
        ui.screen("general", screens, |ui| { ui.text("Settings/General"); });
    })
}
```

### 6.7 Form with validation

Attach validators to each `FormField` with `.validate(..)` and run them all with
`form.validate_all()`. For scrollable forms, the following recipe also exposes
field feedback:

<!-- slt-check: {"name":"focus_scrolling_form","features":["crossterm"],"context":"module"} -->
```rust
fn scrollable_form() -> std::io::Result<()> {
    let mut form = slt::FormState::new()
        .field(slt::FormField::new("Email").validate(|value| {
            if value.contains('@') { Ok(()) } else { Err("Invalid email".into()) }
        }))
        .field(slt::FormField::new("Password").masked(true));
    let mut scroll = slt::ScrollState::new();
    let mut focused_label = String::new();
    slt::run(|ui| {
        ui.scrollable(&mut scroll).h(6).col(|ui| {
            for field in &mut form.fields {
                let response = ui.form_field_response(field);
                if response.gained_focus { focused_label.clone_from(&field.label); }
            }
        });
        ui.text(format!("Editing: {focused_label}"));
        if ui.form_submit("Submit").clicked && form.validate_all() { ui.quit(); }
        if ui.raw_key_code(slt::KeyCode::Esc) { ui.quit(); }
    })
}
```

Full layout queries (`focused_layout_rect`, `measured_layout_rect`) are separate
from clipped `Response.rect`. Timing and coordinates: [the previous-frame rule](../guide/02-state.md#the-previous-frame-rule).

<!-- slt-check: {"name": "form_validation", "features": ["crossterm"], "context": "module"} -->
```rust
fn main() -> std::io::Result<()> {
    let mut form = FormState::new()
        .field(FormField::new("Email").placeholder("you@example.com").validate(|v| {
            if v.contains('@') { Ok(()) } else { Err("Invalid email".into()) }
        }))
        .field(FormField::new("Password").validate(|v| {
            if v.len() >= 8 { Ok(()) } else { Err("Too short".into()) }
        }));

    slt::run(move |ui| {
        ui.form(&mut form, |ui, form| {
            for field in &mut form.fields { ui.form_field(field); }
            if ui.form_submit("Submit").clicked && form.validate_all() {
                form.submitted = true;
            }
        });
    })
}
```

For single inputs without `FormState`:
<!-- slt-check: {"name": "single_input_validation", "features": [], "context": "ui"} -->
```rust
let mut email = TextInputState::with_placeholder("you@example.com");
ui.text_input(&mut email);
email.validate_now(|v| if v.contains('@') { Ok(()) } else { Err("Invalid".into()) });
if let Some(err) = &email.validation_error { ui.alert(err, AlertLevel::Error); }
```

### 6.8 Modal confirmation
<!-- slt-check: {"name": "modal_confirmation", "features": [], "context": "ui"} -->
```rust
let show_confirm = ui.use_state(|| false);
if *show_confirm.get(ui) {
    ui.modal(|ui| {
        ui.text("Delete item?").bold();
        ui.container().gap(1).row(|ui| {
            if ui.button("Cancel").clicked {
                *show_confirm.get_mut(ui) = false;
            }
            if ui.button("Delete").variant(ButtonVariant::Danger).clicked {
                // perform delete
                *show_confirm.get_mut(ui) = false;
            }
        });
    });
}
```

### 6.9 Toast + fire-and-forget notify
<!-- slt-check: {"name": "toast_notify", "features": ["crossterm"], "context": "module"} -->
```rust
fn main() -> std::io::Result<()> {
    let mut toasts = ToastState::new();
    slt::run(move |ui| {
        if ui.button("Save").clicked { toasts.success("Saved!", ui.tick()); }
        // OR, without owning a ToastState:
        if ui.button("Ping").clicked { ui.notify("ping!", ToastLevel::Info); }
        ui.toast(&mut toasts);
    })
}
```

### 6.10 Command palette
<!-- slt-check: {"name": "command_palette", "features": ["crossterm"], "context": "module"} -->
```rust
fn main() -> std::io::Result<()> {
    let mut palette = CommandPaletteState::new(vec![
        PaletteCommand::new("New File", "Create a new file"),
        PaletteCommand::new("Open", "Open a file"),
        PaletteCommand::new("Save", "Write changes"),
    ]);
    slt::run(move |ui| {
        if ui.key_mod('p', KeyModifiers::CONTROL) { palette.toggle(); }
        let r = ui.command_palette(&mut palette);
        if r.changed {
            if let Some(i) = palette.last_selected { ui.text(format!("ran command {i}")); }
        }
    })
}
```

### 6.11 Real-time updates with tick
<!-- slt-check: {"name": "tick_updates", "features": ["crossterm"], "context": "module"} -->
```rust
fn main() -> std::io::Result<()> {
    let start = std::time::Instant::now();
    slt::run(move |ui| {
        let elapsed = start.elapsed();
        ui.text(format!("uptime: {}s", elapsed.as_secs()));
        // tick counter is monotonic frame index, not wall-clock
        ui.progress((ui.tick() % 60) as f64 / 60.0);
    })
}
```

### 6.12 Async messages
<!-- slt-check: {"name":"async_messages", "features":["async","crossterm"], "context":"module", "tokio":true} -->
```rust
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut messages: Vec<String> = Vec::new();
    let run = slt::run_async::<String>(move |ui, msgs| {
        messages.extend(msgs.drain(..));
        ui.col(|ui| { for m in &messages { ui.text(m); } });
    })?;
    let tx = run.sender();
    tokio::spawn(async move {
        for i in 0..10 {
            let _ = tx.send(format!("tick {i}")).await;
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
    tokio::signal::ctrl_c().await?;
    run.cancel_and_join().await.map_err(std::io::Error::other)
}
```

### 6.13 Dashboard composition
<!-- slt-check: {"name":"dashboard", "features":[], "context":"ui"} -->
```rust
ui.container().gap(1).row(|ui| {
    ui.bordered(Border::Rounded).title("Stats").p(1).w(30).col(|ui| {
        ui.stat("Users", "1,293");
        ui.stat_trend("Revenue", "$48k", Trend::Up);
    });
    ui.bordered(Border::Rounded).title("Traffic").grow(1).p(1).col(|ui| {
        ui.sparkline(&[3.0, 4.0, 2.0, 5.0, 6.0, 4.0], 20);
        ui.chart(|c| { c.line(&[(1.0, 1.0), (2.0, 4.0), (3.0, 9.0), (4.0, 16.0), (5.0, 25.0)]); c.grid(true); }, 40, 10);
    });
});
```

### 6.14 File picker dialog
<!-- slt-check: {"name": "file_picker_dialog", "features": [], "context": "ui"} -->
```rust
let mut picker = FilePickerState::new(".").show_hidden(false).extensions(&["rs", "toml"]);
if ui.file_picker(&mut picker).clicked {
    if let Some(path) = picker.selected_file() { println!("picked: {:?}", path); }
}
```

### 6.15 Custom Widget
<!-- slt-check: {"name": "custom_widget", "features": [], "context": "module"} -->
```rust
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
        let stars: String = (0..self.max).map(|i| if i < self.value { '★' } else { '☆' }).collect();
        ui.styled(stars, Style::new().fg(if focused { Color::Yellow } else { Color::White }));
        changed
    }
}
// usage
fn rating_row(ui: &mut Context, rating: &mut Rating) {
    if ui.widget(rating) { ui.text(format!("rated {}", rating.value)); }
}
```

### 6.16 Responsive layout
<!-- slt-check: {"name": "responsive_layout", "features": [], "context": "ui"} -->
```rust
match ui.breakpoint() {
    Breakpoint::Xs | Breakpoint::Sm => {
        ui.col(|ui| { ui.text("stacked"); });
    }
    _ => {
        ui.row(|ui| { ui.text("side by side"); });
    }
}
// Or per-breakpoint style:
ui.container().w(20).md_w(40).lg_w(60).col(|ui| { ui.text("sized"); });
```

### 6.17 Animation
<!-- slt-check: {"name":"animation", "features":["crossterm"], "context":"module"} -->
```rust
fn main() -> std::io::Result<()> {
    let items = ["First", "Second", "Third"];
    let mut fade = Tween::new(0.0, 1.0, 30).easing(slt::anim::ease_out_quad);
    let mut spring = Spring::new(0.0, 0.2, 0.85);
    let mut stagger = Stagger::new(0.0, 1.0, 20).delay(3).items(items.len());
    let mut pulse = Keyframes::new(60)
        .stop(0.0, 0.0).stop(0.5, 1.0).stop(1.0, 0.0)
        .loop_mode(LoopMode::Repeat);
    let mut sequence = Sequence::new()
        .then(0.0, 1.0, 30, slt::anim::ease_linear)
        .then(1.0, 0.0, 30, slt::anim::ease_out_quad);
    slt::run(move |ui| {
        if ui.key('q') { ui.quit(); }
        let hovered = ui.button("Target").hovered;
        spring.set_target(if hovered { 1.0 } else { 0.0 });
        spring.tick();
        let alpha = fade.value(ui.tick());
        ui.text("Hello").fg(Color::Rgb(255, 255, (alpha * 255.0) as u8));
        ui.text(format!("{:.2} {:.2} {:.2}", spring.value(), pulse.value(ui.tick()), sequence.value(ui.tick())));
        for (i, item) in items.iter().enumerate() {
            let a = stagger.value(ui.tick(), i);
            ui.text(*item).fg(Color::Rgb(255, 255, (a * 255.0) as u8));
        }
    })
}
```

### 6.18 Raw draw (direct buffer access)
<!-- slt-check: {"name": "raw_draw", "features": [], "context": "ui", "setup": "fn compute_points() -> Vec<(u32, u32)> { vec![(1, 1), (3, 2)] }"} -->
```rust
// Fire-and-forget draw
ui.container().w(40).h(10).draw(|buf, rect| {
    for x in 0..rect.width {
        buf.set_char(rect.x + x, rect.y, '-', Style::new().fg(Color::Cyan));
    }
});

// With per-frame data ownership (closure is 'static):
let points: Vec<(u32, u32)> = compute_points();
ui.container().w(40).h(20).draw_with(points, |buf, rect, pts| {
    for &(x, y) in pts {
        if rect.contains(x, y) { buf.set_char(x, y, '●', Style::new()); }
    }
});

// With hit testing:
let r = ui.container().w(40).h(10).draw_interactive(|buf, rect| {
    buf.set_string(rect.x, rect.y, "Click me!", Style::new());
});
if r.clicked { /* ... */ }
```

### 6.19 Error boundary
<!-- slt-check: {"name": "error_boundary", "features": [], "context": "ui", "setup": "fn risky_render(ui: &mut Context) { ui.text(\"risky\"); }"} -->
```rust
ui.error_boundary(|ui| {
    risky_render(ui);
});
// or custom fallback:
ui.error_boundary_with(
    |ui| risky_render(ui),
    |ui, msg| { ui.text(format!("Recovered: {msg}")).fg(Color::Red); },
);
```

### 6.19b Tables with sort / filter / paginate
<!-- slt-check: {"name": "table_sort_paginate", "features": ["crossterm"], "context": "module"} -->
```rust
fn main() -> std::io::Result<()> {
    let mut table = TableState::new(
        vec!["ID", "Name", "Status"],
        vec![
            vec!["001", "Alice", "Active"],
            vec!["002", "Bob", "Paused"],
            vec!["003", "Carol", "Active"],
        ],
    );
    table.page_size = 10;       // 0 disables pagination
    table.zebra = true;

    slt::run(move |ui| {
        if ui.key('s') { table.toggle_sort(1); }  // toggle sort on Name column
        if ui.key_code(KeyCode::Right) { table.next_page(); }
        if ui.key_code(KeyCode::Left)  { table.prev_page(); }

        let r = ui.table(&mut table);
        if r.changed {
            if let Some(row) = table.selected_row() {
                ui.text(row.join(" | ")); // row: &[String]
            }
        }
    })
}
```

### 6.19c Tree view from filesystem paths
<!-- slt-check: {"name": "directory_tree_paths", "features": [], "context": "ui"} -->
```rust
let mut tree = DirectoryTreeState::from_paths(&[
    "src/lib.rs", "src/context/runtime.rs", "docs/guide/01-quick-start.md",
]);
let r = ui.directory_tree(&mut tree);
if r.changed {
    if let Some(label) = tree.selected_label() { ui.text(format!("selected {label}")); }
}
```

### 6.19d Virtualized list for 100k rows
<!-- slt-check: {"name": "virtual_list_rows", "features": [], "context": "ui"} -->
```rust
let rows: Vec<String> = (0..100_000).map(|i| format!("row {i}")).collect();
let mut list = ListState::new(rows.clone());
ui.virtual_list(&mut list, 20, |ui, idx| {
    ui.text(&rows[idx]).dim(); // idx is a data index into the items
});
```

### 6.19e Typing with autocomplete
<!-- slt-check: {"name": "autocomplete_input", "features": [], "context": "ui"} -->
```rust
let mut input = TextInputState::with_placeholder("type a language");
input.set_suggestions(vec![
    "rust".into(), "ruby".into(), "python".into(), "typescript".into(),
]);
ui.text_input(&mut input);
for s in input.matched_suggestions() {
    ui.text(format!("• {s}")).dim();
}
```

### 6.19f Streaming LLM output
<!-- slt-check: {"name": "streaming_markdown", "features": [], "context": "ui"} -->
```rust
let mut s = StreamingMarkdownState::new();
// In whatever loop feeds new tokens:
s.push("**Answer:** the");
s.push(" quick brown fox");
// Inside the closure, just render what's accumulated:
ui.streaming_markdown(&mut s);
```

### 6.19g Canvas — animated shape
<!-- slt-check: {"name": "canvas_animation", "features": ["crossterm"], "context": "module"} -->
```rust
fn main() -> std::io::Result<()> {
    let mut angle = 0.0_f64;
    slt::run(move |ui| {
        angle += 0.05;
        ui.canvas(40, 12, |cv| {
            cv.set_color(Color::Cyan);
            let cx = 40; let cy = 24;
            let (sx, sy) = (cx + (angle.cos() * 20.0) as i32, cy + (angle.sin() * 20.0) as i32);
            cv.line(cx as _, cy as _, sx as _, sy as _);
            cv.circle(cx as _, cy as _, 22);
        });
    })
}
```

### 6.20 Keyboard shortcuts with KeyMap + help bar

`KeyMap` is a declarative registry that also drives the help bar. Check the actual key press yourself (`ui.key(...)`, `ui.key_mod(...)`) — `KeyMap` owns labels and descriptions, not input consumption.

<!-- slt-check: {"name": "keymap_help_bar", "features": ["crossterm"], "context": "module"} -->
```rust
fn main() -> std::io::Result<()> {
    let km = KeyMap::new()
        .bind('q', "Quit")
        .bind_code(KeyCode::Up, "Move up")
        .bind_mod('s', KeyModifiers::CONTROL, "Save")
        .bind_hidden('?', "Toggle help");

    slt::run(move |ui| {
        if ui.key('q') { ui.quit(); }
        if ui.key_mod('s', KeyModifiers::CONTROL) { /* save */ }
        ui.help_from_keymap(&km);  // renders the help bar from the map
    })
}
```

`Binding` has fields: `{ key: KeyCode, modifiers: Option<KeyModifiers>, display: String, description: String, visible: bool }`.

---

## 7. Previous-frame rule

`Response.rect`, hit-testing, and `ui.focus_count()` read the layout of the
last completed frame, so they are zero on a widget's first frame. Details:
[State guide: the previous-frame rule](../guide/02-state.md#the-previous-frame-rule).

---

## 8. Theming (condensed)

Presets: `Theme::dark()` (default), `light()`, `dracula()`, `catppuccin()`, `nord()`, `solarized_dark()`, `solarized_light()`, `tokyo_night()`, `gruvbox_dark()`, `one_dark()`.

Custom:
<!-- slt-check: {"name": "custom_theme", "features": ["crossterm"], "context": "module"} -->
```rust
use slt::palette::tailwind::*;
use slt::{RunConfig, Theme};

fn main() -> std::io::Result<()> {
    let theme = Theme::builder()
        .primary(INDIGO.c500).secondary(TEAL.c500).accent(PINK.c500)
        .text(SLATE.c50).text_dim(SLATE.c400).border(SLATE.c700)
        .bg(SLATE.c950).success(EMERALD.c500).warning(AMBER.c500).error(RED.c500)
        .surface(SLATE.c800).surface_hover(SLATE.c700).surface_text(SLATE.c300)
        .is_dark(true).build();
    slt::run_with(RunConfig::default().theme(theme), |ui| { ui.text("themed"); })
}
```

Semantic tokens (`ThemeColor`): use `ui.color(ThemeColor::Primary)` or `WidgetColors::new().theme_bg(ThemeColor::Surface)`.

Runtime switch: `ui.set_theme(Theme::nord())`, `ui.set_dark_mode(true)`, `ui.light_dark(light, dark)`.

Const-buildable recipes:
<!-- slt-check: {"name": "const_container_style", "features": [], "context": "ui"} -->
```rust
const CARD: ContainerStyle = ContainerStyle::new()
    .border(Border::Rounded).p(1).theme_bg(ThemeColor::Surface);
ui.container().apply(&CARD).col(|ui| { ui.text("card"); });
```

---

## 8b. Theming deep dive

### Color constructors
<!-- slt-check: {"name": "color_constructors", "features": [], "context": "ui"} -->
```rust
let colors = [
    Color::Rgb(255, 127, 80),        // true color
    Color::Indexed(240),             // xterm 256 palette index
    Color::Red,                      // named ANSI
    Color::LightCyan,                // bright variant
    Color::Reset,                    // terminal default
];
for c in colors { ui.text("■").fg(c); }
```

### Tailwind palettes (via `slt::palette::tailwind::*`)

Neutrals: `SLATE`, `GRAY`, `ZINC`, `NEUTRAL`, `STONE`.
Colors: `RED`, `ORANGE`, `AMBER`, `YELLOW`, `LIME`, `GREEN`, `EMERALD`, `TEAL`, `CYAN`, `SKY`, `BLUE`, `INDIGO`, `VIOLET`, `PURPLE`, `FUCHSIA`, `PINK`, `ROSE`.

Shades (const fields on each palette): `.c50`, `.c100`, `.c200`, `.c300`, `.c400`, `.c500`, `.c600`, `.c700`, `.c800`, `.c900`, `.c950`.

<!-- slt-check: {"name": "tailwind_palette", "features": [], "context": "ui"} -->
```rust
use slt::palette::tailwind::{BLUE, ROSE, SLATE};
let primary = BLUE.c500;
let danger  = ROSE.c600;
let muted   = SLATE.c400;
ui.text("Save").fg(primary);
ui.text("Delete").fg(danger);
ui.text("hint").fg(muted);
```

### `ThemeColor` semantic token cheat sheet

| Token | Meaning |
|---|---|
| `Primary` | Focused borders, highlights |
| `Secondary` | Less-prominent highlights |
| `Accent` | Decorative pops |
| `Text` | Default foreground |
| `TextDim` | Secondary labels |
| `Border` | Unfocused borders |
| `Bg` | Background (often `Color::Reset`) |
| `Success` / `Warning` / `Error` | Feedback states |
| `SelectedBg` / `SelectedFg` | List/table selection |
| `Surface` / `SurfaceHover` / `SurfaceText` | Cards and elevated content |
| `Info` / `Link` / `FocusRing` | Aliases of `primary` (future-extensible) |
| `Custom(Color)` | Literal passthrough |

Resolve: `ui.color(ThemeColor::Primary)`, `ui.theme().resolve(ThemeColor::Surface)`.

### `WidgetColors` precedence

Resolution order per field: `theme_*` > literal field > theme default.

<!-- slt-check: {"name": "widget_colors_precedence", "features": [], "context": "ui", "setup": "let mut tbl = TableState::new(vec![\"Name\"], vec![vec![\"Alice\"]]);"} -->
```rust
let c = WidgetColors::new()
    .theme_bg(ThemeColor::Surface)     // takes precedence over bg below
    .bg(Color::Black)
    .theme_accent(ThemeColor::Accent);
if ui.button("Save").colors(&c).clicked { /* save */ }
ui.table_colored(&mut tbl, &c);
```

### Style chain

<!-- slt-check: {"name": "style_chain", "features": [], "context": "ui"} -->
```rust
let style = Style::new().fg(Color::Cyan).bg(Color::Reset).bold().italic().dim().underline().reversed().strikethrough();
ui.styled("styled", style);
```

Modifiers can be built as flags and assigned:
<!-- slt-check: {"name": "modifier_flags", "features": [], "context": "ui"} -->
```rust
let mut s = Style::new().fg(Color::Cyan);
s.modifiers = Modifiers::BOLD | Modifiers::UNDERLINE;
ui.styled("bold + underline", s);
```
(`Style::new()`, `.fg(...)`, `.bg(...)` are `const`, usable in `const` recipes.)

---

## 9. Testing

<!-- slt-check: {"name":"headless_testing", "features":[], "context":"module"} -->
```rust
use slt::{Border, Color, EventBuilder, TestBackend};

fn headless_snapshot_and_click() {
    // Snapshot
    let mut tb = TestBackend::new(40, 8);
    tb.render(|ui| {
        ui.bordered(Border::Rounded).p(1).col(|ui| {
            ui.text("Hello").bold().fg(Color::Cyan);
        });
    });
    tb.assert_contains("Hello");
    assert!(tb.line(2).contains("Hello"));

    // Interaction (multi-frame: session state persists across render calls)
    let mut count = 0;
    tb.render(|ui| { if ui.button("+1").clicked { count += 1; } });
    tb.render_with_events(
        EventBuilder::new().click(2, 0).build(),
        /*focus_index=*/0, /*prev_focus_count=*/1,
        |ui| { if ui.button("+1").clicked { count += 1; } },
    );
    assert_eq!(count, 1);
}
```

`tb.to_string_trimmed()` returns the whole buffer as a `\n`-joined string for `insta::assert_snapshot!`.

---

## 10. Error modes AI commonly hits

1. **Closure capture lifetime.** Deferred `.draw(f)`, `.draw_with(d, f)`, and `.draw_interactive(f)` require `'static`. Use `.draw_precomputed(w, h, f)` when the draw can target known cell dimensions and should borrow local data without a deep clone.
2. **Hook ordering.** `use_state` / `use_memo` must be called in the same order every frame. Putting them inside a conditional that flips between frames will panic with a type mismatch message.
3. **Focus on frame 1.** `ui.focus_count()` returns 0 on the first frame; `Response.rect` is zero-sized until the frame after a widget's first appearance. Avoid relying on these for frame-0 decisions.
4. **Borrow across `row()` / `col()`.** The closure passed to `.col(...)` borrows `ui` mutably. You cannot hold a reference to `ui` or to the parent `Response` inside that closure. Capture what you need before or read the `Response` after the call.
5. **`ui.bg(c)` vs container bg.** `ui.text("x").bg(color)` sets the *text* background only. For a container background use `ui.container().bg(color).col(|ui| { ... })`.
6. **`line()` vs `row()`.** Both are horizontal and return `Response`. `row()` has gap and participates in flex fully. `line()` is zero-gap, intended for inline rich text and no interaction.
7. **`_colored` argument order.** Every stateful `_colored` variant takes `&WidgetColors` *as the last argument* (e.g. `ui.list_colored(&mut state, &colors)`, `ui.table_colored(&mut state, &colors)`). Builder widgets take colors as a chained method instead: `ui.button("Save").colors(&colors)`, `ui.badge("ok").color(c)`.
8. **`*State` must outlive the closure.** Create `TextInputState`, `ListState`, etc. **outside** `slt::run(...)` (or use hooks). Creating them fresh inside the closure throws away the state every frame.
9. **`TextInputState` vs `TextareaState`.** Single-line vs multi-line. They are not interchangeable.
10. **`consume_key` vs `key`.** `consume_key(c)` marks the event consumed so other widgets ignore it. Use this in custom widgets or global shortcuts that should not bubble.
11. **Two animation layers.** `ui.animate_bool(id, on)` / `ui.animate_value(id, target, ticks)` are `Context` methods with built-in state (linear, CSS-transition-like). `Tween`, `Spring`, `Keyframes`, `Sequence`, `Stagger` are plain structs: construct outside the closure, mutate/sample inside using `ui.tick()`.
12. **Button color overrides.** `ui.button(label).variant(ButtonVariant::Danger)` picks a theme-aware variant; `ui.button(label).colors(&WidgetColors::new().accent(c))` is a literal override; chaining both layers the colors on top of the variant.
13. **Inline mode does not own the screen.** `run_inline(height, f)` reserves `height` rows below the cursor; other output still scrolls. No alt-screen, no F12 overlay in some terminals.
14. **`cargo add superlighttui`** pulls in the `crossterm` default feature. Without it you lose `run()` / `run_inline()` / clipboard; keep `Backend`, `AppState`, `frame()`.
15. **Feature-gated APIs.** `ui.qr_code(...)` needs `qrcode`. `run_async(...)` needs `async`. `ui.code_block(code).lang(...)` tree-sitter highlighting needs `syntax` or the matching `syntax-*` flag.
16. **`ui.screen` isolates hooks.** `use_state` inside one `screen(...)` does not share state with another. Use plain app state if you need cross-screen values.
17. **Modal traps focus.** Tab and Shift+Tab cycle only within the current modal; outside widgets are not reachable while the modal is open.
18. **Breakpoint method naming.** `xs_w`, `sm_w`, `md_w`, `lg_w`, `xl_w`, `w_at(bp, v)` — the breakpoint is a **prefix** on the base name (not `w_xs`).
19. **Hover needs `MouseMode::Hover`.** The mouse is on by default (`MouseMode::Drag`, or `Click` over SSH/mosh), so clicks, wheel, and drags work with plain `slt::run`, but pointer moves without a button held are only reported in `MouseMode::Hover`: `hovered`, group hover, and tooltips stay inert until you opt in. Over SSH, drags need an explicit `Drag`. Native terminal selection: Shift-drag in most terminals, or `MouseMode::Off`.
20. **Focused text fields own their keys.** While `text_input` / `textarea` / a form field / an open command palette has focus, `ui.key('q')`, `ui.key_code(..)` and `ui.shortcut(..)` skip the keys the field edits with, in any call order. Chords (Ctrl/Super/Meta), Esc, Tab and F-keys still reach shortcuts; `ui.raw_key(c)` sees every press.
21. **Builder arguments cannot borrow `ui`.** `ui.badge("ok").color(ui.theme().success)` does not compile because the builder already holds `&mut ui`. Bind first: `let ok = ui.theme().success; ui.badge("ok").color(ok);`. Same for `Text`, `Button`, `Stat`, `Help`, `Separator`, `ProgressBar`, `Slider`, `LineChart`.
22. **Storing a builder result.** `let r = ui.button("Save");` keeps `ui` borrowed until `r` is last used. If you call `ui.` again before reading `r`, write `let r = ui.button("Save").show();` (an owned, `Copy` `Response`). The same applies to `vec![ui.button("a").show(), ...]` and returning a response from a closure.

---

## 11. Selected crate-root re-export map

These are the high-value names commonly used as `slt::X`. The generated
docs.rs index and `src/lib.rs` are authoritative for the complete, feature-gated
surface. For an app, `use slt::prelude::*;` imports the run entry points,
`Context`, `ContainerBuilder`, `Response`, `Text`, `Button`, `ButtonVariant`,
`WidgetColors`, `State` / `StateHandle`, the style vocabulary, `KeyCode` /
`KeyModifiers` and the common widget state structs in one line (it leaves out
the `crossterm` re-export so it never shadows your own dependency).

```text
// From test_utils
EventBuilder, FrameRecord, TestBackend, TestSequence

// From anim
Keyframes, LoopMode, Sequence, Spring, Stagger, Tween
// (+ slt::anim::{ease_linear, ease_in_quad, ease_out_quad, ease_in_out_quad,
//   ease_in_cubic, ease_out_cubic, ease_in_out_cubic, ease_out_elastic, ease_out_bounce, lerp})

// From buffer
Buffer

// From cell
Cell

// From chart
Candle, ChartBuilder, ChartConfig, Dataset, LegendPosition, Marker

// From context (crate root)
Anchor, Bar, BarChartConfig, BarDirection, BarGroup, Breadcrumb, CanvasContext,
CodeBlock, ContainerBuilder, Context, Gauge, GutterOpts, LineGauge, Memo,
Response, State, StateHandle, TreemapItem, Widget
// Widget builders: Badge, Button, Help, LineChart, ProgressBar, Separator,
// Slider, Stat, Text


// From event
Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKey, MouseButton,
MouseEvent, MouseKind

// From halfblock
HalfBlockImage

// From keymap
Binding, KeyMap, PublishedKeymap, WidgetKeyHelp

// From layout
Direction

// From palette
Palette
// (+ slt::palette::tailwind::{SLATE, GRAY, ZINC, NEUTRAL, STONE,
//   RED, ORANGE, AMBER, YELLOW, LIME, GREEN, EMERALD, TEAL, CYAN, SKY,
//   BLUE, INDIGO, VIOLET, PURPLE, FUCHSIA, PINK, ROSE})

// From rect
Rect

// From style
Align, Border, BorderSides, Breakpoint, Color, ColorDepth, ColorParseError,
Constraints, ContainerStyle, HeightSpec, Justify, Margin, Modifiers, Padding,
Spacing, Style, SyntaxPalette, Theme, ThemeBuilder, ThemeColor, UnderlineStyle,
WidgetColors, WidgetTheme, WidthSpec
// Feature-gated: ThemeWatcher (`theme-watch`), ThemeFile and ThemeLoadError (`serde`)

// From widgets
AlertLevel, ApprovalAction, ButtonVariant, CalendarState, ChordState,
CodeEditorState, ColorPickerState, CommandPaletteState, ContextItem, DirectoryTreeState,
FileEntry, FilePickerScanError,
FilePickerScanOperation, FilePickerScanStatus, FilePickerState, FormField, FormFieldResponse,
FormState, GridColumn, ListState, ModeState, MultiSelectState, PaletteCommand,
NumberInputState, PaginatorState, PaginatorStyle, PickerMode, RadioState,
RichLogEntry, RichLogState, SchedulerState, ScreenState, ScrollState,
SelectState, SliderOpts, SpinnerState, SplitPaneState, StaticOutput,
StreamingMarkdownState, StreamingTextState, TableColumn, TableState, TabsState,
TextInputState, TextareaState, ToastLevel, ToastMessage, ToastState,
ToolApprovalState, TreeNode, TreeState, Trend

// Runtime entry points (most need `crossterm` feature)
Backend, AppState, RunConfig, MouseMode,
run, run_with, run_inline, run_inline_with,
run_static, run_static_with,
frame, frame_owned,
detect_color_scheme, read_clipboard, Blitter, Capabilities, ColorScheme,
InlineTerminal, Terminal
// `async`: run_async, run_async_with, AsyncRunError, AsyncRunHandle, AsyncSender,
// TaskHandle, TaskOutcome
```

---

## 12. Feature flags cheat sheet

| Flag | Unlocks | Default? |
|---|---|---|
| `crossterm` | `run()`, `run_inline()`, `run_static()`, clipboard, sixel_image, color-scheme detection | Yes |
| `async` | `run_async()`, `run_async_with()` — needs tokio | No |
| `serde` | `Serialize` / `Deserialize` for style, theme, layout types | No |
| `theme-watch` | TOML theme hot reload through `ThemeWatcher` | No |
| `bidi` | UAX #9 bidirectional text reordering | Yes |
| `image` | Image-loading helpers | No |
| `qrcode` | `ui.qr_code(...)` | No |
| `syntax` | All tree-sitter `syntax-*` bundles | No |
| `syntax-rust`, `syntax-python`, `syntax-typescript`, ... | Per-language tree-sitter grammar | No |
| `kitty-compress` | zlib-compressed Kitty image uploads | No |
| `full` | `crossterm` + `async` + `serde` + `theme-watch` + `image` + `qrcode` + `kitty-compress` + `bidi` (not `syntax`) | No |

Without `crossterm`, you keep `Backend`, `AppState`, `frame()`, `Context`, all widgets, events, styles, layout, charts — enough for a custom backend (WASM canvas, GUI embed, test harness).

---

## 13. Minimal skeletons to copy

### Full-screen app (most common)
<!-- slt-check: {"name":"fullscreen", "features":["crossterm"], "context":"module"} -->
```rust
use slt::{Border, Color, Context, KeyCode};

fn main() -> std::io::Result<()> {
    slt::run(|ui: &mut Context| {
        if ui.key('q') || ui.key_code(KeyCode::Esc) { ui.quit(); }
        ui.bordered(Border::Rounded).title("App").p(1).col(|ui| {
            ui.text("Body").fg(Color::Cyan);
        });
    })
}
```

### Inline app below prompt
<!-- slt-check: {"name":"inline", "features":["crossterm"], "context":"module"} -->
```rust
fn main() -> std::io::Result<()> {
    slt::run_inline(3, |ui| {
        ui.row(|ui| {
            ui.text("Working... ");
            ui.spinner(&slt::SpinnerState::dots());
        });
    })
}
```

### Custom backend (no crossterm)
<!-- slt-check: {"name":"custom_backend", "features":[], "context":"module"} -->
```rust
use slt::{AppState, Backend, Buffer, Event, Rect, RunConfig};

struct Null { buf: Buffer }
impl Backend for Null {
    fn size(&self) -> (u32, u32) { (self.buf.area.width, self.buf.area.height) }
    fn buffer_mut(&mut self) -> &mut Buffer { &mut self.buf }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}

fn main() -> std::io::Result<()> {
    let mut backend = Null { buf: Buffer::empty(Rect::new(0, 0, 80, 24)) };
    let mut state = AppState::new();
    let config = RunConfig::default();
    let events: Vec<Event> = vec![];
    let _ = slt::frame(&mut backend, &mut state, &config, &events, &mut |ui| {
        ui.text("hello from custom backend");
    })?;
    Ok(())
}
```

### Async with tokio messages
<!-- slt-check: {"name":"async_skeleton", "features":["async","crossterm"], "context":"module", "tokio":true} -->
```rust
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut log: Vec<String> = Vec::new();
    let run = slt::run_async::<String>(move |ui, msgs| {
        log.extend(msgs.drain(..));
        ui.col(|ui| { for m in &log { ui.text(m); } });
    })?;
    let tx = run.sender();
    let _ = tx.send("started".into()).await;
    tokio::signal::ctrl_c().await?;
    run.cancel_and_join().await.map_err(std::io::Error::other)
}
```

---

## 14. Quick index — "how do I ..."

- Quit: `ui.quit()` (Ctrl+C also quits).
- Read a key: `ui.key('q')`, `ui.key_code(KeyCode::Enter)`, `ui.key_mod('s', KeyModifiers::CONTROL)`.
- Read a cross-frame multi-key chord (vi `gg`, leader keys like `<space>ff`): `ui.key_chord("gg")`. Buffers partial input across frames; resets on a mismatching key or after a timeout.
- Mouse click position: `ui.mouse_down()` / `ui.mouse_pos()`.
- Scroll: `ui.scroll_up()` / `ui.scroll_down()`.
- Paste: `ui.paste()`.
- Copy: `ui.copy_to_clipboard("...")`.
- Current frame: `ui.tick()`.
- Terminal size: `ui.width()`, `ui.height()`.
- Current breakpoint: `ui.breakpoint()`.
- Switch theme: `ui.set_theme(Theme::dracula())`.
- Set window title: `RunConfig::default().title("My App")`.
- Change scroll speed: `RunConfig::default().scroll_speed(3)` or `ui.set_scroll_speed(3)`.
- Mouse mode: on by default; `RunConfig::default().mouse_mode(MouseMode::Hover)` for hover/tooltips, `MouseMode::Off` to disable.
- Ask the terminal for a color scheme: `slt::detect_color_scheme()` (returns `ColorScheme`).
- Query clipboard content: `slt::read_clipboard()` (requires `crossterm`).
- Render to a test buffer: `TestBackend::new(w,h).render(|ui| {...}); tb.assert_contains("x")`.

---

## 15. Widget state — detailed field reference

Public fields, constructors and methods of every `*State` type live in the
[State API Guide](state.md), one `## <TypeName>` section per type. For a
one-row-per-widget summary, see the
[state quick reference](widgets.md#state-type-quick-reference).

---

## 16. ChartBuilder — the full multi-series flow

<!-- slt-check: {"name":"chart_series", "features":[], "context":"ui"} -->
```rust
let prices = [(0.0, 3.0), (1.0, 4.0), (100.0, 2.0)];
let points = [(10.0, 8.0), (80.0, 12.0)];
ui.chart(|c| {
    c.line(&prices);                          // series 1
    c.area(&prices).color(Color::Cyan);        // filled area
    c.scatter(&points).marker(Marker::Dot);    // per-series marker
    c.bar(&[(0.0, 3.0), (1.0, 4.0), (2.0, 2.0)]);
    c.grid(true);
    c.xlim(0.0, 100.0);
    c.ylim(0.0, 50.0);
    c.legend(LegendPosition::TopRight);
}, 60, 20);
```

### Bar coordinate contract

General `ChartBuilder::bar` is **ordinal**, not a numeric-x bar plot. Finite
`(x, y)` pairs get equal-width slots in input order. X values contribute to
automatic axis bounds but do not control bar position or spacing. Thus
`[(0.0, 2.0), (1.0, 3.0), (100.0, 4.0)]` produces three equally spaced bars;
reversing those pairs reverses the heights to `4, 3, 2`. Changing only x values
does not move bars. Multiple bar datasets overlay, rather than group, slots.
Line and scatter series do use numeric x positions; overlaying them with
ordinal bars does not imply coordinate alignment. Use `bar_chart` for named
categories. Numeric-x bars would require a separate, explicit API contract.

Exact zero, including `-0.0`, draws no bar. Nonzero values include the baseline
cell; tiny nonzero values rounded to that row and one-row plots retain one
cell. Histograms use ordered equal-width numeric bins, count only finite
inputs, and position ticks at actual bin edges including the final endpoint.
Overlapping tick labels are omitted, never redistributed to different values.

Use `ui.sparkline(data, width)` for a single inline trend. `ui.histogram(data, w, h)` for a frequency chart. `ui.candlestick(&[Candle], up, down)` for OHLC; `_hd` variant renders heavy `┃` wicks.

---

## 17. Rendering internals (when debugging)

The engine runs **four top-level DFS traversals** of the layout tree per frame (steps 4, 5, 6, 7 below). The F12 debug overlay adds one or two more when enabled.

```
Frame N:
  1. poll events (non-blocking tick_rate poll)
  2. your closure runs — every ui.*() records a Command
  3. post-closure normalization: focus keys, notifications, tooltips
  4. build_tree:   commands -> LayoutNode tree                 [DFS 1/4]
  5. flexbox:      compute() walks the tree and resolves sizes [DFS 2/4]
  6. collect_all:  one DFS gathers scroll/hit/focus/group/
                   content/raw-draw data into FrameData        [DFS 3/4]
                   — this single walk replaced 7 separate
                     collect_* sub-walks; the top-level
                     pipeline is still four passes
  7. render:       write cells to back buffer (clip stack +
                   viewport culling)                           [DFS 4/4]
  8. deferred .draw()/.draw_with() callbacks replay into their rects
  9. diff back vs front buffer, apply_style_delta on changed cells, flush
 10. swap buffers, tick += 1
Frame N+1 reads prev_* data populated in steps 6/8.
```

With `RunConfig::idle_redraw(false)` the loop runs steps 2–10 only when
something can change: input, resize, async messages, a resume, a due
`ui.request_frame()` / `ui.request_frame_after(d)` (built-in animations,
spinners, toasts, timers and tooltips request their own), or one follow-up
frame after each of these. Otherwise it only polls input, so `tick` pauses.

Diagnostics: F12 toggles an overlay showing container bounds and FPS. `ui.debug_enabled()` is `true` while it is on. `RunConfig::max_fps(Some(u32))` caps rendering. Synchronized output (DECSET 2026) is emitted automatically on supported terminals to avoid tearing.

---

## 18. Glossary

- **Immediate mode.** No retained tree. Every frame re-describes the UI from scratch. Allocations per frame are dominated by the Command buffer (one `Vec<Command>` reused).
- **Closure.** The function you pass to `slt::run(...)`. Runs once per frame on the main thread.
- **Response.** Interaction record of a widget from hit-testing; see §4.
- **State (hook).** `ui.use_state(|| init)` — per-instance persistent slot in the frame state. Stable across frames if called in the same order.
- **State (widget).** Owned struct like `TextInputState`, `ListState`, etc. that persists across frames because you hold it.
- **Prev-frame feedback.** Layout data collected in frame N is used for hit testing in frame N+1 ([previous-frame rule](../guide/02-state.md#the-previous-frame-rule)).
- **Container.** Any box that holds children — `row`, `col`, `modal`, `overlay`, `scrollable`, `bordered`, `container()`.
- **Raw draw.** Direct buffer access via `.draw(|buf, rect| {...})`. Bypasses widget abstractions.
- **Interaction slot.** Logical index the runtime assigns to each interactive widget so `prev_hit_map` can hit-test it. Call `register_focusable()` / `interaction()` to reserve one.

---

## 19. Compatibility

- MSRV is tracked in `Cargo.toml` under `rust-version`; MSRV bumps only on minor releases.
- `0.x.y` patches are backward compatible. `0.x → 0.y` minor versions may include breaking changes until 1.0.
- Crate-root re-exports are the preferred stable import path; public module paths
  shown by docs.rs are also part of the published Rust API unless explicitly
  documented otherwise.
- Callable methods and named types receive at least one minor release of
  `#[deprecated]` warnings before removal. Before 1.0, invariant-sensitive public
  fields may be encapsulated only in a minor release with explicit migration notes
  and replacement accessors.

---

## 20. See also

- Runnable example targets (`cargo run --example <name>`; `Cargo.toml` sets `autoexamples = false`, so only these 29 names work):
  - Entry: `hello`, `counter`, `demo`, `color_picker`, `inline`, `async_demo` (needs `--features async`), `error_boundary_demo`.
  - Tours: `cookbook_tour`, `showcase_tour`, `text_tour`, `canvas_tour`, `system_tour` (needs `--features async`), `widgets_tour`, `interaction_tour`, `widgets_extra`.
  - Standalone feature demos: `feature_showcase`, `anim`, `demo_fire`, `demo_game`, `demo_kitty_image`, `demo_raw_draw`.
  - Tools and reports: `debug_selection`, `demo_key_test`, `mouse_probe`, `theme_hot_reload` (needs `--features theme-watch`), `perf_interactive`, `perf_regression`, `perf_audit`, `testing_report`.
  - Other files in `examples/` (`demo_dashboard`, `demo_cli`, `gauge`, `modal_trap`, ...) are source modules shown inside a tour or rendered by a test. See [Examples](examples.md) for the mapping.
- Reference: [Widget Guide](widgets.md), [State API Guide](state.md), [AI guide](ai-guide.md), [Cookbook](cookbook.md), [Debugging](debugging.md), [Features](features.md), [Examples](examples.md), [Migration](migration.md).
- Guides: [Quick start](../guide/01-quick-start.md), [State](../guide/02-state.md), [Patterns](../guide/03-patterns.md), [Interaction](../guide/04-interaction.md), [Overlays](../guide/05-overlays.md), [Theming](../guide/06-theming.md), [Animation](../guide/07-animation.md), [Testing](../guide/08-testing.md), [Runtime modes](../guide/09-runtime-modes.md), [Browser](../guide/10-browser.md), [Terminal compatibility](../guide/11-terminal-compatibility.md), [Coming from web](../coming-from/web.md).
- Internals: [Design principles](../internals/design-principles.md), [Architecture](../internals/architecture.md), [Performance](../internals/performance.md).
