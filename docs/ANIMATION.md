# Animation

SLT animations are standalone structs, not Context methods. Each animation computes an `f64` value from tick counts. You pass the computed value to style or layout methods yourself.

```rust
let mut fade = Tween::new(0.0, 1.0, 30).easing(ease_out_quad);
fade.reset(ui.tick());

// Each frame: compute the value, use it however you want
let opacity = fade.value(ui.tick());
ui.text("Hello").fg(Color::Rgb(255, 255, (255.0 * opacity) as u8));
```

## Tick-based model

Animations run on frame ticks, not wall-clock time.

- `ui.tick()` returns a `u64` frame counter that increments every render frame.
- `RunConfig::default().tick_rate(Duration::from_millis(16))` controls the polling interval (default 16ms / ~60fps).
- All animation types use `reset(tick)` to set a start time and `value(tick)` to sample.
- Lower `tick_rate` = smoother animations but more CPU usage.

## Animation types

### Tween

Linear interpolation from A to B over a fixed number of ticks.

```rust
use slt::{Tween, Context, Color};
use slt::anim::ease_out_quad;

let mut tween = Tween::new(0.0, 100.0, 60)
    .easing(ease_out_quad);

slt::run(|ui: &mut Context| {
    if ui.tick() == 0 {
        tween.reset(ui.tick());
    }
    let x = tween.value(ui.tick()) as u16;
    ui.text("sliding").ml(x);
});
```

Key methods:
- `Tween::new(from, to, duration_ticks)` — constructor, linear easing by default
- `.easing(fn)` — set easing function (builder)
- `.on_complete(fn)` — callback when done (builder)
- `.reset(tick)` — start/restart the tween
- `.value(tick) -> f64` — sample current value
- `.is_done() -> bool` — completion check

### Spring

Physics-based damped harmonic oscillator. Unlike Tween, Spring has no fixed duration -- it settles naturally based on stiffness and damping.

```rust
use slt::{Spring, Context};

let mut spring = Spring::new(0.0, 0.2, 0.85);

slt::run(|ui: &mut Context| {
    let hovered = ui.button("Hover me").hovered;
    spring.set_target(if hovered { 10.0 } else { 0.0 });
    spring.tick();

    let offset = spring.value() as u16;
    ui.text("bouncy").ml(offset);
});
```

Key methods:
- `Spring::new(initial, stiffness, damping)` — constructor
  - `stiffness`: acceleration per unit displacement (`0.1`..`0.5`)
  - `damping`: velocity decay per tick, `< 1.0` (`0.8`..`0.95`)
- `.on_settle(fn)` — callback when settled (builder)
- `.set_target(value)` — change the goal position (interactive use)
- `.tick()` — advance simulation by one frame (call once per frame)
- `.value() -> f64` — current position
- `.is_settled() -> bool` — true when velocity and distance are both < 0.01

Spring does not use `reset(tick)`. Call `.tick()` every frame and `.set_target()` to change direction.

### Keyframes

Multi-stop timeline animation, like CSS `@keyframes`. Each segment between stops can use its own easing.

```rust
use slt::anim::{Keyframes, LoopMode, ease_out_quad, ease_in_cubic};

let mut kf = Keyframes::new(90)
    .stop(0.0, 0.0)       // start at 0
    .stop(0.3, 100.0)     // ramp up to 100 at 30%
    .stop(0.7, 100.0)     // hold at 100 until 70%
    .stop(1.0, 40.0)      // ease down to 40
    .segment_easing(0, ease_out_quad)
    .segment_easing(2, ease_in_cubic)
    .loop_mode(LoopMode::PingPong);

kf.reset(ui.tick());
let brightness = kf.value(ui.tick());
```

Key methods:
- `Keyframes::new(duration_ticks)` — constructor
- `.stop(position, value)` — add a stop at normalized position `[0.0, 1.0]`
- `.easing(fn)` — default easing for all segments
- `.segment_easing(index, fn)` — override easing for segment `index` (0 = first-to-second stop)
- `.loop_mode(mode)` — set loop behavior
- `.on_complete(fn)` — callback when done
- `.reset(tick)` — start/restart
- `.value(tick) -> f64` — sample current value
- `.is_done() -> bool` — completion check (always false for looping modes)

