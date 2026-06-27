//! Tests for the zero-dep frame-clock scheduler (issue #248).
//!
//! Time is advanced with short real sleeps because `run_frame_kernel` does not
//! advance `diagnostics.tick`, so the scheduler is deliberately wall-clock
//! driven (an [`Instant`](std::time::Instant) sampled once at frame start). The
//! pure interval arithmetic is proptested separately, without sleeps, via
//! [`crate::widgets::intervals_elapsed`].

use crate::EventBuilder;
use crate::test_utils::TestBackend;
use std::time::Duration;

fn sleep(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

#[test]
fn schedule_fires_once_after_deadline() {
    let mut tb = TestBackend::new(20, 3);
    let dur = Duration::from_millis(20);

    // Frame 1: arm the timer; not yet fired.
    let mut fired = false;
    tb.render(|ui| {
        fired = ui.schedule("t", dur);
    });
    assert!(!fired, "must not fire on the arming frame");

    sleep(30);

    // Frame 2: deadline passed -> fires exactly once.
    tb.render(|ui| {
        fired = ui.schedule("t", dur);
        if fired {
            ui.text("done");
        }
    });
    assert!(fired, "must fire on the first frame after the deadline");
    tb.assert_contains("done");

    // Frame 3: one-shot -> never fires again.
    tb.render(|ui| {
        fired = ui.schedule("t", dur);
    });
    assert!(!fired, "one-shot must not fire a second time");
}

#[test]
fn schedule_not_fired_before_deadline() {
    let mut tb = TestBackend::new(20, 3);
    let dur = Duration::from_secs(10); // far in the future

    let mut fired_any = false;
    for _ in 0..3 {
        tb.render(|ui| {
            if ui.schedule("t", dur) {
                fired_any = true;
            }
        });
    }
    assert!(!fired_any, "must never fire before the deadline elapses");
}

#[test]
fn cancel_rearms() {
    let mut tb = TestBackend::new(20, 3);
    let dur = Duration::from_millis(15);

    tb.render(|ui| {
        ui.schedule("t", dur);
    });
    sleep(25);
    let mut fired = false;
    tb.render(|ui| {
        fired = ui.schedule("t", dur);
    });
    assert!(fired, "first arming should fire");

    // Cancel, then re-arm on a fresh schedule.
    tb.render(|ui| {
        ui.cancel("t");
        // Re-arm in the same frame: a fresh slot with a future deadline.
        assert!(!ui.schedule("t", dur), "re-armed timer is not yet due");
    });
    sleep(25);
    let mut refired = false;
    tb.render(|ui| {
        refired = ui.schedule("t", dur);
    });
    assert!(refired, "cancel then schedule must re-arm and fire again");
}

#[test]
fn every_counts_intervals() {
    let mut tb = TestBackend::new(20, 3);
    let dur = Duration::from_millis(10);

    // Frame 1: establish the baseline `last`.
    tb.render(|ui| {
        assert_eq!(
            ui.every("e", dur),
            0,
            "no full interval yet on arming frame"
        );
    });

    sleep(25);

    // Frame 2: ~25ms -> at least 2 whole 10ms intervals (no dropped ticks).
    let mut count = 0u32;
    tb.render(|ui| {
        count = ui.every("e", dur);
    });
    assert!(
        count >= 2,
        "expected >= 2 intervals after a 25ms gap, got {count}"
    );

    // Frame 3: immediately after -> 0 (no full interval since last frame).
    tb.render(|ui| {
        assert_eq!(ui.every("e", dur), 0, "no new interval immediately after");
    });
}

#[test]
fn debounce_typeahead() {
    let mut tb = TestBackend::new(40, 3);
    let dur = Duration::from_millis(30);

    // Type "abc": each keystroke marks the frame dirty, resetting the window.
    for ch in ['a', 'b', 'c'] {
        let events = EventBuilder::new().key(ch).build();
        let mut fired = false;
        tb.run_with_events(events, |ui| {
            fired = ui.debounce("search", dur, /* dirty */ true);
            if fired {
                ui.text("results");
            }
        });
        assert!(!fired, "debounce must not fire while typing");
        // A tiny gap between keystrokes, still under the 30ms quiet window.
        sleep(5);
    }

    // Quiet window elapses -> fires exactly once.
    sleep(40);
    let mut fired = false;
    tb.render(|ui| {
        fired = ui.debounce("search", dur, false);
        if fired {
            ui.text("results");
        }
    });
    assert!(fired, "debounce must fire once after the quiet window");
    tb.assert_contains("results");

    // Subsequent quiet frame -> does not fire again until re-dirtied.
    tb.render(|ui| {
        assert!(
            !ui.debounce("search", dur, false),
            "debounce fires only once per quiet window"
        );
    });
}

#[test]
fn exclusive_cancels_stale() {
    let mut tb = TestBackend::new(20, 3);

    // Frame 1: q1 claims the group.
    tb.render(|ui| {
        assert!(ui.exclusive("search", "q1"), "first claim wins");
    });

    // Frame 2: q2 supersedes; q1 is now stale.
    tb.render(|ui| {
        assert!(
            ui.exclusive("search", "q2"),
            "new claim supersedes and wins"
        );
        assert!(
            !ui.exclusive("search", "q1"),
            "superseded id must be cancelled"
        );
        // q2 re-polled is still the winner.
        assert!(ui.exclusive("search", "q2"), "current winner re-polls true");
    });

    // Frame 3: q1 stays retired permanently.
    tb.render(|ui| {
        assert!(!ui.exclusive("search", "q1"), "retired id never re-wins");
    });
}

#[test]
fn elapsed_reports_wallclock() {
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        ui.schedule("t", Duration::from_secs(10));
    });
    sleep(15);
    let mut elapsed = None;
    tb.render(|ui| {
        // Keep the slot alive this frame so it is not GC'd before we read it.
        ui.schedule("t", Duration::from_secs(10));
        elapsed = ui.elapsed("t");
    });
    let elapsed = elapsed.expect("elapsed should be known for a live timer");
    assert!(
        elapsed >= Duration::from_millis(15),
        "elapsed {elapsed:?} should be >= 15ms"
    );
}

