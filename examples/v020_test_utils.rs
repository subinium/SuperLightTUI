//! v0.20.0 test-utils demo — exercises the four new test-harness APIs.
//!
//! Demonstrates: #229 (record_frames), #230 (sequence + type_string),
//! #231 (Buffer::snapshot_format), #232 (negative assertions).
//!
//! Non-interactive (stdout report) — runs deterministically and prints
//! captured frames + a styled snapshot so the demo can be eyeballed without
//! a live terminal. `tests/v020_test_utils_demo.rs` calls `render_demo` and
//! `render_step` directly to pin regression coverage.
//!
//! Run: `cargo run --example v020_test_utils`

use slt::{Color, Context, KeyCode, Style, TestBackend};

// Buffer dimensions for the deterministic showcase. Picked once so the demo
// frames, the snapshot tests, and any future tooling all line up.
const DEMO_W: u32 = 30;
const DEMO_H: u32 = 5;

// Slightly wider buffer for the multi-step `record_frames` and `sequence`
// recordings — keeps the rendered text readable without wrapping.
const STEPS_W: u32 = 40;
const STEPS_H: u32 = 6;

/// Render the static layout used by the demo.
///
/// Single source of truth — `main()` and the snapshot tests both call this
/// so the example and its regression coverage cannot drift.
pub fn render_demo(ui: &mut Context) {
    let _ = ui.col(|ui| {
        ui.text("v0.20 test-utils showcase").fg(Color::Cyan).bold();
        ui.text("--------------------------").fg(Color::Cyan);
        ui.text("normal").fg(Color::White);
        ui.text("warning").fg(Color::Yellow).italic();
        ui.text("error").fg(Color::Red).bold();
    });
}

/// Render a single animation step — used to demo `record_frames`.
pub fn render_step(ui: &mut Context, label: &str, n: usize) {
    let _ = ui.col(|ui| {
        ui.text(format!("step {n}: {label}")).bold();
        ui.text("---------------------").fg(Color::DarkGray);
        ui.text(format!("count = {n}")).fg(Color::Green);
    });
}

fn main() {
    println!("=== SLT v0.20.0 test-utils demo ===");
    println!();

    demo_record_frames();
    println!();

    demo_sequence_and_type_string();
    println!();

    demo_snapshot_format();
    println!();

    demo_negative_assertions();
}

// #229 — record_frames captures a `FrameRecord` per render so tests can
// inspect the entire animation history rather than a single end-state.
fn demo_record_frames() {
    let mut tb = TestBackend::new(STEPS_W, STEPS_H).record_frames();
    for n in 0..3 {
        tb.render(|ui| render_step(ui, "tick", n));
    }

    println!("== #229 record_frames ==");
    println!("captured {} frames", tb.frames().len());
    for (i, frame) in tb.frames().iter().enumerate() {
        println!("--- frame {i} ---");
        println!("{}", frame.to_string_trimmed());
    }
}

// #230 — `sequence` chains tick/key/type_string steps and threads frame
// state automatically; `type_string` at the backend level fires one render
// per character so each typed key is observable.
fn demo_sequence_and_type_string() {
    let mut seq_tb = TestBackend::new(STEPS_W, 4).record_frames();
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

    println!("== #230 sequence + type_string ==");
    println!("frames captured by sequence(): {}", seq_tb.frames().len());
    seq_tb.assert_contains("typed: hi");

    let mut typing_tb = TestBackend::new(STEPS_W, 2).record_frames();
    typing_tb.type_string("abc", render_demo);
    println!(
        "frames captured by type_string(\"abc\"): {}",
        typing_tb.frames().len()
    );
}

// #231 — `Buffer::snapshot_format` produces a stable styled-snapshot string
// (named colors, hex RGB, canonical modifier order) suitable for `insta`
// regression tests without committing raw escape sequences.
fn demo_snapshot_format() {
    // Truncate the styled-snapshot preview so the demo stays readable; the
    // test harness exercises the full string elsewhere.
    const PREVIEW_BYTES: usize = 500;

    let mut tb = TestBackend::new(DEMO_W, DEMO_H);
    tb.render(render_demo);
    let snapshot = tb.buffer().snapshot_format();

    println!("== #231 Buffer::snapshot_format ==");
    let preview: String = snapshot.chars().take(PREVIEW_BYTES).collect();
    println!("{preview}");
}

// #232 — negative assertions surface "this should be absent" expectations
// directly, instead of forcing tests to scrape `to_string_trimmed()` and
// invert the match by hand.
fn demo_negative_assertions() {
    let mut tb = TestBackend::new(DEMO_W, DEMO_H);
    tb.render(render_demo);
    tb.assert_not_contains("crash");

    // `assert_empty_line` needs a row that is genuinely blank; render a
    // single-line buffer so the trailing rows are guaranteed empty.
    let mut blank_tb = TestBackend::new(20, 3);
    blank_tb.render(|ui| {
        ui.text("only row 0").fg(Color::White);
    });
    blank_tb.assert_empty_line(2);

    let bold_cyan = Style::new().fg(Color::Cyan).bold();
    tb.assert_style_at(0, 0, bold_cyan);

    println!("== #232 negative assertions ==");
    println!("assert_not_contains(\"crash\")  OK");
    println!("assert_empty_line(2) on blank row  OK");
    println!("assert_style_at(0, 0, cyan|bold)  OK");
}