### Sequence

Chain multiple tween segments end-to-end into a single timeline.

```rust
use slt::anim::{Sequence, LoopMode, ease_linear, ease_out_quad, ease_in_cubic};

let mut seq = Sequence::new()
    .then(0.0, 100.0, 30, ease_out_quad)   // slide right
    .then(100.0, 100.0, 10, ease_linear)   // pause
    .then(100.0, 0.0, 20, ease_in_cubic)   // slide back
    .loop_mode(LoopMode::Repeat);

seq.reset(ui.tick());
let x = seq.value(ui.tick());
```

Key methods:
- `Sequence::new()` — constructor
- `.then(from, to, duration_ticks, easing)` — append a segment
- `.loop_mode(mode)` — set loop behavior
- `.on_complete(fn)` — callback when done
- `.reset(tick)` — start/restart
- `.value(tick) -> f64` — sample current value
- `.is_done() -> bool` — completion check

### Stagger

Apply the same tween to N items with a fixed delay between each start.

```rust
use slt::anim::{Stagger, LoopMode, ease_out_quad};

let items = vec!["Alpha", "Beta", "Gamma", "Delta"];
let mut stagger = Stagger::new(0.0, 1.0, 20)
    .easing(ease_out_quad)
    .delay(5)
    .items(items.len())
    .loop_mode(LoopMode::Once);

stagger.reset(ui.tick());

for (i, label) in items.iter().enumerate() {
    let opacity = stagger.value(ui.tick(), i);
    let gray = (255.0 * opacity) as u8;
    ui.text(*label).fg(Color::Rgb(gray, gray, gray));
}
```

Key methods:
- `Stagger::new(from, to, duration_ticks)` — constructor
- `.easing(fn)` — easing for each item's tween
- `.delay(ticks)` — ticks between consecutive item starts
- `.items(count)` — set item count (inferred from usage if not set)
- `.loop_mode(mode)` — set loop behavior
- `.on_complete(fn)` — callback when done
- `.reset(tick)` — start/restart
- `.value(tick, item_index) -> f64` — sample value for a specific item
- `.is_done() -> bool` — true if the most recently sampled item finished

## Easing functions

All easing functions have signature `fn(f64) -> f64`, mapping `[0, 1]` to `[0, 1]`.

| Function | Curve | Use case |
|----------|-------|----------|
| `ease_linear` | Constant rate | Default, mechanical motion |
| `ease_in_quad` | Slow start | Accelerating elements |
| `ease_out_quad` | Slow end | Decelerating, natural stops |
| `ease_in_out_quad` | Slow both ends | Smooth transitions |
| `ease_in_cubic` | Slower start | Stronger acceleration |
| `ease_out_cubic` | Slower end | Stronger deceleration |
| `ease_in_out_cubic` | Slower both ends | Emphasis on middle speed |
| `ease_out_elastic` | Overshoot + oscillate | Attention-grabbing, playful |
| `ease_out_bounce` | Bouncing ball | Landing, drop effects |

Helper: `lerp(a, b, t)` — linear interpolation, not clamped. Apply easing to `t` before calling.

```rust
use slt::anim::{lerp, ease_out_quad};

let t = ease_out_quad(0.5);
let value = lerp(0.0, 100.0, t); // ~75.0
```

## LoopMode

Controls what happens when an animation reaches its end.

| Mode | Behavior |
|------|----------|
| `LoopMode::Once` | Play once, hold at final value. `is_done()` returns `true`. |
| `LoopMode::Repeat` | Restart from the beginning each cycle. |
| `LoopMode::PingPong` | Alternate forward and backward each cycle. |

Applies to `Keyframes`, `Sequence`, and `Stagger`. `Tween` always plays once (use `Sequence` with `LoopMode::Repeat` for looping tweens). `Spring` has no loop mode -- it settles naturally.

## Common patterns

### Fade-in on mount

