//! v0.20.0 hot-path perf audit.
//!
//! Runs each of the four optimized hot paths 10000 times and prints a
//! timing breakdown. Use after applying the v0.20.0 perf fixes
//! (issues #204, #205, #206, #228) to confirm steady-state behavior.
//!
//! Run with:
//!
//! ```bash
//! cargo run --example v020_perf_audit --release
//! ```
//!
//! `--release` is required for representative numbers — debug builds add
//! ~10x overhead and obscure the diff between the four paths.

use std::time::Instant;

const ITERS: u32 = 10_000;

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
    use slt::TestBackend;

    let mut tb = TestBackend::new(80, 24);
    println!("[#204] FrameState 6-vec/hashset reuse — {} frames:", ITERS);
    bench("steady-state render", ITERS, || {
        tb.render(|ui| {
            let _ = ui.bordered(slt::Border::Rounded).title("audit").col(|ui| {
                ui.text("hello").bold();
                ui.text("world").dim();
            });
        });
    });
}

fn audit_wrap_segments() {
    println!(
        "[#205] wrap_segments String alloc — {} 3-segment wraps at 40w:",
        ITERS
    );

    let segments_template: Vec<(String, slt::Style)> = vec![
        (
            "hello world alpha beta".to_string(),
            slt::Style::new().bold(),
        ),
        (" ".to_string(), slt::Style::default()),
        (
            "gamma delta epsilon zeta eta theta".to_string(),
            slt::Style::new().italic(),
        ),
    ];

    bench("wrap to 40 cols", ITERS, || {
        let _ = slt::__bench_wrap_segments(&segments_template, 40);
    });
}

fn audit_kitty_placement_flush() {
    println!(
        "[#206] kitty placement flush — {} flushes (3 stable images):",
        ITERS
    );
    let mut fx = slt::__bench_new_kitty_fixture(3);
    let mut sink: Vec<u8> = Vec::with_capacity(8192);

    // Warm-up: first flush uploads.
    fx.flush_inline(&mut sink, 5).unwrap();
    sink.clear();

    bench("stable inline flush", ITERS, || {
        fx.flush_inline(&mut sink, 5).unwrap();
    });
}

fn audit_dim_buffer_modal() {
    use slt::buffer::Buffer;
    use slt::rect::Rect;

    let area = Rect::new(0, 0, 200, 60);

    println!(
        "[#228] dim_buffer modal — {} iterations on 200x60 buf:",
        ITERS
    );

    // Small modal: 30x10 in the middle (90% of cells need DIM, 10% inside).
    let small_modal = Rect::new(85, 25, 30, 10);
    // Large modal: 160x50 (80% inside, 20% strip).
    let large_modal = Rect::new(20, 5, 160, 50);
    // Zero modal: triggers the full-buffer fallback path (legacy
    // `dim_entire_buffer` equivalent).
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
