//! Release-mode diagnostic workloads, not pass/fail timing thresholds.
//! Timings include allocator instrumentation; no terminal I/O or idle CPU is measured.

use slt::chart::ChartRenderer;
use slt::{ChartBuilder, Color, LegendPosition, Sequence, Spring, Style, TestBackend};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use std::time::Instant;

struct Allocator;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static CALLS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
static COUNTING: AtomicBool = AtomicBool::new(false);

fn allocated(size: usize) {
    let live = LIVE.fetch_add(size, Relaxed) + size;
    if COUNTING.load(Relaxed) {
        CALLS.fetch_add(1, Relaxed);
        BYTES.fetch_add(size, Relaxed);
        PEAK.fetch_max(live, Relaxed);
    }
}

// Delegate unchanged layouts and pointers to System; bookkeeping never allocates.
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            allocated(layout.size());
        }
        ptr
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            allocated(layout.size());
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let next = unsafe { System.realloc(ptr, layout, size) };
        if !next.is_null() {
            LIVE.fetch_sub(layout.size(), Relaxed);
            allocated(size);
        }
        next
    }
}

#[global_allocator]
static GLOBAL: Allocator = Allocator;

fn measure(name: &str, batch: usize, mut operation: impl FnMut()) {
    if let Ok(filter) = std::env::var("SLT_AUDIT_FILTER")
        && !name.contains(&filter)
    {
        return;
    }
    for _ in 0..32 {
        operation();
    }
    let mut samples = Vec::with_capacity(101);
    let start_live = LIVE.load(Relaxed);
    CALLS.store(0, Relaxed);
    BYTES.store(0, Relaxed);
    PEAK.store(start_live, Relaxed);
    COUNTING.store(true, Relaxed);
    for _ in 0..101 {
        let start = Instant::now();
        for _ in 0..batch {
            operation();
        }
        samples.push(start.elapsed().as_nanos() as f64 / batch as f64);
    }
    COUNTING.store(false, Relaxed);
    let retained = LIVE.load(Relaxed) as i128 - start_live as i128;
    let peak = PEAK.load(Relaxed).saturating_sub(start_live);
    samples.sort_by(f64::total_cmp);
    let operations = (101 * batch) as f64;
    println!(
        "{name},{:.1},{:.1},{:.2},{:.2},{retained},{peak}",
        samples[50],
        samples[95],
        CALLS.load(Relaxed) as f64 / operations,
        BYTES.load(Relaxed) as f64 / operations
    );
}

fn chart_renderer(name: &str, legend: LegendPosition, zero: bool) -> ChartRenderer {
    let mut chart = ChartBuilder::new(80, 24, Style::new(), Style::new());
    chart.legend(legend).ylim(0.0, 1.0);
    chart
        .bar(&[(0.0, if zero { 0.0 } else { 1.0 })])
        .label(name)
        .color(Color::Red);
    ChartRenderer::new(chart.build())
}

fn textarea_memory() {
    // Opt-in because the unoptimized 1 MB/history workloads are expensive.
    if std::env::var("SLT_AUDIT_FILTER").ok().as_deref() != Some("textarea_memory") {
        return;
    }
    for (alphabet, glyph) in [
        ("ascii", "a"),
        ("cjk", "\u{d55c}"),
        ("zwj", "\u{1f469}\u{200d}\u{1f4bb}"),
    ] {
        for size in [1000, 100_000, 1_000_000] {
            for cap in [0, 8] {
                let fixture_start = LIVE.load(Relaxed);
                let row = format!("{}\n", glyph.repeat(40));
                let text = row.repeat((size / row.len()).max(1));
                let paste = format!(
                    "{}\n{}\n{}\n{}\n",
                    glyph.repeat(40),
                    glyph.repeat(40),
                    glyph.repeat(40),
                    glyph.repeat(8)
                );
                let mut input = slt::TextareaState::new().history_max(cap);
                input.set_value(&text);
                input.cursor_row = input.lines.len() - 1;
                input.cursor_col = 0;
                let mut backend = TestBackend::new(80, 24);
                measure(
                    &format!("textarea_memory/{alphabet}/{size}/history={cap}/idle"),
                    1,
                    || {
                        backend.render_with_events(Vec::new(), 0, 1, |ui| {
                            let _ = ui.textarea(&mut input, 23);
                        });
                    },
                );
                measure(
                    &format!("textarea_memory/{alphabet}/{size}/history={cap}/paste_end"),
                    1,
                    || {
                        backend.render_with_events(
                            slt::EventBuilder::new().paste(&paste).build(),
                            0,
                            1,
                            |ui| {
                                let _ = ui.textarea(&mut input, 23);
                            },
                        );
                    },
                );
                println!(
                    "# textarea_fixture {alphabet}/{size}/history={cap}: source_bytes={} history_len={} fixture_live_bytes={} (includes document, source, backend, undo history; requested bytes, not RSS)",
                    text.len(),
                    input.history_len(),
                    LIVE.load(Relaxed).saturating_sub(fixture_start)
                );
                assert!(input.history_len() <= cap);
            }
        }
    }
}

