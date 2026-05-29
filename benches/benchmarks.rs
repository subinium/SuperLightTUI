use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use slt::buffer::Buffer;
use slt::rect::Rect;
use slt::style::Style;
#[cfg(feature = "crossterm")]
use slt::style::{Color, ColorDepth, Modifiers};
use slt::test_utils::TestBackend;
use slt::widgets::{
    CalendarState, ListState, SelectState, TableState, TabsState, TreeNode, TreeState,
};

fn bench_buffer_set_string(c: &mut Criterion) {
    let area = Rect::new(0, 0, 200, 50);
    let style = Style::new();
    c.bench_function("buffer_set_string_200x50", |b| {
        let mut buf = Buffer::empty(area);
        b.iter(|| {
            buf.reset();
            for y in 0..50 {
                buf.set_string(
                    0,
                    y,
                    black_box("Hello World! This is a benchmark string for testing."),
                    style,
                );
            }
        });
    });
}

fn bench_buffer_diff(c: &mut Criterion) {
    let area = Rect::new(0, 0, 200, 50);
    let style = Style::new();
    c.bench_function("buffer_diff_200x50", |b| {
        let prev = Buffer::empty(area);
        let mut curr = Buffer::empty(area);
        for y in 0..25 {
            curr.set_string(0, y, "Changed content here", style);
        }
        b.iter(|| {
            black_box(curr.diff(&prev));
        });
    });
}

fn bench_layout_simple(c: &mut Criterion) {
    c.bench_function("layout_col_10_texts", |b| {
        let mut backend = TestBackend::new(80, 24);
        b.iter(|| {
            backend.render(|ui| {
                let _ = ui.col(|ui| {
                    for i in 0..10 {
                        ui.text(format!("Line {i}"));
                    }
                });
            });
        });
    });
}

fn bench_layout_nested(c: &mut Criterion) {
    c.bench_function("layout_nested_rows_cols", |b| {
        let mut backend = TestBackend::new(120, 40);
        b.iter(|| {
            backend.render(|ui| {
                let _ = ui.col(|ui| {
                    for _ in 0..5 {
                        let _ = ui.row(|ui| {
                            for j in 0..4 {
                                ui.text(format!("Cell {j}"));
                            }
                        });
                    }
                });
            });
        });
    });
}

fn bench_full_render(c: &mut Criterion) {
    c.bench_function("full_render_120x40", |b| {
        let mut backend = TestBackend::new(120, 40);
        b.iter(|| {
            backend.render(|ui| {
                let _ = ui.col(|ui| {
                    ui.text("Header").bold();
                    let _ = ui.separator();
                    for i in 0..20 {
                        ui.text(format!("Row {i}"));
                    }
                    let _ = ui.progress(0.75);
                });
            });
        });
    });
}

/// The canonical "small dashboard" tree shared by `bench_full_render`, the
/// `full_render_dims` group, and both arms of the ratatui head-to-head:
/// a bold header, a separator, 20 rows, and a progress bar. Keeping the body
/// in one place guarantees the dimension sweep and the head-to-head measure
/// the *same* layout, only the terminal size (or framework) differs.
fn render_dashboard(ui: &mut slt::Context) {
    let _ = ui.col(|ui| {
        ui.text("Header").bold();
        let _ = ui.separator();
        for i in 0..20 {
            ui.text(format!("Row {i}"));
        }
        let _ = ui.progress(0.75);
    });
}

/// Full-render across terminal sizes: an 80x24 baseline, the existing
/// 120x40 dashboard size, and an ultra-wide 300x100. The 300x100 case is
/// where an immediate-mode redraw-everything engine is most likely to blow
/// the 16.6 ms budget, so it earns its own committed baseline (issue #270).
fn bench_full_render_dims(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_render_dims");
    for (w, h) in [(80u32, 24u32), (120, 40), (300, 100)] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{w}x{h}")),
            &(w, h),
            |b, &(w, h)| {
                let mut backend = TestBackend::new(w, h);
                b.iter(|| {
                    backend.render(render_dashboard);
                });
            },
        );
    }
    group.finish();
}

