# SuperLightTUI — Complete Reference (LLM-optimized)

> Version: 0.19.2. This document condenses the full SLT API and common patterns into one file. An LLM agent should be able to load this alone and generate any normal SLT app without further reads.

---

## 1. Mental Model

SLT is **immediate-mode**. `slt::run(|ui| { ... })` calls your closure every frame. The closure describes what the UI looks like right now. The library records commands, builds a layout tree, runs flexbox, diffs against the previous frame, and writes only the changed cells.

State lives in **normal Rust** variables, structs, or — for widget-local persistent state — `ui.use_state(|| ...)` hooks. There is no App trait, no Model/Update/View, no message enum, no retained component tree. Control flow is plain `if` / `for`. Tab cycles focus automatically. Ctrl+C quits by default. Interactive widgets return a `Response { clicked, hovered, changed, focused, rect }`. Hit-testing uses the **previous** frame's layout, so `Response.rect` is only meaningful from frame 2 onward.

---

## 2. 60-second grammar

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
| `slt::run(|ui| { ... })` | Full-screen alternate-screen runtime. Default 60 FPS. Needs `crossterm` (default feature). |
| `slt::run_with(config, |ui| { ... })` | Same plus a `RunConfig`. |
| `slt::run_inline(height, |ui| { ... })` | Render a fixed `height` below the current cursor. No alt-screen. |
| `slt::run_inline_with(height, config, f)` | Inline + config. |
| `slt::run_static(&mut out, height, f)` | Static scrollback lines above a fixed inline UI (log-style). |
| `slt::run_static_with(&mut out, height, config, f)` | Same plus config. |
| `slt::run_async(|ui, msgs: &mut Vec<M>| { ... })` | Requires `async` feature. Returns `tokio::sync::mpsc::Sender<M>` for pushing messages to the UI. |
| `slt::run_async_with(config, f)` | Async + config. |
| `slt::frame(&mut backend, &mut state, &config, &events, &mut f)` | Low-level per-frame driver for custom backends. Returns `Ok(true)` to keep going, `Ok(false)` when quit. |
| `slt::frame_owned(&mut backend, &mut state, &config, events, &mut f)` | Same as `frame()` but takes `Vec<Event>` by value (zero-copy when callers already own a vector — no slice→Vec clone). |

`RunConfig` (builder, all setters consume `self`):

```rust
slt::RunConfig::default()
    .tick_rate(Duration::from_millis(16))
    .mouse(true)
    .kitty_keyboard(true)
    .theme(Theme::dracula())
    .color_depth(ColorDepth::TrueColor)
    .max_fps(60)
    .no_fps_cap()             // disable the FPS limit (sets max_fps = None)
    .scroll_speed(3)
    .title("My App")
    .widget_theme(WidgetTheme::new().button(WidgetColors::new().accent(Color::Cyan)))
```

`Backend` trait (for non-terminal targets): implement `fn size(&self) -> (u32, u32)`, `fn buffer_mut(&mut self) -> &mut Buffer`, `fn flush(&mut self) -> io::Result<()>`. Drive with `slt::frame`.

