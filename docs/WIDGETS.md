# Widget API Catalog

This page is the API map, not the onboarding guide.
Use it when you already know the SLT mental model and want to find the right method or state type quickly.

If you are new, read in this order first:

1. `README.md`
2. `docs/QUICK_START.md`
3. this file

For composition advice, see [PATTERNS.md](PATTERNS.md).
For animation, see the `anim` module.

## If you only learn a small core first

Most apps can go a long way with this subset:

- `text()`
- `row()` / `col()`
- `container()` / `bordered()`
- `button()`, `text_input()`, `checkbox()`
- `list()`, `table()`, `tabs()`
- `modal()`, `overlay()`, `tooltip()`
- `use_state()` / `use_memo()`

---

## Return type conventions

Every widget method on `Context` follows one of these return patterns:

| Return type | Meaning | When to use |
|---|---|---|
| `&mut Self` | Style-chainable display element | `text`, `separator`, `progress`, `spinner`, `toast`, `spacer` |
| `Response` | Interactive widget with click/hover/changed/focused/rect | `button`, `list`, `table`, `tabs`, `col`, `row`, `alert`, charts |
| `ContainerBuilder<'a>` | Fluent builder — finalize with `.col()`, `.row()`, or `.draw()` | `container`, `scrollable`, `bordered`, `group` |
| `Option<usize>` | Index of selected segment, or `None` | `breadcrumb` |
| `(Response, Option<usize>)` | Row `Response` plus clicked segment index | `breadcrumb_response`, `breadcrumb_response_with` |
| `()` | Fire-and-forget side-effect | `tooltip`, `scrollbar`, `notify`, `screen` |

**`Response` fields:**

```rust
pub struct Response {
    pub clicked: bool,   // clicked this frame
    pub hovered: bool,   // mouse hovering
    pub changed: bool,   // value changed this frame
    pub focused: bool,   // has keyboard focus
    pub rect: Rect,      // layout rectangle (valid after layout pass)
}
```

---

## Text & Display

All return `&mut Self` for style chaining unless noted.

| Method | Description |
|---|---|
| `text(s)` | Render text. Chain: `.bold()`, `.italic()`, `.dim()`, `.underline()`, `.reversed()`, `.strikethrough()`, `.fg(color)`, `.bg(color)`, `.gradient(a, b)`, `.wrap()`, `.truncate()`, `.grow(n)`, `.align(a)`, `.text_center()`, `.text_right()`, `.w()`, `.h()`, `.min_w()`, `.max_w()`, `.min_h()`, `.max_h()`, `.m()`, `.mx()`, `.my()`, `.mt()`, `.mr()`, `.mb()`, `.ml()`, `.group_hover_fg()`, `.group_hover_bg()` |
| `styled(s, style)` | Text with an explicit `Style` struct |
| `link(text, url)` | Clickable hyperlink (OSC 8). Opens URL on Enter/Space/click |
| `spacer()` | Invisible flex element — pushes siblings apart |
| `separator()` | Horizontal divider line. Variant: `separator_colored(color)` |
| `timer_display(elapsed)` | Format `Duration` as HH:MM:SS.CC |

### Rich text (return `Response`)

| Method | Description |
|---|---|
| `markdown(text)` | Render Markdown with headings, bold, italic, links, lists, code |
| `code_block(code)` | Fenced code block. Variants: `code_block_lang(code, lang)`, `code_block_numbered(code)`, `code_block_numbered_lang(code, lang)` |
| `big_text(s)` | 8x8 bitmap text rendered as half-block pixels (4 rows tall) |

```rust
// Typical text usage
ui.text("Hello").bold().fg(Color::Cyan);
ui.styled("inline", Style::new().underline());
ui.link("Docs", "https://docs.rs/superlighttui");
```

---

## Status & Info

All return `Response` unless noted.

