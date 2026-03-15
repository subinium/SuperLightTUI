//! Animation primitives: tweens, springs, keyframes, sequences, and staggers.
//!
//! All animations are tick-based — call the `value()` method each frame with
//! the current [`Context::tick`](crate::Context::tick) to advance. No timers
//! or threads involved.

use std::f64::consts::PI;

/// Linear interpolation between `a` and `b` at position `t` (0.0..=1.0).
///
/// Values of `t` outside `[0, 1]` are not clamped; use an easing function
/// first if you need clamping.
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Linear easing: constant rate from 0.0 to 1.0.
pub fn ease_linear(t: f64) -> f64 {
    clamp01(t)
}

/// Quadratic ease-in: slow start, fast end.
pub fn ease_in_quad(t: f64) -> f64 {
    let t = clamp01(t);
    t * t
}

/// Quadratic ease-out: fast start, slow end.
pub fn ease_out_quad(t: f64) -> f64 {
    let t = clamp01(t);
    1.0 - (1.0 - t) * (1.0 - t)
}

/// Quadratic ease-in-out: slow start, fast middle, slow end.
pub fn ease_in_out_quad(t: f64) -> f64 {
    let t = clamp01(t);
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

/// Cubic ease-in: slow start, fast end (stronger than quadratic).
pub fn ease_in_cubic(t: f64) -> f64 {
    let t = clamp01(t);
    t * t * t
}

/// Cubic ease-out: fast start, slow end (stronger than quadratic).
pub fn ease_out_cubic(t: f64) -> f64 {
    let t = clamp01(t);
    1.0 - (1.0 - t).powi(3)
}

/// Cubic ease-in-out: slow start, fast middle, slow end (stronger than quadratic).
pub fn ease_in_out_cubic(t: f64) -> f64 {
    let t = clamp01(t);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// Elastic ease-out: overshoots the target and oscillates before settling.
pub fn ease_out_elastic(t: f64) -> f64 {
    let t = clamp01(t);
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else {
        let c4 = (2.0 * PI) / 3.0;
        2f64.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
    }
}

/// Bounce ease-out: simulates a ball bouncing before coming to rest.
pub fn ease_out_bounce(t: f64) -> f64 {
    let t = clamp01(t);
    let n1 = 7.5625;
    let d1 = 2.75;

    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984_375
    }
}

/// Linear interpolation between two values over a duration, with optional easing.
///
/// A `Tween` advances from `from` to `to` over `duration_ticks` render ticks.
/// Call [`Tween::value`] each frame with the current tick to get the
/// interpolated value. The tween is inactive until [`Tween::reset`] is called
/// with a start tick.
///
/// # Example
///
/// ```
/// use slt::Tween;
/// use slt::anim::ease_out_quad;
///
/// let mut tween = Tween::new(0.0, 100.0, 20).easing(ease_out_quad);
/// tween.reset(0);
///
/// let v = tween.value(10); // roughly halfway, eased
/// assert!(v > 50.0);       // ease-out is faster at the start
/// ```
pub struct Tween {
    from: f64,
    to: f64,
    duration_ticks: u64,
    start_tick: u64,
    easing: fn(f64) -> f64,
    done: bool,
    on_complete: Option<Box<dyn FnMut()>>,
}

impl Tween {
    /// Create a new tween from `from` to `to` over `duration_ticks` ticks.
    ///
    /// Uses linear easing by default. Call [`Tween::easing`] to change it.
    /// The tween starts paused; call [`Tween::reset`] with the current tick
    /// before reading values.
    pub fn new(from: f64, to: f64, duration_ticks: u64) -> Self {
        Self {
            from,
            to,
            duration_ticks,
            start_tick: 0,
            easing: ease_linear,
            done: false,
            on_complete: None,
        }
    }

    /// Set the easing function used to interpolate the value.
    ///
    /// Any function with signature `fn(f64) -> f64` that maps `[0, 1]` to
    /// `[0, 1]` works. The nine built-in options are in this module.
    pub fn easing(mut self, f: fn(f64) -> f64) -> Self {
        self.easing = f;
        self
    }

    /// Register a callback that runs once when the tween completes.
    pub fn on_complete(mut self, f: impl FnMut() + 'static) -> Self {
        self.on_complete = Some(Box::new(f));
        self
    }

    /// Return the interpolated value at the given `tick`.
    ///
    /// Returns `to` immediately if the tween has finished or `duration_ticks`
    /// is zero. Marks the tween as done once `tick >= start_tick + duration_ticks`.
    pub fn value(&mut self, tick: u64) -> f64 {
        let was_done = self.done;
        if self.done {
            return self.to;
        }

        if self.duration_ticks == 0 {
            self.done = true;
            if !was_done && self.done {
                if let Some(cb) = &mut self.on_complete {
                    cb();
                }
            }
            return self.to;
        }

        let elapsed = tick.wrapping_sub(self.start_tick);
        if elapsed >= self.duration_ticks {
            self.done = true;
            if !was_done && self.done {
                if let Some(cb) = &mut self.on_complete {
                    cb();
                }
            }
            return self.to;
        }

        let progress = elapsed as f64 / self.duration_ticks as f64;
        let eased = (self.easing)(clamp01(progress));
        lerp(self.from, self.to, eased)
    }

    /// Returns `true` if the tween has reached its end value.
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// Restart the tween, treating `tick` as the new start time.
    pub fn reset(&mut self, tick: u64) {
        self.start_tick = tick;
        self.done = false;
    }
}

/// Defines how an animation behaves after reaching its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMode {
    /// Play once, then stay at the final value.
    Once,
    /// Restart from the beginning each cycle.
    Repeat,
    /// Alternate forward and backward each cycle.
    PingPong,
}

