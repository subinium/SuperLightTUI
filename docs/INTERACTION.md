# Interaction Guide

How mouse, keyboard focus, hover, drag, scrolling, and expand/collapse work in
SLT, how to test them, and where the current limits are.

<!-- slt-check-default: illustrative fragments require application context -->

## Enable the mouse first

Mouse capture is **off by default**. `slt::run` uses `RunConfig::default()`,
which has `mouse: false`, so clicks, hover, drag, and the wheel never reach
your closure until you opt in:

<!-- slt-check: {"name":"ix_enable_mouse","features":["crossterm"],"context":"module"} -->
```rust
use slt::{Context, KeyCode, RunConfig};

fn main() -> std::io::Result<()> {
    let config = RunConfig::default()
        .mouse(true)
        .scroll_speed(3); // rows per wheel notch (default 1)

    slt::run_with(config, |ui: &mut Context| {
        if ui.button("Click me").clicked {
            ui.notify("clicked", slt::ToastLevel::Info);
        }
        if ui.key('q') || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
    })
}
```

What changes when you turn it on:

- SLT enables all-motion reporting, so **every pointer move is an event**. That
  is what makes hover work, and it is also more traffic over a slow SSH link.
- The terminal's own text selection stops working while the pointer is over
  the app. SLT draws its own selection overlay: drag across text and it is
  copied (OSC 52) on release. Most terminals also let you hold a modifier
  (often Shift, or Option on macOS) to bypass capture and select natively.
- Without the mouse, the app is still fully usable from the keyboard: Tab and
  Shift+Tab move focus, Enter or Space activate the focused widget.

## The `Response`

Most widgets and every container return a `#[must_use]` `Response`:

| Field | Meaning | Needs mouse? |
|-------|---------|--------------|
| `clicked` | Left-clicked this frame, or activated with Enter/Space on keyboard-activatable widgets | Click: yes. Keyboard activation: no |
| `double_clicked` | Two left presses on the same cell within ~400 ms (`clicked` is also `true`) | Yes |
| `right_clicked` | Right button pressed inside the widget | Yes |
| `hovered` | Pointer is over the widget | Yes |
| `scroll_delta` | Net wheel motion over the widget this frame (positive = up) | Yes |
| `focused` | Widget currently has keyboard focus | No |
| `gained_focus` / `lost_focus` | Focus arrived / left this frame | No |
| `changed` | The widget's value changed this frame | No |
| `submitted` | Explicit submit gesture, such as Enter in a single-line `text_input` | No |
| `rect` | Where the widget was laid out **last frame** (all zeros on frame 1) | No |

Callback-style chaining is available when it reads better:

<!-- slt-check: {"name":"ix_response_chain","features":[],"context":"ui"} -->
```rust
let mut saves = 0;
let _ = ui
    .button("Save")
    .on_hover(ui, "Write the file to disk")
    .on_click(ui, |_| saves += 1);
```

## Clickable anything

Containers return a `Response`, so any row, card, or panel is clickable without
a wrapper:

<!-- slt-check: {"name":"ix_clickable_row","features":[],"context":"ui"} -->
```rust
let items = ["alpha", "beta", "gamma"];
let mut selected = 0usize;

for (i, item) in items.iter().enumerate() {
    let row = ui
        .container()
        .px(1)
        .with_if(i == selected, |b| b.bg(Color::Indexed(238)))
        .row(|ui| {
            ui.text(*item);
        });
    if row.clicked {
        selected = i;
    }
}
```

For custom content where a container is not the right shape, call
`ui.interaction()` **immediately before** the text, link, or container that
should own the hit area. It reserves a hit-test slot, so its call order must be
stable across frames.

Containers are not keyboard-focusable on their own. To make one reachable with
Tab, register a focus slot before it (see the tool-call log example below) or
use a widget that already does this (`button`, `accordion`, `list`, ...).

## Hover styling

Use `ui.group(name)` for CSS-`:hover`-like styling. The group's own
`group_hover_bg` / `group_hover_border_style`, and those of containers nested
inside it, apply while the pointer is over the group or while a focusable
child of the group has focus.

