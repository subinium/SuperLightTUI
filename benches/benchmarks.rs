use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use slt::buffer::Buffer;
use slt::rect::Rect;
use slt::style::Style;
use slt::test_utils::TestBackend;
use slt::widgets::{ListState, TableState};

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
                ui.col(|ui| {
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
                ui.col(|ui| {
                    for _ in 0..5 {
                        ui.row(|ui| {
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
                ui.col(|ui| {
                    ui.text("Header").bold();
                    ui.separator();
                    for i in 0..20 {
                        ui.text(format!("Row {i}"));
                    }
                    ui.progress(0.75);
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
                ui.list(&mut state);
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
                ui.table(&mut state);
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
                    ui.list(&mut state);
                });
            });
        });
    }
    group.finish();
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
);
criterion_main!(benches);