#[test]
fn elapsed_unknown_for_missing_timer() {
    let mut tb = TestBackend::new(20, 3);
    let mut elapsed = Some(Duration::ZERO);
    tb.render(|ui| {
        elapsed = ui.elapsed("never-scheduled");
    });
    assert!(elapsed.is_none(), "unknown id should report None");
}

#[test]
fn gc_drops_untouched_slots() {
    let mut tb = TestBackend::new(20, 3);

    // Arm a timer.
    tb.render(|ui| {
        ui.schedule("t", Duration::from_secs(10));
    });
    assert_eq!(tb.scheduler_slot_count(), 1, "slot persists while sampled");

    // Render frames that never sample "t" -> the slot is GC'd.
    tb.render(|ui| {
        ui.text("nothing scheduled");
    });
    assert_eq!(
        tb.scheduler_slot_count(),
        0,
        "abandoned timer slot must be garbage-collected"
    );
}

#[test]
fn no_default_features_smoke() {
    // The scheduler must work with the default feature set and without async;
    // this is a plain compile+run smoke test of the public surface.
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        let _ = ui.schedule("a", Duration::from_millis(1));
        let _ = ui.every("b", Duration::from_millis(1));
        let _ = ui.debounce("c", Duration::from_millis(1), false);
        let _ = ui.exclusive("g", "x");
        ui.cancel("a");
        let _ = ui.elapsed("b");
    });
}

#[test]
fn huge_durations_do_not_overflow_instant_arithmetic() {
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        assert!(
            !ui.schedule("huge-once", Duration::MAX),
            "a huge one-shot timer should arm without firing immediately"
        );
        assert_eq!(
            ui.every("huge-every", Duration::MAX),
            0,
            "a huge interval should not fire on the arming frame"
        );
        assert!(
            !ui.debounce("huge-debounce", Duration::MAX, false),
            "a huge debounce window should arm without firing immediately"
        );
    });

    tb.render(|ui| {
        assert!(!ui.schedule("huge-once", Duration::MAX));
        assert_eq!(ui.every("huge-every", Duration::MAX), 0);
        assert!(!ui.debounce("huge-debounce", Duration::MAX, true));
    });
}

mod proptests {
    use super::*;
    use crate::widgets::intervals_elapsed;
    use proptest::prelude::*;

    proptest! {
        /// `every` never drops or double-counts intervals: summing the per-frame
        /// counts over an arbitrary sequence of frame gaps equals
        /// `floor(total_elapsed / interval)`.
        #[test]
        fn every_never_drops_or_double_counts(
            interval_us in 1u64..1_000_000u64,
            gaps_us in proptest::collection::vec(0u64..2_000_000u64, 0..40),
        ) {
            let interval = Duration::from_micros(interval_us);

            // Simulate the `every` advance loop deterministically, mirroring the
            // method: maintain `last`, accumulate `now` by each gap, fire
            // `intervals_elapsed(now - last, interval)`, advance `last`.
            let mut last = Duration::ZERO;
            let mut now = Duration::ZERO;
            let mut total_fired: u128 = 0;
            for g in &gaps_us {
                now += Duration::from_micros(*g);
                let fired = intervals_elapsed(now - last, interval);
                total_fired += fired as u128;
                last += interval * fired;
            }

            let expected = now.as_nanos() / interval.as_nanos();
            prop_assert_eq!(total_fired, expected);
        }

        /// `intervals_elapsed` is monotonic and matches integer division.
        #[test]
        fn intervals_elapsed_matches_floor_div(
            elapsed_ns in 0u64..u64::MAX,
            interval_ns in 1u64..u64::MAX,
        ) {
            let got = intervals_elapsed(
                Duration::from_nanos(elapsed_ns),
                Duration::from_nanos(interval_ns),
            );
            let expected = (elapsed_ns / interval_ns).min(u32::MAX as u64) as u32;
            prop_assert_eq!(got, expected);
        }
    }
}