#[derive(Clone, Copy)]
struct KeyframeStop {
    position: f64,
    value: f64,
}

/// Multi-stop keyframe animation over a fixed tick duration.
///
/// `Keyframes` is similar to CSS `@keyframes`: define multiple stops in the
/// normalized `[0.0, 1.0]` timeline, then sample the value with
/// [`Keyframes::value`] using the current render tick.
///
/// Stops are sorted by position when sampled. Each segment between adjacent
/// stops can use its own easing function.
///
/// # Example
///
/// ```
/// use slt::anim::{ease_in_cubic, ease_out_quad, Keyframes, LoopMode};
///
/// let mut keyframes = Keyframes::new(60)
///     .stop(0.0, 0.0)
///     .stop(0.5, 100.0)
///     .stop(1.0, 40.0)
///     .segment_easing(0, ease_out_quad)
///     .segment_easing(1, ease_in_cubic)
///     .loop_mode(LoopMode::PingPong);
///
/// keyframes.reset(10);
/// let _ = keyframes.value(40);
/// ```
pub struct Keyframes {
    duration_ticks: u64,
    start_tick: u64,
    stops: Vec<KeyframeStop>,
    default_easing: fn(f64) -> f64,
    segment_easing: Vec<fn(f64) -> f64>,
    loop_mode: LoopMode,
    done: bool,
    on_complete: Option<Box<dyn FnMut()>>,
}

impl Keyframes {
    /// Create a new keyframe animation with total `duration_ticks`.
    ///
    /// Uses linear easing by default and [`LoopMode::Once`]. Add stops with
    /// [`Keyframes::stop`], optionally configure easing, then call
    /// [`Keyframes::reset`] before sampling.
    pub fn new(duration_ticks: u64) -> Self {
        Self {
            duration_ticks,
            start_tick: 0,
            stops: Vec::new(),
            default_easing: ease_linear,
            segment_easing: Vec::new(),
            loop_mode: LoopMode::Once,
            done: false,
            on_complete: None,
        }
    }

    /// Add a keyframe stop at normalized `position` with `value`.
    ///
    /// `position` is clamped to `[0.0, 1.0]`.
    pub fn stop(mut self, position: f64, value: f64) -> Self {
        self.stops.push(KeyframeStop {
            position: clamp01(position),
            value,
        });
        if self.stops.len() >= 2 {
            self.segment_easing.push(self.default_easing);
        }
        self.stops.sort_by(|a, b| a.position.total_cmp(&b.position));
        self
    }

    /// Set the default easing used for segments without explicit overrides.
    ///
    /// Existing segments are updated to this easing, unless you later override
    /// them with [`Keyframes::segment_easing`].
    pub fn easing(mut self, f: fn(f64) -> f64) -> Self {
        self.default_easing = f;
        self.segment_easing.fill(f);
        self
    }

    /// Override easing for a specific segment index.
    ///
    /// Segment `0` is between the first and second stop, segment `1` between
    /// the second and third, and so on. Out-of-range indices are ignored.
    pub fn segment_easing(mut self, segment_index: usize, f: fn(f64) -> f64) -> Self {
        if let Some(slot) = self.segment_easing.get_mut(segment_index) {
            *slot = f;
        }
        self
    }