/// Pure-animation churn: the content, progress value, and sparkline shift
/// every iteration, so each frame produces a genuinely non-empty diff —
/// unlike the static `full_render*` benches, which diff an identical tree
/// against itself after the first frame. This exercises the full
/// build → compute → collect → render → diff loop under steady 60 FPS
/// churn (issue #270).
fn bench_animation_churn(c: &mut Criterion) {
    let mut group = c.benchmark_group("animation");
    group.bench_function("churn_200x60", |b| {
        let mut backend = TestBackend::new(200, 60);
        let base: Vec<f64> = (0..50)
            .map(|i| ((i as f64 / 4.0).sin() * 40.0) + 50.0)
            .collect();
        let mut t: f64 = 0.0;

        // Sanity: prove the churn is real — two successive frames with
        // different `t` must produce a non-empty inter-frame diff, i.e. the
        // renderer is not collapsing identical re-renders. Done once, before
        // the timed loop, so it never pollutes the sample.
        #[cfg(debug_assertions)]
        {
            let render_at = |backend: &mut TestBackend, t: f64| {
                let p = (t.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                let data: Vec<f64> = base.iter().map(|v| v + (t * 6.0).sin() * 5.0).collect();
                backend.render(|ui| {
                    let _ = ui.col(|ui| {
                        ui.text(format!("frame {t:.3}"));
                        let _ = ui.progress(p);
                        let _ = ui.sparkline(&data, 50);
                    });
                });
            };
            render_at(&mut backend, 0.0);
            let first = backend.to_string_trimmed();
            render_at(&mut backend, 0.5);
            let second = backend.to_string_trimmed();
            debug_assert_ne!(
                first, second,
                "animation churn must change frame-to-frame output"
            );
        }

        b.iter(|| {
            t += 1.0 / 60.0; // one 60 FPS tick
            let p = (t.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
            let data: Vec<f64> = base.iter().map(|v| v + (t * 6.0).sin() * 5.0).collect();
            backend.render(|ui| {
                let _ = ui.col(|ui| {
                    ui.text(format!("frame {t:.3}"));
                    let _ = ui.progress(p);
                    let _ = ui.sparkline(&data, 50);
                });
            });
        });
    });
    group.finish();
}

/// SLT dashboard render baseline (bold header + separator + 20 rows +
/// progress/gauge) into the in-memory `TestBackend` at 200x60 — the
/// framework's build → layout → render → diff cost only, no terminal I/O
/// (issue #270). The qualitative comparison vs ratatui's equivalent
/// `Terminal::draw` path is documented in `docs/PERFORMANCE.md`; ratatui is
/// intentionally NOT linked as a dev-dependency because it pulls a
/// transitively-advisory `lru 0.12` (RUSTSEC-2026-0002) that would fail the
/// release audit gate for a benchmark-only comparison.
fn bench_headtohead(c: &mut Criterion) {
    let mut group = c.benchmark_group("dashboard_200x60");

    group.bench_function("slt", |b| {
        let mut backend = TestBackend::new(200, 60);
        b.iter(|| {
            backend.render(render_dashboard);
        });
    });

    group.finish();
}

fn bench_widget_list(c: &mut Criterion) {
    c.bench_function("widget_list_100_items", |b| {
        let mut backend = TestBackend::new(80, 40);
        let items: Vec<String> = (0..100).map(|i| format!("Item {i}")).collect();
        b.iter(|| {
            let mut state = ListState::new(items.clone());
            backend.render(|ui| {
                let _ = ui.list(&mut state);
            });
        });
    });
}

fn bench_widget_table(c: &mut Criterion) {
    c.bench_function("widget_table_50_rows", |b| {
        let mut backend = TestBackend::new(120, 60);
        let headers = vec!["Name", "Email", "Role", "Status"];
        let rows: Vec<Vec<String>> = (0..50)
            .map(|i| {
                vec![
                    format!("User {i}"),
                    format!("user{i}@test.com"),
                    "Admin".to_string(),
                    "Active".to_string(),
                ]
            })
            .collect();
        b.iter(|| {
            let mut state = TableState::new(headers.clone(), rows.clone());
            backend.render(|ui| {
                let _ = ui.table(&mut state);
            });
        });
    });
}

fn bench_widget_list_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("widget_list_sizes");
    for size in [10_u32, 100, 500] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let mut backend = TestBackend::new(100, 50);
            let items: Vec<String> = (0..size).map(|i| format!("Item {i}")).collect();
            b.iter(|| {
                let mut state = ListState::new(items.clone());
                backend.render(|ui| {
                    let _ = ui.list(&mut state);
                });
            });
        });
    }
    group.finish();
}

