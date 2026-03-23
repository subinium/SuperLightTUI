# Backends and Run Loops

This guide is for the low-level path: custom render targets, external event loops, inline mode, and static output.

If you just want a normal terminal app, use `slt::run(...)` or `slt::run_with(...)` and stop here.

## Choose the right entry point

| Goal | API |
|------|-----|
| Full-screen terminal app | `run()` / `run_with()` |
| Inline widget below the current prompt | `run_inline()` / `run_inline_with()` |
| Inline widget plus scrollback log output | `run_static()` + `StaticOutput` |
| Non-terminal target or external loop | `Backend` + `AppState` + `frame()` |

## The backend mental model

SLT deliberately keeps the low-level contract small.

SLT owns:

- command recording from your UI closure
- layout computation
- focus and interaction bookkeeping
- rendering into a `Buffer`
- persistent per-session UI state in `AppState`

Your backend owns:

- the current size in terminal-style cells
- the backing `Buffer`
- presenting the finished buffer to a target

That split is intentional. A backend should not need to reimplement layout, focus, or widget logic.

## The stable contract

The public backend surface is intentionally tiny:

```rust
pub trait Backend {
    fn size(&self) -> (u32, u32);
    fn buffer_mut(&mut self) -> &mut Buffer;
    fn flush(&mut self) -> std::io::Result<()>;
}
```

And the runtime contract around it is equally small:

- `frame()` reads `backend.size()` every frame.
- `frame()` renders into `backend.buffer_mut()`.
- `frame()` calls `flush()` at the end of a successful frame.
- `flush()` errors propagate back to the caller.
- `AppState` must be reused across frames for hooks, focus, hit maps, scroll feedback, and tick/FPS state.
- `ui.quit()` causes `frame()` to return `Ok(false)`.

These are not just documentation promises. They are explicitly locked by backend contract tests and by parity tests between `frame()` and `TestBackend`.

## What the built-in backends guarantee

The built-in terminal backends are intentionally boring:

- full-screen and inline terminals share the same diff writer path for buffer, raw sequence, and cursor flushing
- terminal session setup and cleanup are centralized so raw mode, alternate screen, mouse capture, bracketed paste, and Kitty keyboard teardown stay symmetric
- built-in run loops and `TestBackend` share the same frame kernel, which reduces lifecycle drift between production rendering and tests

This matters because SLT is trying to be easy at the API layer without becoming sloppy underneath.

## `RunConfig` in practice

`RunConfig` is the runtime policy object for all built-in loops.

```rust
use slt::{RunConfig, Theme};
use std::time::Duration;

let config = RunConfig::default()
    .tick_rate(Duration::from_millis(16))
    .mouse(true)
    .theme(Theme::light())
    .max_fps(60)
    .scroll_speed(2)
    .title("My App");
```

Important details:

- `tick_rate` controls how often the loop wakes up even if no input arrives
- `max_fps` caps the render rate after work is done
- `mouse(true)` enables clicks, hovers, and wheel input
- `kitty_keyboard(true)` requests richer key events on supported terminals
- `RunConfig` is `#[non_exhaustive]`, so prefer builder methods over struct literals

## Driving `frame()` yourself

```rust
use slt::{AppState, Backend, Buffer, Context, Event, Rect, RunConfig};

struct MyBackend {
    buffer: Buffer,
}

impl Backend for MyBackend {
    fn size(&self) -> (u32, u32) {
        (self.buffer.area.width, self.buffer.area.height)
    }

    fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn main() -> std::io::Result<()> {
    let mut backend = MyBackend {
        buffer: Buffer::empty(Rect::new(0, 0, 80, 24)),
    };
    let mut app = AppState::new();
    let config = RunConfig::default();

    loop {
        let events: Vec<Event> = vec![];
        let keep_going = slt::frame(
            &mut backend,
            &mut app,
            &config,
            &events,
            &mut |ui: &mut Context| {
                ui.text("Hello from a custom backend");
            },
        )?;

        if !keep_going {
            break;
        }
    }

    Ok(())
}
```

## What `AppState` actually stores

`AppState` is the persistent frame-to-frame session state for:

- hook storage (`use_state`, `use_memo`)
- focus position and focus counts
- previous-frame hit areas and scroll bounds
- toast queue and debug overlay state
- smoothed FPS estimate and tick counter

Do not recreate it every frame.
Think of it as the session object for the UI runtime, not as a cache you can throw away casually.

## What `frame()` expects from you

- Reuse the same `AppState` across frames.
- Pass the current frame's `events` slice.
- Rebuild the event list each frame in your outer loop.
- Resize your backend buffer when your host environment changes size.
- Stop when `frame()` returns `Ok(false)` after `ui.quit()`.

`frame()` is intentionally synchronous and explicit.
That makes it easy to embed inside terminals, browser loops, game loops, test harnesses, or remote rendering targets.

## What your backend does not need to do

Your backend does not need to:

- compute layout
- track focus order
- process `Response` state
- diff widgets semantically
- own hook state

If you find yourself rebuilding those layers in a custom backend, the abstraction line is probably wrong.

## Testing custom backends

Use two testing layers:

- `TestBackend` for widget, layout, and interaction rendering tests
- `frame()` + a tiny custom `Backend` for contract tests such as flush propagation, quit behavior, and resize handling

That split mirrors the actual architecture:

- `TestBackend` is the fastest tool for most UI work
- custom backend tests lock the low-level contract itself

See `docs/TESTING.md` for the recommended patterns.

## Inline mode details

`run_inline(height, ...)` is for CLI tools that should remain embedded in normal terminal flow.

- It does not enter alternate screen mode.
- It reserves a fixed display area below the cursor.
- Resize events can change the width, but the reserved height stays the one you requested.
- Pressing `Ctrl+C` still exits the loop like the regular terminal backend.

Use it when the TUI is a helper surface rather than the whole app.

## Static output mode details

`StaticOutput` is a scrollback-friendly companion for inline apps.

```rust
use slt::StaticOutput;

let mut output = StaticOutput::new();
output.push("Build started...");
output.push("Fetching data...");
```

Use it when you want:

- a fixed inline control surface at the bottom
- persistent logs or messages above it
- a CLI tool that mixes streaming text output with interaction

## Feature flag notes

When `default-features = false` and `crossterm` is not enabled:

- built-in terminal loops are unavailable
- terminal clipboard helpers are unavailable
- terminal-owned runtime setup is unavailable

What still remains:

- `Backend`
- `AppState`
- `frame()`
- `Context`, widgets, events, styles, layout, charts

That is what makes SLT usable as a rendering core instead of only a terminal runtime.

## Related APIs

- `docs/FEATURES.md` - feature-gated runtime behavior
- `docs/TESTING.md` - `TestBackend`, backend contract tests, multi-frame patterns
- `docs/DEBUGGING.md` - F12 overlay, one-frame delay, layout debugging
- `docs/PATTERNS.md` - hooks, overlays, custom widgets
- `src/lib.rs` - canonical rustdoc for `Backend`, `AppState`, `frame()`, `RunConfig`
