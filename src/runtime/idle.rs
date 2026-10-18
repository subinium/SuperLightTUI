//! Idle frame skipping for the native and async run loops (issue #490).
//!
//! With [`RunConfig::idle_redraw(false)`](RunConfig::idle_redraw) a loop
//! iteration runs the frame pipeline (closure, layout, render, flush) only
//! when something can change what is on screen:
//!
//! - input, resize or focus events, or input carried over from the previous
//!   frame;
//! - an async message, task completion or sender wake (async loop);
//! - a `SIGCONT` resume repaint;
//! - a frame deadline that is due: [`Context::request_frame`],
//!   [`Context::request_frame_after`], a running built-in animation or
//!   time-based widget, a scheduler timer, a pending tooltip delay or hover
//!   timeout;
//! - a follow-up frame: every frame triggered by one of the signals above is
//!   followed by one more frame, and follow-ups continue while the rendered
//!   output keeps changing. This settles state SLT applies one frame late
//!   (focus moves, hover and scroll extents measured on the previous frame).
//!
//! When nothing is pending the loop keeps polling input at `tick_rate`, which
//! costs one `poll(2)` per tick, but builds no frame and writes no bytes.

#[cfg(feature = "crossterm")]
use std::hash::{Hash, Hasher};

#[cfg(feature = "crossterm")]
use crate::Buffer;
use crate::clock::Instant;

use super::FrameState;
#[cfg(feature = "crossterm")]
use super::RunConfig;

/// Idle-skip bookkeeping carried between frames inside [`FrameState`].
#[derive(Default)]
#[cfg_attr(not(feature = "crossterm"), allow(dead_code))]
pub(crate) struct IdleState {
    /// At least one frame has been rendered this session.
    rendered: bool,
    /// The frame being rendered was triggered by a signal, not a follow-up.
    signalled: bool,
    /// The next iteration must render a follow-up frame.
    follow_up: bool,
    /// A signal raised outside the frame (resize, clipboard status) that the
    /// next iteration must render.
    forced: bool,
    /// The last frame asked for the next tick ([`Context::request_frame`]).
    next_tick: bool,
    /// Earliest future instant the last frame asked to be redrawn at.
    deadline: Option<Instant>,
    /// Output fingerprint of the last rendered frame.
    fingerprint: Option<u64>,
    /// Start of the last rendered frame; the `max_fps` cap measures from it.
    last_render: Option<Instant>,
}

#[cfg(feature = "crossterm")]
impl IdleState {
    /// Request a render on the next iteration (resize, clipboard status).
    pub(crate) fn force(&mut self) {
        self.forced = true;
    }
}

/// Signals observed by a run loop since its last iteration.
#[cfg(feature = "crossterm")]
#[derive(Clone, Copy, Default)]
pub(crate) struct LoopSignals {
    /// Input events are waiting for the next frame.
    pub input: bool,
    /// A `SIGCONT` resume repainted the screen.
    pub resumed: bool,
    /// Async messages, a task completion or a sender wake arrived.
    pub woken: bool,
}

/// Decide whether this loop iteration renders a frame, and mark the frame
/// as signalled or follow-up when it does. Always `true` with the default
/// `idle_redraw(true)`.
#[cfg(feature = "crossterm")]
pub(crate) fn should_render(
    state: &mut FrameState,
    config: &RunConfig,
    size: (u32, u32),
    signals: LoopSignals,
    now: Instant,
) -> bool {
    if config.idle_redraw {
        return true;
    }
    let idle = &state.idle;
    let signalled = !idle.rendered
        || idle.forced
        || signals.input
        || signals.resumed
        || signals.woken
        || !state.pending_input.is_empty()
        || state.layout_feedback.size != Some(size)
        || idle.next_tick
        || idle.deadline.is_some_and(|at| at <= now);
    let render = signalled || idle.follow_up;
    if render {
        let idle = &mut state.idle;
        idle.rendered = true;
        idle.signalled = signalled;
        idle.forced = false;
        idle.follow_up = false;
        idle.next_tick = false;
        idle.deadline = None;
        idle.last_render = Some(now);
    }
    render
}

/// Elapsed time the `max_fps` cap measures. With idle skipping it counts from
/// the last rendered frame, so a skipped iteration does not delay the frame
/// that answers new input.
#[cfg(feature = "crossterm")]
pub(crate) fn idle_fps_elapsed(
    state: &FrameState,
    config: &RunConfig,
    iteration_start: Instant,
) -> std::time::Duration {
    let since = if config.idle_redraw {
        iteration_start
    } else {
        state.idle.last_render.unwrap_or(iteration_start)
    };
    since.elapsed()
}