```rust
let mut fade = Tween::new(0.0, 1.0, 30).easing(ease_out_quad);
let mut started = false;

slt::run(|ui: &mut Context| {
    if !started {
        fade.reset(ui.tick());
        started = true;
    }
    let alpha = fade.value(ui.tick());
    let g = (255.0 * alpha) as u8;
    ui.text("Welcome").fg(Color::Rgb(g, g, g));
});
```

### Button hover spring

```rust
let mut spring = Spring::new(0.0, 0.3, 0.8);

slt::run(|ui: &mut Context| {
    let btn = ui.button("Click me");
    spring.set_target(if btn.hovered { 2.0 } else { 0.0 });
    spring.tick();

    let pad = spring.value() as u16;
    ui.text(">>").ml(pad);
});
```

### Staggered list entry

```rust
let items = vec!["One", "Two", "Three", "Four", "Five"];
let mut stagger = Stagger::new(0.0, 1.0, 15)
    .easing(ease_out_cubic)
    .delay(3)
    .items(items.len());

let mut started = false;

slt::run(|ui: &mut Context| {
    if !started {
        stagger.reset(ui.tick());
        started = true;
    }
    for (i, item) in items.iter().enumerate() {
        let t = stagger.value(ui.tick(), i);
        let brightness = (255.0 * t) as u8;
        ui.text(*item).fg(Color::Rgb(brightness, brightness, brightness));
    }
});
```

### Loading sequence (multi-phase)

```rust
let mut loading = Keyframes::new(120)
    .stop(0.0, 0.0)     // idle
    .stop(0.25, 100.0)  // fill bar
    .stop(0.5, 100.0)   // hold
    .stop(0.75, 0.0)    // reset
    .stop(1.0, 0.0)     // idle
    .easing(ease_in_out_quad)
    .loop_mode(LoopMode::Repeat);

loading.reset(ui.tick());
let pct = loading.value(ui.tick());
ui.text(format!("[{:>3.0}%]", pct));
```

### Chained transitions

```rust
let mut chain = Sequence::new()
    .then(0.0, 50.0, 20, ease_out_quad)    // move to center
    .then(50.0, 50.0, 30, ease_linear)     // hold
    .then(50.0, 100.0, 20, ease_in_cubic); // move to end

chain.reset(ui.tick());
let pos = chain.value(ui.tick()) as u16;
ui.text("->").ml(pos);
```

## API quick reference

### Tween

| | |
|---|---|
| Constructor | `Tween::new(from, to, duration_ticks)` |
| Builder | `.easing(fn)`, `.on_complete(fn)` |
| Control | `.reset(tick)` |
| Sample | `.value(tick) -> f64` |
| Done | `.is_done() -> bool` |

### Spring

| | |
|---|---|
| Constructor | `Spring::new(initial, stiffness, damping)` |
| Builder | `.on_settle(fn)` |
| Control | `.set_target(value)`, `.tick()` |
| Sample | `.value() -> f64` |
| Done | `.is_settled() -> bool` |

### Keyframes

| | |
|---|---|
| Constructor | `Keyframes::new(duration_ticks)` |
| Builder | `.stop(pos, val)`, `.easing(fn)`, `.segment_easing(idx, fn)`, `.loop_mode(mode)`, `.on_complete(fn)` |
| Control | `.reset(tick)` |
| Sample | `.value(tick) -> f64` |
| Done | `.is_done() -> bool` |

### Sequence

| | |
|---|---|
| Constructor | `Sequence::new()` |
| Builder | `.then(from, to, ticks, easing)`, `.loop_mode(mode)`, `.on_complete(fn)` |
| Control | `.reset(tick)` |
| Sample | `.value(tick) -> f64` |
| Done | `.is_done() -> bool` |

### Stagger

| | |
|---|---|
| Constructor | `Stagger::new(from, to, duration_ticks)` |
| Builder | `.easing(fn)`, `.delay(ticks)`, `.items(count)`, `.loop_mode(mode)`, `.on_complete(fn)` |
| Control | `.reset(tick)` |
| Sample | `.value(tick, item_index) -> f64` |
| Done | `.is_done() -> bool` |

## Related docs

- [PATTERNS.md](PATTERNS.md) -- common composition patterns
- [WIDGETS.md](WIDGETS.md) -- widget catalog and usage
