use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use slt::buffer::Buffer;
use slt::rect::Rect;
use slt::style::Style;
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
);
criterion_main!(benches);