| Method | Description |
|---|---|
| `alert(message, level)` | Banner with icon. `AlertLevel`: `Info`, `Success`, `Warning`, `Error` |
| `badge(label)` | Themed badge pill. Variant: `badge_colored(label, color)` |
| `key_hint(key)` | Keyboard shortcut hint badge |
| `stat(label, value)` | Label-value stat display. Variants: `stat_colored(label, value, color)`, `stat_trend(label, value, trend)` where `Trend`: `Up`, `Down` |
| `empty_state(title, desc)` | Centered empty-state message. Variant: `empty_state_action(title, desc, action)` |
| `divider_text(label)` | Horizontal divider with centered label |
| `definition_list(items)` | Key-value list from `&[(&str, &str)]` |
| `accordion(title, open, f)` | Collapsible section. `open: &mut bool` |
| `confirm(question, result)` | Yes/No dialog. `result: &mut bool`, `Response.clicked` on answer |
| `breadcrumb(segments)` | Clickable breadcrumb trail. Returns `Option<usize>`. Variants: `breadcrumb_with(segments, sep)`; `breadcrumb_response(segments)` and `breadcrumb_response_with(segments, sep)` return `(Response, Option<usize>)` so you can read hover/focus/rect alongside the clicked index. |
| `help(bindings)` | Keybinding help bar from `&[(&str, &str)]`. Variant: `help_colored(bindings, key_color, text_color)` |
| `help_from_keymap(keymap)` | Help bar from a `KeyMap` struct |

Note: `separator()` and `separator_colored()` return `&mut Self` (listed under Text & Display).

---

## Layout & Containers

| Method | Returns | Description |
|---|---|---|
| `col(f)` | `Response` | Vertical container. Variant: `col_gap(gap, f)` |
| `row(f)` | `Response` | Horizontal container. Variant: `row_gap(gap, f)` |
| `line(f)` | `&mut Self` | Inline rich text (zero-gap row, no interaction) |
| `line_wrap(f)` | `&mut Self` | Wrapping inline text |
| `grid(cols, f)` | `Response` | Fixed-column grid layout |
| `container()` | `ContainerBuilder` | Fluent builder for borders, padding, grow, constraints, title |
| `scrollable(state)` | `ContainerBuilder` | Scrollable container. `ScrollState` |
| `scrollbar(state)` | `()` | Render scrollbar track for a `ScrollState` |
| `bordered(border)` | `ContainerBuilder` | Shorthand for `container().border(border)` |
| `modal(f)` | `Response` | Modal overlay with dimmed background |
| `overlay(f)` | `Response` | Floating overlay (no dimming) |
| `tooltip(text)` | `()` | Hover tooltip near cursor |
| `group(name)` | `ContainerBuilder` | Named group for shared hover/focus styling |
| `screen(name, &mut screens, f)` | `()` | Conditional rendering when named screen is active. Hook-isolated. `ScreenState` |
| `form(state, f)` | `&mut Self` | Form container. `FormState` |
| `form_field(field)` | `&mut Self` | Single form field. `FormField` |
| `form_submit(label)` | `Response` | Form submit button |

```rust
// Layout composition
ui.row(|ui| {
    ui.container()
        .border(Border::Rounded)
        .pad(1)
        .grow(1)
        .col(|ui| {
            ui.text("Panel content");
        });
    ui.spacer();
    ui.col(|ui| { ui.text("Right side"); });
});
```

---

## Interactive Widgets

All return `Response`. Most have a `_colored(&WidgetColors)` variant.