fn bench_widget_tabs(c: &mut Criterion) {
    c.bench_function("widget_tabs_5", |b| {
        let mut backend = TestBackend::new(80, 24);
        b.iter(|| {
            let mut state = TabsState::new(vec!["Tab1", "Tab2", "Tab3", "Tab4", "Tab5"]);
            backend.render(|ui| {
                let _ = ui.tabs(&mut state);
            });
        });
    });
}

fn bench_widget_checkbox(c: &mut Criterion) {
    c.bench_function("widget_checkbox_10", |b| {
        let mut backend = TestBackend::new(80, 24);
        b.iter(|| {
            let mut checks = [false; 10];
            backend.render(|ui| {
                for (i, checked) in checks.iter_mut().enumerate() {
                    let _ = ui.checkbox(format!("Option {i}"), checked);
                }
            });
        });
    });
}

fn bench_widget_select(c: &mut Criterion) {
    c.bench_function("widget_select_10_items", |b| {
        let mut backend = TestBackend::new(80, 24);
        b.iter(|| {
            let mut state = SelectState::new((0..10).map(|i| format!("Item {i}")).collect());
            backend.render(|ui| {
                let _ = ui.select(&mut state);
            });
        });
    });
}

fn bench_widget_progress(c: &mut Criterion) {
    c.bench_function("widget_progress_10", |b| {
        let mut backend = TestBackend::new(80, 24);
        b.iter(|| {
            backend.render(|ui| {
                for i in 0..10 {
                    let _ = ui.progress(i as f64 / 9.0);
                }
            });
        });
    });
}

fn bench_widget_tree(c: &mut Criterion) {
    c.bench_function("widget_tree_20_nodes_3_levels", |b| {
        let mut backend = TestBackend::new(100, 40);
        b.iter(|| {
            let mut state = TreeState::new(vec![
                TreeNode::new("Root 0").expanded().children(vec![
                    TreeNode::new("Branch 0-0").expanded().children(vec![
                        TreeNode::new("Leaf 0-0-0"),
                        TreeNode::new("Leaf 0-0-1"),
                        TreeNode::new("Leaf 0-0-2"),
                    ]),
                    TreeNode::new("Branch 0-1").expanded().children(vec![
                        TreeNode::new("Leaf 0-1-0"),
                        TreeNode::new("Leaf 0-1-1"),
                        TreeNode::new("Leaf 0-1-2"),
                    ]),
                    TreeNode::new("Branch 0-2").expanded().children(vec![
                        TreeNode::new("Leaf 0-2-0"),
                        TreeNode::new("Leaf 0-2-1"),
                        TreeNode::new("Leaf 0-2-2"),
                    ]),
                ]),
                TreeNode::new("Root 1").expanded().children(vec![
                    TreeNode::new("Branch 1-0").expanded().children(vec![
                        TreeNode::new("Leaf 1-0-0"),
                        TreeNode::new("Leaf 1-0-1"),
                        TreeNode::new("Leaf 1-0-2"),
                    ]),
                    TreeNode::new("Branch 1-1")
                        .expanded()
                        .children(vec![TreeNode::new("Leaf 1-1-0")]),
                ]),
            ]);
            backend.render(|ui| {
                let _ = ui.tree(&mut state);
            });
        });
    });
}

