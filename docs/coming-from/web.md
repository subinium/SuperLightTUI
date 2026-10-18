# Coming from the Web

This guide maps what you already know from HTML, CSS, the DOM, and React onto
SLT. It is a translation table plus the handful of places where a terminal
works differently from a browser.

<!-- slt-check-default: illustrative fragments require application context -->

## The mental model in one paragraph

Your closure is a render function that runs **every frame**, like a React
component that re-renders on every tick. It does not build a DOM you mutate
later; it describes the whole screen from scratch each time. SLT records what
you described, runs a flexbox layout pass, diffs the result against the
last rendered frame **cell by cell** (a cell is one character slot), and writes only
the cells that changed. State lives outside the closure in plain Rust
variables, or inside it in hooks (`ui.use_state`), which play the role of
`useState`.

<!-- slt-check: {"name":"web_counter","features":["crossterm"],"context":"module"} -->
```rust
use slt::Context;

fn main() -> std::io::Result<()> {
    let mut count = 0; // state lives outside the render function

    slt::run(|ui: &mut Context| {
        // This whole closure is your component's render body.
        ui.text(format!("count: {count}"));
        if ui.button("+1").clicked {
            count += 1; // event handling happens inline, no callbacks to wire
        }
    })
}
```

Three consequences worth internalizing early:

- **No event listeners.** A widget returns a `Response` describing what
  happened to it this frame (`clicked`, `hovered`, `changed`, ...). You branch
  on it right where you rendered it.
- **No virtual DOM keys.** Identity comes from call order (hooks, button
  focus) or from state you pass in (`&mut ScrollState`, `&mut TextInputState`:
  a focused input keeps focus when widgets before it come and go). Keep call
  order stable, and use `use_state_keyed` / `register_focusable_keyed` when
  items move around.
