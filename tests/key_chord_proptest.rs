//! Property tests for cross-frame chord matching (issue #262).
//!
//! Drives `Context::key_chord` one keystroke per frame via `TestBackend` and
//! cross-checks the firing behavior against an independent reference simulator
//! of the documented algorithm (buffer = longest suffix that is a prefix of the
//! target; complete-and-clear on full match).

use proptest::prelude::*;
use slt::TestBackend;

/// Reference model of the matcher: returns the per-frame fire flags for feeding
/// `input` one char at a time against `target`, and asserts the buffer length
/// invariant (never exceeds `target.len()`).
fn reference_fires(input: &[char], target: &[char]) -> Vec<bool> {
    let mut buf: Vec<char> = Vec::new();
    let mut fires = Vec::with_capacity(input.len());
    for &c in input {
        buf.push(c);
        // Longest suffix of `buf` that is a prefix of `target`.
        let mut start = 0;
        while start < buf.len() {
            if buf[start..].iter().zip(target).all(|(b, t)| b == t) {
                break;
            }
            start += 1;
        }
        buf.drain(0..start);
        assert!(
            buf.len() <= target.len(),
            "buffer length must never exceed target length"
        );
        if buf.len() == target.len() {
            fires.push(true);
            buf.clear();
        } else {
            fires.push(false);
        }
    }
    fires
}

fn char_strategy() -> impl Strategy<Value = char> {
    prop_oneof![Just('g'), Just('x'), Just('a'), Just(' ')]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn prop_chord_matches_reference(input in prop::collection::vec(char_strategy(), 0..24)) {
        let target = ['g', 'g'];
        let expected = reference_fires(&input, &target);

        let mut tb = TestBackend::new(20, 3);
        let mut actual = Vec::with_capacity(input.len());
        for &c in &input {
            tb.render_with_events(
                slt::EventBuilder::new().key(c).build(),
                0,
                1,
                |ui| {
                    actual.push(ui.key_chord("gg"));
                    ui.text("x");
                },
            );
        }

        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn prop_chord_fires_iff_contiguous_run_exists(
        input in prop::collection::vec(char_strategy(), 0..24)
    ) {
        // The chord fires at least once iff the input contains the target as a
        // contiguous run somewhere.
        let target = ['g', 'g'];
        let has_run = input.windows(target.len()).any(|w| w == target);

        let mut tb = TestBackend::new(20, 3);
        let mut any_fire = false;
        for &c in &input {
            tb.render_with_events(
                slt::EventBuilder::new().key(c).build(),
                0,
                1,
                |ui| {
                    if ui.key_chord("gg") {
                        any_fire = true;
                    }
                    ui.text("x");
                },
            );
        }

        prop_assert_eq!(any_fire, has_run);
    }
}
