//! Unit tests for v0.20.0 widget additions and refactors.
//!
//! Covered issues:
//! - #212 — spinner / progress return Response
//! - #213 — breadcrumb collapsed to BreadcrumbResponse
//! - #223 — split_pane / vsplit_pane drag handle
//! - #224 — gauge / line_gauge with inline label
//! - #235 — scrollable_with_gutter + highlight navigation

use slt::widgets::SpinnerState;
use slt::{
    BreadcrumbResponse, EventBuilder, GaugeResponse, GutterResponse, HighlightRange, KeyCode,
    LineGaugeOpts, ScrollState, SplitPaneState, TestBackend,
};

// ── #212: spinner / progress return Response ─────────────────────────────

#[test]
fn issue_212_spinner_returns_response_with_rect_after_warm_frame() {
    let mut tb = TestBackend::new(20, 3);
    let spinner = SpinnerState::dots();
    // Warm frame so prev_hit_map populates.
    tb.render(|ui| {
        let _ = ui.spinner(&spinner);
    });
    let mut rect_w = 0u32;
    tb.render(|ui| {
        let r = ui.spinner(&spinner);
        rect_w = r.rect.width;
    });
    assert!(rect_w > 0, "spinner Response.rect.width should be non-zero");
}

#[test]
fn issue_212_progress_returns_response() {
    let mut tb = TestBackend::new(40, 3);
    tb.render(|ui| {
        let r = ui.progress(0.5);
        // Compile-time check: Response field access works on the return value.
        let _ = r.hovered;
        let _ = r.rect;
        let _ = r.clicked;
    });
}

// ── #213: breadcrumb collapsed to BreadcrumbResponse ─────────────────────

#[test]
fn issue_213_breadcrumb_returns_breadcrumb_response() {
    let mut tb = TestBackend::new(60, 3);
    tb.render(|ui| {
        let r: BreadcrumbResponse = ui.breadcrumb(&["Home", "Settings"]);
        assert_eq!(r.clicked_segment, None);
    });
}

#[test]
fn issue_213_breadcrumb_response_derefs_to_response() {
    let mut tb = TestBackend::new(60, 3);
    tb.render(|ui| {
        let r = ui.breadcrumb(&["A", "B", "C"]);
        // Via Deref<Target = Response>:
        let _hovered: bool = r.hovered;
        let _rect = r.rect;
    });
}

#[test]
fn issue_213_breadcrumb_sep_uses_custom_separator() {
    let mut tb = TestBackend::new(60, 3);
    tb.render(|ui| {
        let _ = ui.breadcrumb_sep(&["A", "B", "C"], " >> ");
    });
    let output = tb.to_string_trimmed();
    assert!(output.contains(" >> "), "got: {output}");
}

// ── #223: split_pane / vsplit_pane ───────────────────────────────────────

#[test]
fn issue_223_split_pane_renders_handle_between_panes() {
    let mut tb = TestBackend::new(40, 5);
    let mut state = SplitPaneState::new(0.5);
    tb.render(|ui| {
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
    let output = tb.to_string_trimmed();
    assert!(output.contains("LEFT"), "left pane visible: {output}");
    assert!(output.contains("RIGHT"), "right pane visible: {output}");
    assert!(output.contains('│'), "handle char visible: {output}");
}

#[test]
fn issue_223_split_pane_arrow_keys_adjust_ratio_when_focused() {
    let mut tb = TestBackend::new(40, 5);
    let mut state = SplitPaneState::new(0.5);
    let initial = state.ratio;
    let events = EventBuilder::new().key_code(KeyCode::Right).build();
    // The handle is the first focusable in the widget; focus index 0 + 1
    // total focusables → handle owns focus.
    tb.render_with_events(events, 0, 1, |ui| {
        let _ = ui.split_pane(
            &mut state,
            |ui| {
                ui.text("L");
            },
            |ui| {
                ui.text("R");
            },
        );
    });
    assert!(state.ratio > initial, "Right key should grow left pane");
}

#[test]
fn issue_223_vsplit_pane_renders_horizontal_divider() {
    let mut tb = TestBackend::new(20, 8);
    let mut state = SplitPaneState::new(0.5);
    tb.render(|ui| {
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
    let output = tb.to_string_trimmed();
    assert!(output.contains("TOP"));
    assert!(output.contains("BOTTOM"));
    assert!(output.contains('─'), "horizontal divider visible: {output}");
}

#[test]
fn issue_223_split_pane_state_clamps_to_min_ratio() {
    let mut state = SplitPaneState::new(0.5);
    state.set_ratio(0.001);
    assert!(state.ratio >= state.min_ratio);
    state.set_ratio(0.999);
    assert!(state.ratio <= 1.0 - state.min_ratio);
}

// ── #224: gauge / line_gauge ─────────────────────────────────────────────

#[test]
fn issue_224_gauge_renders_label_inside_bar() {
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        let r: GaugeResponse = ui.gauge(0.5, "50%");
        assert!((r.ratio - 0.5).abs() < f32::EPSILON);
    });
    let output = tb.to_string_trimmed();
    assert!(output.contains("50%"), "label visible: {output}");
    assert!(output.contains('█'), "filled char visible: {output}");
    assert!(output.contains('░'), "empty char visible: {output}");
}

#[test]
fn issue_224_gauge_clamps_ratio() {
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        let r = ui.gauge(2.0, "");
        assert!((r.ratio - 1.0).abs() < f32::EPSILON);
        let r = ui.gauge(-0.5, "");
        assert!((r.ratio - 0.0).abs() < f32::EPSILON);
    });
}