Embedding with your own event loop (issue #278): the built-in crossterm backends `slt::Terminal` / `slt::InlineTerminal` are public — construct one, render into `buffer_mut()`, call `flush()`, and feed input you read yourself through `slt::event::from_crossterm`. The `crossterm` crate is re-exported as `slt::crossterm` so you never pin a mismatched version.

`AppState::new()` holds frame-to-frame state; reuse the same instance across frames. `AppState::tick()`, `::fps()`, `::set_debug(bool)`.

---

## 4. Core types (from `src/lib.rs` re-exports)

### Runtime and backend
| Type | One-line |
|---|---|
| `Context` | Per-frame handle you call widgets on (`ui`). |
| `Response` | `{ clicked: bool, hovered: bool, changed: bool, focused: bool, rect: Rect }`. `Response::none()` for a zero struct. |
| `State<T>` | Handle returned from `use_state`; read with `.get(ui)`, mutate with `.get_mut(ui)`. |
| `Widget` | Trait for custom widgets (`type Response; fn ui(&mut self, ctx: &mut Context) -> Self::Response`). |
| `ContainerBuilder` | Fluent builder returned by `container()`, `bordered()`, `scrollable()`, `group()`. Finalize with `.col(f)`, `.row(f)`, `.draw(f)`, `.draw_with(d, f)`, `.draw_interactive(f)`. |
| `CanvasContext` | Braille canvas passed to `ui.canvas(w, h, |cv| { ... })`. |
| `Backend` | Trait for custom render targets. |
| `AppState` | Opaque session state passed to `frame()`. |
| `RunConfig` | Per-run configuration. |
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
| `Keyframes` | Timeline with stops. `Keyframes::new(duration).at(0.0, 0.0).at(0.5, 100.0).at(1.0, 0.0).loop_mode(LoopMode::Loop)`. |
| `Sequence` | Chained tween segments in order. |
| `Stagger` | `Stagger::new(from, to, duration).delay(ticks).items(count)`. `.value(tick, item_index)`. `.is_done()` reports completion of the last sampled item; `.is_all_done(tick, item_count) -> bool` reports completion of every item in the batch. |
| `LoopMode` | `Once`, `Loop`, `PingPong`. |
| Easings (under `slt::anim::*`) | `ease_linear`, `ease_in_quad`, `ease_out_quad`, `ease_in_out_quad`, `ease_in_cubic`, `ease_out_cubic`, `ease_in_out_cubic`, `ease_out_elastic`, `ease_out_bounce`. |

### Charts and visualization
| Type | Notes |
|---|---|
| `ChartBuilder` | Passed to `ui.chart(|c| {...}, w, h)`. `.line(&[f64])`, `.area(...)`, `.scatter(...)`, `.bar(...)`, `.grid(bool)`, `.x_range(min, max)`, `.y_range(min, max)`, `.marker(..)`, `.legend(pos)`, ... |
| `ChartConfig` | Config struct for chart builder internals. |
| `Dataset` | Single series description. |
| `Marker` | Marker shapes: `Dot`, `Block`, `Bar`, `HalfBlock`, `Braille`. |
| `Candle` | `{ open, high, low, close: f64 }`. |
| `LegendPosition` | `Top`, `Bottom`, `Left`, `Right`, `None`. |
| `Bar` | `Bar::new(label, value).color(c).text_value(s)`. |
| `BarGroup` | `BarGroup::new(label, bars).group_gap(n)`. |
| `BarDirection` | `Horizontal`, `Vertical`. |
| `BarChartConfig` | Bar chart configuration for `bar_chart_with` / `bar_chart_grouped_with`. |
| `TreemapItem` | `TreemapItem::new(label, value, color)`. |
| `HalfBlockImage` | Half-block 2-px-per-cell image. |

### Widget state types (all re-exported at `slt::`)

| State | Widget it drives |
|---|---|
| `TextInputState` | `text_input`, `text_input_colored` |
| `TextareaState` | `textarea` |
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
| `ButtonVariant` | `button_with` (`Default`, `Primary`, `Danger`, `Outline`) |
| `Trend` | `stat_trend` (`Up`, `Down`) |
| `FormState`, `FormField` | `form`, `form_field`, `form_submit` |
| `ScreenState` | `screen(name, &mut screens, f)` |
| `ModeState` | Multi-mode wrapper; each mode owns its own `ScreenState` |
| `StreamingTextState`, `StreamingMarkdownState` | `streaming_text`, `streaming_markdown` |
| `ToolApprovalState`, `ApprovalAction` | `tool_approval` |
| `ContextItem` | `context_bar(items)` |
| `RichLogState`, `RichLogEntry` | `rich_log` |
| `GridColumn` | Per-column spec for `grid_with` |
| `StaticOutput` | Scrollback buffer for `run_static` |

### Palette

`slt::Palette` plus `slt::palette::tailwind::{SLATE, GRAY, ZINC, NEUTRAL, STONE, RED, ORANGE, AMBER, YELLOW, LIME, GREEN, EMERALD, TEAL, CYAN, SKY, BLUE, INDIGO, VIOLET, PURPLE, FUCHSIA, PINK, ROSE}` — each palette exposes `.c50 .c100 .c200 .c300 .c400 .c500 .c600 .c700 .c800 .c900 .c950`.

---

## 5. Widget catalog (dense)

Legend: `Response = { clicked, hovered, changed, focused, rect }`. `&mut Self` means chain-return for style. `ContainerBuilder` means fluent builder you finalize with `.col/.row/.draw/.draw_interactive`. Every row lists: call, return, 1-line description, tiny example where load-bearing.

### 5.1 Text and inline display (`&mut Self` unless noted)

| Call | Notes |
|---|---|
| `ui.text(s)` | Render text. Chain: `.bold() .italic() .dim() .underline() .reversed() .strikethrough() .fg(c) .bg(c) .gradient(a,b) .wrap() .truncate() .grow(n) .align(a) .text_center() .text_right() .w(n) .h(n) .min_w(n) .max_w(n) .min_h(n) .max_h(n) .m(n) .mx(n) .my(n) .mt(n) .mr(n) .mb(n) .ml(n) .group_hover_fg(c) .group_hover_bg(c)`. |
| `ui.styled(s, style)` | Text with a prebuilt `Style`. |
| `ui.link(text, url)` | OSC 8 hyperlink. Opens URL on Enter/Space/click. |
| `ui.spacer()` | Invisible flex element pushes siblings apart. |
| `ui.separator()` | Horizontal divider. Variant: `ui.separator_colored(color)`. |
| `ui.timer_display(duration)` | Format `Duration` as HH:MM:SS.CC. |
| `ui.line(|ui| {...})` | Inline rich text — zero-gap row, no interaction. |
| `ui.line_wrap(|ui| {...})` | Same but wraps at word boundaries. |

### 5.2 Rich text (return `Response`)

| Call | Notes |
|---|---|
| `ui.markdown(text)` | Headings, bold, italic, links, lists, fenced code. |
| `ui.code_block(code)` | Plain fenced code. |
| `ui.code_block_lang(code, lang)` | Lang-tagged code block; picks tree-sitter grammar when the matching `syntax-*` feature is enabled. |
| `ui.code_block_numbered(code)` | With line numbers. |
| `ui.code_block_numbered_lang(code, lang)` | With line numbers + highlighting. |
| `ui.big_text(s)` | 8x8 bitmap text rendered as half-block pixels (4 rows tall). |

### 5.3 Status and info (return `Response` unless noted)

| Call | Notes |
|---|---|
| `ui.alert(msg, level)` | `AlertLevel`: `Info`, `Success`, `Warning`, `Error`. |
| `ui.badge(label)` / `ui.badge_colored(label, color)` | Pill. |
| `ui.key_hint(key)` | Keyboard shortcut badge. |
| `ui.stat(label, value)` / `ui.stat_colored(label, value, color)` | Label + value. |
| `ui.stat_trend(label, value, trend)` | `Trend::Up` or `Trend::Down`. |
| `ui.empty_state(title, desc)` / `ui.empty_state_action(title, desc, action)` | Centered empty message. |
| `ui.divider_text(label)` | Horizontal divider with centered label. |
| `ui.definition_list(&[(&str, &str)])` | Key/value list. |
| `ui.accordion(title, &mut open, |ui| {...})` | Collapsible. |
| `ui.confirm(question, &mut result)` | Yes/No dialog; `result` set on click. |
| `ui.breadcrumb(&[&str]) -> Option<usize>` | Clickable breadcrumb; returns clicked segment. |
| `ui.breadcrumb_with(&[&str], separator) -> Option<usize>` | Custom separator. |
| `ui.breadcrumb_response(&[&str]) -> (Response, Option<usize>)` | Same as `breadcrumb()` but also exposes the row `Response` (hover/focus/rect). |
| `ui.breadcrumb_response_with(&[&str], separator) -> (Response, Option<usize>)` | Custom separator + `Response`. The plain `breadcrumb()` / `breadcrumb_with()` are wrappers that drop the `Response`. |
| `ui.help(&[(&str, &str)])` | Key/description help bar. `ui.help_colored(bindings, key_color, text_color)`. |
| `ui.help_from_keymap(&keymap)` | From a `KeyMap`. |

### 5.4 Layout (row/col/container)

| Call | Returns | Notes |
|---|---|---|
| `ui.col(|ui| {...})` / `ui.col_gap(n, |ui| {...})` | `Response` | Vertical stack. |
| `ui.row(|ui| {...})` / `ui.row_gap(n, |ui| {...})` | `Response` | Horizontal stack. |
| `ui.grid(cols, |ui| {...})` | `Response` | Fixed-column grid. |
| `ui.grid_with(&[GridColumn], |ui| {...})` | `Response` | Per-column fractional/fixed widths. |
| `ui.container()` | `ContainerBuilder` | Fluent — chain style then `.col/.row/.draw`. |
| `ui.bordered(Border::Rounded)` | `ContainerBuilder` | Same as `container().border(...)`. |
| `ui.scrollable(&mut scroll)` | `ContainerBuilder` | Scrollable subtree. |
| `ui.scroll_col(&mut scroll, |ui|{...})` | `Response` | Vertically scrollable column — shortcut for `scrollable(state).grow(1).col(f)`. |
| `ui.scroll_row(&mut scroll, |ui|{...})` | `Response` | Horizontally scrollable row — shortcut for `scrollable(state).grow(1).row(f)`. |
| `ui.scrollbar(&scroll)` | `()` | Draw scrollbar track. |
| `ui.modal(|ui| {...})` | `Response` | Dimmed overlay, focus trapped. |
| `ui.overlay(|ui| {...})` | `Response` | Float without dimming. |
| `ui.tooltip(text)` | `()` | Shown near cursor on hover. |
| `ui.group(name)` | `ContainerBuilder` | Named group for shared hover/focus styling. |
| `ui.screen(name, &mut screens, |ui|{...})` | `()` | Render only when `screens.current() == name`. Isolates hook state and focus per screen. |
| `ui.push_screen(name)` / `ui.pop_screen()` / `ui.reset_screen()` | `()` | Navigate from **inside** a `screen(...)` closure. State updates when the closure returns; the destination renders next frame (issue #279). |
| `ui.form(&mut form_state, |ui|{...})` | `&mut Self` | Form container. |
| `ui.form_field(&mut field)` | `&mut Self` | One field (label + input + error). |
| `ui.form_submit(label)` | `Response` | Submit button. |

### 5.5 `ContainerBuilder` — full method list

Chain in any order; finalize with `.col(f)`, `.row(f)`, `.draw(f)`, `.draw_with(data, f)`, or `.draw_interactive(f)`.

Border & decoration:
`.border(Border)`, `.border_sides(BorderSides)`, `.border_top(bool)`, `.border_right(bool)`, `.border_bottom(bool)`, `.border_left(bool)`, `.border_x()`, `.border_y()`, `.rounded()`, `.border_style(Style)`, `.border_fg(Color)`, `.dark_border_style(Style)`, `.bg(Color)`, `.dark_bg(Color)`, `.text_color(Color)`, `.group_hover_bg(Color)`, `.group_hover_border_style(Style)`, `.title(impl Into<String>)`, `.title_styled(s, Style)`.

Spacing:
`.p(n)` / `.pad(n)`, `.px(n)`, `.py(n)`, `.pt(n)`, `.pr(n)`, `.pb(n)`, `.pl(n)`, `.padding(Padding)`, `.m(n)`, `.mx(n)`, `.my(n)`, `.mt(n)`, `.mr(n)`, `.mb(n)`, `.ml(n)`, `.margin(Margin)`, `.gap(n)`, `.row_gap(n)`, `.col_gap(n)`.

Size:
`.w(n)`, `.h(n)`, `.min_w(n)`, `.max_w(n)`, `.min_h(n)`, `.max_h(n)`, `.min_width(n)`, `.max_width(n)`, `.min_height(n)`, `.max_height(n)`, `.w_pct(u8)`, `.h_pct(u8)`, `.constraints(Constraints)`.

Layout:
`.grow(n)`, `.align(Align)`, `.center()`, `.align_self(Align)`, `.justify(Justify)`, `.space_between()`, `.space_around()`, `.space_evenly()`, `.flex_center()`, `.scroll_offset(u32)`.

Responsive (breakpoint-conditional; replace `w`/`h`/`min_w`/`max_w`/`p`/`m`/`gap` etc.):
`.xs_w(v)`, `.sm_w(v)`, `.md_w(v)`, `.lg_w(v)`, `.xl_w(v)`, `.w_at(Breakpoint, v)` (analogous for `h`, `min_w`, `max_w`, `p`, `gap`, …).

Style recipe:
`.apply(&ContainerStyle)`.

Finalization:
- `.col(|ui| {...}) -> Response`
- `.row(|ui| {...}) -> Response`
- `.line(|ui| {...}) -> Response` (inline row, zero-gap)
- `.draw(|buf, rect| { ... })` — raw buffer access; closure must be `'static`. No interaction.
- `.draw_with(data, |buf, rect, &data| { ... })` — same but owns per-frame `data` moved into the closure.
- `.draw_interactive(|buf, rect| { ... }) -> Response` — raw draw that also reports click/hover.

### 5.6 Inputs and actions (return `Response`)

| Call | State | Notes |
|---|---|---|
| `ui.button(label)` | — | Click button. Variants: `button_colored(label, &colors)`, `button_with(label, ButtonVariant)` (`Default`, `Primary`, `Danger`, `Outline`). |
| `ui.checkbox(label, &mut bool)` / `checkbox_colored` | — | Checkbox toggle. |
| `ui.toggle(label, &mut bool)` / `toggle_colored` | — | Toggle switch. |
| `ui.slider(label, &mut f64, range)` | — | Horizontal slider, `range: RangeInclusive<f64>`. Default step is `span / 20`. |
| `ui.slider_with_step(label, &mut f64, range, step)` | — | Slider with explicit step size — use when the default step is too coarse/fine (integers need `1.0`, fine controls `0.1`). |
| `ui.text_input(&mut state)` / `text_input_colored` | `TextInputState` | Single-line input. |
| `ui.textarea(&mut state, visible_rows)` | `TextareaState` | Multi-line editor. |
| `ui.select(&mut state)` / `select_colored` | `SelectState` | Dropdown. |
| `ui.radio(&mut state)` / `radio_colored` | `RadioState` | Radio group. |
| `ui.multi_select(&mut state)` | `MultiSelectState` | Multi-select with Space. |
| `ui.tabs(&mut state)` / `tabs_colored` | `TabsState` | Tab bar. |
| `ui.list(&mut state)` / `list_colored` | `ListState` | Scrollable selection list. |
| `ui.virtual_list(&mut state, visible_height, |ui, idx| {...})` | `ListState` | Renders only visible rows. |
| `ui.table(&mut state)` / `table_colored` | `TableState` | Data table with sort/paginate/filter. |
| `ui.tree(&mut state)` | `TreeState` | Expandable tree. |
| `ui.directory_tree(&mut state)` | `DirectoryTreeState` | Filesystem tree. |
| `ui.calendar(&mut state)` | `CalendarState` | Month calendar with date selection. |
| `ui.file_picker(&mut state)` | `FilePickerState` | File browser dialog. |
| `ui.command_palette(&mut state)` | `CommandPaletteState` | Modal fuzzy-search. |

### 5.7 Feedback and progress

| Call | Returns | State | Notes |
|---|---|---|---|
| `ui.progress(ratio)` | `&mut Self` | — | Progress bar, `f64` 0.0..1.0. |
| `ui.progress_bar(ratio, width)` | `&mut Self` | — | Fixed-width. `progress_bar_colored(r, w, color)`. |
| `ui.spinner(&state)` | `&mut Self` | `SpinnerState` | `SpinnerState::dots()` or `::line()`. |
| `ui.toast(&mut state)` | `&mut Self` | `ToastState` | Render active toasts. |
| `ui.notify(message, level)` | `()` | — | Fire-and-forget toast. `ToastLevel`: `Info`, `Success`, `Warning`, `Error`. |

### 5.8 Visualization (return `Response`)

| Call | Notes |
|---|---|
| `ui.bar_chart(&[(&str, f64)], max_width)` | Horizontal bar chart. |
| `ui.bar_chart_with(bars, configure, max_size)` | Per-bar colors; `configure: impl FnOnce(&mut BarChartConfig)`. |
| `ui.bar_chart_grouped(&[BarGroup], max_width)` / `..._with` | Grouped bars. |
| `ui.bar_chart_stacked(&[BarGroup], max_height)` / `..._with` | Stacked bars. |
| `ui.sparkline(&[f64], width)` | Inline sparkline. |
| `ui.sparkline_styled(&[(f64, Option<Color>)], width)` | Per-point color. |
| `ui.line_chart(&[f64], w, h)` / `line_chart_colored(..., color)` | Line chart. |
| `ui.area_chart(&[f64], w, h)` / `area_chart_colored(..., color)` | Filled area. |
| `ui.scatter(&[(f64, f64)], w, h)` | Braille scatter. |
| `ui.histogram(&[f64], w, h)` / `histogram_with(..., configure, w, h)` | Histogram. |
| `ui.candlestick(&[Candle], up_color, down_color)` / `candlestick_hd(...)` | OHLC. |
| `ui.heatmap(data, w, h, low, high)` / `heatmap_halfblock(...)` | 2D color gradient. |
| `ui.treemap(&[TreemapItem])` | Squarified treemap. Uses `grow(1)` for auto-size. |
| `ui.canvas(w, h, |cv| {...})` | Braille canvas — see 5.11. |
| `ui.chart(|c| {...}, w, h)` | Multi-series chart via `ChartBuilder`. |
| `ui.qr_code(data)` | Requires `qrcode` feature. |

### 5.9 Images (return `Response`)

| Call | Notes |
|---|---|
| `ui.image(&HalfBlockImage)` | 2-px-per-cell half-block image. |
| `ui.kitty_image(rgba, pw, ph, cols, rows)` | Kitty graphics protocol. |
| `ui.kitty_image_fit(rgba, sw, sh, cols)` | Auto-fit to column width. |
| `ui.sixel_image(rgba, pw, ph, cols, rows)` | Sixel protocol (requires `crossterm`). |

### 5.10 AI-native and rich terminal (return `Response`)

| Call | State | Notes |
|---|---|---|
| `ui.streaming_text(&mut state)` | `StreamingTextState` | Incrementally rendered text. `state.push(chunk)`, `.clear()`. |
| `ui.streaming_markdown(&mut state)` | `StreamingMarkdownState` | Markdown stream. Same `.push()`/`.clear()`. |
| `ui.tool_approval(&mut state)` | `ToolApprovalState` | Tool call dialog. `ApprovalAction`: `Pending`, `Approved`, `Rejected`. |
| `ui.context_bar(&[ContextItem])` | — | Context display with token counts. `ContextItem::new(label, tokens)`. |
| `ui.rich_log(&mut state)` | `RichLogState` | Styled log viewer; `state.push(text, style)`, `.push_plain(t)`, `.push_segments(segs)`, `.clear()`, `.auto_scroll` bool. |

### 5.11 `CanvasContext` (the `cv` in `ui.canvas(w, h, |cv| {...})`)

Coordinate units are braille pixels: `cv.width() == cols*2`, `cv.height() == rows*4`.

| Call | Notes |
|---|---|
| `cv.dot(x, y)` | Single pixel. |
| `cv.line(x0, y0, x1, y1)` | Bresenham line. |
| `cv.rect(x, y, w, h)` / `cv.filled_rect(x, y, w, h)` | Rectangle. |
| `cv.circle(cx, cy, r)` / `cv.filled_circle(cx, cy, r)` | Circle. |
| `cv.triangle(x0,y0,x1,y1,x2,y2)` / `cv.filled_triangle(...)` | Triangle. |
| `cv.points(&[(usize, usize)])` | Batch dots. |
| `cv.polyline(&[(usize, usize)])` | Connected segments. |
| `cv.print(x, y, text)` | Text overlay at pixel position. |
| `cv.set_color(Color)` / `cv.color() -> Color` | Drawing color. |
| `cv.layer()` | Start new z-layer — later layers overlay earlier. |
| `cv.width()` / `cv.height()` | Pixel dimensions. |

### 5.12 Runtime and hook methods on `Context`

State:
- `ui.use_state(|| init) -> State<T>`. Read: `state.get(ui)`. Write: `*state.get_mut(ui) = v`.
- `ui.use_memo(&deps, |deps| compute) -> &T`. Recomputes only when `deps` changes (must impl `PartialEq + Clone`).

Focus:
- `ui.register_focusable() -> bool` — register current widget as focusable; returns whether it has focus this frame.
- `ui.interaction() -> Response` — reserve a click/hover slot without wrapping in a container.
- `ui.focus_index() -> usize` / `ui.set_focus_index(i)` / `ui.focus_count() -> usize` (previous-frame count).

Widgets & error handling:
- `ui.widget(&mut w) -> W::Response` — render a custom `Widget`.
- `ui.error_boundary(|ui| {...})` — panic-catching subtree.
- `ui.error_boundary_with(|ui| {...}, |ui, msg| {...})` — custom fallback.

Keyboard (non-consuming unless noted):
- `ui.key(c) -> bool`
- `ui.key_code(KeyCode) -> bool`
- `ui.key_mod(c, KeyModifiers) -> bool`
- `ui.key_chord("gg") -> bool` — cross-frame multi-key sequence (vi `gg`, leader keys); buffers partial input across frames and times out on inactivity. `key_chord_timeout(seq, ticks)` sets a per-call timeout.
- `ui.key_seq("gg") -> bool` — **deprecated** alias for `key_chord`.
- `ui.key_release(c)` / `ui.key_code_release(code)`
- `ui.raw_key_code(code)` / `ui.raw_key_mod(c, mods)` — ignore consumed state; for global shortcuts.
- `ui.consume_key(c)` / `ui.consume_key_code(code)` — mark event consumed.

Mouse:
- `ui.mouse_down() -> Option<(u32, u32)>`, `ui.mouse_up()`, `ui.mouse_drag()`
- `ui.mouse_down_button(button)`, `ui.mouse_up_button(button)`, `ui.mouse_drag_button(button)`
- `ui.mouse_pos() -> Option<(u32, u32)>`
- `ui.scroll_up()` / `ui.scroll_down()` / `ui.scroll_left()` / `ui.scroll_right()` — `-> bool`.

Clipboard and system:
- `ui.paste() -> Option<&str>` — bracketed paste content.
- `ui.copy_to_clipboard(text)` — OSC 52.
- `ui.quit()`.

Theme and environment:
- `ui.theme() -> &Theme`, `ui.set_theme(Theme)`.
- `ui.color(ThemeColor) -> Color`.
- `ui.spacing() -> Spacing`.
- `ui.is_dark_mode() -> bool`, `ui.set_dark_mode(bool)`.
- `ui.light_dark(light, dark) -> Color`.
- `ui.width() -> u32`, `ui.height() -> u32`, `ui.breakpoint() -> Breakpoint`.
- `ui.tick() -> u64`, `ui.debug_enabled() -> bool`.
- `ui.set_scroll_speed(u32)`, `ui.scroll_speed() -> u32`.

---

## 6. Common patterns (compressed)

### 6.1 App state in plain Rust
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
```rust
let count = ui.use_state(|| 0i32);
ui.text(format!("{}", count.get(ui)));
if ui.button("+1").clicked { *count.get_mut(ui) += 1; }
```
Hook calls must run in the same order every frame. Do not put them inside `if`/`match` branches.

### 6.3 Derived state
```rust
let filtered = ui.use_memo(&(query.clone(), items.len()), |(q, _)| {
    items.iter().filter(|i| i.contains(q)).cloned().collect::<Vec<_>>()
});
```

### 6.4 Helper extraction
When a builder chain repeats, extract a helper:
```rust
fn panel(ui: &mut Context, title: &str, f: impl FnOnce(&mut Context)) {
    let _ = ui.bordered(Border::Rounded).title(title).p(1).grow(1).col(f);
}
```

### 6.5 Screen navigation
```rust
let mut screens = ScreenState::new("home");
slt::run(|ui| {
    ui.screen("home", &mut screens, |ui| {
        if ui.button("Settings").clicked { ui.push_screen("settings"); }
    });
    ui.screen("settings", &mut screens, |ui| {
        if ui.button("Back").clicked { ui.pop_screen(); }
    });
});
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
```rust
let mut modes = ModeState::new("app", "home");
modes.add_mode("settings", "general");
slt::run(|ui| {
    if ui.key('1') { modes.try_switch_mode("app"); }
    if ui.key('2') { modes.try_switch_mode("settings"); }
    let screens = modes.screens_mut();
    ui.screen("home", screens, |ui| { ui.text("Home"); });
    ui.screen("general", screens, |ui| { ui.text("Settings/General"); });
});
```

### 6.7 Form with validation

`FormValidator` is `type FormValidator = fn(&str) -> Result<(), String>`. Pass a fixed-size slice of function pointers.

```rust
let mut form = FormState::new()
    .field(FormField::new("Email").placeholder("you@example.com"))
    .field(FormField::new("Password"));

slt::run(move |ui| {
    ui.form(&mut form, |ui, form| {
        for field in &mut form.fields { let _ = ui.form_field(field); }
        if ui.form_submit("Submit").clicked {
            let ok = form.validate(&[
                |v| if v.contains('@') { Ok(()) } else { Err("Invalid email".into()) },
                |v| if v.len() >= 8 { Ok(()) } else { Err("Too short".into()) },
            ]);
            if ok { form.submitted = true; }
        }
    });
});
```

For single inputs without `FormState`:
```rust
let mut email = TextInputState::with_placeholder("you@example.com");
ui.text_input(&mut email);
email.validate(|v| if v.contains('@') { Ok(()) } else { Err("Invalid".into()) });
if let Some(err) = &email.validation_error { ui.alert(err, AlertLevel::Error); }
```

### 6.8 Modal confirmation
```rust
let show_confirm = ui.use_state(|| false);
if *show_confirm.get(ui) {
    ui.modal(|ui| {
        ui.text("Delete item?").bold();
        ui.row_gap(1, |ui| {
            if ui.button_with("Cancel", ButtonVariant::Default).clicked {
                *show_confirm.get_mut(ui) = false;
            }
            if ui.button_with("Delete", ButtonVariant::Danger).clicked {
                // perform delete
                *show_confirm.get_mut(ui) = false;
            }
        });
    });
}
```

### 6.9 Toast + fire-and-forget notify
```rust
let mut toasts = ToastState::new();
slt::run(move |ui| {
    if ui.button("Save").clicked { toasts.success("Saved!", ui.tick()); }
    // OR, without owning a ToastState:
    if ui.button("Ping").clicked { ui.notify("ping!", ToastLevel::Info); }
    ui.toast(&mut toasts);
});
```

### 6.10 Command palette
```rust
let mut palette = CommandPaletteState::new(vec![
    PaletteCommand::new("New File", "Create a new file"),
    PaletteCommand::new("Open", "Open a file"),
    PaletteCommand::new("Save", "Write changes"),
]);
slt::run(move |ui| {
    if ui.key_mod('p', KeyModifiers::CONTROL) { palette.toggle(); }
    let r = ui.command_palette(&mut palette);
    if r.changed {
        if let Some(i) = palette.last_selected { /* run command i */ }
    }
});
```

### 6.11 Real-time updates with tick
```rust
let start = std::time::Instant::now();
slt::run(move |ui| {
    let elapsed = start.elapsed();
    ui.text(format!("uptime: {}s", elapsed.as_secs()));
    // tick counter is monotonic frame index, not wall-clock
    ui.progress((ui.tick() % 60) as f64 / 60.0);
});
```

### 6.12 Async messages
```rust
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut messages: Vec<String> = Vec::new();
    let tx = slt::run_async::<String>(move |ui, msgs| {
        messages.extend(msgs.drain(..));
        ui.col(|ui| { for m in &messages { ui.text(m); } });
    })?;
    tokio::spawn(async move {
        for i in 0..10 {
            let _ = tx.send(format!("tick {i}")).await;
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
    tokio::signal::ctrl_c().await?;
    Ok(())
}
```

### 6.13 Dashboard composition
```rust
ui.row_gap(1, |ui| {
    ui.bordered(Border::Rounded).title("Stats").p(1).w(30).col(|ui| {
        ui.stat("Users", "1,293");
        ui.stat_trend("Revenue", "$48k", Trend::Up);
    });
    ui.bordered(Border::Rounded).title("Traffic").grow(1).p(1).col(|ui| {
        ui.sparkline(&[3.0, 4.0, 2.0, 5.0, 6.0, 4.0], 20);
        ui.chart(|c| { c.line(&[1.0, 4.0, 9.0, 16.0, 25.0]); c.grid(true); }, 40, 10);
    });
});
```

### 6.14 File picker dialog
```rust
let mut picker = FilePickerState::new(".").show_hidden(false).extensions(&["rs", "toml"]);
if ui.file_picker(&mut picker).clicked {
    if let Some(path) = picker.selected() { println!("picked: {:?}", path); }
}
```

### 6.15 Custom Widget
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
let mut rating = Rating { value: 3, max: 5 };
let _ = ui.widget(&mut rating);
```

### 6.16 Responsive layout
```rust
match ui.breakpoint() {
    Breakpoint::Xs | Breakpoint::Sm => {
        ui.col(|ui| { /* stacked */ });
    }
    _ => {
        ui.row(|ui| { /* side by side */ });
    }
}
// Or per-breakpoint style:
ui.container().w(20).md_w(40).lg_w(60).col(|ui| { /* ... */ });
```

### 6.17 Animation
```rust
// At construction:
let mut fade = Tween::new(0.0, 1.0, 30).easing(slt::anim::ease_out_quad);
// First frame:
// fade.reset(ui.tick());
// Every frame:
let alpha = fade.value(ui.tick());
ui.text("Hello").fg(Color::Rgb(255, 255, (alpha * 255.0) as u8));

// Spring with mutable target
let mut s = Spring::new(0.0, 0.2, 0.85);
s.set_target(if hovered { 1.0 } else { 0.0 });
s.tick();
let scale = s.value();

// Stagger across list
let mut st = Stagger::new(0.0, 1.0, 20).delay(3).items(items.len());
for (i, item) in items.iter().enumerate() {
    let a = st.value(ui.tick(), i);
    ui.text(item).fg(Color::Rgb(255, 255, (a * 255.0) as u8));
}
```

### 6.18 Raw draw (direct buffer access)
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
```rust
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
    if ui.key_mod('f', KeyModifiers::CONTROL) {
        // naive "focus filter box" — use your own text_input separately:
    }
    if ui.key('s') { table.toggle_sort(1); }  // toggle sort on Name column
    if ui.key_code(KeyCode::Right) { table.next_page(); }
    if ui.key_code(KeyCode::Left)  { table.prev_page(); }

    let r = ui.table(&mut table);
    if r.changed {
        if let Some(row) = table.selected_row() {
            // row: &[String]
        }
    }
});
```

### 6.19c Tree view from filesystem paths
```rust
let mut tree = DirectoryTreeState::from_paths(&[
    "src/lib.rs", "src/context/runtime.rs", "docs/QUICK_START.md",
]);
let r = ui.directory_tree(&mut tree);
if r.changed {
    let label = tree.selected_label();
}
```

### 6.19d Virtualized list for 100k rows
```rust
let mut list = ListState::new((0..100_000).map(|i| format!("row {i}")).collect::<Vec<_>>());
ui.virtual_list(&mut list, 20, |ui, idx| {
    ui.text(&list.items[idx]).dim();
});
```

### 6.19e Typing with autocomplete
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
```rust
let mut s = StreamingMarkdownState::new();
// In whatever loop feeds new tokens:
s.push("**Answer:** the");
s.push(" quick brown fox");
// Inside the closure, just render what's accumulated:
ui.streaming_markdown(&mut s);
```

### 6.19g Canvas — animated shape
```rust
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
});
```

### 6.20 Keyboard shortcuts with KeyMap + help bar

`KeyMap` is a declarative registry that also drives the help bar. Check the actual key press yourself (`ui.key(...)`, `ui.key_mod(...)`) — `KeyMap` owns labels and descriptions, not input consumption.

```rust
let km = KeyMap::new()
    .bind('q', "Quit")
    .bind_code(KeyCode::Up, "Move up")
    .bind_mod('s', KeyModifiers::CONTROL, "Save")
    .bind_hidden('?', "Toggle help");

slt::run(move |ui| {
    if ui.key('q') { ui.quit(); }
    if ui.key_mod('s', KeyModifiers::CONTROL) { /* save */ }
    ui.help_from_keymap(&km);  // renders the help bar from the map
});
```

`Binding` has fields: `{ key: KeyCode, modifiers: Option<KeyModifiers>, display: String, description: String, visible: bool }`.

---

## 7. Previous-frame rule

`Response.rect` is zero on the very first frame because hit-testing and widget rects are collected **after** the closure runs and fed back to the next frame. Write any measurement or overlap logic guarded by frame count:

```rust
let r = ui.bordered(Border::Single).col(|ui| ui.text("measured panel"));
if ui.tick() > 0 {
    // Now r.rect is the actual laid-out rectangle from the previous frame.
    println!("{}x{}", r.rect.width, r.rect.height);
}
```

Same rule applies to hover/click reads that depend on having been positioned once. `ui.focus_count()` likewise reads the settled count from the previous frame, so on frame 0 it is `0`.

---

## 8. Theming (condensed)

Presets: `Theme::dark()` (default), `light()`, `dracula()`, `catppuccin()`, `nord()`, `solarized_dark()`, `solarized_light()`, `tokyo_night()`, `gruvbox_dark()`, `one_dark()`.

Custom:
```rust
let theme = Theme::builder()
    .primary(INDIGO.c500).secondary(TEAL.c500).accent(PINK.c500)
    .text(SLATE.c50).text_dim(SLATE.c400).border(SLATE.c700)
    .bg(SLATE.c950).success(EMERALD.c500).warning(AMBER.c500).error(RED.c500)
    .surface(SLATE.c800).surface_hover(SLATE.c700).surface_text(SLATE.c300)
    .is_dark(true).build();
slt::run_with(RunConfig::default().theme(theme), |ui| { /* ... */ });
```

Semantic tokens (`ThemeColor`): use `ui.color(ThemeColor::Primary)` or `WidgetColors::new().theme_bg(ThemeColor::Surface)`.

Runtime switch: `ui.set_theme(Theme::nord())`, `ui.set_dark_mode(true)`, `ui.light_dark(light, dark)`.

Const-buildable recipes:
```rust
const CARD: ContainerStyle = ContainerStyle::new()
    .border(Border::Rounded).p(1).theme_bg(ThemeColor::Surface);
ui.container().apply(&CARD).col(|ui| { ui.text("card"); });
```

---

## 8b. Theming deep dive

### Color constructors
```rust
Color::Rgb(255, 127, 80)             // true color
Color::Indexed(240)                  // xterm 256 palette index
Color::Red                           // named ANSI
Color::LightCyan                     // bright variant
Color::Reset                         // terminal default
```

### Tailwind palettes (via `slt::palette::tailwind::*`)

Neutrals: `SLATE`, `GRAY`, `ZINC`, `NEUTRAL`, `STONE`.
Colors: `RED`, `ORANGE`, `AMBER`, `YELLOW`, `LIME`, `GREEN`, `EMERALD`, `TEAL`, `CYAN`, `SKY`, `BLUE`, `INDIGO`, `VIOLET`, `PURPLE`, `FUCHSIA`, `PINK`, `ROSE`.

Shades (const fields on each palette): `.c50`, `.c100`, `.c200`, `.c300`, `.c400`, `.c500`, `.c600`, `.c700`, `.c800`, `.c900`, `.c950`.

```rust
use slt::palette::tailwind::{BLUE, ROSE, SLATE};
let primary = BLUE.c500;
let danger  = ROSE.c600;
let muted   = SLATE.c400;
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

```rust
let c = WidgetColors::new()
    .theme_bg(ThemeColor::Surface)     // takes precedence over bg below
    .bg(Color::Black)
    .theme_accent(ThemeColor::Accent);
ui.button_colored("Save", &c);
ui.table_colored(&mut tbl, &c);
```

### Style chain

```rust
Style::new().fg(Color::Cyan).bg(Color::Reset).bold().italic().dim().underline().reversed().strikethrough()
```

Modifiers can be built as flags and assigned:
```rust
let mut s = Style::new().fg(Color::Cyan);
s.modifiers = Modifiers::BOLD | Modifiers::UNDERLINE;
```
(`Style::new()`, `.fg(...)`, `.bg(...)` are `const`, usable in `const` recipes.)

---

## 9. Testing

```rust
use slt::{TestBackend, EventBuilder, KeyCode};

// Snapshot
let mut tb = TestBackend::new(40, 8);
tb.render(|ui| {
    ui.bordered(Border::Rounded).p(1).col(|ui| {
        ui.text("Hello").bold().fg(Color::Cyan);
    });
});
tb.assert_contains("Hello");
assert!(tb.line(1).contains("Hello"));

// Interaction (multi-frame: session state persists across render calls)
let mut count = 0;
tb.render(|ui| { if ui.button("+1").clicked { count += 1; } });
tb.render_with_events(
    EventBuilder::new().click(2, 0).build(),
    /*focus_index=*/0, /*prev_focus_count=*/1,
    |ui| { if ui.button("+1").clicked { count += 1; } },
);
```

`tb.to_string_trimmed()` returns the whole buffer as a `\n`-joined string for `insta::assert_snapshot!`.

---

## 10. Error modes AI commonly hits

1. **Closure capture lifetime.** `.draw(f)`, `.draw_with(d, f)`, `.draw_interactive(f)` all require `'static` closures because they run after layout. Move owned data in (`draw_with`), or snapshot any borrowed state into an owned `Vec`/`String` first.
2. **Hook ordering.** `use_state` / `use_memo` must be called in the same order every frame. Putting them inside a conditional that flips between frames will panic with a type mismatch message.
3. **Focus on frame 1.** `ui.focus_count()` returns 0 on the first frame; `Response.rect` is zero-sized until the frame after a widget's first appearance. Avoid relying on these for frame-0 decisions.
4. **Borrow across `row()` / `col()`.** The closure passed to `.col(...)` borrows `ui` mutably. You cannot hold a reference to `ui` or to the parent `Response` inside that closure. Capture what you need before or read the `Response` after the call.
5. **`ui.bg(c)` vs container bg.** `ui.text("x").bg(color)` sets the *text* background only. For a container background use `ui.container().bg(color).col(|ui| { ... })`.
6. **`line()` vs `row()`.** Both are horizontal. `row()` returns `Response`, has gap, participates in flex fully. `line()` returns `&mut Self`, zero-gap, intended for inline rich text and no interaction.
7. **`_colored` argument order.** Every `_colored` variant takes `&WidgetColors` *as the last argument* (e.g. `ui.button_colored("Save", &colors)`, `ui.list_colored(&mut state, &colors)`).
8. **`*State` must outlive the closure.** Create `TextInputState`, `ListState`, etc. **outside** `slt::run(...)` (or use hooks). Creating them fresh inside the closure throws away the state every frame.
9. **`TextInputState` vs `TextareaState`.** Single-line vs multi-line. They are not interchangeable.
10. **`consume_key` vs `key`.** `consume_key(c)` marks the event consumed so other widgets ignore it. Use this in custom widgets or global shortcuts that should not bubble.
11. **Animations are *not* on `Context`.** `Tween`, `Spring`, `Keyframes`, `Sequence`, `Stagger` are plain structs. Construct outside the closure, mutate/sample inside using `ui.tick()`.
12. **Button color overrides.** `button_with(label, ButtonVariant::Danger)` picks a theme-aware variant; `button_colored(label, &WidgetColors::new().accent(c))` is a literal override.
13. **Inline mode does not own the screen.** `run_inline(height, f)` reserves `height` rows below the cursor; other output still scrolls. No alt-screen, no F12 overlay in some terminals.
14. **`cargo add superlighttui`** pulls in the `crossterm` default feature. Without it you lose `run()` / `run_inline()` / clipboard; keep `Backend`, `AppState`, `frame()`.
15. **Feature-gated APIs.** `ui.qr_code(...)` needs `qrcode`. `run_async(...)` needs `async`. `code_block_lang(...)` tree-sitter highlighting needs `syntax` or the matching `syntax-*` flag.
16. **`ui.screen` isolates hooks.** `use_state` inside one `screen(...)` does not share state with another. Use plain app state if you need cross-screen values.
17. **Modal traps focus.** Tab and Shift+Tab cycle only within the current modal; outside widgets are not reachable while the modal is open.
18. **Breakpoint method naming.** `xs_w`, `sm_w`, `md_w`, `lg_w`, `xl_w`, `w_at(bp, v)` — the breakpoint is a **prefix** on the base name (not `w_xs`).

---

## 11. Full public re-export list (from `src/lib.rs`)

These are everything usable as `slt::X`.

```text
// From test_utils
EventBuilder, TestBackend

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
Bar, BarChartConfig, BarDirection, BarGroup, CanvasContext, ContainerBuilder,
Context, Response, State, TreemapItem, Widget

// From event
Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseKind

// From halfblock
HalfBlockImage

// From keymap
Binding, KeyMap

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
Align, Border, BorderSides, Breakpoint, Color, ColorDepth, Constraints,
ContainerStyle, Justify, Margin, Modifiers, Padding, Spacing, Style,
Theme, ThemeBuilder, ThemeColor, WidgetColors, WidgetTheme

// From widgets
AlertLevel, ApprovalAction, ButtonVariant, CalendarState, CommandPaletteState,
ContextItem, DirectoryTreeState, FileEntry, FilePickerState, FormField, FormState,
GridColumn, ListState, ModeState, MultiSelectState, PaletteCommand, RadioState,
RichLogEntry, RichLogState, ScreenState, ScrollState, SelectState, SpinnerState,
StaticOutput, StreamingMarkdownState, StreamingTextState, TableState, TabsState,
TextInputState, TextareaState, ToastLevel, ToastMessage, ToastState,
ToolApprovalState, TreeNode, TreeState, Trend

// Runtime entry points (most need `crossterm` feature)
Backend, AppState, RunConfig,
run, run_with, run_inline, run_inline_with,
run_static, run_static_with,
run_async, run_async_with,
frame,
detect_color_scheme, read_clipboard, ColorScheme
```

---

## 12. Feature flags cheat sheet

| Flag | Unlocks | Default? |
|---|---|---|
| `crossterm` | `run()`, `run_inline()`, `run_static()`, clipboard, sixel_image, color-scheme detection | Yes |
| `async` | `run_async()`, `run_async_with()` — needs tokio | No |
| `serde` | `Serialize` / `Deserialize` for style, theme, layout types | No |
| `image` | Image-loading helpers | No |
| `qrcode` | `ui.qr_code(...)` | No |
| `syntax` | All tree-sitter `syntax-*` bundles | No |
| `syntax-rust`, `syntax-python`, `syntax-typescript`, ... | Per-language tree-sitter grammar | No |
| `kitty-compress` | zlib-compressed Kitty image uploads | No |
| `full` | `crossterm` + `async` + `serde` + `image` + `qrcode` + `kitty-compress` (not `syntax`) | No |

Without `crossterm`, you keep `Backend`, `AppState`, `frame()`, `Context`, all widgets, events, styles, layout, charts — enough for a custom backend (WASM canvas, GUI embed, test harness).

---

## 13. Minimal skeletons to copy

### Full-screen app (most common)
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
```rust
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut log: Vec<String> = Vec::new();
    let tx = slt::run_async::<String>(move |ui, msgs| {
        log.extend(msgs.drain(..));
        ui.col(|ui| { for m in &log { ui.text(m); } });
    })?;
    let _ = tx.send("started".into()).await;
    tokio::signal::ctrl_c().await?;
    Ok(())
}
```

---

## 14. Quick index — "how do I ..."

- Quit: `ui.quit()` (Ctrl+C also quits).
- Read a key: `ui.key('q')`, `ui.key_code(KeyCode::Enter)`, `ui.key_mod('s', KeyModifiers::CONTROL)`.
- Read a cross-frame multi-key chord (vi `gg`, leader keys like `<space>ff`): `ui.key_chord("gg")`. Buffers partial input across frames; resets on a mismatching key or after a timeout. (`key_seq` is a deprecated alias.)
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
- Enable mouse: `RunConfig::default().mouse(true)`.
- Ask the terminal for a color scheme: `slt::detect_color_scheme()` (returns `ColorScheme`).
- Query clipboard content: `slt::read_clipboard()` (requires `crossterm`).
- Render to a test buffer: `TestBackend::new(w,h).render(|ui| {...}); tb.assert_contains("x")`.

---

## 15. Widget state — detailed field reference

Use this when you need to read back what a widget currently holds.

### TextInputState (single-line)
```text
value: String                     // current text (pub)
cursor: usize                     // character index (pub)
placeholder: String               // shown when value is empty (pub)
max_length: Option<usize>         // (pub)
validation_error: Option<String>  // latest error (pub)
masked: bool                      // password-style • chars (pub)
suggestions: Vec<String>          // autocomplete candidates (pub)
suggestion_index: usize            // (pub)
show_suggestions: bool            // (pub)
// (private) validators + validation_errors read via errors()
```
Constructors: `TextInputState::new()`, `TextInputState::with_placeholder("...")`.
Builders: `.max_length(n)`.
Methods: `.validate(|v| -> Result<(),String>)`, `.add_validator(f)`, `.run_validators()`, `.errors() -> &[String]`, `.set_suggestions(Vec<String>)`, `.matched_suggestions() -> Vec<&str>`.

### TextareaState (multi-line)
```text
lines: Vec<String>
cursor_row: usize
cursor_col: usize
max_length: Option<usize>
wrap_width: Option<u32>
scroll_offset: usize
```
Constructors: `TextareaState::new()`.
Builders: `.max_length(n)`, `.word_wrap(width)`.
Methods: `.value() -> String`, `.set_value(s)`, `.lines() -> &[String]`, `.clear()`.

### ListState
```text
items: Vec<String>
selected: usize
filter: String
```
Constructors: `ListState::new(vec![...])`.
Methods: `.set_items(vec![...])`, `.set_filter("foo bar")` (space-separated AND tokens), `.visible_indices() -> &[usize]`, `.selected_item() -> Option<&str>`.

### TableState
```text
headers: Vec<String>
rows: Vec<Vec<String>>
selected: usize
sort_column: Option<usize>
sort_ascending: bool
filter: String
page: usize
page_size: usize        // 0 disables pagination
zebra: bool
```
Constructors: `TableState::new(headers, rows)`.
Methods: `.set_rows(rows)`, `.toggle_sort(col)`, `.sort_by(col)`, `.clear_sort()`, `.set_filter(s)`, `.next_page()`, `.prev_page()`, `.total_pages() -> usize`, `.selected_row() -> Option<&[String]>`.

### TabsState
```text
labels: Vec<String>
selected: usize
```
Constructors: `TabsState::new(vec!["A", "B"])`. `.selected_label() -> Option<&str>`.

### SelectState / RadioState / MultiSelectState
```text
// SelectState
items: Vec<String>
selected: usize
open: bool
placeholder: String

// RadioState
items: Vec<String>
selected: usize

// MultiSelectState
items: Vec<String>
cursor: usize               // keyboard focus index
selected: HashSet<usize>    // checked items
```
Methods: `.selected_item() -> Option<&str>` (Select, Radio), `.selected_items() -> Vec<&str>` (Multi), `.toggle(i)` (Multi).

### TreeState / TreeNode / DirectoryTreeState
```text
// TreeNode
label: String
children: Vec<TreeNode>
expanded: bool

// TreeState
nodes: Vec<TreeNode>
selected: usize             // index into the flattened visible tree

// DirectoryTreeState
tree: TreeState
show_icons: bool
```
`TreeNode::new(label).expanded().children(vec![TreeNode::new("child")])`.
`DirectoryTreeState::from_paths(&["a/b.txt", "a/c.txt"])` builds a tree from slash-delimited paths. `.selected_label() -> Option<&str>`.

### CalendarState
```text
year: i32
month: u32
selected_day: Option<u32>
// cursor_day is private
```
Constructors: `CalendarState::new()` (current month), `CalendarState::from_ym(year, month)`.
Methods: `.selected_date() -> Option<(i32, u32, u32)>` (year, month, day), `.prev_month()`, `.next_month()`.

### FilePickerState / FileEntry
```text
current_dir: PathBuf
entries: Vec<FileEntry>
selected: usize
selected_file: Option<PathBuf>
show_hidden: bool
extensions: Vec<String>     // lowercase, no leading dot
dirty: bool                 // set after config changes

// FileEntry
name: String
path: PathBuf
is_dir: bool
size: u64
```
Constructor: `FilePickerState::new(".")`.
Builders: `.show_hidden(bool)`, `.extensions(&["rs", "toml"])`.
Methods: `.selected() -> Option<&PathBuf>`, `.refresh()`.

### CommandPaletteState / PaletteCommand
```text
commands: Vec<PaletteCommand>
input: String
cursor: usize
open: bool
last_selected: Option<usize>

// PaletteCommand
label: String
description: String
```
`.toggle()` opens/closes, clearing the input on open. `PaletteCommand::new(label, desc)`. Fuzzy-matched against `input`.

### ScrollState
```text
offset: usize              // current vertical offset
// content_height and viewport_height are private — read via getters
```
`.can_scroll_up()`, `.can_scroll_down()`, `.content_height()`, `.viewport_height()`.

### SpinnerState
`SpinnerState::dots()` → braille dots. `SpinnerState::line()` → `| / - \`. `.frame(tick) -> char`.

### ToastState / ToastMessage
```text
// ToastState
messages: Vec<ToastMessage>

// ToastMessage
text: String
level: ToastLevel
created_tick: u64
duration_ticks: u64
```
Methods: `.info(msg, ui.tick())`, `.success(...)`, `.warning(...)`, `.error(...)` — different default durations (30/30/50/80 ticks). Generic: `.push(msg, level, tick, duration_ticks)`. `.cleanup(current_tick)` is auto-called by `ui.toast(...)`.

### FormState / FormField
```text
// FormField
label: String
input: TextInputState
error: Option<String>

// FormState
fields: Vec<FormField>
submitted: bool
```
Builders: `FormState::new().field(FormField::new("Email").placeholder("..."))`. `.validate(&[FormValidator]) -> bool` returns whether every field validated. `.value(i) -> &str`.

### ScreenState / ModeState
```text
// ScreenState: a single stack
// Methods: .current() -> &str, .push(name), .pop(), .depth(), .can_pop(), .reset()

// ModeState: map<mode_name, ScreenState>
// Methods: .add_mode(mode, screen), .switch_mode(mode) (panics on miss),
//          .try_switch_mode(mode) -> bool, .active_mode(), .screens(), .screens_mut()
```

### StreamingTextState / StreamingMarkdownState
```text
content: String             // accumulated text
streaming: bool             // true while streaming
```
`.push(chunk)` appends and marks streaming. `.clear()` resets.

### ToolApprovalState
```text
tool_name: String
description: String
action: ApprovalAction      // Pending | Approved | Rejected
// and internal UI state
```
Constructor: `ToolApprovalState::new(name, desc)`.

### ContextItem
```text
label: String
tokens: usize
```
Constructor: `ContextItem::new("agent.rs", 1200)`.

### RichLogState / RichLogEntry
```text
// RichLogEntry: single log row with per-segment styles
// RichLogState
entries: Vec<RichLogEntry>
auto_scroll: bool
```
Methods: `.push(text, style)`, `.push_plain(text)`, `.push_segments(Vec<(String, Style)>)`, `.clear()`.

### GridColumn
Per-column spec for `ui.grid_with(&[GridColumn], |ui| { ... })` — fixed or fractional widths.

### StaticOutput
```text
// For run_static:
// .println(line) queues a line to write above the inline UI.
```

### Chart types
```text
// Candle
open: f64, high: f64, low: f64, close: f64

// TreemapItem
TreemapItem::new(label: impl Into<String>, value: f64, color: Color)

// Bar
Bar::new(label, value).color(c).text_value(s)

// BarGroup
BarGroup::new(label, Vec<Bar>).group_gap(n)
```

---

## 16. ChartBuilder — the full multi-series flow

```rust
ui.chart(|c| {
    c.line(&prices);                          // series 1
    c.area(&prices).fg(Color::Cyan);          // filled area
    c.scatter(&points);                       // (f64, f64) pairs
    c.bar(&[3.0, 4.0, 2.0]);
    c.grid(true);
    c.x_range(0.0, 100.0);
    c.y_range(0.0, 50.0);
    c.legend(LegendPosition::Top);
    c.marker(Marker::Dot);
}, 60, 20);
```

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

Diagnostics: F12 toggles an overlay showing container bounds and FPS. `ui.debug_enabled()` is `true` while it is on. `RunConfig::max_fps(Some(u32))` caps rendering. Synchronized output (DECSET 2026) is emitted automatically on supported terminals to avoid tearing.

---

## 18. Glossary

- **Immediate mode.** No retained tree. Every frame re-describes the UI from scratch. Allocations per frame are dominated by the Command buffer (one `Vec<Command>` reused).
- **Closure.** The function you pass to `slt::run(...)`. Runs once per frame on the main thread.
- **Response.** Interaction record of a widget from hit-testing; see §3.
- **State (hook).** `ui.use_state(|| init)` — per-instance persistent slot in the frame state. Stable across frames if called in the same order.
- **State (widget).** Owned struct like `TextInputState`, `ListState`, etc. that persists across frames because you hold it.
- **Prev-frame feedback.** Layout data collected in frame N is used for hit testing in frame N+1.
- **Container.** Any box that holds children — `row`, `col`, `modal`, `overlay`, `scrollable`, `bordered`, `container()`.
- **Raw draw.** Direct buffer access via `.draw(|buf, rect| {...})`. Bypasses widget abstractions.
- **Interaction slot.** Logical index the runtime assigns to each interactive widget so `prev_hit_map` can hit-test it. Call `register_focusable()` / `interaction()` to reserve one.

---

## 19. Compatibility

- MSRV is tracked in `Cargo.toml` under `rust-version`; MSRV bumps only on minor releases.
- `0.x.y` patches are backward compatible. `0.x → 0.y` minor versions may include breaking changes until 1.0.
- The public API surface is strictly everything re-exported from `src/lib.rs`; deep imports are not semver stable.
- Deprecation: a minimum of one minor release with `#[deprecated]` before removal.

---

## 20. See also

- Runnable examples in `examples/`:
  - `hello`, `counter`, `demo`, `demo_dashboard`, `demo_cli`, `demo_infoviz`, `demo_table`, `demo_design_system`, `demo_spreadsheet`, `demo_game`, `demo_trading`, `demo_website`, `demo_wiki`, `demo_raw_draw`, `demo_pretext`, `demo_fire`, `demo_kitty_image`, `demo_ime`, `inline`, `async_demo`, `anim`, `error_boundary_demo`, `perf_interactive`, `perf_regression`, `test_mouse`, `demo_key_test`, `debug_selection`.
- Per-topic guides: `QUICK_START.md`, `WIDGETS.md`, `PATTERNS.md`, `AI_GUIDE.md`, `DESIGN_PRINCIPLES.md`, `ARCHITECTURE.md`, `THEMING.md`, `ANIMATION.md`, `BACKENDS.md`, `TESTING.md`, `DEBUGGING.md`, `FEATURES.md`, `EXAMPLES.md`.
