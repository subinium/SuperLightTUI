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

    /// Return the interpolated value at the given `tick`.
    ///
    /// Returns `to` immediately if the tween has finished or `duration_ticks`
    /// is zero. Marks the tween as done once `tick >= start_tick + duration_ticks`.
    pub fn value(&mut self, tick: u64) -> f64 {
        if self.done {
            return self.to;
        }

        if self.duration_ticks == 0 {
            self.done = true;
            return self.to;
        }

        let elapsed = tick.wrapping_sub(self.start_tick);
        if elapsed >= self.duration_ticks {
            self.done = true;
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
        }
    }

    /// Set the target value the spring will move toward.
    pub fn set_target(&mut self, target: f64) {
        self.target = target;
    }

    /// Advance the spring simulation by one tick.
    ///
    /// Call this once per frame before reading [`Spring::value`].
    pub fn tick(&mut self) {
        let displacement = self.target - self.value;
        let spring_force = displacement * self.stiffness;
        self.velocity = (self.velocity + spring_force) * self.damping;
        self.value += self.velocity;
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
    fn lerp_interpolates_values() {
        assert_eq!(lerp(0.0, 10.0, 0.0), 0.0);
        assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
        assert_eq!(lerp(0.0, 10.0, 1.0), 10.0);
    }
}
