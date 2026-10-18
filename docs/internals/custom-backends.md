# Custom Backends

<!-- slt-check-default: illustrative fragments require application context -->

The low-level path: custom render targets and external event loops built on
`Backend` + `AppState` + `frame()`. For picking a built-in run loop
(`run`, `run_inline`, `run_static`) and configuring it, see
[Runtime modes](../guide/09-runtime-modes.md).

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

## Driving `frame()` yourself

<!-- slt-check: {"name": "custom_backend_frame_loop", "features": [], "context": "module"} -->
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
- [previous-frame](../guide/02-state.md#the-previous-frame-rule) hit areas and scroll bounds
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

See the [Testing guide](../guide/08-testing.md) for the recommended patterns.

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

## Related docs

- [Runtime modes](../guide/09-runtime-modes.md) — built-in loops and `RunConfig`
- [Testing guide](../guide/08-testing.md) — `TestBackend`, backend contract tests, multi-frame patterns
- [Architecture](architecture.md) — frame lifecycle and module map
- [`src/lib.rs`](../../src/lib.rs) — canonical rustdoc for `Backend`, `AppState`, `frame()`