<!-- slt-check: {"name":"ix_group_hover","features":[],"context":"ui"} -->
```rust
for i in 0..3 {
    // Each row needs its own group name; a shared name highlights every row.
    let _ = ui
        .group(&format!("file-row-{i}"))
        .px(1)
        .group_hover_bg(Color::Indexed(237))
        .row(|ui| {
            ui.text(format!("file_{i}.rs"));
            ui.spacer();
            ui.text("12 KB").dim();
        });
}
```

For one-off logic, branch on `response.hovered` instead.

## Tooltips

<!-- slt-check: {"name":"ix_tooltips","features":[],"context":"ui"} -->
```rust
// Bound to this response: safe to chain more widgets afterwards.
let _ = ui.button("Deploy").on_hover(ui, "Push the current build to staging");

// Or: tooltip for the previously rendered widget.
let _ = ui.button("Rollback");
ui.tooltip("Restore the last good build");
```

Tooltips appear under the widget (or above it when there is no room), wrap at
38 columns, and need mouse capture because they follow `hovered`. Text-only;
use `on_hover_ui` plus an overlay for richer content.

## Drag and splitters

For resizable panes, use the built-in split containers. The handle is dragged
with the mouse, or focused with Tab and moved with the arrow keys:

<!-- slt-check: {"name":"ix_split","features":[],"context":"ui"} -->
```rust
let split = ui.use_state(|| SplitPaneState::new(0.3));
let mut state = split.get(ui).clone();
let _ = ui.split_pane(
    &mut state,
    |ui| {
        ui.text("sidebar");
    },
    |ui| {
        ui.text("editor");
    },
);
*split.get_mut(ui) = state;
// `vsplit_pane` stacks the two panes vertically instead.
```

In real apps keep the `SplitPaneState` in your app struct and pass `&mut`
directly; the hook round-trip above only keeps the snippet self-contained.

For custom drag gestures, read the raw pointer events. Each returns the cell
position of an unconsumed event this frame:

<!-- slt-check: {"name":"ix_drag","features":[],"context":"ui"} -->
```rust
let drag_start = ui.use_state(|| None::<(u32, u32)>);

if let Some(pos) = ui.mouse_down() {
    *drag_start.get_mut(ui) = Some(pos);
}
if let (Some((x0, y0)), Some((x, y))) = (*drag_start.get(ui), ui.mouse_drag()) {
    ui.text(format!("dragging from {x0},{y0} to {x},{y}"));
}
if ui.mouse_up().is_some() {
    *drag_start.get_mut(ui) = None;
}
// Other buttons: mouse_down_button / mouse_drag_button / mouse_up_button.
// Last known pointer position: ui.mouse_pos().
```

`SplitPaneState::dragging` and `ScrollState::dragging` tell you when a
built-in handle or scrollbar thumb is being held.

## Wheel and scrollbars

`ui.scrollable(&mut ScrollState)` handles the wheel over its area, and
`ui.scrollbar(&mut state)` draws a clickable, draggable bar for the same
state. Shift+wheel (or a horizontal wheel) scrolls horizontally inside
`scroll_row`.

<!-- slt-check: {"name":"ix_scroll","features":[],"context":"ui"} -->
```rust
let mut scroll = ScrollState::new();

let _ = ui.row(|ui| {
    let _ = ui.scrollable(&mut scroll).grow(1).col(|ui| {
        for i in 0..200 {
            ui.text(format!("line {i}"));
        }
    });
    let _ = ui.scrollbar(&mut scroll);
});

// Wheel over any widget, without a scroll container:
let chart = ui.container().h(5).col(|ui| {
    ui.text("zoomable");
});
if chart.scroll_delta > 0 {
    // zoom in
}

// Frame-global wheel checks also exist: ui.scroll_up(), ui.scroll_down(),
// ui.scroll_left(), ui.scroll_right().
```

`ScrollState::follow_focus` (on by default) scrolls a newly focused child into
view, so Tab through a long form keeps the focused field visible. Set it to
`false` when your app controls the offset (`set_offset`, `scroll_down`).

## Expand and collapse

Three options, from least to most code:

- `ui.accordion(title, &mut open, |ui| ..)`: a header that toggles on click,
  Enter, or Space, with the body rendered only while open.