| Method | State type | Description |
|---|---|---|
| `button(label)` | — | Click button. Variants: `button_colored(label, colors)`, `button_with(label, variant)` where `ButtonVariant`: `Default`, `Primary`, `Danger`, `Outline` |
| `checkbox(label, checked)` | `&mut bool` | Checkbox toggle. Variant: `checkbox_colored(...)` |
| `toggle(label, on)` | `&mut bool` | Toggle switch. Variant: `toggle_colored(...)` |
| `slider(label, value, range)` | `&mut f64`, `RangeInclusive<f64>` | Horizontal slider (default step = `span / 20`) |
| `slider_with_step(label, value, range, step)` | `&mut f64`, `RangeInclusive<f64>`, `f64` | Slider with explicit step size — use for integer counters (`1.0`) or fine controls (`0.1`) |
| `text_input(state)` | `TextInputState` | Single-line text input. Variant: `text_input_colored(...)` |
| `textarea(state, visible_rows)` | `TextareaState` | Multi-line text editor |
| `select(state)` | `SelectState` | Dropdown select. Variant: `select_colored(...)` |
| `radio(state)` | `RadioState` | Radio button group. Variant: `radio_colored(...)` |
| `multi_select(state)` | `MultiSelectState` | Multi-select checkbox list |
| `tabs(state)` | `TabsState` | Tab bar. Variant: `tabs_colored(...)` |
| `list(state)` | `ListState` | Scrollable list with selection. Variant: `list_colored(...)` |
| `table(state)` | `TableState` | Data table with sorting, pagination, filtering. Variant: `table_colored(...)` |
| `tree(state)` | `TreeState` | Expandable tree view |
| `directory_tree(state)` | `DirectoryTreeState` | Filesystem tree |
| `calendar(state)` | `CalendarState` | Month calendar with date selection |
| `file_picker(state)` | `FilePickerState` | File browser dialog |
| `command_palette(state)` | `CommandPaletteState` | Modal fuzzy-search command palette |
| `virtual_list(state, visible_height, f)` | `ListState` | Virtualized list — only renders visible rows |

```rust
// Interactive widget with Response
let mut tabs = TabsState::new(vec!["Overview", "Details", "Settings"]);
if ui.tabs(&mut tabs).changed {
    // tab switched
}
```

---

## Feedback & Progress

| Method | Returns | State type | Description |
|---|---|---|---|
| `progress(ratio)` | `&mut Self` | — | Progress bar, `f64` 0.0..1.0 |
| `progress_bar(ratio, width)` | `&mut Self` | — | Fixed-width progress bar. Variant: `progress_bar_colored(ratio, width, color)` |
| `spinner(state)` | `&mut Self` | `SpinnerState` | Animated spinner (dots, line, etc.) |
| `toast(state)` | `&mut Self` | `ToastState` | Render active toast notifications |
| `notify(message, level)` | `()` | — | Fire-and-forget toast. `ToastLevel`: `Info`, `Success`, `Warning`, `Error` |

---

## Visualization

All return `Response`.

| Method | Description |
|---|---|
| `bar_chart(data, max_width)` | Horizontal bar chart from `&[(&str, f64)]`. Variant: `bar_chart_with(bars, configure, max_size)` for per-bar colors and layout options |
| `bar_chart_grouped(groups, max_width)` | Grouped bar chart from `&[BarGroup]`. Variant: `bar_chart_grouped_with(groups, configure, max_size)` |
| `sparkline(data, width)` | Inline sparkline from `&[f64]`. Variant: `sparkline_styled(data, width)` with per-point `Option<Color>` |
| `line_chart(data, width, height)` | Line chart. Variant: `line_chart_colored(...)` |
| `area_chart(data, width, height)` | Filled area chart. Variant: `area_chart_colored(...)` |
| `scatter(data, width, height)` | Braille scatter plot from `&[(f64, f64)]` |
| `histogram(data, width, height)` | Histogram from `&[f64]`. Variant: `histogram_with(data, configure, w, h)` |
| `bar_chart_stacked(groups, max_height)` | Stacked bar chart from `&[BarGroup]`. Variant: `bar_chart_stacked_with(groups, configure, max_height)` |
| `candlestick(candles, up_color, down_color)` | OHLC candlestick chart |
| `candlestick_hd(candles, up_color, down_color)` | HD candlestick with heavy `┃` wicks and center-aligned bodies |
| `heatmap(data, w, h, low_color, high_color)` | 2D color gradient heatmap |
| `heatmap_halfblock(data, w, h, low_color, high_color)` | Half-block heatmap with 2× vertical resolution using `▀` fg/bg |
| `treemap(items)` | Squarified treemap from `&[TreemapItem]`. Auto-sizes via `grow(1)` |
| `canvas(width, height, draw)` | Braille drawing canvas. Closure receives `&mut CanvasContext` |
| `chart(configure, width, height)` | Multi-series chart via `ChartBuilder`. Closure receives `&mut ChartBuilder` |
| `qr_code(data)` | QR code (requires `qrcode` feature) |

