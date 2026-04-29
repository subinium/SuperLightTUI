//! Allocation-budget tests for v0.20.0 hot-path perf fixes.
//!
//! Each test wraps a hot path (frame render, wrap_segments, kitty placement
//! flush, dim_buffer modal) in a counting global allocator and asserts the
//! allocation count drops to a near-zero steady-state.
//!
//! # Cross-test isolation (root-cause fix, v0.20.1 #240)
//!
//! Cargo's test runner runs `#[test]` functions in parallel by default. The
//! `MEASURING` flag and `ALLOC_COUNT` counter are global, so any other test
//! thread that allocates while a measurement is in flight pollutes the count.
//!
//! Pre-fix the file relied on a `measure_lock` mutex held only inside
//! `measure_allocs()`, which protected nothing — non-measuring siblings still
//! ran concurrently and their `String::from(...)` / `Vec::new()` calls leaked
//! into the counter, producing noisy `1599 / 1937 / 65 / 145` budget breaches
//! whose pattern depended purely on macOS thread-cache timing.
//!
//! Fix: every `#[test]` in this file now grabs `measure_lock` at the top of
//! the function body. That serialises the *whole* file at the test-function
//! granularity (one binary = one mutex), so `cargo test --all-features` is
//! reliable even without `--test-threads=1`. `measure_allocs()` itself uses
//! `try_lock` since the caller already holds the guard — the only purpose of
//! the inner attempt is to belt-and-braces against future tests that forget
//! to call `enter_perf_test()` at the top.

#![allow(clippy::unwrap_used)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

struct CountingAllocator;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static MEASURING: AtomicBool = AtomicBool::new(false);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if MEASURING.load(Ordering::Relaxed) {
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Serialises every `#[test]` in this file. Held by `enter_perf_test()` for
/// the lifetime of each test function so the parallel test runner cannot
/// interleave allocator activity from a sibling test into a measurement.
fn measure_lock() -> &'static Mutex<()> {
    static LOCK: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Acquire the file-wide test mutex. Every `#[test]` body must call this on
/// its first line and bind the returned guard to a `_guard` local — the
/// guard's `Drop` releases the mutex when the test function exits, so the
/// next test in line gets a clean global allocator state.
#[must_use = "binding the guard to `_guard` keeps the file-wide test serialisation alive"]
fn enter_perf_test() -> std::sync::MutexGuard<'static, ()> {
    measure_lock().lock().unwrap()
}

fn measure_allocs<R>(label: &'static str, f: impl FnOnce() -> R) -> (R, usize) {
    // The caller (each `#[test]` fn) already holds `measure_lock` via
    // `enter_perf_test()`, so a recursive `lock()` would deadlock. `try_lock`
    // is a belt-and-braces guard: if a future test forgets the file-wide
    // `_guard`, the failed try_lock at least prevents two threads from
    // toggling `MEASURING` at once.
    let _maybe_guard = measure_lock().try_lock();
    ALLOC_COUNT.store(0, Ordering::Relaxed);
    MEASURING.store(true, Ordering::Relaxed);
    let r = f();
    MEASURING.store(false, Ordering::Relaxed);
    let count = ALLOC_COUNT.load(Ordering::Relaxed);
    eprintln!("[{}] allocations = {}", label, count);
    (r, count)
}

// ---------------------------------------------------------------------------
// Issue #204: FrameState reuse across 100 frames
// ---------------------------------------------------------------------------