- `ui.tree(&mut TreeState)` / `ui.directory_tree(&mut DirectoryTreeState)`:
  nested nodes; Left/Right collapse and expand, Up/Down move.
- Hand-rolled: a `bool` you flip from a header's `clicked`, plus a focus slot
  for keyboard users.

<!-- slt-check: {"name":"ix_accordion_tree","features":[],"context":"ui"} -->
```rust
let mut open = false;
let _ = ui.accordion("Advanced settings", &mut open, |ui| {
    ui.text("Only rendered while open");
});

let mut tree = TreeState::new(vec![
    TreeNode::new("src")
        .expanded()
        .children(vec![TreeNode::new("lib.rs"), TreeNode::new("main.rs")]),
    TreeNode::new("Cargo.toml"),
]);
let _ = ui.tree(&mut tree);
```

### Example: an expandable tool-call log

A scrollable list of blocks. Each block has a header that toggles on click,
and also on Enter or Space once it is focused with Tab. Because the scroll
container follows focus, tabbing to a header below the fold scrolls it into
view.

<!-- slt-check: {"name":"ix_tool_log","features":["crossterm"],"context":"module"} -->
```rust
use slt::{Border, Color, Context, KeyCode, RunConfig, ScrollState};

struct ToolCall {
    name: &'static str,
    summary: &'static str,
    output: Vec<&'static str>,
    open: bool,
}

fn tool_call(ui: &mut Context, call: &mut ToolCall) {
    // Keyboard path: claim a focus slot for the header container that follows.
    let focused = ui.register_focusable();
    // Consume the key so later global shortcuts do not also see it.
    let pressed = focused && (ui.consume_key_code(KeyCode::Enter) || ui.consume_key(' '));

    // Mouse path: the header container's Response is the click target.
    let icon = if call.open { "v" } else { ">" };
    let header = ui
        .container()
        .px(1)
        .gap(1)
        .with_if(focused, |b| b.bg(Color::Indexed(237)))
        .row(|ui| {
            ui.text(icon).dim();
            ui.text(call.name).bold();
            ui.text(call.summary).dim().truncate();
        });

    if header.clicked || pressed {
        call.open = !call.open;
    }
    if call.open {
        let _ = ui.container().pl(4).col(|ui| {
            for line in &call.output {
                ui.text(*line);
            }
        });
    }
}

fn tool_log(ui: &mut Context, calls: &mut [ToolCall], scroll: &mut ScrollState) {
    let _ = ui.bordered(Border::Rounded).title("Tool calls").grow(1).row(|ui| {
        let _ = ui.scrollable(scroll).grow(1).col(|ui| {
            for call in calls.iter_mut() {
                tool_call(ui, call);
            }
        });
        let _ = ui.scrollbar(scroll);
    });
}

fn main() -> std::io::Result<()> {
    let mut calls: Vec<ToolCall> = (0..30)
        .map(|i| ToolCall {
            name: ["read_file", "grep", "bash"][i % 3],
            summary: "src/lib.rs",
            output: vec!["ok", "42 lines", "exit 0"],
            open: false,
        })
        .collect();
    let mut scroll = ScrollState::new();

    slt::run_with(RunConfig::default().mouse(true), |ui: &mut Context| {
        tool_log(ui, &mut calls, &mut scroll);
        if ui.key('q') || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
    })
}
```

Two details matter here. The header is clickable *and* focusable, but nothing
inside it is clickable, so a click is never reported twice (see Known
limitations). And collapsing a block above the viewport shifts everything
below it up, because SLT has no scroll anchoring.

## Keyboard focus and shortcuts

- Tab / Shift+Tab cycle focus through focusable widgets in render order.
- Enter / Space activate a focused button, checkbox, accordion, and similar.
- `ui.register_focusable_named("search")` + `ui.focus_by_name("search")` move
  focus programmatically.
- Focus follows the widget, not its position: when a focusable appears or
  disappears before the focused `text_input` / `textarea` / form field / named
  slot, focus stays on that widget (like a browser element). Custom widgets
  opt in with `ui.register_focusable_keyed(key)`; plain
  `ui.register_focusable()` slots, buttons and other stateless widgets remain
  positional. If the focused widget itself goes away, focus moves to the
  nearest remaining slot on the next frame.