```rust
// Multi-series chart with builder
ui.chart(|c| {
    c.line(&data1);
    c.area(&data2);
    c.grid(true);
}, 60, 20);

// Braille canvas
ui.canvas(40, 12, |cv| {
    cv.set_color(Color::Cyan);
    cv.circle(40, 24, 20);
    cv.line(0, 0, 79, 47);
});
```

---

## AI-Native & Rich Terminal

All return `Response`.

| Method | State type | Description |
|---|---|---|
| `streaming_text(state)` | `StreamingTextState` | Incrementally rendered text stream |
| `streaming_markdown(state)` | `StreamingMarkdownState` | Incrementally rendered Markdown stream |
| `tool_approval(state)` | `ToolApprovalState` | Tool call approval dialog. `ApprovalAction`: `Approve`, `Deny`, `Edit` |
| `context_bar(items)` | `&[ContextItem]` | Context display with token counts |
| `image(img)` | `HalfBlockImage` | Half-block image rendering (2 pixels per cell) |
| `kitty_image(rgba, pw, ph, cols, rows)` | — | Kitty graphics protocol image |
| `kitty_image_fit(rgba, sw, sh, cols)` | — | Auto-fit Kitty image to column width |
| `sixel_image(rgba, pw, ph, cols, rows)` | — | Sixel protocol image (requires `crossterm`) |
| `rich_log(state)` | `RichLogState` | Scrollable styled log viewer. `RichLogState::new()` is bounded at `RichLogState::DEFAULT_MAX_ENTRIES` (10,000 entries) — older entries drop from the front. Use `RichLogState::new_unbounded()` only when growth is bounded elsewhere. |

---

## Runtime & Hooks

These are not widgets but essential `Context` methods for state, input, and environment.

### State management

| Method | Returns | Description |
|---|---|---|
| `use_state(init)` | `State<T>` | Persistent state across frames. Read with `.get(ui)`, mutate with `.get_mut(ui)` |
| `use_memo(deps, compute)` | `&T` | Memoized computation, recomputes when `deps` changes |

### Focus & interaction

| Method | Returns | Description |
|---|---|---|
| `register_focusable()` | `bool` | Register current widget as focusable, returns whether it has focus |
| `interaction()` | `Response` | Get click/hover Response for current interaction slot |
| `widget(w)` | `W::Response` | Render a custom `Widget` trait implementor |
| `error_boundary(f)` | — | Catch panics in closure, render error message. Variant: `error_boundary_with(f, fallback)` |
| `focus_index()` | `usize` | Current focus index |
| `set_focus_index(index)` | — | Set focus index programmatically |
| `focus_count()` | `usize` | Total focusable widget count |

### Keyboard input

| Method | Returns | Description |
|---|---|---|
| `key(c)` | `bool` | Char key pressed (non-consuming) |
| `key_code(code)` | `bool` | KeyCode pressed (non-consuming) |
| `raw_key_code(code)` | `bool` | KeyCode pressed, ignoring consumed state |
| `consume_key(c)` | `bool` | Char key pressed (marks as consumed) |
| `consume_key_code(code)` | `bool` | KeyCode pressed (marks as consumed) |
| `key_mod(c, mods)` | `bool` | Char + modifier (Ctrl, Alt, Shift) |
| `raw_key_mod(c, mods)` | `bool` | Char + modifier, ignoring consumed state |
| `key_chord(seq)` | `bool` | Cross-frame multi-key sequence (vi `gg`, leader keys); buffers partial input and times out on inactivity |
| `key_chord_timeout(seq, ticks)` | `bool` | `key_chord` with an explicit per-call timeout in ticks |
| `key_seq(seq)` | `bool` | **Deprecated** — alias for `key_chord` |
| `key_release(c)` | `bool` | Char key released |
| `key_code_release(code)` | `bool` | KeyCode released |

### Mouse input

| Method | Returns | Description |
|---|---|---|
| `mouse_down()` | `Option<(u32, u32)>` | Mouse button down position |
| `mouse_pos()` | `Option<(u32, u32)>` | Current mouse position |
| `scroll_up()` / `scroll_down()` | `bool` | Vertical scroll events |
| `scroll_left()` / `scroll_right()` | `bool` | Horizontal scroll events |

### Clipboard & system