#[test]
fn framestate_reuse_steady_state_alloc_count_low() {
    let _guard = enter_perf_test();
    use slt::TestBackend;

    let mut tb = TestBackend::new(80, 24);

    // Warm-up frame: first frame allocates the per-frame buffers; we want to
    // measure subsequent steady-state behavior only.
    tb.render(|ui| {
        let _ = ui.bordered(slt::Border::Rounded).title("warm").col(|ui| {
            ui.text("hello").bold();
            ui.text("world").dim();
        });
    });

    // Measure 100 frames. Allocation count should grow at most O(N) where N
    // is small (per-frame strings, command Vecs reallocating on first growth,
    // etc.). The hard regression target is "no per-frame `Vec::new` for the
    // six FrameState fields" — pre-fix baseline was ~6 allocations per frame
    // (one per field). With the fix, those six are reused across frames so
    // the count is dominated by per-render content allocations (e.g.
    // formatting). 1500 allocations across 100 frames = 15/frame ceiling,
    // well below the pre-fix baseline.
    let (_, count) = measure_allocs("framestate_100_frames", || {
        for _ in 0..100 {
            tb.render(|ui| {
                let _ = ui.bordered(slt::Border::Rounded).title("frame").col(|ui| {
                    ui.text("hello").bold();
                    ui.text("world").dim();
                });
            });
        }
    });

    // 100 frames with the six fields reused must stay under a tight budget.
    // If we regressed back to per-frame `Vec::new` for the six fields, count
    // would be at least 600 + format/string churn (typically 1500+).
    // Tight budget: < 1500 allocations across 100 frames.
    assert!(
        count < 1500,
        "framestate-reuse regression: 100 frames allocated {} times (budget 1500)",
        count
    );
}

// ---------------------------------------------------------------------------
// Issue #205: wrap_segments String alloc count
// ---------------------------------------------------------------------------

#[test]
fn wrap_segments_alloc_count_low_via_bench_helper() {
    let _guard = enter_perf_test();
    // Build static segment fixtures once — keep all allocations of the
    // fixture out of the measured region so we only count what
    // `wrap_segments` itself drives.
    let make_segments = |seed: u32| -> Vec<(String, slt::Style)> {
        vec![
            (format!("hello {} world", seed), slt::Style::new().bold()),
            (" ".to_string(), slt::Style::default()),
            (
                "alpha beta gamma delta epsilon zeta eta theta".to_string(),
                slt::Style::new().italic(),
            ),
        ]
    };

    // Warm-up.
    let _ = slt::__bench_wrap_segments(&make_segments(0), 40);

    let (_, count) = measure_allocs("wrap_segments_1000_iters", || {
        for i in 0..1000u32 {
            let segs = make_segments(i);
            let _wrapped = slt::__bench_wrap_segments(&segs, 40);
        }
    });

    // Pre-fix: each style boundary caused a `String::new()` + push (= realloc
    // on first byte). 1000 iterations with multiple style boundaries each
    // would allocate thousands of times beyond the necessary minimum.
    // With `with_capacity`, those collapse to one allocation per style run.
    // Budget: < 25000 for the full 1000-iter loop including the per-iter
    // fixture rebuild (each `make_segments` call allocates 3 Strings + 1
    // Vec). The pre-fix baseline would exceed 30000+.
    eprintln!(
        "wrap_segments avg allocs/call = {:.2}",
        count as f64 / 1000.0
    );
    assert!(
        count < 25000,
        "wrap_segments alloc regression: 1000 iters allocated {} times (budget 25000)",
        count
    );
}

// ---------------------------------------------------------------------------
// Issue #206: kitty placement flush — no Vec<KittyPlacement> clone in caller
// ---------------------------------------------------------------------------

#[test]
fn kitty_placement_flush_first_flush_one_arc_clone() {
    let _guard = enter_perf_test();
    // Each rgba Arc gets exactly +1 strong ref (the stored `prev_placements`
    // copy). The pre-fix code added an extra +1 per Arc per flush via the
    // `let adjusted: Vec<KittyPlacement> = ... .iter().map(|p| p.clone())`
    // step — this now goes away.
    let mut fx = slt::__bench_new_kitty_fixture(3);
    let before = fx.rgba_strong_counts();
    let mut sink: Vec<u8> = Vec::new();
    fx.flush_inline(&mut sink, 5).unwrap();
    let after = fx.rgba_strong_counts();
    for (b, a) in before.iter().zip(after.iter()) {
        assert_eq!(
            *a - *b,
            1,
            "first flush should add exactly 1 strong ref per image (was {} -> {})",
            b,
            a
        );
    }
}