**Focused text fields own their keys.** While a `text_input`, `textarea`,
form field or open command palette has focus, `ui.key(..)`, `ui.key_code(..)`,
`ui.key_mod(..)`, `ui.consume_key*(..)` and `ui.shortcut(..)` skip the keys it
edits with: printable characters without Ctrl / Super / Meta, Backspace,
Delete, Left, Right, Home and End (plus Enter, Up and Down for `textarea`).
The rule does not depend on call order, so shortcut checks can sit anywhere in
the closure:

<!-- slt-check: {"name":"ix_shortcut_order","features":[],"context":"ui"} -->
```rust
let mut search = TextInputState::with_placeholder("Search...");

// Safe before the input: typing "q" into the focused field inserts it and
// does not quit. Esc and Ctrl chords still reach the checks.
if ui.shortcut(KeyCode::Char('q'), KeyModifiers::NONE) || ui.key_code(KeyCode::Esc) {
    ui.quit();
}
if ui.shortcut(KeyCode::Char('s'), KeyModifiers::CONTROL) {
    // save
}

let _ = ui.text_input(&mut search);
```

Ownership is decided by the slot that holds focus as the frame starts (or a
text slot focused earlier in the same frame). A click that moves focus to a
button releases the keys in the same event batch; on an app's very first
frame nothing owns them yet. `ui.shortcut` matches modifiers exactly (Shift is
ignored for characters).

To claim a key so nothing later sees it, use `ui.consume_key(c)` or
`ui.consume_key_code(code)`. While a modal is open, `ui.key(..)` outside the
modal returns `false`. `ui.raw_key(..)`, `ui.raw_key_code(..)` and
`ui.raw_key_mod(..)` ignore both modal state and text focus for truly global
shortcuts.

Ctrl+C quits by default (`RunConfig::handle_ctrl_c(false)` hands it to your
closure), and on Unix Ctrl+Z suspends cleanly (`handle_suspend(false)` turns
that off). Key release events and bare modifier keys need
`RunConfig::kitty_keyboard(true)` (plus `report_all_keys(true)` for
modifier-only presses) on a terminal that supports the Kitty keyboard
protocol; see [TERMINAL_COMPATIBILITY.md](TERMINAL_COMPATIBILITY.md).

## Hit-testing uses the previous frame

Layout runs after your closure returns, so `clicked`, `hovered`, and `rect`
are computed against **last frame's** layout:

- On frame 1, `Response.rect` is all zeros and nothing is hovered.
- After a layout change (expanding a block, resizing), the first frame still
  hit-tests against the old geometry.
- Anything you position from a `rect` (custom popovers, scroll-to-item) can be
  one frame stale. Tolerate it rather than fight it.

[PREVIOUS_FRAME_GUIDE.md](PREVIOUS_FRAME_GUIDE.md) explains the model and its
edge cases.

## Modals block the background

`ui.modal(..)`, `ui.modal_at(..)`, and `ui.modal_at_offset(..)` dim the screen
and suppress clicks, hover, and keys for everything outside the modal.
`ui.overlay(..)` and `ui.overlay_at(..)` do **not**: a click inside a non-modal
overlay also reaches the widget drawn underneath it. Use a modal for anything
that must block input, such as confirmation dialogs and pickers.

<!-- slt-check: {"name":"ix_modal","features":[],"context":"ui"} -->
```rust
let mut confirm_open = true;
let mut deleted = false;

if ui.button("Delete").clicked {
    confirm_open = true;
}
if confirm_open {
    let _ = ui.modal_at(Anchor::Center, |ui| {
        let _ = ui.bordered(Border::Rounded).p(1).gap(1).col(|ui| {
            ui.text("Delete 3 files?");
            let _ = ui.row(|ui| {
                if ui.button("Delete").clicked {
                    deleted = true;
                    confirm_open = false;
                }
                if ui.button("Cancel").clicked {
                    confirm_open = false;
                }
            });
        });
    });
}
```

## Testing interactions

`TestBackend` renders headlessly; `EventBuilder` injects input. Render once so
layout exists, then send events at coordinates you look up with `find_text`:

<!-- slt-check: {"name":"ix_testing","features":[],"context":"module"} -->
```rust
use slt::{EventBuilder, KeyCode, TestBackend};

fn view(ui: &mut slt::Context, open: &mut [bool; 2]) {
    let _ = ui.accordion("First", &mut open[0], |ui| {
        ui.text("first body");
    });
    let _ = ui.accordion("Second", &mut open[1], |ui| {
        ui.text("second body");
    });
}

#[test]
fn click_and_keyboard_toggle() {
    let mut open = [false; 2];
    let mut tb = TestBackend::new(40, 10);

    // Frame 1: no layout yet, so nothing is clickable.
    tb.render(|ui| view(ui, &mut open));

    // Click the second header.
    let (x, y) = tb.find_text("Second").unwrap();
    let click = EventBuilder::new().click(x, y).build();
    tb.run_with_events(click, |ui| view(ui, &mut open));
    assert_eq!(open, [false, true]);

    // Enter with focus on the first focusable (focus index 0 of 2).
    let enter = EventBuilder::new().key_code(KeyCode::Enter).build();
    tb.render_with_events(enter, 0, 2, |ui| view(ui, &mut open));
    assert_eq!(open, [true, true]);

    tb.render(|ui| view(ui, &mut open));
    tb.assert_contains("first body");
}
```

Notes:

- `EventBuilder` also has `.scroll_up(x, y)`, `.scroll_down(x, y)`,
  `.drag(x, y)`, `.mouse_up(x, y)`, `.key(c)`, `.key_with(code, mods)`,
  `.paste(text)`, and `.resize(w, h)`. `.click(x, y)` sends a left-button
  press.
- `run_with_events` starts every call with focus on index 0. Use
  `render_with_events(events, focus_index, focus_count, ..)` to pick the
  focused widget, or `tb.sequence()` to chain steps (for example
  `.key(KeyCode::Tab, ..)` then `.key(KeyCode::Enter, ..)`) with focus carried
  between them.
- `TestBackend` injects mouse events directly, so a click test passes even if
  your real `main` forgot `.mouse(true)`.

[TESTING.md](TESTING.md) covers snapshots, style assertions, and sequences.

## Known limitations (v0.25)

These are current behaviors, not misuse. Design around them for now.

1. **Nested scrollables do not hand off the wheel.** When an inner scrollable
   reaches its end, further wheel motion over it does not scroll the outer
   one. Avoid nesting vertical scrollables, or give the inner one a fixed
   height and keep it short.
2. **Clicks fire on press and propagate.** `clicked` is set on mouse *down*,
   not release, and is not stopped at the innermost target: a button inside a
   clickable row reports `clicked` on both, and a click inside a non-modal
   overlay also hits what is underneath. Do not make a parent and a child
   clickable at the same time; use `modal` for popups that must block input.
3. **Only the last click per frame is kept.** Several clicks arriving in one
   frame collapse into one.
4. **Text selection can start during handle drags.** Dragging a scrollbar
   thumb or a split handle may also activate the built-in selection overlay
   and copy text on release.
5. **No scroll anchoring.** Collapsing or inserting content above the
   viewport shifts the visible content, and there is no stick-to-bottom mode
   for logs. Adjust `ScrollState::offset` yourself when you need either.
6. **Emoji width disagreements.** Some multiplexers and terminals measure ZWJ
   sequences, skin-tone modifiers, and VS16 emoji differently from SLT, which
   can leave artifacts on that row.
7. **Missing primitives.** No disabled or active (pressed) widget states, no
   context-menu widget (use `right_clicked` plus your own overlay), and no
   cursor-shape API.

## Related docs

- [PREVIOUS_FRAME_GUIDE.md](PREVIOUS_FRAME_GUIDE.md) - why geometry is one frame behind
- [TERMINAL_COMPATIBILITY.md](TERMINAL_COMPATIBILITY.md) - mouse, keys, and clipboard per terminal
- [WEB_DEVELOPERS.md](WEB_DEVELOPERS.md) - DOM events mapped to SLT
- [TESTING.md](TESTING.md) - headless testing in depth