| Method | Returns | Description |
|---|---|---|
| `paste()` | `Option<&str>` | Bracketed paste content |
| `copy_to_clipboard(text)` | — | Copy text via OSC 52 |
| `quit()` | — | Exit the application |

### Theme & environment

| Method | Returns | Description |
|---|---|---|
| `theme()` | `&Theme` | Current theme |
| `color(ThemeColor)` | `Color` | Resolve a semantic color token |
| `spacing()` | `Spacing` | Current spacing scale |
| `set_theme(theme)` | — | Change theme |
| `is_dark_mode()` | `bool` | Terminal dark mode state |
| `set_dark_mode(dark)` | — | Override dark mode |
| `light_dark(light, dark)` | `Color` | Pick color based on dark mode |
| `width()` | `u32` | Terminal width in columns |
| `height()` | `u32` | Terminal height in rows |
| `breakpoint()` | `Breakpoint` | Responsive breakpoint (`Xs`, `Sm`, `Md`, `Lg`, `Xl`) |
| `tick()` | `u64` | Frame counter |
| `debug_enabled()` | `bool` | Whether debug mode is active |
| `set_scroll_speed(lines)` | — | Set scroll speed |
| `scroll_speed()` | `u32` | Current scroll speed |

---

## State Type Quick Reference

| Widget | State type | Key fields / methods |
|---|---|---|
| `text_input` | `TextInputState` | `.value: String`, `.cursor: usize`, `.placeholder`, `::with_placeholder(s)`, `.suggestions: Vec<String>`, `.matched_suggestions()` |
| `textarea` | `TextareaState` | `.lines: Vec<String>`, `.cursor_row`, `.cursor_col`, `.wrap_width: Option<u32>`, `.max_length` |
| `select` | `SelectState` | `.items: Vec<String>`, `.selected: usize`, `.open: bool` |
| `radio` | `RadioState` | `.items: Vec<String>`, `.selected: usize` |
| `multi_select` | `MultiSelectState` | `.items: Vec<String>`, `.selected: HashSet<usize>` |
| `tabs` | `TabsState` | `.labels: Vec<String>`, `.selected: usize`, `::new(labels)` |
| `list` | `ListState` | `.items: Vec<String>`, `.selected: usize`, `.filter` |
| `table` | `TableState` | `.headers`, `.rows`, `.selected`, `.sort_column`, `.sort_ascending`, `.page`, `.page_size`, `.filter`, `::new(headers, rows)` |
| `tree` | `TreeState` | `.nodes`, `.selected`, `.expanded: HashSet` |
| `directory_tree` | `DirectoryTreeState` | `.root: PathBuf`, `.selected_path()` |
| `calendar` | `CalendarState` | `.year`, `.month`, `.cursor_day`, `.selected_date()` |
| `file_picker` | `FilePickerState` | `.current_dir`, `.selected_path()` |
| `command_palette` | `CommandPaletteState` | `.commands: Vec<PaletteCommand>`, `.open`, `.input`, `.last_selected` |
| `scroll` | `ScrollState` | `.offset`, `.content_height`, `.viewport_height` |
| `spinner` | `SpinnerState` | `::dots()`, `::line()`, `.frame(tick)` |
| `toast` | `ToastState` | `.push(msg, level)`, `.messages`, `.cleanup(tick)` |
| `form` | `FormState` | `.fields: Vec<FormField>`, `.submitted: bool` |
| `form_field` | `FormField` | `.label`, `.input: TextInputState`, `.error` |
| `screen` | `ScreenState` | `.current()`, `.push(name)`, `.pop()`, `::new(initial)` |
| (modes) | `ModeState` | `.active_mode()`, `.switch_mode(name)`, `.screens_mut()`, `::new(mode, screen)` |
| `streaming_text` | `StreamingTextState` | `.push(text)`, `.content`, `.streaming` |
| `streaming_markdown` | `StreamingMarkdownState` | `.push(text)`, `.streaming` |
| `tool_approval` | `ToolApprovalState` | `.tool_name`, `.description`, `.action: ApprovalAction` (`Pending`, `Approved`, `Rejected`) |
| `rich_log` | `RichLogState` | `.push(text, style)`, `.entries`, `.auto_scroll`, `.max_entries: Option<usize>`. `::new()` caps at 10,000; `::new_unbounded()` opts out (use only when growth is bounded elsewhere). |
| `treemap` | `TreemapItem` | `.label: String`, `.value: f64`, `.color: Color`, `::new(label, value, color)` |