    /// Set loop behavior used after the first full pass.
    pub fn loop_mode(mut self, mode: LoopMode) -> Self {
        self.loop_mode = mode;
        self
    }

    /// Register a callback that runs once when the animation completes.
    pub fn on_complete(mut self, f: impl FnMut() + 'static) -> Self {
        self.on_complete = Some(Box::new(f));
        self
    }

    /// Return the interpolated keyframe value at `tick`.
    pub fn value(&mut self, tick: u64) -> f64 {
        let was_done = self.done;
        if self.stops.is_empty() {
            self.done = true;
            if !was_done && self.done {
                if let Some(cb) = &mut self.on_complete {
                    cb();
                }
            }
            return 0.0;
        }
        if self.stops.len() == 1 {
            self.done = true;
            if !was_done && self.done {
                if let Some(cb) = &mut self.on_complete {
                    cb();
                }
            }
            return self.stops[0].value;
        }

        let stops = &self.stops;

        let end_value = stops.last().map_or(0.0, |s| s.value);
        let loop_tick = match map_loop_tick(
            tick,
            self.start_tick,
            self.duration_ticks,
            self.loop_mode,
            &mut self.done,
        ) {
            Some(v) => v,
            None => {
                if !was_done && self.done {
                    if let Some(cb) = &mut self.on_complete {
                        cb();
                    }
                }
                return end_value;
            }
        };

        let progress = loop_tick as f64 / self.duration_ticks as f64;

        if progress <= stops[0].position {
            return stops[0].value;
        }
        if progress >= 1.0 {
            return end_value;
        }

        for i in 0..(stops.len() - 1) {
            let a = stops[i];
            let b = stops[i + 1];
            if progress <= b.position {
                let span = b.position - a.position;
                if span <= f64::EPSILON {
                    return b.value;
                }
                let local = clamp01((progress - a.position) / span);
                let easing = self
                    .segment_easing
                    .get(i)
                    .copied()
                    .unwrap_or(self.default_easing);
                let eased = easing(local);
                return lerp(a.value, b.value, eased);
            }
        }

        end_value
    }

    /// Returns `true` if the animation finished in [`LoopMode::Once`].
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// Restart the keyframe animation from `tick`.
    pub fn reset(&mut self, tick: u64) {
        self.start_tick = tick;
        self.done = false;
    }
}

#[derive(Clone, Copy)]
struct SequenceSegment {
    from: f64,
    to: f64,
    duration_ticks: u64,
    easing: fn(f64) -> f64,
}

/// Sequential timeline that chains multiple animation segments.
///
/// Use [`Sequence::then`] to append segments. Sampling automatically advances
/// through each segment as ticks increase.
///
/// # Example
///
/// ```
/// use slt::anim::{ease_in_cubic, ease_out_quad, LoopMode, Sequence};
///
/// let mut seq = Sequence::new()
///     .then(0.0, 100.0, 30, ease_out_quad)
///     .then(100.0, 50.0, 20, ease_in_cubic)
///     .loop_mode(LoopMode::Repeat);
///
/// seq.reset(0);
/// let _ = seq.value(25);
/// ```
pub struct Sequence {
    segments: Vec<SequenceSegment>,
    loop_mode: LoopMode,
    start_tick: u64,
    done: bool,
    on_complete: Option<Box<dyn FnMut()>>,
}

impl Default for Sequence {
    fn default() -> Self {
        Self::new()
    }
}

