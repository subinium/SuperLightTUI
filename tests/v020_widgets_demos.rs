//! Snapshot-style smoke tests for the v0.20.0 widget demos.
//!
//! These render the same widgets used by the
//! `examples/v020_*` binaries inside a `TestBackend`, so we catch
//! regressions in visual output without actually running the example as a
//! process. The intent is a fast, deterministic smoke check — not a pixel
//! perfect comparison (the existing `tests/visual_snapshots.rs` covers
//! that for high-stakes baselines).

use slt::widgets::SpinnerState;
use slt::{GutterOpts, HighlightRange, ScrollState, SplitPaneState, TestBackend};

#[test]
fn demo_v020_progress_response_renders() {
    let mut tb = TestBackend::new(60, 8);
    let spinner = SpinnerState::dots();
    tb.render(|ui| {
        let _ = ui
            .bordered(slt::Border::Rounded)
            .title("progress_response")
            .p(1)
            .col(|ui| {
                let _ = ui.row(|ui| {
                    let _ = ui.spinner(&spinner);
                    ui.text(" Loading...").dim();
                });
                let _ = ui.progress(0.42);
            });
    });
    // Both widgets must show their fingerprints.
    tb.assert_contains("Loading");
    let out = tb.to_string_trimmed();
    assert!(out.contains('█') || out.contains('░'), "got: {out}");
}

#[test]
fn demo_v020_breadcrumb_renders_segments_and_separator() {
    let mut tb = TestBackend::new(60, 4);
    tb.render(|ui| {
        let _ = ui.breadcrumb(&["Home", "Projects", "v0.20.0"]);
    });
    tb.assert_contains("Home");
    tb.assert_contains("v0.20.0");
    let out = tb.to_string_trimmed();
    assert!(out.contains('›'));
}

#[test]
fn demo_v020_split_pane_renders_handle() {
    let mut tb = TestBackend::new(60, 8);
    let mut state = SplitPaneState::new(0.4);
    tb.render(|ui| {
        let _ = ui
            .bordered(slt::Border::Rounded)
            .title("split_pane")
            .p(1)
            .grow(1)
            .col(|ui| {
                let _ = ui.split_pane(
                    &mut state,
                    |ui| {
                        ui.text("LEFT");
                    },
                    |ui| {
                        ui.text("RIGHT");
                    },
                );
            });
    });
    tb.assert_contains("LEFT");
    tb.assert_contains("RIGHT");
    let out = tb.to_string_trimmed();
    assert!(out.contains('│'), "vertical handle visible: {out}");
}

#[test]
fn demo_v020_vsplit_pane_renders_handle() {
    let mut tb = TestBackend::new(40, 12);
    let mut state = SplitPaneState::new(0.5);
    tb.render(|ui| {
        let _ = ui
            .bordered(slt::Border::Rounded)
            .title("vsplit_pane")
            .p(1)
            .grow(1)
            .col(|ui| {
                let _ = ui.vsplit_pane(
                    &mut state,
                    |ui| {
                        ui.text("TOP");
                    },
                    |ui| {
                        ui.text("BOTTOM");
                    },
                );
            });
    });
    tb.assert_contains("TOP");
    tb.assert_contains("BOTTOM");
    let out = tb.to_string_trimmed();
    assert!(out.contains('─'), "horizontal handle visible: {out}");
}

#[test]
fn demo_v020_gauge_renders_label_in_filled_bar() {
    let mut tb = TestBackend::new(60, 8);
    tb.render(|ui| {
        let _ = ui
            .bordered(slt::Border::Rounded)
            .title("gauge")
            .p(1)
            .col(|ui| {
                ui.gauge(0.5).label("50%").width(24);
                ui.gauge(0.85).label("85%").width(24);
            });
    });
    tb.assert_contains("50%");
    tb.assert_contains("85%");
    let out = tb.to_string_trimmed();
    assert!(out.contains('█'));
    assert!(out.contains('░'));
}

#[test]
fn demo_v020_line_gauge_renders_labels_after_bar() {
    let mut tb = TestBackend::new(60, 7);
    tb.render(|ui| {
        let _ = ui
            .bordered(slt::Border::Rounded)
            .title("line_gauge")
            .p(1)
            .col(|ui| {
                ui.line_gauge(0.6).label("60%").width(24);
                ui.line_gauge(0.3)
                    .filled('#')
                    .empty('.')
                    .width(24)
                    .label("30%");
            });
    });
    tb.assert_contains("60%");
    tb.assert_contains("30%");
    let out = tb.to_string_trimmed();
    assert!(out.contains('━'));
    assert!(out.contains('#'));
}

#[test]
fn demo_v020_gutter_highlights_render_search_navigation() {
    let mut tb = TestBackend::new(60, 12);
    let mut state = ScrollState::new();
    let lines: Vec<String> = (0..30)
        .map(|i| match i {
            5 => "ERROR upstream timeout".into(),
            12 => "ERROR database lost".into(),
            _ => format!("INFO  line {i}"),
        })
        .collect();
    state.set_highlights(&[HighlightRange::line(5), HighlightRange::line(12)]);
    tb.render(|ui| {
        let _ = ui
            .bordered(slt::Border::Rounded)
            .title("gutter_highlights")
            .p(1)
            .grow(1)
            .col(|ui| {
                let r = ui.scrollable_with_gutter(
                    &mut state,
                    GutterOpts::new(lines.len(), 8, |idx| format!("{:>3}", idx + 1)),
                    |ui, abs| {
                        if let Some(line) = lines.get(abs) {
                            ui.text(line.clone());
                        }
                    },
                );
                assert_eq!(r.total_highlights, 2);
            });
    });
    tb.assert_contains("ERROR");
    let out = tb.to_string_trimmed();
    // gutter labels: at least the first few line numbers.
    assert!(out.contains('1') && out.contains('2'));
}
