# Debugging Guide

This guide covers the runtime behaviors that are easy to misread if you only look at widget code.

## The big idea: interaction is often previous-frame data

SLT is immediate mode.

1. your closure records commands
2. layout runs later
3. hit areas and focus rectangles are known after layout
4. the next frame reads that data back through `prev_*`

That means hover/click/focus-sensitive behavior can feel one frame delayed in the implementation even though it looks instant at runtime.

## F12 layout debugger

Press `F12` in a running terminal app to toggle the debug overlay.

Use it when you need to inspect:

- container bounds
- nesting depth
- widget count (broken down by layer)
- frame timing / FPS
- current terminal dimensions

This is the fastest way to debug clipping, spacing, and invisible layout issues.

### Reading the colored outlines

Each layer family is tinted with a distinct hue, matching the convention used by
Chrome DevTools' layout overlay, the React DevTools component highlighter, and the
Flutter Inspector's widget tree boundaries. Within each family the color lightens
with depth so nested containers stay visually separable.

| Color family | Layer | Triggered by |
|--------------|-------|--------------|
| Green | **Base** | `ui.col`, `ui.row`, `ui.container().*`, every non-overlay widget |
| Red | **Overlay** | `ui.overlay`, `ui.overlay_at`, `ui.overlay_at_offset`, `ui.tooltip` |
| Blue | **Modal** | `ui.modal`, `ui.modal_at`, `ui.modal_at_offset` |

Tooltips currently share the Overlay (red) family because they ride the same
non-modal overlay plumbing — the layout tree does not yet carry a separate
tooltip tag. If you need to tell them apart visually, give the tooltip
container a unique title or background while debugging.

### Reading the status line

The status bar at the bottom of the screen shows a per-layer breakdown when
more than one layer family is populated:

```
[SLT Debug] 120x40 | 14 widgets (8 base, 5 overlay, 1 modal) | 1.7ms | 60fps
```

- **`14 widgets`** — total leaf widgets the renderer drew this frame.
- **`(8 base, 5 overlay, 1 modal)`** — only present when at least two layer
  families have widgets, so a base-only scene keeps the short status line.
- **`1.7ms`** — last frame time.
- **`60fps`** — exponential moving average frame rate.

Use `Context::set_debug_layer(DebugLayer::TopMost)` to outline only the active
modal/overlay (helpful when an overlay is fighting the base layout for space)
or `DebugLayer::BaseOnly` to keep the legacy pre-fix behavior of skipping
overlays entirely.

## Common failure modes

### 1. Hover or click seems "off"

Check:

- is mouse capture on at all? (see failure mode 6)
- was the widget laid out in the previous frame?
- are you reading a child response when the container owns the interaction area?
- are you conditionally rendering the widget in a way that changes call order?

### 2. Hook panic or state mismatch

`use_state()` and `use_memo()` must be called in the same order every frame.

Bad:

```rust
if show_sidebar {
    let state = ui.use_state(|| 0);
}
```

Good:

```rust
let state = ui.use_state(|| 0);
if show_sidebar {
    ui.text(format!("{}", state.get(ui)));
}
```

If you genuinely need a hook inside an `if` or `match` arm, use `ui.use_state_named(id, init)` or `ui.use_state_named_default::<T>(id)`. They key by the supplied `&'static str`, so conditional placement is safe. The positional `use_state` rule still applies.

### 3. `Response.rect` is empty

`Response.rect` is meaningful only after the widget has participated in layout.
If the widget was not laid out or the response comes from a helper path that has no visible rect yet, you may see a zero-sized rect.

### 4. `focused` is not what you expected

`Response.focused` is for the widget that actually registered focus.
Wrapping a focused child inside `row()` or `container()` does not automatically mean the container response is focused.

### 5. Raw draw output is clipped strangely

`raw_draw` is clipped by the same layout and viewport rules as other content.
Verify:

- parent container bounds
- scrollable viewport
- draw rect size
- child margin/padding

### 6. Clicks, hover, or scrolling do nothing