impl Sequence {
    /// Create an empty sequence.
    ///
    /// Defaults to [`LoopMode::Once`]. Add segments with [`Sequence::then`]
    /// and call [`Sequence::reset`] before sampling.
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            loop_mode: LoopMode::Once,
            start_tick: 0,
            done: false,
            on_complete: None,
        }
    }

    /// Append a segment from `from` to `to` over `duration_ticks` ticks.
    pub fn then(mut self, from: f64, to: f64, duration_ticks: u64, easing: fn(f64) -> f64) -> Self {
        self.segments.push(SequenceSegment {
            from,
            to,
            duration_ticks,
            easing,
        });
        self
    }

    /// Set loop behavior used after the first full pass.
    pub fn loop_mode(mut self, mode: LoopMode) -> Self {
        self.loop_mode = mode;
        self
    }

    /// Register a callback that runs once when the sequence completes.
    pub fn on_complete(mut self, f: impl FnMut() + 'static) -> Self {
        self.on_complete = Some(Box::new(f));
        self
    }

    /// Return the sequence value at `tick`.
    pub fn value(&mut self, tick: u64) -> f64 {
        let was_done = self.done;
        if self.segments.is_empty() {
            self.done = true;
            if !was_done && self.done {
                if let Some(cb) = &mut self.on_complete {
                    cb();
                }
            }
            return 0.0;
        }

        let total_duration = self
            .segments
            .iter()
            .fold(0_u64, |acc, s| acc.saturating_add(s.duration_ticks));
        let end_value = self.segments.last().map_or(0.0, |s| s.to);

        let loop_tick = match map_loop_tick(
            tick,
            self.start_tick,
            total_duration,
            self.loop_mode,
            &mut self.done,
        ) {
            Some(v) => v,
            None => {
                if !was_done && self.done {
                    if let Some(cb) = &mut self.on_complete {
                        cb();
                    }
                }
                return end_value;
            }
        };

        let mut remaining = loop_tick;
        for segment in &self.segments {
            if segment.duration_ticks == 0 {
                continue;
            }
            if remaining < segment.duration_ticks {
                let progress = remaining as f64 / segment.duration_ticks as f64;
                let eased = (segment.easing)(clamp01(progress));
                return lerp(segment.from, segment.to, eased);
            }
            remaining -= segment.duration_ticks;
        }

        end_value
    }

    /// Returns `true` if the sequence finished in [`LoopMode::Once`].
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// Restart the sequence, treating `tick` as the new start time.
    pub fn reset(&mut self, tick: u64) {
        self.start_tick = tick;
        self.done = false;
    }
}

/// Parallel staggered animation where each item starts after a fixed delay.
///
/// `Stagger` applies one tween configuration to many items. The start tick for
/// each item is `start_tick + delay_ticks * item_index`.
///
/// By default the animation plays once ([`LoopMode::Once`]). Use
/// [`Stagger::loop_mode`] to repeat or ping-pong. The total cycle length
/// includes the delay of every item, so all items finish before the next
/// cycle begins.
///
/// # Example
///
/// ```
/// use slt::anim::{ease_out_quad, Stagger, LoopMode};
///
/// let mut stagger = Stagger::new(0.0, 100.0, 30)
///     .easing(ease_out_quad)
///     .delay(5)
///     .loop_mode(LoopMode::Repeat);
///
/// stagger.reset(100);
/// let _ = stagger.value(120, 3);
/// ```
pub struct Stagger {
    from: f64,
    to: f64,
    duration_ticks: u64,
    start_tick: u64,
    delay_ticks: u64,
    easing: fn(f64) -> f64,
    loop_mode: LoopMode,
    item_count: usize,
    done: bool,
    on_complete: Option<Box<dyn FnMut()>>,
}

impl Stagger {
    /// Create a new stagger animation template.
    ///
    /// Uses linear easing, zero delay, and [`LoopMode::Once`] by default.
    pub fn new(from: f64, to: f64, duration_ticks: u64) -> Self {
        Self {
            from,
            to,
            duration_ticks,
            start_tick: 0,
            delay_ticks: 0,
            easing: ease_linear,
            loop_mode: LoopMode::Once,
            item_count: 0,
            done: false,
            on_complete: None,
        }
    }

    /// Set easing for each item's tween.
    pub fn easing(mut self, f: fn(f64) -> f64) -> Self {
        self.easing = f;
        self
    }

    /// Set delay in ticks between consecutive item starts.
    pub fn delay(mut self, ticks: u64) -> Self {
        self.delay_ticks = ticks;
        self
    }

    /// Set loop behavior. [`LoopMode::Repeat`] restarts after all items
    /// finish; [`LoopMode::PingPong`] reverses direction each cycle.
    pub fn loop_mode(mut self, mode: LoopMode) -> Self {
        self.loop_mode = mode;
        self
    }

    /// Register a callback that runs once when the sampled item completes.
    pub fn on_complete(mut self, f: impl FnMut() + 'static) -> Self {
        self.on_complete = Some(Box::new(f));
        self
    }

    /// Set the number of items for cycle length calculation.
    ///
    /// When using [`LoopMode::Repeat`] or [`LoopMode::PingPong`], the total
    /// cycle length is `duration_ticks + delay_ticks * (item_count - 1)`.
    /// If not set, it is inferred from the highest `item_index` seen.
    pub fn items(mut self, count: usize) -> Self {
        self.item_count = count;
        self
    }

