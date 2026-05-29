//! Issue #273 — property test for the opt-in scoped cached region.
//!
//! The central invariant: a `ContainerBuilder::cached(version_key, f)` region
//! is **byte-for-byte identical** to the same subtree rendered inside a plain
//! `.col(f)` container, for ANY sequence of version keys and content — because
//! `cached` always re-runs its body and only records a hit/miss classification.
//! This is the guarantee that keeps the immediate-mode invariant intact and
//! makes the feature safe to land as a pure opt-in.

use proptest::prelude::*;
use slt::TestBackend;

/// Render one frame of "static chrome + a streaming-ish line" through the
/// cached path and the plain path, threading the SAME `TestBackend` across the
/// key sequence so `FrameState` (and the persisted region keys) evolve exactly
/// as they would in a real run. Returns the two snapshots for the final frame.
fn render_pair(keys: &[u64], content: &[String]) -> (String, String) {
    let mut cached = TestBackend::new(60, 12);
    let mut plain = TestBackend::new(60, 12);

    for &key in keys {
        cached.render(|ui| {
            let _ = ui.container().cached(key, |ui| {
                for line in content {
                    ui.text(line.as_str());
                }
            });
            // An uncached, always-changing line below the cached region,
            // mirroring "cache the chrome, not the stream".
            ui.text(format!("token {key}"));
        });
        plain.render(|ui| {
            let _ = ui.container().col(|ui| {
                for line in content {
                    ui.text(line.as_str());
                }
            });
            ui.text(format!("token {key}"));
        });
    }

    (
        cached.buffer().snapshot_format(),
        plain.buffer().snapshot_format(),
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// For a random sequence of version keys (with repeats, which exercise the
    /// hit path) and random static content, the cached render must match the
    /// uncached render on the final frame.
    #[test]
    fn cached_output_matches_uncached_under_random_keys(
        keys in prop::collection::vec(0u64..=5, 1..=12),
        content in prop::collection::vec("[a-z ]{0,20}", 0..=6),
    ) {
        let (cached_snapshot, plain_snapshot) = render_pair(&keys, &content);
        prop_assert_eq!(cached_snapshot, plain_snapshot);
    }
}