fn main() {
    println!(
        "# v024_audit; release={}; arch={}; os={}; warmup=32; samples=101; p95=sorted[95]; timings=instrumented kernel wall ns; bytes=allocator requests, not RSS; retained/peak=sample-window requested live bytes; no terminal flush/idle CPU",
        !cfg!(debug_assertions),
        std::env::consts::ARCH,
        std::env::consts::OS
    );
    println!(
        "workload,p50_ns,p95_ns,alloc_calls,requested_bytes,retained_delta_bytes,peak_extra_bytes"
    );
    textarea_memory();
    for (width, height) in [(80, 24), (120, 40), (240, 80)] {
        let mut backend = TestBackend::new(width, height);
        measure(&format!("idle_kernel/{width}x{height}"), 1, || {
            backend.render(|ui| {
                ui.text("Static header");
                ui.text("Static body");
            });
        });
        let mut input = slt::TextInputState::new();
        measure(&format!("typing_kernel/{width}x{height}"), 1, || {
            input = slt::TextInputState::new();
            backend.render_with_events(slt::EventBuilder::new().key('a').build(), 0, 1, |ui| {
                let _ = ui.text_input(&mut input);
            });
        });
        let paste = "line of input\n".repeat(100);
        measure(
            &format!("paste_1400_bytes_kernel/{width}x{height}"),
            1,
            || {
                let mut textarea = slt::TextareaState::new();
                backend.render_with_events(
                    slt::EventBuilder::new().paste(&paste).build(),
                    0,
                    1,
                    |ui| {
                        let _ = ui.textarea(&mut textarea, height - 1);
                    },
                );
            },
        );
        let mut list =
            slt::ListState::new((0..10_000).map(|i| format!("item {i}")).collect::<Vec<_>>());
        list.set_filter("99");
        measure(
            &format!("filtered_list_10000_kernel/{width}x{height}"),
            1,
            || {
                backend.render(|ui| {
                    let _ = ui.list(&mut list);
                });
            },
        );
        let mut frame = 0;
        measure(
            &format!("static_log_enqueue_kernel/{width}x{height}"),
            1,
            || {
                frame += 1;
                backend.render(|ui| {
                    ui.static_log(format!("entry {frame}"));
                    ui.text("Status");
                });
            },
        );
        let data: Vec<_> = (0..1000)
            .map(|i| (i as f64, (i as f64 / 10.0).sin()))
            .collect();
        measure(
            &format!("line_chart_1000_kernel/{width}x{height}"),
            1,
            || {
                backend.render(|ui| {
                    let _ = ui.chart(
                        |c| {
                            c.line(&data);
                        },
                        width,
                        height,
                    );
                });
            },
        );
    }
    for bins in [9, 11] {
        let mut backend = TestBackend::new(80, 24);
        measure(&format!("issue398_histogram_ticks/{bins}"), 2, || {
            backend.render(|ui| {
                let _ = ui.histogram_with(
                    &[0.0, 9.0],
                    |h| {
                        h.bins(bins);
                    },
                    80,
                    24,
                );
            });
        });
    }
    for size in [1000, 100_000] {
        let values: Vec<_> = (0..size)
            .map(|i| ((i * 48271_u64) % 2147483647) as f64)
            .rev()
            .collect();
        let mut backend = TestBackend::new(80, 24);
        measure(&format!("issue420_histogram/{size}"), 1, || {
            backend.render(|ui| {
                let _ = ui.histogram_with(
                    black_box(&values),
                    |h| {
                        h.bins(32);
                    },
                    80,
                    24,
                );
            });
        });
    }
    for (label, legend, name, zero) in [
        ("issue397_zero_bar", LegendPosition::None, "", true),
        ("issue399_left_ascii", LegendPosition::TopLeft, "AB", false),
        (
            "issue399_left_cjk",
            LegendPosition::TopLeft,
            "\u{d55c}\u{ae00}",
            false,
        ),
    ] {
        let chart = chart_renderer(name, legend, zero);
        measure(label, 2, || {
            black_box(chart.render());
        });
    }
    let mut spring = Spring::new(0.0, 0.2, 0.85);
    measure("issue400_same_target", 1000, || {
        spring.set_target(black_box(1.0));
        spring.tick();
        black_box(spring.value());
    });
    for count in [4, 64, 1024] {
        let mut sequence = Sequence::new();
        for _ in 0..count {
            sequence = sequence.then(0.0, 1.0, 10, slt::anim::ease_linear);
        }
        measure(&format!("issue420_sequence_early/{count}"), 100, || {
            black_box(sequence.value(black_box(3)));
        });
        measure(&format!("issue420_sequence_late/{count}"), 100, || {
            black_box(sequence.value(black_box(count * 10 - 3)));
        });
    }
    for cached in [false, true] {
        let mut backend = TestBackend::new(120, 40);
        let mut frame = 0;
        let lines: Vec<_> = (0..2000)
            .map(|i| format!("history line {i}: unchanged bounded transcript"))
            .collect();
        measure(&format!("issue419_chrome_2000/cached={cached}"), 1, || {
            frame += 1;
            backend.render(|ui| {
                let body = |ui: &mut slt::Context| {
                    for line in &lines {
                        ui.text(line);
                    }
                };
                if cached {
                    let _ = ui.container().cached(1, body);
                } else {
                    let _ = ui.container().col(body);
                }
                ui.text(format!("token {}", frame % 10));
            });
        });
    }
    #[cfg(feature = "syntax-cpp")]
    {
        let theme = slt::Theme::dark();
        let source = "// cbase\nint main(){ return 0; }\nconst char* s = \"text\";";
        measure("issue382_cpp_highlight_warm", 20, || {
            black_box(slt::syntax::highlight_code(source, "cpp", &theme));
        });
    }
}
