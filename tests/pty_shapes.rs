//! Issue #452: text cursor shape (DECSCUSR) and pointer shape (OSC 22)
//! bytes through the real flush pipeline.
//!
//! Gated behind the dev-only `pty-test` feature.
#![cfg(feature = "pty-test")]

use slt::{Context, CursorShape, EventBuilder, PointerShape, PtyBackend, RunConfig};

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

/// Count DECSCUSR sequences (`CSI n SP q`) in a frame.
fn decscusr(raw: &[u8]) -> Vec<u8> {
    raw.windows(5)
        .filter(|w| w[0] == 0x1b && w[1] == b'[' && w[2].is_ascii_digit() && &w[3..] == b" q")
        .map(|w| w[2] - b'0')
        .collect()
}

#[test]
fn cursor_shape_is_written_once_per_change() {
    let mut pb = PtyBackend::new(20, 2);
    let mut shape: Option<(CursorShape, bool)> = None;
    let mut frame = |pb: &mut PtyBackend, wanted: Option<(CursorShape, bool)>| {
        shape = wanted;
        pb.render(|ui: &mut Context| {
            ui.text("cursor");
            if let Some((s, blink)) = shape {
                ui.set_cursor_shape(s, blink);
            }
        })
        .raw
        .clone()
    };
    let initial = frame(&mut pb, None);
    assert!(
        decscusr(&initial).is_empty(),
        "nothing written without a request"
    );
    let bar = frame(&mut pb, Some((CursorShape::Bar, true)));
    assert_eq!(decscusr(&bar), [5], "blinking bar");
    let same = frame(&mut pb, Some((CursorShape::Bar, true)));
    assert!(
        decscusr(&same).is_empty(),
        "unchanged shape is not rewritten"
    );
    let block = frame(&mut pb, Some((CursorShape::Block, false)));
    assert_eq!(decscusr(&block), [2], "steady block");
    let reset = frame(&mut pb, None);
    assert_eq!(decscusr(&reset), [0], "no request restores the default");
    let idle = frame(&mut pb, None);
    assert!(decscusr(&idle).is_empty());
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
const OSC22_DEFAULT: &str = "\u{1b}]22;default\u{1b}\\";

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
fn pointer_shape_follows_the_hovered_slot_and_changes_only() {
    let mut pb = PtyBackend::new(30, 2).with_run_config(RunConfig::default().pointer_shapes(true));
    pb.render(shaped);
    assert!(!contains(pb.last_raw(), "\u{1b}]22;"));
    pb.render_with_events(EventBuilder::new().mouse_move(1, 0).build(), shaped);
    assert!(contains(pb.last_raw(), OSC22_POINTER), "hovering LINK");
    // Moving within the same slot does not rewrite the shape.
    pb.render_with_events(EventBuilder::new().mouse_move(3, 0).build(), shaped);
    assert!(!contains(pb.last_raw(), "\u{1b}]22;"));
    // A slot without a shape gets the default back.
    pb.render_with_events(EventBuilder::new().mouse_move(12, 0).build(), shaped);
    assert!(contains(pb.last_raw(), OSC22_DEFAULT));
    pb.render_with_events(EventBuilder::new().mouse_move(13, 0).build(), shaped);
    assert!(!contains(pb.last_raw(), "\u{1b}]22;"));
}