---

## ContainerBuilder Quick Reference

Obtained via `ui.container()`, `ui.bordered(border)`, `ui.scrollable(state)`, or `ui.group(name)`.
Finalize with `.col(f)`, `.row(f)`, or `.draw(f)`.

### Border & style

| Method | Description |
|---|---|
| `.border(border)` | Set border type (`Border::Single`, `Rounded`, `Double`, `Heavy`, etc.) |
| `.border_sides(sides)` | Which sides to draw. Shortcuts: `.border_x()`, `.border_y()`, `.border_top(bool)`, `.border_right(bool)`, `.border_bottom(bool)`, `.border_left(bool)` |
| `.rounded()` | Shorthand for `.border(Border::Rounded)` |
| `.border_style(style)` | Style for border lines |
| `.border_fg(color)` | Border foreground color |
| `.dark_border_style(style)` | Border style override in dark mode |
| `.bg(color)` / `.dark_bg(color)` | Background color (with dark mode variant) |
| `.text_color(color)` | Default text color for children |
| `.group_hover_bg(color)` | Background on group hover |
| `.group_hover_border_style(style)` | Border style on group hover |
| `.title(text)` / `.title_styled(text, style)` | Title displayed in border |

### Spacing

| Method | Description |
|---|---|
| `.p(n)` / `.pad(n)` | Uniform padding |
| `.px(n)` / `.py(n)` | Horizontal / vertical padding |
| `.pt(n)` / `.pr(n)` / `.pb(n)` / `.pl(n)` | Individual side padding |
| `.padding(Padding)` | Full `Padding` struct |
| `.m(n)` | Uniform margin |
| `.mx(n)` / `.my(n)` | Horizontal / vertical margin |
| `.mt(n)` / `.mr(n)` / `.mb(n)` / `.ml(n)` | Individual side margin |
| `.margin(Margin)` | Full `Margin` struct |
| `.gap(n)` | Gap between children |
| `.row_gap(n)` / `.col_gap(n)` | Direction-specific gap |

### Size constraints

| Method | Description |
|---|---|
| `.w(n)` / `.h(n)` | Fixed width / height |
| `.min_w(n)` / `.max_w(n)` | Min / max width |
| `.min_h(n)` / `.max_h(n)` | Min / max height |
| `.w_pct(p)` / `.h_pct(p)` | Percentage of parent (`u8`, 0-100) |
| `.constraints(Constraints)` | Full `Constraints` struct |

### Layout

| Method | Description |
|---|---|
| `.grow(n)` | Flex grow factor |
| `.align(Align)` | Cross-axis alignment (`Start`, `Center`, `End`) |
| `.center()` | Shorthand for `.align(Align::Center)` |
| `.align_self(Align)` | Override parent's cross-axis alignment |
| `.justify(Justify)` | Main-axis justification |
| `.space_between()` / `.space_around()` / `.space_evenly()` | Justify shortcuts |
| `.flex_center()` | `.center().justify(Justify::Center)` |
| `.scroll_offset(n)` | Manual scroll offset |

### Responsive (breakpoint variants)

Most size/spacing methods have breakpoint variants: `_xs`, `_sm`, `_md`, `_lg`, `_xl`, `_at(breakpoint)`.
Example: `.w_sm(40)` sets width to 40 only at the `Sm` breakpoint.

### Finalization

| Method | Description |
|---|---|
| `.col(f)` | Render as vertical container, returns `Response` |
| `.row(f)` | Render as horizontal container, returns `Response` |
| `.draw(f)` | Raw buffer access. Closure: `FnOnce(&mut Buffer, Rect) + 'static` |
| `.apply(style)` | Apply a `ContainerStyle` struct |

---

## CanvasContext Quick Reference

Received in `ui.canvas(w, h, |cv| { ... })`. Coordinates are in pixel space (cols*2 x rows*4 braille resolution).

