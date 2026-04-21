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
                    ui.separator();
                    for i in 0..20 {
                        ui.text(format!("Row {i}"));
                    }
                    let _ = ui.progress(0.75);
                });
            });
        });
    });
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
        let prev = Buffer::empty(area);
        let mut curr = Buffer::empty(area);
        fill_realistic(&mut curr, 1);
        // Sanity: make sure we actually have a non-trivial diff workload.
        debug_assert!(!curr.diff(&prev).is_empty());

        let mut sink: Vec<u8> = Vec::with_capacity(256 * 1024);
        b.iter(|| {
            sink.clear();
            slt::__bench_flush_buffer_diff(
                &mut sink,
                black_box(&curr),
                black_box(&prev),
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
            slt::__bench_flush_buffer_diff(
                &mut sink,
                black_box(&curr),
                black_box(&prev),
                ColorDepth::TrueColor,
            )
            .expect("flush into Vec<u8> cannot fail");
            black_box(sink.len());
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
    bench_flush_group,
);
criterion_main!(benches);
