//! v0.20.0 test-utils showcase — exercises all four new APIs
//! in a single self-contained example.
//!
//! New in v0.20.0:
//!
//! 1. `TestBackend::record_frames()` (#229) — capture a `FrameRecord` per render.
//! 2. `TestBackend::sequence()` + `type_string()` (#230) — multi-step interaction
//!    builders that thread frame state for you.
//! 3. `Buffer::snapshot_format()` (#231) — stable styled-snapshot string for
//!    `insta`-based regression tests (named colors, hex RGB, canonical modifier
//!    order).
//! 4. `assert_not_contains` / `assert_empty_line` / `assert_style_at` (#232) —
//!    negative assertions for sharper test diagnostics.
//!
//! Run with `cargo run --example v020_test_utils`. The demo renders
//! deterministically and prints captured frames + a styled snapshot to stdout
//! so it can be eyeballed without a live terminal.
//!
//! `tests/v020_test_utils_demo.rs` exercises this module under the test
//! harness for snapshot regression coverage.

use slt::{Color, Context, KeyCode, Style, TestBackend};

/// Render the static layout used by the demo.
///
/// Single source of truth — main() and the snapshot test both call this so
/// the example and its regression test cannot drift.
pub fn render_demo(ui: &mut Context) {
    let _ = ui.col(|ui| {
        ui.text("v0.20 test-utils showcase").fg(Color::Cyan).bold();
        ui.text("--------------------------").fg(Color::Cyan);
        ui.text("normal").fg(Color::White);
        ui.text("warning").fg(Color::Yellow).italic();
        ui.text("error").fg(Color::Red).bold();
    });
}

/// Render a single frame of an animation step (used to demo `record_frames`).
pub fn render_step(ui: &mut Context, label: &str, n: usize) {
    let _ = ui.col(|ui| {
        ui.text(format!("step {n}: {label}")).bold();
        ui.text("---------------------").fg(Color::DarkGray);
        ui.text(format!("count = {n}")).fg(Color::Green);
    });
}

fn main() {
    // ------------------------------------------------------------------
    // (1) record_frames — capture a history of every rendered frame.
    // ------------------------------------------------------------------
    let mut tb = TestBackend::new(40, 6).record_frames();
    for n in 0..3 {
        tb.render(|ui| render_step(ui, "tick", n));
    }
    println!("== #229 record_frames ==");
    println!("captured {} frames", tb.frames().len());
    for (i, frame) in tb.frames().iter().enumerate() {
        println!("--- frame {i} ---");
        println!("{}", frame.to_string_trimmed());
    }

    // ------------------------------------------------------------------
    // (2) sequence + type_string — multi-step interaction without manual
    //     focus_index threading.
    // ------------------------------------------------------------------
    let mut seq_tb = TestBackend::new(40, 4).record_frames();
    seq_tb
        .sequence()
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
    println!("\n== #230 sequence + type_string ==");
    println!("frames captured by sequence(): {}", seq_tb.frames().len());
    seq_tb.assert_contains("typed: hi");

    // type_string at the backend level fires one render per character.
    let mut typing_tb = TestBackend::new(40, 2).record_frames();
    typing_tb.type_string("abc", render_demo);
    println!(
        "frames captured by type_string(\"abc\"): {}",
        typing_tb.frames().len()
    );

    // ------------------------------------------------------------------
    // (3) Buffer::snapshot_format — stable styled snapshot string.
    // ------------------------------------------------------------------
    let mut snap_tb = TestBackend::new(30, 5);
    snap_tb.render(render_demo);
    let snapshot = snap_tb.buffer().snapshot_format();
    println!("\n== #231 Buffer::snapshot_format ==");
    // Print only the first 500 bytes of the snapshot so the demo output stays
    // readable; the test harness exercises the full string elsewhere.
    let preview: String = snapshot.chars().take(500).collect();
    println!("{preview}");

    // ------------------------------------------------------------------
    // (4) Negative assertions — assert_not_contains, assert_empty_line,
    //     assert_style_at.
    // ------------------------------------------------------------------
    let mut neg_tb = TestBackend::new(30, 5);
    neg_tb.render(render_demo);

    // assert_not_contains: expectation that a substring is absent.
    neg_tb.assert_not_contains("crash");

    // assert_empty_line: row 5 is past content but inside the buffer? actually
    // the buffer is height=5 so rows 0..=4. Row 4 may be blank — if not we
    // demo on a fresh buffer below.
    let mut blank_tb = TestBackend::new(20, 3);
    blank_tb.render(|ui| {
        ui.text("only row 0").fg(Color::White);
    });
    blank_tb.assert_empty_line(2);

    // assert_style_at: cell (0,0) of render_demo's first text was cyan + bold.
    let bold_cyan = Style::new().fg(Color::Cyan).bold();
    snap_tb.assert_style_at(0, 0, bold_cyan);

    println!("\n== #232 negative assertions ==");
    println!("assert_not_contains(\"crash\")  OK");
    println!("assert_empty_line(2) on blank row  OK");
    println!("assert_style_at(0, 0, cyan|bold)  OK");
}