    /// Return the value for `item_index` at `tick`.
    pub fn value(&mut self, tick: u64, item_index: usize) -> f64 {
        let was_done = self.done;
        if item_index >= self.item_count {
            self.item_count = item_index + 1;
        }

        let total_cycle = self.total_cycle_ticks();

        let effective_tick = if self.loop_mode == LoopMode::Once {
            tick
        } else {
            let elapsed = tick.wrapping_sub(self.start_tick);
            let mapped = match self.loop_mode {
                LoopMode::Repeat => {
                    if total_cycle == 0 {
                        0
                    } else {
                        elapsed % total_cycle
                    }
                }
                LoopMode::PingPong => {
                    if total_cycle == 0 {
                        0
                    } else {
                        let full = total_cycle.saturating_mul(2);
                        let phase = elapsed % full;
                        if phase < total_cycle {
                            phase
                        } else {
                            full - phase
                        }
                    }
                }
                LoopMode::Once => unreachable!(),
            };
            self.start_tick.wrapping_add(mapped)
        };

        let delay = self.delay_ticks.wrapping_mul(item_index as u64);
        let item_start = self.start_tick.wrapping_add(delay);

        if effective_tick < item_start {
            self.done = false;
            return self.from;
        }

        if self.duration_ticks == 0 {
            self.done = true;
            if !was_done && self.done {
                if let Some(cb) = &mut self.on_complete {
                    cb();
                }
            }
            return self.to;
        }

        let elapsed = effective_tick - item_start;
        if elapsed >= self.duration_ticks {
            self.done = true;
            if !was_done && self.done {
                if let Some(cb) = &mut self.on_complete {
                    cb();
                }
            }
            return self.to;
        }

        self.done = false;
        let progress = elapsed as f64 / self.duration_ticks as f64;
        let eased = (self.easing)(clamp01(progress));
        lerp(self.from, self.to, eased)
    }

    fn total_cycle_ticks(&self) -> u64 {
        let max_delay = self
            .delay_ticks
            .wrapping_mul(self.item_count.saturating_sub(1) as u64);
        self.duration_ticks.saturating_add(max_delay)
    }

    /// Returns `true` if the most recently sampled item reached its end value.
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// Restart stagger timing, treating `tick` as the base start time.
    pub fn reset(&mut self, tick: u64) {
        self.start_tick = tick;
        self.done = false;
    }
}

fn map_loop_tick(
    tick: u64,
    start_tick: u64,
    duration_ticks: u64,
    loop_mode: LoopMode,
    done: &mut bool,
) -> Option<u64> {
    if duration_ticks == 0 {
        *done = true;
        return None;
    }

    let elapsed = tick.wrapping_sub(start_tick);
    match loop_mode {
        LoopMode::Once => {
            if elapsed >= duration_ticks {
                *done = true;
                None
            } else {
                *done = false;
                Some(elapsed)
            }
        }
        LoopMode::Repeat => {
            *done = false;
            Some(elapsed % duration_ticks)
        }
        LoopMode::PingPong => {
            *done = false;
            let cycle = duration_ticks.saturating_mul(2);
            if cycle == 0 {
                return Some(0);
            }
            let phase = elapsed % cycle;
            if phase < duration_ticks {
                Some(phase)
            } else {
                Some(cycle - phase)
            }
        }
    }
}

/// Spring physics animation that settles toward a target value.
///
/// Models a damped harmonic oscillator. Call [`Spring::set_target`] to change
/// the goal, then call [`Spring::tick`] once per frame to advance the
/// simulation. Read the current position with [`Spring::value`].
///
/// Tune behavior with `stiffness` (how fast it accelerates toward the target)
/// and `damping` (how quickly oscillations decay). A damping value close to
/// 1.0 is overdamped (no oscillation); lower values produce more bounce.
///
/// # Example
///
/// ```
/// use slt::Spring;
///
/// let mut spring = Spring::new(0.0, 0.2, 0.85);
/// spring.set_target(100.0);
///
/// for _ in 0..200 {
///     spring.tick();
///     if spring.is_settled() { break; }
/// }
///
/// assert!((spring.value() - 100.0).abs() < 0.01);
/// ```
pub struct Spring {
    value: f64,
    target: f64,
    velocity: f64,
    stiffness: f64,
    damping: f64,
    settled: bool,
    on_settle: Option<Box<dyn FnMut()>>,
}

