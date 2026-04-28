//! v0.20.0 hot-path perf audit — timing breakdown for the four optimized paths.
//!
//! Demonstrates: #204 (FrameState reuse), #205 (wrap_segments alloc),
//! #206 (kitty placement flush), #228 (modal-aware dim_buffer).
//!
//! Non-interactive (stdout report) — runs each hot path `ITERS` times and
//! prints total + per-iter timing. Use after applying the v0.20.0 perf
//! fixes to confirm steady-state behavior.
//!
//! Run: `cargo run --release --example v020_perf_audit`
//!
//! `--release` is required for representative numbers — debug builds add
//! ~10x overhead and obscure the diff between the four paths.

use std::time::Instant;

use slt::buffer::Buffer;
use slt::rect::Rect;
use slt::{Border, Style, TestBackend};

// Iteration count is shared across all four benchmarks so the per-iter
// numbers are directly comparable. 10k strikes a balance between sample
// stability and a sub-second total wall time per benchmark.
const ITERS: u32 = 10_000;

// Steady-state render fixture. 80x24 is the canonical "small terminal"
// reference size used elsewhere in the test suite.
const RENDER_W: u32 = 80;
const RENDER_H: u32 = 24;

// dim_buffer fixture geometry. 200x60 is large enough that the
// modal-aware path materially beats the full-buffer scan.
const DIM_W: u32 = 200;
const DIM_H: u32 = 60;

// Wrap fixture — three styled segments wrapping at 40 columns. Segment
// content is mixed-style so the bench exercises the per-style allocation
// path rather than a single contiguous run.
const WRAP_COLS: u32 = 40;

// Kitty fixture — three stable image placements. Above 1 image we exercise
// the placement-diff fast path; 3 keeps the cost noticeable but bounded.
const KITTY_IMAGES: usize = 3;
const KITTY_ROW_OFFSET: u32 = 5;

fn bench<F: FnMut()>(label: &'static str, iters: u32, mut f: F) {
    let start = Instant::now();
    for _ in 0..iters {
        f();
    }
    let elapsed = start.elapsed();
    let per_iter = elapsed / iters;
    println!(
        "  {:<32} {:>10?} total | {:>9?} / iter",
        label, elapsed, per_iter
    );
}

fn audit_framestate_reuse() {
    println!("[#204] FrameState 6-vec/hashset reuse — {ITERS} frames:");

    let mut tb = TestBackend::new(RENDER_W, RENDER_H);
    bench("steady-state render", ITERS, || {
        tb.render(|ui| {
            let _ = ui.bordered(Border::Rounded).title("audit").col(|ui| {
                ui.text("hello").bold();
                ui.text("world").dim();
            });
        });
    });
}

fn audit_wrap_segments() {
    println!("[#205] wrap_segments String alloc — {ITERS} 3-segment wraps at {WRAP_COLS}w:");

    let segments: Vec<(String, Style)> = vec![
        ("hello world alpha beta".to_string(), Style::new().bold()),
        (" ".to_string(), Style::default()),
        (
            "gamma delta epsilon zeta eta theta".to_string(),
            Style::new().italic(),
        ),
    ];

    bench("wrap to 40 cols", ITERS, || {
        let _ = slt::__bench_wrap_segments(&segments, WRAP_COLS);
    });
}

fn audit_kitty_placement_flush() {
    println!("[#206] kitty placement flush — {ITERS} flushes ({KITTY_IMAGES} stable images):");

    let mut fx = slt::__bench_new_kitty_fixture(KITTY_IMAGES);
    let mut sink: Vec<u8> = Vec::with_capacity(8192);

    // First flush uploads images; we measure the steady-state diff path.
    fx.flush_inline(&mut sink, KITTY_ROW_OFFSET).unwrap();
    sink.clear();

    bench("stable inline flush", ITERS, || {
        fx.flush_inline(&mut sink, KITTY_ROW_OFFSET).unwrap();
    });
}

fn audit_dim_buffer_modal() {
    println!("[#228] dim_buffer modal — {ITERS} iterations on {DIM_W}x{DIM_H} buf:");

    let area = Rect::new(0, 0, DIM_W, DIM_H);

    // Three modal sizes exercise the strip-based, mostly-inside, and full-
    // buffer fallback paths respectively.
    let small_modal = Rect::new(85, 25, 30, 10);
    let large_modal = Rect::new(20, 5, 160, 50);
    let zero_modal = Rect::new(0, 0, 0, 0);

    bench("modal-aware (small 30x10 modal)", ITERS, || {
        let mut buf = Buffer::empty(area);
        slt::__bench_dim_buffer_around(&mut buf, small_modal);
    });
    bench("modal-aware (large 160x50 modal)", ITERS, || {
        let mut buf = Buffer::empty(area);
        slt::__bench_dim_buffer_around(&mut buf, large_modal);
    });
    bench("full-buffer scan (legacy path)", ITERS, || {
        let mut buf = Buffer::empty(area);
        slt::__bench_dim_buffer_around(&mut buf, zero_modal);
    });

    println!("  (large modal should beat full-buffer; small modal beats it less.)");
}

fn main() {
    println!("=== SLT v0.20.0 hot-path perf audit ===");
    println!();

    audit_framestate_reuse();
    println!();

    audit_wrap_segments();
    println!();

    audit_kitty_placement_flush();
    println!();

    audit_dim_buffer_modal();
    println!();

    println!("Tip: re-run with `--release` for steady-state numbers.");
}