fn bench_widget_sparkline(c: &mut Criterion) {
    c.bench_function("widget_sparkline_50_points", |b| {
        let mut backend = TestBackend::new(80, 24);
        let data: Vec<f64> = (0..50)
            .map(|i| ((i as f64 / 4.0).sin() * 40.0) + 50.0)
            .collect();
        b.iter(|| {
            backend.render(|ui| {
                let _ = ui.sparkline(&data, 50);
            });
        });
    });
}

fn bench_layout_grid(c: &mut Criterion) {
    c.bench_function("layout_grid_3x12", |b| {
        let mut backend = TestBackend::new(80, 24);
        b.iter(|| {
            backend.render(|ui| {
                let _ = ui.grid(3, |ui| {
                    for i in 0..12 {
                        ui.text(format!("Cell {i}"));
                    }
                });
            });
        });
    });
}

fn bench_widget_calendar(c: &mut Criterion) {
    c.bench_function("widget_calendar_2024_03", |b| {
        let mut backend = TestBackend::new(80, 24);
        b.iter(|| {
            let mut state = CalendarState::from_ym(2024, 3);
            backend.render(|ui| {
                let _ = ui.calendar(&mut state);
            });
        });
    });
}

// ---------------------------------------------------------------------------
// Flush-path benches (stdout-emit cost).
//
// `bench_buffer_diff_200x50` above measures only the cell-diff computation.
// These benches feed two `Buffer`s into `flush_buffer_diff` against a
// hermetic `Vec<u8>` sink so we can measure the actual ANSI/SGR emit cost —
// the path that issue #62's run-length coalescing aims to shrink.
//
// No real terminal, no stdout, no I/O. `Vec<u8>` is reused across iterations
// (truncated each iter) to keep allocator noise out of the sample.
// ---------------------------------------------------------------------------
#[cfg(feature = "crossterm")]
fn realistic_colors() -> [Color; 8] {
    [
        Color::Reset,
        Color::Red,
        Color::Green,
        Color::Blue,
        Color::Yellow,
        Color::Rgb(200, 100, 50),
        Color::Rgb(50, 200, 100),
        Color::Indexed(33),
    ]
}

/// Populate `buf` with a realistic mix of run-length-friendly segments,
/// occasional single-cell color flips, and periodic bold/underline toggles.
/// Every cell in the buffer area ends up written so that diffing against an
/// empty previous buffer produces a full-redraw workload.
#[cfg(feature = "crossterm")]
fn fill_realistic(buf: &mut Buffer, seed: u32) {
    let colors = realistic_colors();
    let width = buf.area.width;
    let height = buf.area.height;
    // Reusable single-char stack buffer for UTF-8 encode.
    let glyphs = ['A', 'B', 'C', 'D', '0', '1', '2', '3', ' ', '.'];

    for y in 0..height {
        // Row base style runs across most of the row — rewards coalescing.
        let base_fg = colors[((y + seed) as usize) % colors.len()];
        let base_bg = colors[((y.wrapping_mul(3) + seed) as usize) % colors.len()];
        let row_style = Style::new().fg(base_fg).bg(base_bg);

        for x in 0..width {
            let ch = glyphs[((x + y) as usize) % glyphs.len()];
            let mut style = row_style;

            // Every 17th cell flips fg (single-cell break in the run).
            if (x.wrapping_add(y.wrapping_mul(7)) + seed) % 17 == 0 {
                style = style.fg(colors[((x + seed) as usize) % colors.len()]);
            }
            // Every 31st cell toggles bold (modifier-only change).
            if (x + y) % 31 == 0 {
                style.modifiers |= Modifiers::BOLD;
            }
            // Every 53rd cell toggles underline.
            if (x + y + seed) % 53 == 0 {
                style.modifiers |= Modifiers::UNDERLINE;
            }

            let cell = buf.get_mut(x, y);
            let mut tmp = [0u8; 4];
            cell.set_symbol(ch.encode_utf8(&mut tmp));
            cell.set_style(style);
            // Clear any stale hyperlink from previous calls.
            cell.hyperlink = None;
        }

        // One hyperlink span per few rows (8 cells) to exercise OSC 8.
        // Use the public `set_string_linked` helper so we never touch
        // `Cell::hyperlink`'s CompactString type directly from the bench.
        if (y + seed) % 4 == 0 && width >= 8 {
            let start = ((y * 7) + seed) % (width - 7);
            buf.set_string_linked(start, y, "linkcell", row_style, "https://example.com/bench");
        }
    }
}