| Method | Description |
|---|---|
| `.width()` / `.height()` | Pixel dimensions |
| `.dot(x, y)` | Set single pixel |
| `.line(x0, y0, x1, y1)` | Bresenham line |
| `.rect(x, y, w, h)` | Rectangle outline |
| `.filled_rect(x, y, w, h)` | Filled rectangle |
| `.circle(cx, cy, r)` | Circle outline |
| `.filled_circle(cx, cy, r)` | Filled circle |
| `.triangle(x0, y0, x1, y1, x2, y2)` | Triangle outline |
| `.filled_triangle(x0, y0, x1, y1, x2, y2)` | Filled triangle |
| `.points(pts)` | Multiple dots from `&[(usize, usize)]` |
| `.polyline(pts)` | Connected line segments from `&[(usize, usize)]` |
| `.print(x, y, text)` | Text label overlay at pixel position |
| `.set_color(color)` / `.color()` | Set/get drawing color |
| `.layer()` | Start a new z-layer (later layers overlay earlier ones) |

---

## Where to look in the codebase

| Path | Contents |
|---|---|
| `src/context/widgets_display.rs` | Display/layout module root |
| `src/context/widgets_display/text.rs` | `text`, `styled`, `link`, `spacer`, `timer_display`, `help_from_keymap` |
| `src/context/widgets_display/status.rs` | `alert`, `badge`, `stat`, `breadcrumb`, `accordion`, `code_block`, `divider_text`, `definition_list`, `empty_state`, `confirm`, `key_hint` |
| `src/context/widgets_display/rich_output.rs` | `big_text`, `image`, `kitty_image`, `sixel_image`, `streaming_text`, `streaming_markdown`, `tool_approval`, `context_bar` |
| `src/context/widgets_display/layout.rs` | `col`, `row`, `line`, `line_wrap`, `modal`, `overlay`, `tooltip`, `group`, `container`, `scrollable`, `scrollbar`, `bordered`, `screen`, `form`, `form_field`, `form_submit`, `separator`, `separator_colored` |
| `src/context/widgets_input/text_input.rs` | `text_input`, `text_input_colored` |
| `src/context/widgets_input/textarea_progress.rs` | `textarea`, `progress`, `progress_bar` |
| `src/context/widgets_input/feedback.rs` | `spinner`, `toast`, `slider`, `notify` |
| `src/context/widgets_interactive/selection.rs` | `table`, `tabs`, `button`, `checkbox`, `toggle`, `select`, `radio`, `multi_select` |
| `src/context/widgets_interactive/collections.rs` | `grid`, `list`, `calendar`, `file_picker` |
| `src/context/widgets_interactive/tree_widgets.rs` | `tree`, `directory_tree` |
| `src/context/widgets_interactive/rich_markdown.rs` | `markdown`, `virtual_list`, `command_palette`, `rich_log`, `key_seq` |
| `src/context/widgets_interactive/events.rs` | Key/mouse input, clipboard, theme, environment queries, `help`, `help_colored` |
| `src/context/widgets_viz.rs` | `bar_chart`, `bar_chart_with`, `bar_chart_grouped`, `bar_chart_stacked`, `sparkline`, `candlestick`, `candlestick_hd`, `heatmap`, `heatmap_halfblock`, `treemap`, `histogram`, `chart`, `scatter`, `canvas`, `qr_code` |
| `src/context/runtime.rs` | `use_state`, `use_memo`, `register_focusable`, `widget`, `error_boundary`, `notify`, `light_dark`, focus/scroll management |
| `src/context/container.rs` | `ContainerBuilder`, `CanvasContext` |
| `src/context/state.rs` | `Response`, `State<T>` |
| `src/widgets.rs` | All `*State` types (facade) |
| `src/widgets/` | Grouped state definitions: `input.rs`, `collections.rs`, `feedback.rs`, `selection.rs`, `commanding.rs` |

If you are contributing a new widget, read [CONTRIBUTING.md](../CONTRIBUTING.md), [DESIGN_PRINCIPLES.md](DESIGN_PRINCIPLES.md), and [ARCHITECTURE.md](ARCHITECTURE.md) first.