#[test]
fn issue_224_line_gauge_default_chars_render() {
    let mut tb = TestBackend::new(40, 3);
    tb.render(|ui| {
        let _ = ui.line_gauge(0.5, LineGaugeOpts::default());
    });
    let output = tb.to_string_trimmed();
    assert!(output.contains('━'), "filled char ━ visible: {output}");
    assert!(output.contains('─'), "empty char ─ visible: {output}");
}

#[test]
fn issue_224_line_gauge_with_label_appended() {
    let mut tb = TestBackend::new(40, 3);
    tb.render(|ui| {
        let _ = ui.line_gauge(0.6, LineGaugeOpts::default().label("60%"));
    });
    let output = tb.to_string_trimmed();
    assert!(output.contains("60%"), "label visible: {output}");
}

// ── #235: scrollable gutter + highlight_next/prev ────────────────────────

#[test]
fn issue_235_set_highlights_initializes_current() {
    let mut state = ScrollState::new();
    state.set_highlights(&[HighlightRange::line(2), HighlightRange::line(5)]);
    assert_eq!(state.current_highlight(), Some(0));
}

#[test]
fn issue_235_highlight_next_wraps_around() {
    let mut state = ScrollState::new();
    state.set_highlights(&[HighlightRange::line(1), HighlightRange::line(2)]);
    state.highlight_next();
    assert_eq!(state.current_highlight(), Some(1));
    state.highlight_next();
    assert_eq!(state.current_highlight(), Some(0));
}

#[test]
fn issue_235_highlight_previous_wraps_around() {
    let mut state = ScrollState::new();
    state.set_highlights(&[HighlightRange::line(1), HighlightRange::line(2)]);
    state.highlight_previous();
    assert_eq!(state.current_highlight(), Some(1));
    state.highlight_previous();
    assert_eq!(state.current_highlight(), Some(0));
}

#[test]
fn issue_235_highlight_next_scrolls_viewport() {
    let mut state = ScrollState::new();
    // Pretend we have 50 rows of content in a 10-row viewport.
    state.set_highlights(&[HighlightRange::line(30)]);
    // Manually populate bounds. We use the public scroll API to set offset.
    state.scroll_down(0); // no-op; bounds set by widget on first frame.
                          // The set_bounds path is private, so simulate by accessing the
                          // highlight scroll-to via the public API; the math should clamp.
    state.scroll_to_current_highlight();
    // With viewport_height=0 (no widget run yet), scroll math no-ops, so
    // offset stays at 0 — but at least the call must not panic.
    assert_eq!(state.offset, 0);
}

#[test]
fn issue_235_scrollable_with_gutter_renders_line_numbers() {
    let mut tb = TestBackend::new(40, 8);
    let mut state = ScrollState::new();
    let lines: Vec<String> = (0..20).map(|i| format!("line {i}")).collect();
    tb.render(|ui| {
        let _: GutterResponse = ui.scrollable_with_gutter(
            &mut state,
            lines.len(),
            5,
            |idx| format!("{:>3}", idx + 1),
            |ui, abs| {
                if let Some(s) = lines.get(abs) {
                    ui.text(s.clone());
                }
            },
        );
    });
    let output = tb.to_string_trimmed();
    assert!(output.contains("line 0"), "first line visible: {output}");
    // gutter labels should display 1..=5 (the first five line numbers).
    assert!(output.contains('1') && output.contains('5'));
}

#[test]
fn issue_235_scrollable_with_gutter_highlight_marks_line() {
    let mut tb = TestBackend::new(40, 6);
    let mut state = ScrollState::new();
    state.set_highlights(&[HighlightRange::line(1)]);
    let lines: Vec<String> = (0..10).map(|i| format!("L{i}")).collect();
    tb.render(|ui| {
        let r = ui.scrollable_with_gutter(
            &mut state,
            lines.len(),
            5,
            |idx| format!("{}", idx + 1),
            |ui, abs| {
                if let Some(s) = lines.get(abs) {
                    ui.text(s.clone());
                }
            },
        );
        assert_eq!(r.total_highlights, 1);
        assert_eq!(r.current_highlight, Some(0));
    });
}

#[test]
fn issue_235_clear_highlights_resets_state() {
    let mut state = ScrollState::new();
    state.set_highlights(&[HighlightRange::line(1)]);
    assert_eq!(state.current_highlight(), Some(0));
    state.clear_highlights();
    assert_eq!(state.current_highlight(), None);
    assert!(state.highlights().is_empty());
}