Mouse capture is opt-in. `slt::run(...)` uses `RunConfig::default()`, where
`mouse` is `false`, so the terminal never sends mouse events. Start the app with:

```rust
slt::run_with(slt::RunConfig::default().mouse(true), |ui| {
    // ...
})
```

Buttons still work from the keyboard without it (Tab to focus, Enter / Space to
activate), which can make the missing flag easy to overlook. `TestBackend`
tests inject mouse events directly, so they pass either way. Run
`cargo run --example mouse_probe` to check whether your terminal delivers mouse
events at all.

### 7. A shortcut fires while typing into an input

Since 0.26 a focused `text_input` / `textarea` / form field keeps the keys it
edits with, so `ui.key('q')` stays `false` while the user types into it, in
either call order. If a shortcut still fires, check that it is not
`ui.raw_key(..)` / `raw_events()` (those ignore text focus), and that the input
actually had focus when the frame started: on an app's very first frame
nothing has registered focus yet.

## Layout debugging checklist

1. Toggle F12.
2. Reduce the case to one container and one child.
3. Replace dynamic content with `ui.text("X")` to prove layout is correct first.
4. Check whether you want `row()` or `line()`.
5. If using overlays or modals, verify whether background interaction should be blocked.

## Focus and modal debugging

Focus handling changes inside modal layers.

- `register_focusable()` inside a modal participates in modal-local focus.
- widgets outside the modal are effectively inert while the modal is active.
- `interaction()` also short-circuits outside the active modal layer.

If a custom widget behaves strangely in overlays, inspect both `focused` and whether it is inside the active overlay depth.

## Clipboard and terminal caveats

- `read_clipboard()` depends on OSC 52 support in the terminal.
- `copy_to_clipboard(...)` writes OSC 52 through the built-in terminal backend after the frame. It never prints to stderr or fails the frame; check `ui.clipboard_status()` on the next frame: `Sent` (written, not necessarily received: OSC 52 has no acknowledgement), `TooLarge { bytes, max }` (over `RunConfig::clipboard_max_bytes`, default 74,994 bytes, nothing written), or `WriteFailed(kind)`.
- Inside tmux, the default `set-clipboard external` drops OSC 52. Use `set -g set-clipboard on`, or `RunConfig::clipboard_passthrough(true)` with `set -g allow-passthrough on` (GNU screen gets DCS-chunked passthrough). Measured results for tmux 3.5a are in [TERMINAL_COMPATIBILITY.md](TERMINAL_COMPATIBILITY.md).
- Kitty keyboard: on a direct terminal that answers startup queries, the protocol is pushed only after a `CSI ? u` reply; `ui.capabilities().kitty_keyboard` reports it. Elsewhere it is best-effort and unsupported terminals ignore it. `SLT_FORCE_KITTY_KEYBOARD=1` skips the confirmation.
- Title, progress, program status, and cursor-shape sequences are suppressed for `TERM=dumb`; `SLT_FORCE_PROGRESS=1` sends OSC 9;4 to terminals SLT does not recognize as progress hosts.

## Hot reload (experimental)