/// Mutate `curr` so that only ~5% of its cells differ from `prev`.
/// Expects `curr` to start as a clone of `prev`.
#[cfg(feature = "crossterm")]
fn fill_sparse(curr: &mut Buffer, _prev: &Buffer) {
    let width = curr.area.width;
    let height = curr.area.height;
    let colors = realistic_colors();

    let total = (width as u64) * (height as u64);
    let target = (total / 20).max(1); // ~5%

    for i in 0..target {
        let x = ((i * 131) % (width as u64)) as u32;
        let y = ((i * 97) % (height as u64)) as u32;
        let ch = match i % 5 {
            0 => '*',
            1 => '#',
            2 => '+',
            3 => '-',
            _ => '?',
        };
        let style = Style::new()
            .fg(colors[(i as usize) % colors.len()])
            .bg(colors[((i + 3) as usize) % colors.len()]);
        let cell = curr.get_mut(x, y);
        let mut tmp = [0u8; 4];
        cell.set_symbol(ch.encode_utf8(&mut tmp));
        cell.set_style(style);
    }
}

#[cfg(feature = "crossterm")]
fn bench_flush_full_redraw_200x60(c: &mut Criterion) {
    let mut group = c.benchmark_group("flush");
    group.bench_function("full_redraw_200x60", |b| {
        let area = Rect::new(0, 0, 200, 60);
        let mut prev = Buffer::empty(area);
        let mut curr = Buffer::empty(area);
        fill_realistic(&mut curr, 1);
        // Sanity: make sure we actually have a non-trivial diff workload.
        debug_assert!(!curr.diff(&prev).is_empty());

        let mut sink: Vec<u8> = Vec::with_capacity(256 * 1024);
        b.iter(|| {
            sink.clear();
            // Issue #171: use the mutable bench entry point so the per-row
            // hash refresh is part of the measured cost (matches what
            // `Terminal::flush` does in production).
            slt::__bench_flush_buffer_diff_mut(
                &mut sink,
                black_box(&mut curr),
                black_box(&mut prev),
                ColorDepth::TrueColor,
            )
            .expect("flush into Vec<u8> cannot fail");
            black_box(sink.len());
        });
    });
    group.finish();
}

#[cfg(feature = "crossterm")]
fn bench_flush_sparse_change_200x60(c: &mut Criterion) {
    let mut group = c.benchmark_group("flush");
    group.bench_function("sparse_change_200x60", |b| {
        let area = Rect::new(0, 0, 200, 60);
        // Build two independent buffers seeded identically so they start
        // cell-for-cell equal, then mutate `curr` sparsely. We cannot use
        // `Buffer::clone` — `Buffer` intentionally does not implement
        // `Clone` (it would hide the cost of duplicating the full grid).
        let mut prev = Buffer::empty(area);
        let mut curr = Buffer::empty(area);
        fill_realistic(&mut prev, 1);
        fill_realistic(&mut curr, 1);
        // Sanity: start equal.
        debug_assert!(curr.diff(&prev).is_empty());
        fill_sparse(&mut curr, &prev);
        debug_assert!(!curr.diff(&prev).is_empty());

        let mut sink: Vec<u8> = Vec::with_capacity(64 * 1024);
        b.iter(|| {
            sink.clear();
            slt::__bench_flush_buffer_diff_mut(
                &mut sink,
                black_box(&mut curr),
                black_box(&mut prev),
                ColorDepth::TrueColor,
            )
            .expect("flush into Vec<u8> cannot fail");
            black_box(sink.len());
        });
    });
    group.finish();
}

