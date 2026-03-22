# Backends and Run Loops

This guide covers the low-level path: custom render targets, external event loops, inline mode, and static output.

If you just want a normal terminal app, use `slt::run(...)` and stop here.

## Choose the right entry point

| Goal | API |
|------|-----|
| Full-screen terminal app | `run()` / `run_with()` |
| Inline widget below the current prompt | `run_inline()` / `run_inline_with()` |
| Inline widget plus scrollback log output | `run_static()` + `StaticOutput` |
| Non-terminal target or external loop | `Backend` + `AppState` + `frame()` |

## Full-screen vs inline vs static

- `run()` enters alternate screen mode and owns the terminal session.
- `run_inline(height, ...)` keeps you in the normal terminal screen and reserves `height` rows below the current cursor position for the interactive UI.
- `run_static(output, dynamic_height, ...)` keeps a fixed inline UI at the bottom while `StaticOutput` writes permanent log lines into scrollback above it.

## `RunConfig` in practice

`RunConfig` is the runtime policy object for all built-in loops.

```rust
use slt::{RunConfig, Theme};
use std::time::Duration;

let config = RunConfig::default()
    .tick_rate(Duration::from_millis(33))
    .mouse(true)
    .theme(Theme::light())
    .max_fps(60)
    .scroll_speed(2)
    .title("My App");
```

Important details:

- `tick_rate` controls how often the loop wakes up even if no input arrives.
- `max_fps` caps the render rate after work is done.
- `mouse(true)` enables clicks, hovers, and wheel input.
- `kitty_keyboard(true)` requests richer key events on supported terminals.
- `RunConfig` is `#[non_exhaustive]`, so use the builder methods instead of struct literals.

## Custom backend model

The low-level contract is intentionally small.

```rust
pub trait Backend {
    fn size(&self) -> (u32, u32);
    fn buffer_mut(&mut self) -> &mut Buffer;
    fn flush(&mut self) -> std::io::Result<()>;
}
```

SLT renders into your `Buffer`. Your backend decides how to present it: terminal, WASM DOM, texture, SSH stream, test harness, or something else.

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
        let keep_going = slt::frame(&mut backend, &mut app, &config, &events, &mut |ui: &mut Context| {
            ui.text("Hello from a custom backend");
        })?;

        if !keep_going {
            break;
        }
    }

    Ok(())
}
```

### What `AppState` actually stores

`AppState` is the persistent frame-to-frame session state for:

- hook storage (`use_state`, `use_memo`)
- focus position and focus counts
- previous-frame hit areas and scroll bounds
- toast queue and debug overlay state
- smoothed FPS estimate and tick counter

Do not recreate it every frame. Reuse one instance for the whole session.

### What `frame()` expects from you

- Reuse the same `AppState` across frames.
- Pass the current frame's `events` slice.
- Rebuild the event list each frame in your outer loop.
- Stop when `frame()` returns `Ok(false)` after `ui.quit()`.

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

## Related APIs

- `docs/FEATURES.md` - feature-gated runtime behavior
- `docs/DEBUGGING.md` - F12 overlay, one-frame delay, layout debugging
- `docs/PATTERNS.md` - hooks, overlays, custom widgets
- `src/lib.rs` - canonical rustdoc for `Backend`, `AppState`, `frame()`, `RunConfig`