/// How long the native loops may wait for input: the tick rate, shortened to
/// a frame deadline that falls inside it.
#[cfg(feature = "crossterm")]
pub(crate) fn idle_poll_wait(
    state: &FrameState,
    config: &RunConfig,
    tick_wait: std::time::Duration,
) -> std::time::Duration {
    if config.idle_redraw || state.idle.follow_up || state.idle.next_tick {
        return tick_wait;
    }
    match state.idle.deadline {
        Some(at) => tick_wait.min(at.saturating_duration_since(Instant::now())),
        None => tick_wait,
    }
}

/// Whether a delayed frame request or a forced repaint is due now, so the
/// async wait loop stops before the tick ends.
#[cfg(all(feature = "crossterm", feature = "async"))]
pub(crate) fn frame_due(state: &FrameState, config: &RunConfig, now: Instant) -> bool {
    !config.idle_redraw && (state.idle.forced || state.idle.deadline.is_some_and(|at| at <= now))
}

/// Record the next frame deadline at the end of a frame (frame kernel).
///
/// `requested` is the earliest deadline set during the frame through
/// [`Context::request_frame`] / [`Context::request_frame_after`] (which
/// built-in animations and time-based widgets call); `now` is the frame
/// instant. Deadlines owned by runtime state (scheduler timers, tooltip
/// delay, hover timeout, toast notifications, chord timeout, selection
/// auto-scroll, a resolvable `focus_by_name`) are added here.
pub(crate) fn schedule_wake(state: &mut FrameState, requested: Option<Instant>, now: Instant) {
    let mut next_tick = false;
    let mut deadline: Option<Instant> = None;
    let mut at = |instant: Instant| {
        if instant <= now {
            next_tick = true;
        } else {
            deadline = Some(deadline.map_or(instant, |current| current.min(instant)));
        }
    };
    if let Some(requested) = requested {
        at(requested);
    }

    // Tick-driven state: expiry is counted in rendered frames, so keep
    // rendering at the tick rate while it is active.
    if !state.diagnostics.notification_queue.is_empty()
        || !state.chord_states.pending.is_empty()
        || state.pointer.selection_autoscroll.is_some()
    {
        at(now);
    }
    if let Some(name) = state.focus.pending_focus_name.as_deref()
        && state.focus.focus_name_map_prev.contains_key(name)
    {
        at(now);
    }

    // Wall-clock scheduler timers: wake at the next deadline.
    for slot in state
        .scheduler
        .named
        .values()
        .chain(state.scheduler.keyed.values())
    {
        if let Some(deadline) = scheduler_deadline(slot) {
            at(deadline);
        }
    }

    // #452: a hovered widget's tooltip appears after the configured delay,
    // and a stale hover clears after the hover timeout, without new input.
    let pointer = &state.pointer;
    if let Some((Some(_), since)) = pointer.hover_tracked
        && !pointer.tooltip_delay.is_zero()
        && let Some(deadline) = since.checked_add(pointer.tooltip_delay)
        && deadline > now
    {
        at(deadline);
    }
    if let (Some(timeout), Some(last), Some(_)) = (
        pointer.hover_timeout,
        pointer.last_motion,
        state.layout_feedback.last_mouse_pos,
    ) && let Some(deadline) = last.checked_add(timeout)
    {
        at(deadline);
    }

    state.idle.next_tick = next_tick;
    state.idle.deadline = deadline;
}

fn scheduler_deadline(slot: &crate::widgets::SchedulerSlot) -> Option<Instant> {
    use crate::widgets::SchedKind;
    match slot.kind {
        SchedKind::Once { dur, fired: false } => slot.started.checked_add(dur),
        SchedKind::Every { interval, last } => last.checked_add(interval),
        SchedKind::Debounce {
            dur,
            quiet_started,
            fired: false,
        } => quiet_started.checked_add(dur),
        _ => None,
    }
}

/// Fingerprint the frame about to be flushed and decide whether the next
/// iteration renders a follow-up (frame kernel, before the backend flush).
///
/// Refreshing the row hashes here is the same work the terminal flush does
/// first; the flush then finds every row already hashed.
#[cfg(feature = "crossterm")]
pub(crate) fn record_output(state: &mut FrameState, buffer: &mut Buffer) {
    buffer.recompute_line_hashes();
    let mut hasher = crate::buffer::Fnv1a::default();
    buffer.area.hash(&mut hasher);
    buffer.cursor_pos().hash(&mut hasher);
    buffer.line_hashes.hash(&mut hasher);
    let fingerprint = hasher.finish();
    let idle = &mut state.idle;
    let changed = idle.fingerprint != Some(fingerprint);
    idle.fingerprint = Some(fingerprint);
    idle.follow_up = idle.signalled || changed;
}