/// 0%-dirty (static) flush baseline for issue #171's GO/NO-GO decision.
///
/// Two identical buffers — `flush_buffer_diff` walks every cell, finds no
/// difference, and emits nothing. This is the worst case for the per-cell
/// scan because every cell pays the comparison cost while no output is
/// produced. If this bench stays under 50 µs on 200×60 we do **not**
/// implement the per-row hash skip (issue #171 NO-GO).
#[cfg(feature = "crossterm")]
fn bench_flush_static_200x60(c: &mut Criterion) {
    let mut group = c.benchmark_group("flush");
    group.bench_function("static_200x60", |b| {
        let area = Rect::new(0, 0, 200, 60);
        let mut prev = Buffer::empty(area);
        let mut curr = Buffer::empty(area);
        fill_realistic(&mut prev, 1);
        fill_realistic(&mut curr, 1);
        // Sanity: 0% dirty — diff must be empty.
        debug_assert!(curr.diff(&prev).is_empty());

        let mut sink: Vec<u8> = Vec::with_capacity(1024);
        b.iter(|| {
            sink.clear();
            slt::__bench_flush_buffer_diff_mut(
                &mut sink,
                black_box(&mut curr),
                black_box(&mut prev),
                ColorDepth::TrueColor,
            )
            .expect("flush into Vec<u8> cannot fail");
            black_box(sink.len());
        });
    });
    group.finish();
}

/// Ultra-wide full-redraw flush (issue #270). Same harness as
/// `bench_flush_full_redraw_200x60` but on a 300×100 area — the largest
/// committed flush target, stressing the ANSI/SGR emit path at ultra-wide
/// terminal size.
#[cfg(feature = "crossterm")]
fn bench_flush_full_redraw_300x100(c: &mut Criterion) {
    let mut group = c.benchmark_group("flush");
    group.bench_function("full_redraw_300x100", |b| {
        let area = Rect::new(0, 0, 300, 100);
        let mut prev = Buffer::empty(area);
        let mut curr = Buffer::empty(area);
        fill_realistic(&mut curr, 1);
        // Sanity: full-redraw workload must produce a non-trivial diff.
        debug_assert!(!curr.diff(&prev).is_empty());

        let mut sink: Vec<u8> = Vec::with_capacity(512 * 1024);
        b.iter(|| {
            sink.clear();
            slt::__bench_flush_buffer_diff_mut(
                &mut sink,
                black_box(&mut curr),
                black_box(&mut prev),
                ColorDepth::TrueColor,
            )
            .expect("flush into Vec<u8> cannot fail");
            black_box(sink.len());
        });
    });
    group.finish();
}

/// Ultra-wide sparse-change flush (issue #270). ~5% of cells differ on a
/// 300×100 area — the steady-state churn workload at ultra-wide size.
#[cfg(feature = "crossterm")]
fn bench_flush_sparse_change_300x100(c: &mut Criterion) {
    let mut group = c.benchmark_group("flush");
    group.bench_function("sparse_change_300x100", |b| {
        let area = Rect::new(0, 0, 300, 100);
        let mut prev = Buffer::empty(area);
        let mut curr = Buffer::empty(area);
        fill_realistic(&mut prev, 1);
        fill_realistic(&mut curr, 1);
        // Sanity: start cell-for-cell equal, then mutate ~5%.
        debug_assert!(curr.diff(&prev).is_empty());
        fill_sparse(&mut curr, &prev);
        debug_assert!(!curr.diff(&prev).is_empty());

        let mut sink: Vec<u8> = Vec::with_capacity(128 * 1024);
        b.iter(|| {
            sink.clear();
            slt::__bench_flush_buffer_diff_mut(
                &mut sink,
                black_box(&mut curr),
                black_box(&mut prev),
                ColorDepth::TrueColor,
            )
            .expect("flush into Vec<u8> cannot fail");
            black_box(sink.len());
        });
    });
    group.finish();
}