SLT re-invokes one closure every frame, so [Subsecond](https://docs.rs/subsecond)
(the Dioxus hot-patching runtime) can swap that closure's code while the
terminal session and hook state stay alive. SLT needs no feature flag or
library code for this; the wiring lives in your app crate. Treat it as a
debug-build convenience, not a supported API.

Requirements: `dioxus-cli` 0.7 (`cargo install dioxus-cli --version 0.7.10 --locked`,
about 4 minutes on an M-series Mac), and `subsecond` plus `dioxus-devtools`
0.7.10 in your app (MIT OR Apache-2.0, they build on SLT's MSRV 1.88). Patching
works only in debug builds, and Subsecond patches only the crate that holds
`main`. Edits to SLT or to other workspace crates are not applied.

```toml
[dependencies]
subsecond = { version = "0.7.10", optional = true }
dioxus-devtools = { version = "0.7.10", optional = true }
libc = { version = "0.2", optional = true }

[features]
hot = ["dep:subsecond", "dep:dioxus-devtools", "dep:libc"]
```

```rust,ignore
use slt::Context;

fn app(ui: &mut Context) {
    let count = ui.use_state(|| 0u64);
    *count.get_mut(ui) += 1;
    let n = *count.get(ui);
    ui.text(format!("frames: {n}"));
}

fn main() -> std::io::Result<()> {
    #[cfg(feature = "hot")]
    {
        attach_tty();
        dioxus_devtools::connect_subsecond();
    }
    slt::run(|ui| {
        #[cfg(feature = "hot")]
        subsecond::HotFn::current(app as fn(&mut Context)).call((ui,));
        #[cfg(not(feature = "hot"))]
        app(ui);
    })
}

/// `dx serve` pipes the app's stdout, so point stdin/stdout at the
/// terminal named by `SLT_TTY` (for example another tmux pane).
#[cfg(feature = "hot")]
fn attach_tty() {
    use std::os::fd::IntoRawFd;
    let Ok(path) = std::env::var("SLT_TTY") else {
        return;
    };
    let tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("SLT_TTY must name a terminal device");
    let fd = tty.into_raw_fd();
    // SAFETY: `fd` is an open descriptor we own; dup2 only replaces 0 and 1.
    unsafe {
        libc::dup2(fd, 0);
        libc::dup2(fd, 1);
    }
}
```

Run it with the UI in a second terminal of the same size:

```sh
# terminal B (the UI): print its device, then leave it idle
tty && sleep 100000
# terminal A (dx): pass terminal B's device
SLT_TTY=/dev/ttys002 dx serve --hotpatch --features hot
```

Gotchas, all measured on the setup below:

- **Use `HotFn::current(app as fn(&mut Context))`, not `subsecond::call(|| app(ui))`.**
  The closure form applies the patch (dx logs it and patch handlers run) but
  keeps rendering the old code. A closure that captures only `ui` is one pointer
  wide, so Subsecond treats it as a function pointer and looks the wrong address
  up in its jump table. Passing `app` as a function pointer looks up `app` itself.
- **`dx serve` pipes stdout.** Without the `SLT_TTY` redirect, `slt::run` exits
  with `stdout is not a terminal`. The terminal size still comes from dx's
  controlling terminal, so keep both terminals the same size.
- **A panic ends the session.** dx does not relaunch a crashed app on the next
  save. Press `r` in dx to rebuild and relaunch. Hook state is lost.

| Edit while running | Result |
|---|---|
| Text, styles, constants, layout in `app` | Applied on the next frame; `use_state` values kept |
| New helper function or container called from `app` | Applied; state kept |
| New `use_state` appended after existing hooks | Applied; new slot initialized, old slots kept |
| `use_state` inserted before existing hooks | Panic: `use_state type mismatch at hook index N` |
| `use_state` inserted before a `use_state_named` hook | Applied; named value kept |
| Fields added to a struct stored in `use_state` | Panic: type mismatch (observed: its `TypeId` changed) |
| Unrelated edit with a tip-crate struct in `use_state` | Applied; struct value kept |
| Code in `main` or outside `app` (`RunConfig`, setup) | Not applied until restart |

Timings: 237 to 794 ms from save to applied patch, and the new code renders on
the next tick. The first `dx serve` build took 34.5 s and later relaunches took
0.5 to 2.7 s. The dev binary is 20 MB and each patch dylib about 300 KB.
Enabling `hot` grows the app's normal dependency graph from 31 to 113 unique
crates (`dioxus-devtools` pulls `dioxus-core`, `dioxus-signals`,
`tungstenite`). Default builds are unaffected because the dependencies are
optional and the calls are behind `cfg`.

To survive hook insertions, use `use_state_named(...)`, which keys by id rather
than position. Measured on dioxus-cli 0.7.10, subsecond 0.7.10, Rust 1.98, and
macOS 26.5 on arm64. Linux and Windows are listed as supported by Subsecond but
were not tested here.

## Related docs

- `docs/ARCHITECTURE.md` - frame lifecycle and module map
- `docs/PATTERNS.md` - hooks, overlays, custom widgets
- `docs/TESTING.md` - headless testing patterns
