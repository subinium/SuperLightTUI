//! Runtime state that is not a widget — static (scrollback) output and the
//! frame-clock scheduler.
//!
//! Driven by `context/runtime.rs` and the run loops in `src/runtime/`.

/// Accumulated static output lines for [`crate::run_static`].
///
/// Use [`println`](Self::println) to append lines above the dynamic inline TUI.
#[derive(Debug, Clone, Default)]
pub struct StaticOutput {
    lines: Vec<String>,
    new_lines: Vec<String>,
}

impl StaticOutput {
    /// Create an empty static output buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one line of static output.
    pub fn println(&mut self, line: impl Into<String>) {
        let line = line.into();
        self.lines.push(line.clone());
        self.new_lines.push(line);
    }

    /// Return all accumulated static lines.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Drain and return only lines added since the previous drain.
    pub fn drain_new(&mut self) -> Vec<String> {
        std::mem::take(&mut self.new_lines)
    }

    /// Clear all accumulated lines.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.new_lines.clear();
    }
}

// ── Frame-clock scheduler (issue #248) ────────────────────────────────

/// The kind of timer a [`SchedulerSlot`] holds.
///
/// Sampled once per frame against the frame's wall-clock
/// [`std::time::Instant`]. Intentionally **not** keyed on the frame tick:
/// `run_frame_kernel` does not advance `diagnostics.tick`, so a tick-based
/// deadline would never elapse under `TestBackend`. See issue #248.
pub(crate) enum SchedKind {
    /// One-shot timer that fires exactly once after `dur` has elapsed from the
    /// slot's `started` instant.
    Once {
        dur: std::time::Duration,
        fired: bool,
    },
    /// Recurring timer that reports whole `interval`s elapsed since `last`.
    Every {
        interval: std::time::Duration,
        last: crate::clock::Instant,
    },
    /// Debounce timer: rearmed to `quiet_started` on every dirty frame, fires
    /// once when the quiet window `dur` elapses.
    Debounce {
        dur: std::time::Duration,
        quiet_started: crate::clock::Instant,
        fired: bool,
    },
}

/// A single live timer in the [`SchedulerState`] table.
pub(crate) struct SchedulerSlot {
    /// Wall-clock instant the slot was first created. Backs [`Context::elapsed`].
    pub(crate) started: crate::clock::Instant,
    /// The timer behavior for this slot.
    pub(crate) kind: SchedKind,
    /// GC flag: set true every frame the slot is sampled; slots left `false`
    /// at frame end are dropped so abandoned timers do not leak.
    pub(crate) touched_this_frame: bool,
}

/// Persistent timer table backing the frame-clock scheduler (issue #248).
///
/// Round-tripped through the per-frame state exactly like the named-state
/// map: moved into [`Context`](crate::Context) at frame start and moved back
/// at frame end, where untouched slots are garbage-collected. Drives
/// [`Context::schedule`](crate::Context::schedule),
/// [`every`](crate::Context::every), [`debounce`](crate::Context::debounce),
/// [`exclusive`](crate::Context::exclusive), [`cancel`](crate::Context::cancel),
/// and [`elapsed`](crate::Context::elapsed).
///
/// This type is public so it appears in `cargo doc`, but all fields are
/// `pub(crate)`: you never construct or inspect it directly — the `Context`
/// timer methods are the entire API surface.
///
/// # Example
///
/// ```no_run
/// use std::time::Duration;
///
/// slt::run(|ui: &mut slt::Context| {
///     // The scheduler state is managed for you behind the timer methods.
///     if ui.schedule("greet", Duration::from_millis(500)) {
///         ui.text("Half a second has passed.");
///     }
/// })?;
/// # Ok::<_, std::io::Error>(())
/// ```
#[derive(Default)]
pub struct SchedulerState {
    /// `&'static str`-keyed slots (mirrors `named_states`).
    pub(crate) named: std::collections::HashMap<&'static str, SchedulerSlot>,
    /// Runtime-`String`-keyed slots for dynamic ids (mirrors `keyed_states`).
    pub(crate) keyed: std::collections::HashMap<String, SchedulerSlot>,
    /// Exclusive-group claim table: `group -> claim state` (issue #248).
    pub(crate) exclusive: std::collections::HashMap<String, ExclusiveGroup>,
}