impl Spring {
    /// Create a new spring at `initial` position with the given physics parameters.
    ///
    /// - `stiffness`: acceleration per unit of displacement (try `0.1`..`0.5`)
    /// - `damping`: velocity multiplier per tick, `< 1.0` (try `0.8`..`0.95`)
    pub fn new(initial: f64, stiffness: f64, damping: f64) -> Self {
        Self {
            value: initial,
            target: initial,
            velocity: 0.0,
            stiffness,
            damping,
            settled: true,
            on_settle: None,
        }
    }

    /// Register a callback that runs once when the spring settles.
    pub fn on_settle(mut self, f: impl FnMut() + 'static) -> Self {
        self.on_settle = Some(Box::new(f));
        self
    }

    /// Set the target value the spring will move toward.
    pub fn set_target(&mut self, target: f64) {
        self.target = target;
        self.settled = self.is_settled();
    }

    /// Advance the spring simulation by one tick.
    ///
    /// Call this once per frame before reading [`Spring::value`].
    pub fn tick(&mut self) {
        let displacement = self.target - self.value;
        let spring_force = displacement * self.stiffness;
        self.velocity = (self.velocity + spring_force) * self.damping;
        self.value += self.velocity;

        let is_settled = self.is_settled();
        if !self.settled && is_settled {
            self.settled = true;
            if let Some(cb) = &mut self.on_settle {
                cb();
            }
        }
    }

    /// Return the current spring position.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the spring has effectively settled at its target.
    ///
    /// Settled means both the distance to target and the velocity are below
    /// `0.01`.
    pub fn is_settled(&self) -> bool {
        (self.target - self.value).abs() < 0.01 && self.velocity.abs() < 0.01
    }
}

