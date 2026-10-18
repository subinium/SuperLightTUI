//! Unit tests for the display widget family.

use super::*;
use crate::TestBackend;
use std::time::Duration;

#[test]
fn gradient_text_renders_content() {
    let mut backend = TestBackend::new(20, 4);
    backend.render(|ui| {
        ui.text("ABCD").gradient(Color::Red, Color::Blue);
    });

    backend.assert_contains("ABCD");
}

#[test]
fn timer_display_formats_minutes_seconds_centis() {
    let mut backend = TestBackend::new(20, 4);
    backend.render(|ui| {
        ui.timer_display(Duration::from_secs(83) + Duration::from_millis(450));
    });

    backend.assert_contains("01:23.45");
}

// divider_text symmetric centering regression (issue #186).
#[test]
fn divider_text_centers_label_evenly() {
    // 20-col, label "Hi" (width 2). total_separator = 20 - (2 + 2) = 16.
    // left = 8, right = 8 → label sits in center.
    let mut backend = TestBackend::new(20, 4);
    backend.render(|ui| {
        let _ = ui.divider_text("Hi");
    });

    let buf = backend.buffer();
    // First row should be: 8 dashes, " Hi ", 8 dashes
    let row: String = (0..20).map(|x| buf.get(x, 0).symbol.clone()).collect();
    assert_eq!(row, "──────── Hi ────────", "label should be centered");
}

#[test]
fn divider_text_odd_width_right_takes_extra() {
    // 21-col, label "Hi" (width 2). total_separator = 21 - 4 = 17.
    // left = 8, right = 9 → right gets the extra cell on odd widths.
    let mut backend = TestBackend::new(21, 4);
    backend.render(|ui| {
        let _ = ui.divider_text("Hi");
    });

    let buf = backend.buffer();
    let row: String = (0..21).map(|x| buf.get(x, 0).symbol.clone()).collect();
    assert_eq!(
        row, "──────── Hi ─────────",
        "right side gets +1 on odd width"
    );
}
