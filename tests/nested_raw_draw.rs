//! Tests for the kitty clip info stack refactor (push/pop instead of a single
//! `Option`). Ensures that multiple raw-draw regions in one frame, and
//! successive frames, behave correctly without stale scroll-clip state
//! leaking between callbacks.
//!
//! The stack push/pop helpers themselves are `pub(crate)`, so the direct
//! nesting invariant is covered by unit tests inside `src/buffer.rs`. The
//! public raw-draw closure receives `&mut Buffer` (not `&mut Context`), so
//! nesting another raw-draw region from inside a draw callback is not
//! expressible via the public API — this file therefore covers the
//! externally observable consequences of the refactor:
//!
//! 1. Multiple sequential raw-draw regions in a single frame each render
//!    into their own rect without clobbering each other.
//! 2. State is clean across frames (the `debug_assert!` in `run_frame_kernel`
//!    enforces the stack is empty at end of frame; this exercises that path).

use slt::{Style, TestBackend};

#[test]
fn multiple_raw_draws_in_one_frame_do_not_clobber() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        let _ = ui.col(|ui| {
            ui.container().w(10).h(1).draw(|buf, rect| {
                buf.set_string(rect.x, rect.y, "first", Style::new());
            });
            ui.container().w(10).h(1).draw(|buf, rect| {
                buf.set_string(rect.x, rect.y, "second", Style::new());
            });
            ui.container().w(10).h(1).draw(|buf, rect| {
                buf.set_string(rect.x, rect.y, "third", Style::new());
            });
        });
    });
    tb.assert_contains("first");
    tb.assert_contains("second");
    tb.assert_contains("third");
}

#[test]
fn raw_draw_state_clean_across_frames() {
    let mut tb = TestBackend::new(40, 5);

    // Frame 1: single raw draw.
    tb.render(|ui| {
        ui.container().w(5).h(1).draw(|buf, rect| {
            buf.set_string(rect.x, rect.y, "aaaaa", Style::new());
        });
    });
    tb.assert_contains("aaaaa");

    // Frame 2: different raw draw. If the stack leaked across frames the
    // debug_assert! in run_frame_kernel would fire.
    tb.render(|ui| {
        ui.container().w(5).h(1).draw(|buf, rect| {
            buf.set_string(rect.x, rect.y, "bbbbb", Style::new());
        });
    });
    tb.assert_contains("bbbbb");

    // Frame 3: no raw draws at all — confirms no residue.
    tb.render(|ui| {
        ui.text("plain");
    });
    tb.assert_contains("plain");
}

#[test]
fn frame_with_no_raw_draws_has_empty_clip_stack() {
    // Sanity: a frame that never touches raw-draw paths must still have an
    // empty stack (it was empty to begin with). This guards against a future
    // regression where push_kitty_clip is called outside the intended path.
    let mut tb = TestBackend::new(20, 5);
    tb.render(|ui| {
        ui.text("hello");
    });
    // The public `Buffer::kitty_clip_info_stack` field is `pub(crate)` and
    // hidden from integration tests — but the debug_assert inside
    // run_frame_kernel would have fired if the stack were non-empty.
    assert_eq!(tb.line(0), "hello");
}
