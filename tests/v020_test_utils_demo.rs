//! Snapshot regression for `examples/v020_test_utils.rs`.
//!
//! Pulls the deterministic `render_demo` / `render_step` functions from the
//! example via `#[path]` so the test harness re-uses the exact code that
//! ships in the binary. This guards against silent drift between the demo
//! and its locked-in expected output, and exercises every new v0.20 test-utils
//! API end-to-end.

// `main` is unused when this file is included as a module rather than built
// as a binary — the test harness exercises render_* functions directly.
#[allow(dead_code)]
#[path = "../examples/v020_test_utils.rs"]
mod demo;

use slt::{Color, KeyCode, Style, TestBackend};

#[test]
fn demo_record_frames_captures_three_steps() {
    let mut tb = TestBackend::new(40, 6).record_frames();
    for n in 0..3 {
        tb.render(|ui| demo::render_step(ui, "tick", n));
    }
    assert_eq!(tb.frames().len(), 3);
    tb.frames()[0].assert_contains("step 0: tick");
    tb.frames()[1].assert_contains("count = 1");
    tb.frames()[2].assert_contains("step 2: tick");
}

#[test]
fn demo_sequence_runs_all_steps() {
    let mut tb = TestBackend::new(40, 4).record_frames();
    tb.sequence()
        .tick(|ui| {
            ui.text("ready").fg(Color::Green);
        })
        .key(KeyCode::Tab, |ui| {
            ui.text("after tab").fg(Color::Yellow);
        })
        .type_string("hi", |ui| {
            ui.text("typed: hi").fg(Color::Cyan);
        })
        .run();
    assert_eq!(tb.frames().len(), 3);
    tb.frames()[0].assert_contains("ready");
    tb.frames()[1].assert_contains("after tab");
    tb.frames()[2].assert_contains("typed: hi");
}

#[test]
fn demo_type_string_emits_one_frame_per_char() {
    let mut tb = TestBackend::new(40, 2).record_frames();
    tb.type_string("abc", demo::render_demo);
    assert_eq!(tb.frames().len(), 3);
}

#[test]
fn demo_snapshot_format_is_stable() {
    // The demo's render output must match a byte-for-byte snapshot. This
    // catches accidental format-string drift in the snapshot serializer or
    // the demo body without a full-buffer text comparison.
    let mut tb = TestBackend::new(30, 5);
    tb.render(demo::render_demo);
    let snap = tb.buffer().snapshot_format();
    // First chunk: header in cyan + bold (column 0, row 0).
    assert!(
        snap.starts_with("[fg=cyan,bold]\"v0.20 test-utils showcase"),
        "snapshot prefix drift: {}",
        &snap[..snap.len().min(80)]
    );
    // Determinism check.
    let mut tb2 = TestBackend::new(30, 5);
    tb2.render(demo::render_demo);
    assert_eq!(snap, tb2.buffer().snapshot_format());
}

#[test]
fn demo_negative_assertions_pass_on_clean_render() {
    let mut tb = TestBackend::new(30, 5);
    tb.render(demo::render_demo);

    // (a) assert_not_contains: known-absent token.
    tb.assert_not_contains("crash");
    tb.assert_not_contains("CRITICAL");

    // (b) assert_empty_line: row 5 is past the buffer; rendering only writes
    //     5 rows of content, so the buffer.height-1 row index 4 is content.
    //     Use a smaller render to demo assert_empty_line.
    let mut blank_tb = TestBackend::new(20, 4);
    blank_tb.render(|ui| {
        ui.text("solo");
    });
    // Rows 1, 2, 3 should all be empty.
    blank_tb.assert_empty_line(1);
    blank_tb.assert_empty_line(2);
    blank_tb.assert_empty_line(3);

    // (c) assert_style_at: header is fg=Cyan + bold at (0,0).
    let bold_cyan = Style::new().fg(Color::Cyan).bold();
    tb.assert_style_at(0, 0, bold_cyan);
}

#[test]
#[should_panic(expected = "Buffer unexpectedly contains")]
fn demo_assert_not_contains_panics_with_match() {
    let mut tb = TestBackend::new(30, 5);
    tb.render(demo::render_demo);
    // "error" IS rendered at row 4, so this must panic.
    tb.assert_not_contains("error");
}

#[test]
#[should_panic(expected = "Style mismatch")]
fn demo_assert_style_at_panics_on_wrong_color() {
    let mut tb = TestBackend::new(30, 5);
    tb.render(demo::render_demo);
    // Header is cyan, not red — assertion must panic.
    let wrong = Style::new().fg(Color::Red);
    tb.assert_style_at(0, 0, wrong);
}