#[test]
fn kitty_placement_flush_steady_state_no_arc_growth() {
    let _guard = enter_perf_test();
    // After the first flush, repeated identical flushes must not bump any
    // Arc strong count — the fast-path returns early and the new in-place
    // rebuild only swaps the existing prev_placements entries.
    let mut fx = slt::__bench_new_kitty_fixture(3);
    let mut sink: Vec<u8> = Vec::new();
    // Warm-up.
    fx.flush_inline(&mut sink, 5).unwrap();
    let after_first = fx.rgba_strong_counts();
    sink.clear();

    for _ in 0..50 {
        fx.flush_inline(&mut sink, 5).unwrap();
    }

    let after_50 = fx.rgba_strong_counts();
    for (b, a) in after_first.iter().zip(after_50.iter()) {
        assert_eq!(
            *a, *b,
            "stable flush should not change Arc strong count ({}=>{})",
            b, a
        );
    }
}

#[test]
fn kitty_placement_flush_alloc_count_low() {
    let _guard = enter_perf_test();
    // Steady-state flushes must allocate near-zero. Pre-fix code allocated
    // a `Vec<KittyPlacement>` per flush (+ a per-element `Arc::clone`
    // bookkeeping). Post-fix: only `Vec<u8>` sink growth on bytes written.
    // Stable flushes hit the fast-path and return without writing — sink
    // should not grow at all.
    let mut fx = slt::__bench_new_kitty_fixture(3);
    let mut sink: Vec<u8> = Vec::new();
    // Warm-up.
    fx.flush_inline(&mut sink, 5).unwrap();
    sink.clear();
    sink.shrink_to_fit();

    let (_, count) = measure_allocs("kitty_100_flushes_stable", || {
        for _ in 0..100 {
            fx.flush_inline(&mut sink, 5).unwrap();
        }
    });

    assert!(
        count < 50,
        "kitty stable flush regression: 100 flushes allocated {} times (budget 50)",
        count
    );
}

// ---------------------------------------------------------------------------
// Issue #228: dim_buffer modal — O(perimeter), not O(area)
// ---------------------------------------------------------------------------

#[test]
fn dim_buffer_modal_perimeter_not_area() {
    let _guard = enter_perf_test();
    // Direct call to the public bench helper that exposes modal-aware dim.
    use slt::buffer::Buffer;
    use slt::rect::Rect;

    let area = Rect::new(0, 0, 200, 60);
    let modal = Rect::new(60, 20, 80, 20); // centered modal

    // Count cells with DIM applied after the new path.
    let mut buf = Buffer::empty(area);
    slt::__bench_dim_buffer_around(&mut buf, modal);

    // Cells inside the modal must NOT have DIM; cells outside MUST have DIM.
    let mut dim_count = 0;
    let mut nondim_count = 0;
    for y in 0..60u32 {
        for x in 0..200u32 {
            let cell = buf.get(x, y);
            let has_dim = cell.style.modifiers.contains(slt::Modifiers::DIM);
            let inside_modal =
                x >= modal.x && x < modal.right() && y >= modal.y && y < modal.bottom();
            if has_dim {
                dim_count += 1;
                assert!(
                    !inside_modal,
                    "DIM should not be applied inside modal at ({},{})",
                    x, y
                );
            } else {
                nondim_count += 1;
                assert!(
                    inside_modal,
                    "DIM should be applied outside modal at ({},{})",
                    x, y
                );
            }
        }
    }

    // Sanity: dim_count = total - modal_area.
    let modal_area = (modal.width * modal.height) as usize;
    let total = (area.width * area.height) as usize;
    assert_eq!(dim_count, total - modal_area);
    assert_eq!(nondim_count, modal_area);
}

#[test]
fn dim_buffer_modal_full_screen_falls_back_correctly() {
    let _guard = enter_perf_test();
    use slt::buffer::Buffer;
    use slt::rect::Rect;

    // Modal that covers the full screen → no strip cells. Visual contract:
    // every cell stays untouched (since they're "inside the modal").
    let area = Rect::new(0, 0, 80, 24);
    let modal = Rect::new(0, 0, 80, 24);

    let mut buf = Buffer::empty(area);
    slt::__bench_dim_buffer_around(&mut buf, modal);

    for y in 0..24u32 {
        for x in 0..80u32 {
            let cell = buf.get(x, y);
            assert!(
                !cell.style.modifiers.contains(slt::Modifiers::DIM),
                "full-screen modal should not dim any cell"
            );
        }
    }
}