/// Issue #273 — Phase 0 streaming baseline.
///
/// The motivating workload: an LLM emits one token, the whole frame is
/// re-described, and the entire pipeline (closure body → `build_tree` →
/// flexbox `compute` → `collect_all` → `render`) re-runs for ~2000 lines of
/// static chrome above a tiny streaming region. This bench measures the full
/// per-token frame cost so the win an upstream gate could capture is *measured,
/// not assumed* (the issue makes Phase 0 blocking).
///
/// `chrome_uncached` is the baseline: the static transcript is re-described
/// every token. `chrome_cached` wraps that transcript in
/// [`slt::ContainerBuilder::cached`] keyed off a stable version — output is
/// byte-identical (the cache currently always re-runs the body; see the method
/// docs), so the two numbers quantify the recording/classification overhead of
/// the gate itself, and the absolute cost is the headroom a future cell-level
/// replay could reclaim.
const STREAM_CHROME_LINES: usize = 2000;

fn bench_streaming_append_chat(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_append_chat");
    let transcript: Vec<String> = (0..STREAM_CHROME_LINES)
        .map(|i| format!("[{i:04}] assistant: a prior turn of the conversation history"))
        .collect();

    // Baseline: re-describe the full static chrome every token.
    group.bench_function("chrome_uncached", |b| {
        let mut backend = TestBackend::new(120, 40);
        let mut stream = slt::StreamingTextState::new();
        b.iter(|| {
            stream.push("tok ");
            backend.render(|ui| {
                let _ = ui.col(|ui| {
                    for line in &transcript {
                        ui.text(line.as_str());
                    }
                });
                let _ = ui.streaming_text(black_box(&mut stream));
            });
        });
    });

    // Author declares the chrome stable via `cached`; the stream stays
    // uncached. The chrome's key never changes across tokens, so every frame
    // after the first is a cache hit (visible via `region_cache_hits`).
    group.bench_function("chrome_cached", |b| {
        let mut backend = TestBackend::new(120, 40);
        let mut stream = slt::StreamingTextState::new();
        let chrome_version: u64 = 1; // stable: the transcript never changes here
        b.iter(|| {
            stream.push("tok ");
            backend.render(|ui| {
                let _ = ui.container().cached(black_box(chrome_version), |ui| {
                    for line in &transcript {
                        ui.text(line.as_str());
                    }
                });
                let _ = ui.streaming_text(black_box(&mut stream));
            });
        });
    });

    group.finish();
}

/// Register flush-path benches (only when `crossterm` feature is enabled,
/// which is the default for benches). When the feature is off, this is a
/// no-op so the file still compiles under `--no-default-features`.
fn bench_flush_group(c: &mut Criterion) {
    #[cfg(feature = "crossterm")]
    {
        bench_flush_full_redraw_200x60(c);
        bench_flush_sparse_change_200x60(c);
        bench_flush_static_200x60(c);
        bench_flush_full_redraw_300x100(c);
        bench_flush_sparse_change_300x100(c);
    }
    let _ = c;
}

criterion_group!(
    benches,
    bench_buffer_set_string,
    bench_buffer_diff,
    bench_layout_simple,
    bench_layout_nested,
    bench_full_render,
    bench_full_render_dims,
    bench_animation_churn,
    bench_headtohead,
    bench_widget_list,
    bench_widget_table,
    bench_widget_list_sizes,
    bench_widget_tabs,
    bench_widget_checkbox,
    bench_widget_select,
    bench_widget_progress,
    bench_widget_tree,
    bench_widget_sparkline,
    bench_layout_grid,
    bench_widget_calendar,
    bench_streaming_append_chat,
    bench_flush_group,
);
criterion_main!(benches);