/// Maximum stale claim ids retained per exclusive group. This bounds scheduler
/// memory while covering substantially more superseded in-flight work than a
/// UI should launch concurrently.
pub(crate) const EXCLUSIVE_RETIRED_LIMIT: usize = 256;

/// Per-group claim state for [`Context::exclusive`](crate::Context::exclusive)
/// (issue #248). Tracks the current winning id plus ids that were superseded
/// recently enough that in-flight stale work may still re-poll them.
#[derive(Default)]
pub(crate) struct ExclusiveGroup {
    /// The most-recently-claimed id; the only id that returns `true`.
    pub(crate) winner: String,
    /// Recently superseded ids. The bounded window prevents ordinary stale
    /// in-flight work from reclaiming the group without retaining every id for
    /// the lifetime of the application.
    pub(crate) retired: std::collections::HashSet<String>,
    /// FIFO order for bounded eviction from `retired`.
    retired_order: std::collections::VecDeque<String>,
    /// GC flag matching scheduler timer slots.
    touched_this_frame: bool,
}

impl ExclusiveGroup {
    pub(crate) fn claim(&mut self, id: &str) -> bool {
        self.touched_this_frame = true;
        if self.winner == id {
            return true;
        }
        if self.retired.contains(id) {
            return false;
        }

        if !self.winner.is_empty() {
            let old = std::mem::take(&mut self.winner);
            if self.retired.insert(old.clone()) {
                self.retired_order.push_back(old);
            }
            while self.retired_order.len() > EXCLUSIVE_RETIRED_LIMIT {
                if let Some(expired) = self.retired_order.pop_front() {
                    self.retired.remove(&expired);
                }
            }
        }
        self.winner = id.to_owned();
        true
    }
}

/// Pure interval-counting kernel for [`Context::every`](crate::Context::every)
/// (issue #248). Returns how many whole `interval`s fit into `elapsed`,
/// saturating at [`u32::MAX`]. Extracted so the no-drop / no-double-count
/// invariant can be proptested deterministically without real sleeps.
pub(crate) fn intervals_elapsed(
    elapsed: std::time::Duration,
    interval: std::time::Duration,
) -> u32 {
    let nanos = interval.as_nanos().max(1);
    let count = elapsed.as_nanos() / nanos;
    count.min(u32::MAX as u128) as u32
}

impl SchedulerState {
    /// Drop every slot that was not sampled this frame, then reset the
    /// per-frame `touched` flag on the survivors. Called at frame end from
    /// `run_frame_kernel`, mirroring the `named_states` writeback lifecycle.
    pub(crate) fn gc_untouched(&mut self) {
        self.named.retain(|_, slot| slot.touched_this_frame);
        self.keyed.retain(|_, slot| slot.touched_this_frame);
        self.exclusive.retain(|_, group| group.touched_this_frame);
        for slot in self.named.values_mut() {
            slot.touched_this_frame = false;
        }
        for slot in self.keyed.values_mut() {
            slot.touched_this_frame = false;
        }
        for group in self.exclusive.values_mut() {
            group.touched_this_frame = false;
        }
    }

    /// Total number of live timer slots (named + keyed). Test-only accessor
    /// used to assert GC of abandoned timers (issue #248).
    #[cfg(test)]
    pub(crate) fn slot_count(&self) -> usize {
        self.named.len() + self.keyed.len()
    }
}

#[cfg(test)]
mod exclusive_group_tests {
    use super::*;

    #[test]
    fn exclusive_history_stays_bounded_under_thousands_of_claims() {
        let mut group = ExclusiveGroup::default();
        for id in 0..10_000 {
            assert!(group.claim(&format!("claim-{id}")));
            assert!(group.retired.len() <= EXCLUSIVE_RETIRED_LIMIT);
            assert!(group.retired_order.len() <= EXCLUSIVE_RETIRED_LIMIT);
        }

        assert_eq!(group.winner, "claim-9999");
        assert!(group.claim("claim-9999"));
        assert!(!group.claim("claim-9998"));
    }

    #[test]
    fn untouched_exclusive_groups_are_garbage_collected() {
        let mut scheduler = SchedulerState::default();
        scheduler
            .exclusive
            .entry("search".to_owned())
            .or_default()
            .claim("first");

        scheduler.gc_untouched();
        assert_eq!(scheduler.exclusive.len(), 1);
        scheduler.gc_untouched();
        assert!(scheduler.exclusive.is_empty());
    }
}