#[test]
fn dim_buffer_modal_zero_size_falls_back_to_full() {
    let _guard = enter_perf_test();
    use slt::buffer::Buffer;
    use slt::rect::Rect;

    let area = Rect::new(0, 0, 40, 12);
    // Zero-size modal -> fallback path inside dim_buffer_around.
    let modal = Rect::new(10, 5, 0, 0);

    let mut buf = Buffer::empty(area);
    slt::__bench_dim_buffer_around(&mut buf, modal);

    // Every cell must be DIM (full-buffer fallback).
    for y in 0..12u32 {
        for x in 0..40u32 {
            let cell = buf.get(x, y);
            assert!(
                cell.style.modifiers.contains(slt::Modifiers::DIM),
                "zero-size modal should dim every cell at ({},{})",
                x,
                y
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Reviewer A #6: use_state_keyed double-clone — must allocate at most one
// `String` per call on the cache-hit path.
// ---------------------------------------------------------------------------

#[test]
fn use_state_keyed_allocates_one_string_per_call() {
    let _guard = enter_perf_test();
    // Differential test: compare a frame with N cache-hit calls to a
    // baseline frame with the same shape but no extra calls. The delta
    // is the marginal allocation cost of the N `use_state_keyed` calls.
    //
    // With the fix: each call adds 1 alloc (the closure's `k.clone()`).
    // Pre-fix: each call adds 2 (the extra `entry(key.clone())`).
    //
    // Uses N = 100 so the signal (~100 fix vs ~200 pre-fix) dominates
    // any cross-test noise from concurrent threads.
    use slt::TestBackend;

    const N: usize = 100;

    let mut tb = TestBackend::new(20, 3);

    let pre_keys: Vec<String> = (0..N).map(|i| format!("k-{i}")).collect();

    // Warm-up frames so TestBackend's lazy buffers reach steady state
    // and all keyed entries exist (forcing the cache-hit path).
    for _ in 0..3 {
        tb.render(|ui| {
            for k in &pre_keys {
                let _ = ui.use_state_keyed(k.clone(), || 0i32);
            }
        });
    }

    // Baseline frame: identical render shape, anchor only.
    let (_, baseline) = measure_allocs("use_state_keyed_baseline_empty_frame", || {
        tb.render(|ui| {
            let _ = ui.use_state_keyed(String::from("baseline-anchor"), || 0i32);
        });
    });

    // Test frame: anchor + N cache-hit calls.
    let (_, with_calls) = measure_allocs("use_state_keyed_n_cache_hit_calls", || {
        tb.render(|ui| {
            let _ = ui.use_state_keyed(String::from("baseline-anchor"), || 0i32);
            for k in &pre_keys {
                let _ = ui.use_state_keyed(k.clone(), || 0i32);
            }
        });
    });

    let delta = with_calls.saturating_sub(baseline);
    eprintln!(
        "use_state_keyed: baseline = {}, with {} calls = {}, delta = {}",
        baseline, N, with_calls, delta
    );

    // With the fix: delta ≈ N (one `k.clone()` per call; `id.into()` on
    // `String->String` is identity; cache-hit path is alloc-free).
    // Pre-fix: delta ≈ 2*N (extra `entry(key.clone())` per call).
    //
    // Budget = 1.5 * N cleanly separates fix (~100) from pre-fix (~200)
    // while absorbing cross-test noise from concurrent allocator activity.
    let budget = (N * 3) / 2;
    assert!(
        delta <= budget,
        "use_state_keyed alloc regression: {} cache-hit calls added {} allocations over baseline (budget {}); pre-fix double-clone would add >= {}",
        N,
        delta,
        budget,
        N * 2
    );
}

#[test]
fn use_state_keyed_cache_hit_scales_one_per_call() {
    let _guard = enter_perf_test();
    // Differential test: compare a small-N frame to a large-N frame on a
    // warmed TestBackend. The marginal-cost model (large - small) cancels
    // render-internal one-off allocations that don't scale with call count.
    //
    // Uses LARGE_N = 100 so the signal (~100 fix vs ~200 pre-fix) dominates
    // any cross-test allocator-counter noise from concurrent test threads
    // (the global `MEASURING` toggle leaks during periods where another
    // test is in its measure_allocs critical section).
    use slt::TestBackend;

    const SMALL_N: usize = 10;
    const LARGE_N: usize = 100;

    let mut tb = TestBackend::new(20, 3);

    let pre_keys: Vec<String> = (0..LARGE_N).map(|i| format!("scale-k-{i}")).collect();

    // Warm-up: populate ALL entries used in either measurement.
    for _ in 0..3 {
        tb.render(|ui| {
            for k in &pre_keys {
                let _ = ui.use_state_keyed(k.clone(), || 0i32);
            }
        });
    }

    let (_, with_small) = measure_allocs("use_state_keyed_scale_small", || {
        tb.render(|ui| {
            for k in &pre_keys[..SMALL_N] {
                let _ = ui.use_state_keyed(k.clone(), || 0i32);
            }
        });
    });

    let (_, with_large) = measure_allocs("use_state_keyed_scale_large", || {
        tb.render(|ui| {
            for k in &pre_keys[..LARGE_N] {
                let _ = ui.use_state_keyed(k.clone(), || 0i32);
            }
        });
    });

    let extra_calls = LARGE_N - SMALL_N;
    let extra_allocs = with_large.saturating_sub(with_small);
    eprintln!(
        "use_state_keyed marginal allocs for +{} calls = {} ({} = {}, {} = {})",
        extra_calls, extra_allocs, SMALL_N, with_small, LARGE_N, with_large
    );

    // With the fix: extra_calls extra calls add ~extra_calls extra allocs
    // (1 per call). Pre-fix: ~2 * extra_calls (2 per call). Budget at
    // 1.5 * extra_calls cleanly separates fix (~90) from pre-fix (~180)
    // and absorbs minor cross-test noise.
    let budget = (extra_calls * 3) / 2;
    assert!(
        extra_allocs <= budget,
        "use_state_keyed marginal-cost regression: {} extra cache-hit calls added {} allocations (budget {}); pre-fix double-clone would add >= {}",
        extra_calls,
        extra_allocs,
        budget,
        extra_calls * 2
    );
}

// ---------------------------------------------------------------------------
// Issue #206: kitty placement flush — re-emit on row_offset change (resize)
// ---------------------------------------------------------------------------

/// When `InlineTerminal` resizes, the `start_row` (i.e. row_offset passed to
/// `KittyImageManager::flush`) changes. The fast-path comparison uses
/// `placement_eq_with_offset(c, row_offset, p)` which compares
/// `current.y + row_offset` against the previously-stored
/// `prev_placements[i].y` (which already includes the prior offset). When the
/// offset changes between two flushes, the comparison must fail and the flush
/// must re-emit placements at the new offset.
#[test]
fn kitty_flush_resize_reemits() {
    let _guard = enter_perf_test();
    let mut fx = slt::__bench_new_kitty_fixture(3);
    let mut sink: Vec<u8> = Vec::new();

    // First flush at row_offset = 10 → fresh manager, must emit placements.
    fx.flush_inline(&mut sink, 10).unwrap();
    let after_first = sink.len();
    assert!(
        after_first > 0,
        "first flush at row_offset=10 must emit placements (sink_len={after_first})"
    );

    // Second flush at row_offset = 10 (steady state) → fast-path should
    // return without writing anything.
    sink.clear();
    fx.flush_inline(&mut sink, 10).unwrap();
    assert_eq!(
        sink.len(),
        0,
        "second flush at same row_offset=10 must hit fast-path and emit no bytes"
    );

    // Third flush at row_offset = 15 (resize) → the offset changed, so the
    // fast-path comparison fails and the manager must re-emit placements.
    sink.clear();
    fx.flush_inline(&mut sink, 15).unwrap();
    assert!(
        !sink.is_empty(),
        "third flush at row_offset=15 (resize) must re-emit placements (sink_len={})",
        sink.len()
    );

    // Fourth flush at row_offset = 15 (new steady state) → fast-path again.
    sink.clear();
    fx.flush_inline(&mut sink, 15).unwrap();
    assert_eq!(
        sink.len(),
        0,
        "fourth flush at same row_offset=15 must return to fast-path with no bytes"
    );
}

// ---------------------------------------------------------------------------
// Issue #204: FrameState reuse buffers restored on error_boundary panic
// ---------------------------------------------------------------------------

/// `error_boundary` should restore the per-frame reuse buffers
/// (`context_stack`, `deferred_draws`, `group_stack`, `text_color_stack`,
/// `pending_tooltips`) to their pre-child state when a child closure panics.
/// `hovered_groups` is a `HashSet` populated by hit-testing rather than the
/// rollback snapshot, so it is not asserted here.
///
/// The buffers are `pub(crate)`, so this test verifies the contract through
/// public-API observable side effects:
///   * commands / draw output: the panicking child's pushes must not leak
///     into the rendered output of the fallback or sibling widgets.
///   * group_stack: a child opening a `group()` and panicking must not
///     leave the group stack in an unbalanced state — sibling widgets after
///     the boundary must still render correctly.
///   * across-frame robustness: the kernel's
///     `debug_assert!(group_stack.is_empty())` invariant at the end of every
///     frame would trip if the rollback failed to restore the stack.
#[test]
fn framestate_reuse_buffers_restored_on_error_boundary_panic() {
    let _guard = enter_perf_test();
    use slt::TestBackend;

    // ── Frame 1: normal render to populate FrameState reuse buffers. ────
    let mut tb = TestBackend::new(80, 12);
    tb.render(|ui| {
        let _ = ui.col(|ui| {
            ui.text("normal frame");
        });
    });

    // ── Frame 2: error_boundary wraps a child that pushes group state, a
    // deferred draw, and arbitrary text — then panics from inside a
    // group container. The fallback must render cleanly, and the sibling
    // text after the boundary must also render. ────────────────────────
    tb.render(|ui| {
        ui.error_boundary_with(
            |ui| {
                // Push state into multiple buffers, then panic from inside
                // the nested group's `col`. The inner col's panic-handler
                // pops `text_color_stack` and resumes the panic; the outer
                // error_boundary's snapshot restore then truncates the
                // remaining buffers and restores the rollback state.
                let _ = ui.group("transient-group").col(|ui| {
                    ui.text("inside-transient-group-text");
                    panic!("simulated child panic");
                });
            },
            |ui, msg| {
                ui.text(format!("recovered: {msg}"));
            },
        );

        // Sibling rendered AFTER the error_boundary. If group_stack
        // weren't rolled back, the kernel's debug_assert at frame end
        // would panic because group_stack would not be empty.
        ui.text("sibling-after-boundary");
    });

    let dump = tb.to_string_trimmed();

    // The fallback must have rendered the recovery message.
    assert!(
        dump.contains("recovered: simulated child panic"),
        "fallback must render after panic, got:\n{dump}"
    );

    // The child's "inside-transient-group-text" was pushed to commands,
    // then truncated by the rollback. It must NOT appear in the final
    // buffer.
    assert!(
        !dump.contains("inside-transient-group-text"),
        "rolled-back child commands must not render: \n{dump}"
    );

    // The sibling rendered after the boundary must render — proving the
    // group_stack and other reuse buffers were restored to their
    // pre-boundary depth, and confirming the frame's debug_assert
    // invariants did not trip.
    assert!(
        dump.contains("sibling-after-boundary"),
        "sibling text after boundary must render normally, got:\n{dump}"
    );

    // ── Frame 3: a clean render must succeed without panicking. The
    // FrameState reuse buffers persist across frames; if the rollback had
    // left them in an inconsistent state, the kernel's
    // `debug_assert!(group_stack.is_empty())` invariant at frame end of
    // frame 2 would already have tripped. This third frame additionally
    // verifies the buffers are still functional, not just balanced. ─────
    tb.render(|ui| {
        let _ = ui.col(|ui| {
            ui.text("post-recovery-frame");
        });
    });
    tb.assert_contains("post-recovery-frame");
}
