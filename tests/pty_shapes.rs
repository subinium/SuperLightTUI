//! Issue #452: pointer shape (OSC 22) bytes through the real flush pipeline.
//! The text cursor shape (DECSCUSR) is covered by the #468 metadata tests.
//!
//! Gated behind the dev-only `pty-test` feature.
#![cfg(feature = "pty-test")]

use slt::{Context, EventBuilder, PointerShape, PtyBackend, RunConfig};

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

fn shaped(ui: &mut Context) {
    let _ = ui.row(|ui| {
        let _ = ui
            .container()
            .w(10)
            .pointer_shape(PointerShape::Pointer)
            .row(|ui| {
                ui.text("LINK");
            });
        let _ = ui.container().w(10).row(|ui| {
            ui.text("PLAIN");
        });
    });
}

const OSC22_POINTER: &str = "\u{1b}]22;pointer\u{1b}\\";

#[test]
fn pointer_shape_needs_opt_in() {
    let mut pb = PtyBackend::new(30, 2);
    pb.render(shaped);
    pb.render_with_events(EventBuilder::new().mouse_move(1, 0).build(), shaped);
    pb.render(shaped);
    assert!(
        pb.frames_raw().all(|raw| !contains(raw, "\u{1b}]22;")),
        "OSC 22 is never written unless RunConfig::pointer_shapes(true)"
    );
}

#[test]
fn pointer_shape_follows_the_hovered_slot() {
    // `PtyBackend` drives a fresh terminal per frame, so each frame shows the
    // shape that frame asks for; "written only on change" is covered by the
    // metadata emitter unit tests.
    let mut pb = PtyBackend::new(30, 2).with_run_config(RunConfig::default().pointer_shapes(true));
    pb.render(shaped);
    assert!(!contains(pb.last_raw(), "\u{1b}]22;"), "nothing hovered");
    pb.render_with_events(EventBuilder::new().mouse_move(1, 0).build(), shaped);
    assert!(contains(pb.last_raw(), OSC22_POINTER), "hovering LINK");
    pb.render_with_events(EventBuilder::new().mouse_move(12, 0).build(), shaped);
    assert!(
        !contains(pb.last_raw(), OSC22_POINTER),
        "a slot without a shape asks for the default pointer"
    );
}