- **Layout is one frame behind.** Hit-testing and `Response.rect` use the
  last layout, the way `getBoundingClientRect()` would if you could only call it
  before the browser reflowed. See the [previous-frame rule](../guide/02-state.md#the-previous-frame-rule).

## Elements and layout

| HTML / CSS | SLT |
|------------|-----|
| `<div>` | `ui.container().col(\|ui\| ..)` or `ui.col(\|ui\| ..)` |
| `display: flex; flex-direction: row` | `ui.row(\|ui\| ..)` / `ui.container().row(..)` |
| `display: flex; flex-direction: column` | `ui.col(\|ui\| ..)` (column is the natural block flow) |
| `<span>` / inline run of text | `ui.line(\|ui\| ..)` (row of text pieces) |
| `<p>`, text node | `ui.text("..")`, `.wrap()` for multi-line wrapping |
| `<button>` | `ui.button("..")` |
| `<input type="text">` | `ui.text_input(&mut TextInputState)` |
| `<textarea>` | `ui.textarea(&mut TextareaState, rows)` |
| `<input type="checkbox">` | `ui.checkbox(label, &mut bool)` / `ui.toggle(..)` |
| `<select>` | `ui.select(&mut SelectState)` |
| `<table>` | `ui.table(&mut TableState)` |
| `<ul>` with selection | `ui.list(&mut ListState)` |
| `<a href>` | `ui.link(text, url)` |
| `<details>` / `<summary>` | `ui.accordion(title, &mut bool, \|ui\| ..)` |
| `<hr>` | `ui.separator()` |
| `<dialog>` | `ui.modal(\|ui\| ..)` |
| `gap`, `row-gap`, `column-gap` | `.gap(n)`, `.row_gap(n)`, `.col_gap(n)` |
| `padding`, `padding-x/y/top/bottom` | `.p(n)`, `.px(n)`, `.py(n)`, `.pt(n)`, `.pb(n)`, `.pl(n)`, `.pr(n)` |
| `margin` | `.m(n)`, `.mx(n)`, `.my(n)` |
| `flex-grow` / `flex: 1` | `.grow(n)` / `.fill()` |
| `flex-shrink` | `.shrink()` (opt-in; see below) |
| `flex-basis` | `.basis(cells)` |
| `flex-wrap: wrap` | `.wrap()` on a row container |
| `align-items` / `justify-content` | `.align(Align::Center)`, `.justify(Justify::SpaceBetween)` |
| `justify-content: space-between` | `.space_between()` |
| centering both axes | `.center()` |
| `align-self` | `.align_self(Align::End)` |
| `border` / `border-radius` | `.border(Border::Single)` / `.rounded()`; `ui.bordered(Border::Rounded)` |
| `border-color` | `.border_fg(color)` |
| `background-color` | `.bg(color)` |
| `color` (inherited) | `.text_color(color)` on a container, `.fg(color)` on text |
| `font-weight: bold`, `opacity`-like dimming | `.bold()`, `.dim()` |
| `text-overflow: ellipsis` | `.truncate()` |
| `<fieldset><legend>` | `ui.bordered(Border::Rounded).title("Legend")` |

One difference that surprises people: in CSS every flex item shrinks by
default (`flex-shrink: 1`). In SLT a fixed-size child keeps its size and the
overflow is clipped unless you mark it `.shrink()`.

## Sizing units

There are no pixels. Everything is measured in **terminal cells**.

| CSS | SLT | Notes |
|-----|-----|-------|
| `width: 200px` | `.w(20)` | 20 columns. A cell is roughly twice as tall as it is wide, so `w(20).h(10)` is close to square. |
| `width: 50%` | `.w_pct(50)` | Percent of the parent. Same for `.h_pct(..)`. |
| `min-width` / `max-width` | `.min_w(n)` / `.max_w(n)` | Also `.min_h`, `.max_h`. |
| `height: 100vh` | `.grow(1)` in the root column, or read `ui.height()` | `ui.width()` / `ui.height()` give the terminal size in cells. |
| `rem`, `em`, `line-height` | none | One line of text is exactly one row. |
| `@media (min-width: ..)` | `ui.breakpoint()` and `.xs_w(..)` ... `.xl_w(..)` | Breakpoints: `Xs` < 40, `Sm` 40-79, `Md` 80-119, `Lg` 120-159, `Xl` >= 160 columns. |

<!-- slt-check: {"name":"web_responsive","features":[],"context":"ui"} -->
```rust
// Responsive: stack on narrow terminals, side-by-side on wide ones.
match ui.breakpoint() {
    Breakpoint::Xs | Breakpoint::Sm => {
        ui.col(|ui| {
            ui.text("sidebar");
            ui.text("main");
        });
    }
    _ => {
        ui.row(|ui| {
            ui.container().w(24).col(|ui| {
                ui.text("sidebar");
            });
            ui.container().grow(1).col(|ui| {
                ui.text("main");
            });
        });
    }
}

// Or per-breakpoint widths on one container (like Tailwind's md:w-*).
ui.container().w(20).md_w(40).lg_w(60).col(|ui| {
    ui.text("panel");
});
```

## State and hooks (React to SLT)

| React | SLT | Notes |
|-------|-----|-------|
| `useState` | `ui.use_state(\|\| init)` returns a `State<T>` handle; read with `.get(ui)`, write with `.get_mut(ui)` | Order-based, like React's rules of hooks. |
| `useState` inside a `.map()` or an `if` | `ui.use_state_keyed(key, \|\| init)` | Key-based (`&'static str` or `String`), survives reordering and conditional calls. |

| `useMemo` | `ui.use_memo(&deps, \|deps\| ..)` | Recomputes when `deps` changes (`PartialEq`). |
| `useEffect` | `ui.use_effect(\|deps\| .., &deps)` | Runs on first frame and when `deps` changes. No cleanup closure. |
| Context provider / `useContext` | `ui.provide(value, \|ui\| ..)` / `ui.use_context::<T>()` | Scoped to the closure body. |
| Redux / app store | a plain Rust struct you own and pass `&mut` | Usually the clearest option. |
| `props` | function arguments | Components are `fn panel(ui: &mut Context, ..)`. |

<!-- slt-check: {"name":"web_hooks","features":[],"context":"ui"} -->
```rust
// A self-contained "component" with local state, like a React function component.
fn like_button(ui: &mut Context) {
    let likes = ui.use_state(|| 0u32);
    if ui.button("Like").clicked {
        *likes.get_mut(ui) += 1;
    }
    let n = *likes.get(ui);
    ui.text(format!("{n} likes"));
}

like_button(ui);
```

Prefer a plain struct for anything shared between screens or needed outside
rendering; reach for hooks for small, local UI state. [Patterns guide](../guide/03-patterns.md)
covers the trade-off, and [State API guide](../reference/state.md) lists every state
helper.

## Events (DOM to SLT)

Mouse input is **on by default**: clicks, the wheel, and drags reach your
closure with plain `slt::run` (over SSH the default drops to clicks and the
wheel only). `:hover`, `onMouseEnter`, and tooltips need every pointer move,
so opt in with `RunConfig::default().mouse_mode(MouseMode::Hover)`; see
[Interaction guide](../guide/04-interaction.md#the-mouse-is-on-by-default). Keyboard focus
(Tab / Shift+Tab) and activation (Enter / Space) work either way.

| DOM | SLT |
|-----|-----|
| `onClick` | `if ui.button("..").clicked { .. }` or `.on_click(ui, \|ui\| ..)` |
| click on any `<div>` | the container's `Response`: `ui.container().col(..).clicked` |
| `onDoubleClick` | `response.double_clicked` |
| `onMouseDown` / `onMouseUp` | `response.pressed` / `response.released` |
| `click` fires on release (drag-off cancels) | `RunConfig::default().click_on_release(true)`; the default fires `clicked` on press |
| pointer capture + `pointermove` while dragging | `response.dragging` + `response.drag_delta` (the press captures the pointer until release) |
| `onAuxClick` (middle button) | `response.middle_clicked` |
| `event.shiftKey` / `ctrlKey` / `altKey` on a click | `response.modifiers.contains(KeyModifiers::SHIFT)` |
| `onContextMenu` + a menu | `ui.context_menu(&mut MenuState, &response)`: opens on right click, `Shift+F10` or the `Menu` key; returns the activated item id |
| `onMouseEnter` / `:hover` | `response.hovered`, or `.hover_bg(..)` styling (below); needs `MouseMode::Hover` |
| `onMouseDown` held / `:active` | `.active_bg(..)` on a container: applies from press until release |
| `title` attribute | `.on_hover(ui, "tooltip")` or `ui.tooltip("..")`; needs `MouseMode::Hover`, shows after `RunConfig::tooltip_delay` (500 ms) |
| `cursor: pointer` / `text` / `grab` ... | `ui.container().pointer_shape(PointerShape::Pointer)` with `RunConfig::pointer_shapes(true)` (OSC 22) |
| `caret-shape` | `ui.set_cursor_shape(CursorShape::SteadyBar)` (DECSCUSR) |
| `onChange` | `response.changed` |
| `onSubmit` / Enter in an input | `response.submitted` |
| `onFocus` / `onBlur` | `response.gained_focus` / `response.lost_focus`; `response.focused` while focused |
| `element.focus()` | `ui.register_focusable_named("name")` + `ui.focus_by_name("name")` |
| `onWheel` | `response.scroll_delta`, `ui.scroll_up()` / `ui.scroll_down()` |
| `onMouseDown` / `onMouseMove` / `onMouseUp` | `ui.mouse_down()`, `ui.mouse_drag()`, `ui.mouse_up()`, `ui.mouse_pos()` |
| `keydown` listener on `window` | `ui.key('k')`, `ui.key_code(KeyCode::Up)`, `ui.shortcut(KeyCode::Char('s'), KeyModifiers::CONTROL)` |
| `event.stopPropagation()` for keys | `ui.consume_key(c)` / `ui.consume_key_code(code)` |
| `onPaste` | `ui.paste()` |

Two rules do not exist on the web:

1. **Keys are polled, not dispatched.** `ui.key('q')` sees presses nobody has
   consumed. Like a browser, a focused text field keeps the keys it edits
   with: while a `text_input` has focus, `ui.key('q')` stays `false` in any
   call order, and chords such as Ctrl+S still reach `ui.shortcut(..)`.
   `ui.raw_key('q')` is the "listen on `window` in the capture phase" escape
   hatch.
2. **Clicks stop at the innermost target unless the parent opts in.** A press
   sets `clicked` only on the topmost, innermost slot under the pointer, as if
   every handler called `stopPropagation()`. Overlay content blocks what is
   drawn under it. Add `.bubble_clicks()` to a container that should also
   react to clicks on its descendants, or read `Response::clicked_any` for
   plain containment. [Interaction guide](../guide/04-interaction.md) covers this and the
   other hit-testing rules.

## Positioning and overflow

| CSS | SLT | Notes |
|-----|-----|-------|
| `overflow: auto` / `scroll` | `ui.scrollable(&mut ScrollState).col(..)` | Wheel, keyboard focus, and `ui.scrollbar(&mut state)`. `scroll_col` / `scroll_row` are shorthands. |
| `overscroll-behavior: auto` (scroll chaining) | the default: an inner scrollable at its end hands the wheel to the enclosing one | |
| `overscroll-behavior: contain` | `ui.scrollable(&mut s).overscroll_contain()` | Keeps the wheel inside the region at its limit. |
| `overflow-anchor: auto` | the default: `ScrollState::anchor` keeps the visible content in place when content above changes height | `scroll.anchor = false` opts out. |
| chat "stick to bottom" | `ScrollState::new().stick_to_bottom(true)` | Follows new content while at the end; scrolling up disengages it. |
| `scrollIntoView()` on focus | automatic: `ScrollState::follow_focus` (on by default) | Set `follow_focus = false` for app-controlled scrolling. |
| `overflow: hidden` | the default: children clip to their container | |
| `position: fixed` | `ui.overlay_at(Anchor::TopRight, \|ui\| ..)`, `ui.overlay_at_offset(anchor, dx, dy, ..)` | Positioned against the whole screen. |
| `position: absolute` next to an element / `popover` + anchor positioning | `ui.popover(response.rect, Placement::Below, \|ui\| ..)` | Floats next to the element without reflowing the page; flips at the screen edge (`position-try: flip-block`) and is clamped inside it. `PopoverResponse::dismissed` reports `Esc` or an outside press (light dismiss, the press still reaches what is underneath). |
| `<dialog>` / `showModal()` | `ui.modal(..)`, `ui.modal_with(ModalOpts::new().anchor(Anchor::TopRight).offset(2, 1), ..)` | Dims the background, blocks clicks and keys underneath, and traps Tab inside. Focus returns to the previous widget when it closes. Non-modal overlays block clicks only where their content is. |
| `<select>` dropdown | `ui.select(&mut SelectState)` | The open list floats over the content below; an outside click closes it without clicking through. |
| context menu / menubar | `ui.context_menu(&mut MenuState, &response)`, `ui.menu_bar(&mut MenuBarState)` | Keyboard navigation, submenus, `Esc` and outside click close (the click is swallowed), focus returns to the trigger. |
| `z-index` | none: overlays stack in call order | Later overlays draw on top. |
| `position: sticky` | none | Render the header outside the scrollable instead. |

## Styling

| CSS | SLT |
|-----|-----|
| stylesheet / class | none: style is chained at the call site. Factor repeated chains into helper functions. |
| CSS variables (`--primary`) | theme tokens: `ui.color(ThemeColor::Primary)`; `ui.theme()` |
| switching themes | `ui.set_theme(Theme::light())`, or `RunConfig::default().theme(..)` |
| a subtree with a different theme | `ui.container().theme(Theme::light()).col(..)` |
| `prefers-color-scheme` | call `slt::detect_color_scheme()` before `run` (asks the terminal for its background; `Unknown` when queries are unavailable) and pick a theme. Inside the closure, `ui.is_dark_mode()` follows the active theme, and `.dark_bg(..)` / `.dark_border_style(..)` apply in dark mode |
| Tailwind color palette | `slt::palette::tailwind::BLUE.c500` (shades `c50` to `c950`) |
| `transition` | `ui.animate_bool("id", flag)` / `ui.animate_value("id", target, ticks)` return a 0..1 or interpolated `f64`; `ui.animate_bool_keyed(format!("row-{id}"), flag)` per row |
| keyframes, springs | `Tween`, `Spring`, `Keyframes` ([Animation guide](../guide/07-animation.md)) |
| `:hover` | `ui.container().hover_bg(..)` / `.hover_fg(..)` / `.hover_border_fg(..)`: per element, so the same code in a loop styles each row on its own |
| `:focus` | `.focus_bg(..)` / `.focus_fg(..)` / `.focus_border_fg(..)` on the container right after `ui.register_focusable()` |
| `:focus-within` | `.focus_within_bg(..)` / `.focus_within_border_fg(..)`: a widget inside the container holds focus |
| `:active` | `.active_bg(..)` / `.active_fg(..)` / `.active_border_fg(..)`: left button held after pressing the container |
| `:disabled` / `<fieldset disabled>` | `ui.disabled(cond, \|ui\| ..)`: widgets inside keep their Tab position but skip focus, ignore clicks, Enter and typing, render dimmed, and report `Response::disabled` |
| `:hover` on a named group | `ui.group("card")` + `.group_hover_bg(..)` / `.group_hover_border_style(..)` (hover of that group instance only, never focus) |
| `font-family`, `font-size` | none: the user's terminal decides |

<!-- slt-check: {"name":"web_styling","features":[],"context":"ui"} -->
```rust
use slt::palette::tailwind;

let accent = ui.color(ThemeColor::Primary);
ui
    .bordered(Border::Rounded)
    .border_fg(accent)
    .bg(tailwind::SLATE.c900)
    .p(1)
    .col(|ui| {
        ui.text("Tailwind slate background").fg(tailwind::SKY.c300);
    });

// A 0..1 transition value, like `transition: opacity 200ms`.
let expanded = true;
let t = ui.animate_bool("panel::expand", expanded);
let width = (10.0 + 30.0 * t) as u32;
ui.container().w(width).bg(tailwind::INDIGO.c700).col(|ui| {
    ui.text("grows smoothly");
});
```

Colors are downsampled automatically on terminals with fewer colors, so an RGB
value may look different on a 256-color or 16-color terminal. See
[Terminal compatibility guide](../guide/11-terminal-compatibility.md) and
[Theming guide](../guide/06-theming.md).

## Browser APIs

| Browser | SLT |
|---------|-----|
| `<a href>` / `window.open` | `ui.link(text, url)`: focusable; click or Enter opens the URL with the system opener (`open` / `xdg-open` / `start`) on the machine running the app, and OSC 8 is emitted for terminals with native hyperlinks |
| `navigator.clipboard.writeText` | `ui.copy_to_clipboard(text)` (OSC 52; the terminal must allow it). The promise result maps to `ui.clipboard_status()` on the next frame: `Sent`, `TooLarge`, or `WriteFailed` |
| `fetch` + `await` | `ui.spawn(async { .. })` + `ui.poll(&handle)` (`async` feature), or `slt::run_async` with a message channel |
| `setTimeout` | `ui.schedule("id", duration)` returns `true` once when the delay has passed |
| `setInterval` | `ui.every("id", duration)` returns how many intervals elapsed |
| `requestAnimationFrame` | the closure already runs every frame; `RunConfig::tick_rate` / `max_fps` set the cadence. With `RunConfig::idle_redraw(false)` an idle loop renders nothing and `ui.request_frame()` / `ui.request_frame_after(d)` ask for the next frame, like `requestAnimationFrame` |
| `localStorage` | none built in: read and write files yourself (`serde` feature helps) |
| `document.title` | `RunConfig::title` for the initial value, `ui.set_title(..)` at runtime (restored on exit) |
| `console.log` | not while the UI owns the terminal: log to a file or a `rich_log` panel |

<!-- slt-check: {"name":"web_timers","features":[],"context":"ui"} -->
```rust
use std::time::Duration;

// setTimeout: fires once, ~2s after the first frame that called it.
if ui.schedule("toast::hide", Duration::from_secs(2)) {
    ui.text("two seconds passed");
}

// setInterval: count elapsed one-second intervals.
let ticks = ui.every("clock", Duration::from_secs(1));
if ticks > 0 {
    // advance a clock by `ticks` seconds
}

// Hyperlink and clipboard.
ui.link("Docs", "https://docs.rs/superlighttui");
if ui.button("Copy URL").clicked {
    ui.copy_to_clipboard("https://docs.rs/superlighttui");
}
```

## What has no equivalent

Be honest with yourself about these before porting a design:

- **No CSS files, selectors, or cascade.** Styling is code. Inherited text color
  (`text_color`) and themes are the only cascading pieces.
- **No `grid-template`.** Use nested rows and columns, `.wrap()` rows, or
  `ui.grid(..)` for fixed column counts.
- **No `z-index`.** Overlays stack in call order.
- **State styles are per container, not per selector.** `:hover`, `:focus`,
  `:focus-within` and `:active` are builder calls on the container they style;
  there is no `:nth-child`, `:has()` or sibling combinator.
- **`cursor:` is best effort.** `ContainerBuilder::pointer_shape(..)` maps to
  OSC 22 and is opt-in (`RunConfig::pointer_shapes(true)`) because many
  terminals ignore or print it. The text cursor's shape is settable with
  `ui.set_cursor_shape(..)` (block, underline, bar).
- **No sub-cell positioning, fonts, or font sizes.** One character per cell.
- **No native text selection while the mouse is captured** (the default),
  the way a browser page has it. SLT draws its own selection overlay and
  copies on release; widget drags never select, the way `user-select: none`
  keeps a slider drag from selecting. `RunConfig::text_selection(false)` turns
  the overlay off app-wide. Most terminals bypass mouse capture while Shift is
  held (Shift-drag selects natively); the modifier differs on some macOS
  terminals. `RunConfig::default().mouse_mode(MouseMode::Off)` restores native
  selection entirely.
- **No guaranteed capabilities.** Colors, images, hyperlinks, clipboard, and
  extra key events depend on the user's terminal and multiplexer. Design for a
  graceful fallback; see [Terminal compatibility guide](../guide/11-terminal-compatibility.md).

## End-to-end example: a card list

A scrollable list of cards. Hovering a card highlights it, clicking a card
expands it, and the wheel scrolls the list.

<!-- slt-check: {"name":"web_card_list","features":["crossterm"],"context":"module"} -->
```rust
use slt::{Border, Color, Context, KeyCode, MouseMode, RunConfig, ScrollState};

struct Card {
    title: String,
    body: String,
    expanded: bool,
}

fn card_list(ui: &mut Context, cards: &mut [Card], scroll: &mut ScrollState) {
    // overflow-y: auto
    ui.scrollable(scroll).grow(1).p(1).gap(1).col(|ui| {
        for card in cards.iter_mut() {
            // :hover is per element: only the card under the pointer lights up.
            let response = ui
                .container()
                .border(Border::Rounded)
                .border_fg(Color::Indexed(240))
                .hover_border_fg(Color::Cyan) // :hover border
                .hover_bg(Color::Indexed(236)) // :hover background
                .active_bg(Color::Indexed(238)) // :active while pressed
                .px(1)
                .col(|ui| {
                    let marker = if card.expanded { "-" } else { "+" };
                    ui.text(format!("{marker} {}", card.title)).bold();
                    if card.expanded {
                        ui.text(&card.body).wrap();
                    }
                });
            // The container itself is the click target: no wrapper element needed.
            if response.clicked {
                card.expanded = !card.expanded;
            }
        }
    });
}

fn main() -> std::io::Result<()> {
    let mut cards: Vec<Card> = (1..=20)
        .map(|i| Card {
            title: format!("Card {i}"),
            body: format!("Details for card {i}. Click the card again to collapse it."),
            expanded: false,
        })
        .collect();
    let mut scroll = ScrollState::new();

    // Click and wheel work by default; hover highlighting needs every
    // pointer move, which is MouseMode::Hover.
    let config = RunConfig::default().mouse_mode(MouseMode::Hover);
    slt::run_with(config, |ui: &mut Context| {
        card_list(ui, &mut cards, &mut scroll);

        // Global shortcuts; a focused text field keeps the keys it edits with.
        if ui.key('q') || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
    })
}
```

What to notice:

- `hover_*` and `active_*` styles belong to the container itself and use its
  own [previous-frame](../guide/02-state.md#the-previous-frame-rule) rect, so the same code in a loop highlights one card at a
  time (hover needs `MouseMode::Hover` above). `ui.group(name)` with
  `group_hover_*` is still there when nested containers should react to the
  hover of a named parent.
- The card container's `Response` is clickable. Clicks register on mouse
  press (or on release with `RunConfig::click_on_release(true)`), and
  hit-testing uses last frame's layout, so a card that just expanded has its
  new size from the next frame on.
- These cards are mouse-only, like a `<div onClick>` without `tabindex`. To
  make them keyboard-reachable, use `ui.accordion(..)` per card, or register a
  focus slot yourself as shown in the expandable tool-call log in
  [Interaction guide](../guide/04-interaction.md).

## Next

- [Quick start](../guide/01-quick-start.md) - the shortest path to a running app
- [Interaction guide](../guide/04-interaction.md) - mouse, focus, hover, drag, scrolling, and their limits
- [Patterns guide](../guide/03-patterns.md) - structuring a real app
- [Widget guide](../reference/widgets.md) - the widget catalog
- [Theming guide](../guide/06-theming.md) - themes, tokens, and dark mode