fn clamp01(t: f64) -> f64 {
    t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    fn assert_endpoints(f: fn(f64) -> f64) {
        assert_eq!(f(0.0), 0.0);
        assert_eq!(f(1.0), 1.0);
    }

    #[test]
    fn easing_functions_have_expected_endpoints() {
        let easing_functions: [fn(f64) -> f64; 9] = [
            ease_linear,
            ease_in_quad,
            ease_out_quad,
            ease_in_out_quad,
            ease_in_cubic,
            ease_out_cubic,
            ease_in_out_cubic,
            ease_out_elastic,
            ease_out_bounce,
        ];

        for easing in easing_functions {
            assert_endpoints(easing);
        }
    }

    #[test]
    fn tween_returns_start_middle_end_values() {
        let mut tween = Tween::new(0.0, 10.0, 10);
        tween.reset(100);

        assert_eq!(tween.value(100), 0.0);
        assert_eq!(tween.value(105), 5.0);
        assert_eq!(tween.value(110), 10.0);
        assert!(tween.is_done());
    }

    #[test]
    fn tween_reset_restarts_animation() {
        let mut tween = Tween::new(0.0, 1.0, 10);
        tween.reset(0);
        let _ = tween.value(10);
        assert!(tween.is_done());

        tween.reset(20);
        assert!(!tween.is_done());
        assert_eq!(tween.value(20), 0.0);
        assert_eq!(tween.value(30), 1.0);
        assert!(tween.is_done());
    }

    #[test]
    fn tween_on_complete_fires_once() {
        let count = Rc::new(Cell::new(0));
        let callback_count = Rc::clone(&count);
        let mut tween = Tween::new(0.0, 10.0, 10).on_complete(move || {
            callback_count.set(callback_count.get() + 1);
        });

        tween.reset(0);
        assert_eq!(count.get(), 0);

        assert_eq!(tween.value(5), 5.0);
        assert_eq!(count.get(), 0);

        assert_eq!(tween.value(10), 10.0);
        assert_eq!(count.get(), 1);

        assert_eq!(tween.value(11), 10.0);
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn spring_settles_to_target() {
        let mut spring = Spring::new(0.0, 0.2, 0.85);
        spring.set_target(10.0);

        for _ in 0..300 {
            spring.tick();
            if spring.is_settled() {
                break;
            }
        }

        assert!(spring.is_settled());
        assert!((spring.value() - 10.0).abs() < 0.01);
    }

    #[test]
    fn spring_on_settle_fires_once() {
        let count = Rc::new(Cell::new(0));
        let callback_count = Rc::clone(&count);
        let mut spring = Spring::new(0.0, 0.2, 0.85).on_settle(move || {
            callback_count.set(callback_count.get() + 1);
        });
        spring.set_target(10.0);

        for _ in 0..500 {
            spring.tick();
            if spring.is_settled() {
                break;
            }
        }

        assert!(spring.is_settled());
        assert_eq!(count.get(), 1);

        for _ in 0..50 {
            spring.tick();
        }

        assert_eq!(count.get(), 1);
    }

    #[test]
    fn lerp_interpolates_values() {
        assert_eq!(lerp(0.0, 10.0, 0.0), 0.0);
        assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
        assert_eq!(lerp(0.0, 10.0, 1.0), 10.0);
    }

    #[test]
    fn keyframes_interpolates_across_multiple_stops() {
        let mut keyframes = Keyframes::new(100)
            .stop(0.0, 0.0)
            .stop(0.3, 100.0)
            .stop(0.7, 50.0)
            .stop(1.0, 80.0)
            .easing(ease_linear);

        keyframes.reset(0);
        assert_eq!(keyframes.value(0), 0.0);
        assert_eq!(keyframes.value(15), 50.0);
        assert_eq!(keyframes.value(30), 100.0);
        assert_eq!(keyframes.value(50), 75.0);
        assert_eq!(keyframes.value(70), 50.0);
        assert_eq!(keyframes.value(85), 65.0);
        assert_eq!(keyframes.value(100), 80.0);
        assert!(keyframes.is_done());
    }

    #[test]
    fn keyframes_repeat_loop_restarts() {
        let mut keyframes = Keyframes::new(10)
            .stop(0.0, 0.0)
            .stop(1.0, 10.0)
            .loop_mode(LoopMode::Repeat);

        keyframes.reset(0);
        assert_eq!(keyframes.value(5), 5.0);
        assert_eq!(keyframes.value(10), 0.0);
        assert_eq!(keyframes.value(12), 2.0);
        assert!(!keyframes.is_done());
    }

    #[test]
    fn keyframes_pingpong_reverses_direction() {
        let mut keyframes = Keyframes::new(10)
            .stop(0.0, 0.0)
            .stop(1.0, 10.0)
            .loop_mode(LoopMode::PingPong);

        keyframes.reset(0);
        assert_eq!(keyframes.value(8), 8.0);
        assert_eq!(keyframes.value(10), 10.0);
        assert_eq!(keyframes.value(12), 8.0);
        assert_eq!(keyframes.value(15), 5.0);
        assert!(!keyframes.is_done());
    }

    #[test]
    fn sequence_chains_segments_in_order() {
        let mut sequence = Sequence::new()
            .then(0.0, 100.0, 30, ease_linear)
            .then(100.0, 50.0, 20, ease_linear)
            .then(50.0, 200.0, 40, ease_linear);

        sequence.reset(0);
        assert_eq!(sequence.value(15), 50.0);
        assert_eq!(sequence.value(30), 100.0);
        assert_eq!(sequence.value(40), 75.0);
        assert_eq!(sequence.value(50), 50.0);
        assert_eq!(sequence.value(70), 125.0);
        assert_eq!(sequence.value(90), 200.0);
        assert!(sequence.is_done());
    }

    #[test]
    fn sequence_loop_modes_repeat_and_pingpong_work() {
        let mut repeat = Sequence::new()
            .then(0.0, 10.0, 10, ease_linear)
            .loop_mode(LoopMode::Repeat);
        repeat.reset(0);
        assert_eq!(repeat.value(12), 2.0);
        assert!(!repeat.is_done());

        let mut pingpong = Sequence::new()
            .then(0.0, 10.0, 10, ease_linear)
            .loop_mode(LoopMode::PingPong);
        pingpong.reset(0);
        assert_eq!(pingpong.value(12), 8.0);
        assert!(!pingpong.is_done());
    }

    #[test]
    fn stagger_applies_per_item_delay() {
        let mut stagger = Stagger::new(0.0, 100.0, 20).easing(ease_linear).delay(5);

        stagger.reset(0);
        assert_eq!(stagger.value(4, 3), 0.0);
        assert_eq!(stagger.value(15, 3), 0.0);
        assert_eq!(stagger.value(20, 3), 25.0);
        assert_eq!(stagger.value(35, 3), 100.0);
        assert!(stagger.is_done());
    }
}