#[cfg(all(test, feature = "crossterm"))]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::{Context, Rect, SpinnerState, Tween};

    fn skip_config() -> RunConfig {
        RunConfig::default().idle_redraw(false)
    }

    fn frame(
        state: &mut FrameState,
        config: &RunConfig,
        now: Instant,
        mut f: impl FnMut(&mut Context),
    ) -> Buffer {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 5));
        state.diagnostics.clock_override = Some(now);
        let _ = super::super::run_frame_kernel(
            &mut buffer,
            state,
            config,
            (40, 5),
            Vec::new(),
            false,
            &mut f,
        );
        buffer
    }

    #[test]
    fn static_frame_requests_nothing() {
        let mut state = FrameState::default();
        frame(&mut state, &skip_config(), Instant::now(), |ui| {
            ui.text("static");
        });
        assert!(!state.idle.next_tick);
        assert_eq!(state.idle.deadline, None);
    }

    #[test]
    fn animated_widgets_request_the_next_tick() {
        let config = skip_config();
        let now = Instant::now();
        let spinner = SpinnerState::dots();
        let mut state = FrameState::default();
        frame(&mut state, &config, now, |ui| {
            ui.spinner(&spinner);
        });
        assert!(state.idle.next_tick, "spinner");

        let mut tween = Tween::new(0.0, 1.0, 3);
        let mut state = FrameState::default();
        frame(&mut state, &config, now, |ui| {
            let _ = tween.value(ui.tick());
        });
        assert!(state.idle.next_tick, "running tween");
        let _ = tween.value(10);
        frame(&mut state, &config, now, |ui| {
            let _ = tween.value(ui.tick());
        });
        assert!(!state.idle.next_tick, "finished tween");

        let mut state = FrameState::default();
        frame(&mut state, &config, now, |ui| {
            ui.notify("saved", crate::ToastLevel::Info);
        });
        assert!(state.idle.next_tick, "notification expiry counts ticks");
    }

    #[test]
    fn delayed_requests_and_timers_set_the_earliest_deadline() {
        let now = Instant::now();
        let mut state = FrameState::default();
        frame(&mut state, &skip_config(), now, |ui| {
            ui.request_frame_after(Duration::from_millis(400));
            let _ = ui.schedule("later", Duration::from_millis(250));
            ui.request_frame_after(Duration::from_secs(5));
        });
        assert!(!state.idle.next_tick);
        assert_eq!(state.idle.deadline, Some(now + Duration::from_millis(250)));
    }

    #[test]
    fn default_config_keeps_no_idle_bookkeeping() {
        let spinner = SpinnerState::dots();
        let mut state = FrameState::default();
        let config = RunConfig::default();
        frame(&mut state, &config, Instant::now(), |ui| {
            ui.spinner(&spinner);
        });
        assert!(!state.idle.next_tick);
        let idle = LoopSignals::default();
        assert!(should_render(
            &mut state,
            &config,
            (40, 5),
            idle,
            Instant::now()
        ));
    }

    #[test]
    fn loop_renders_for_signals_and_follow_ups_only() {
        let config = skip_config();
        let size = (40, 5);
        let now = Instant::now();
        let mut state = FrameState::default();
        let idle = LoopSignals::default();

        // First frame always renders; it is signalled, so one follow-up runs.
        assert!(should_render(&mut state, &config, size, idle, now));
        let mut buffer = frame(&mut state, &config, now, |ui| {
            ui.text("same");
        });
        record_output(&mut state, &mut buffer);
        assert!(should_render(&mut state, &config, size, idle, now));
        let mut buffer = frame(&mut state, &config, now, |ui| {
            ui.text("same");
        });
        record_output(&mut state, &mut buffer);
        // Unchanged follow-up: the loop goes idle.
        assert!(!should_render(&mut state, &config, size, idle, now));

        let input = LoopSignals {
            input: true,
            ..LoopSignals::default()
        };
        assert!(should_render(&mut state, &config, size, input, now));
        state.idle.deadline = Some(now + Duration::from_millis(5));
        assert!(!should_render(&mut state, &config, size, idle, now));
        let later = now + Duration::from_millis(5);
        assert!(should_render(&mut state, &config, size, idle, later));
        // A resized terminal renders even without an event.
        assert!(should_render(&mut state, &config, (41, 5), idle, now));
    }

    #[test]
    fn changed_output_keeps_follow_ups_running() {
        let config = skip_config();
        let now = Instant::now();
        let mut state = FrameState::default();
        let idle = LoopSignals::default();
        assert!(should_render(&mut state, &config, (40, 5), idle, now));
        let mut buffer = frame(&mut state, &config, now, |ui| {
            ui.text("one");
        });
        record_output(&mut state, &mut buffer);
        assert!(should_render(&mut state, &config, (40, 5), idle, now));
        let mut buffer = frame(&mut state, &config, now, |ui| {
            ui.text("two");
        });
        record_output(&mut state, &mut buffer);
        assert!(
            should_render(&mut state, &config, (40, 5), idle, now),
            "output changed on a follow-up frame"
        );
    }
}
