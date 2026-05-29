//! Integration tests for bidirectional (UAX #9) text reordering at the
//! widget-render boundary. Renders text through the public `Context` API and
//! reads the resulting buffer in visual (left-to-right column) order via
//! `TestBackend::line`, which is exactly the order a human reader perceives.

use slt::*;

/// LTR text must render unchanged whether or not the `bidi` feature is on —
/// this is the regression guard that the reorder path never touches LTR runs.
#[test]
fn ltr_text_renders_in_logical_order() {
    let mut tb = TestBackend::new(20, 1);
    tb.render(|ui| {
        ui.text("Hello world");
    });
    tb.assert_line(0, "Hello world");
}

/// Pure Hebrew must render reversed (visual order). "שלום" is logical
/// ש,ל,ו,ם; visually it reads "םולש" left-to-right.
#[cfg(feature = "bidi")]
#[test]
fn pure_rtl_renders_in_visual_order() {
    let mut tb = TestBackend::new(20, 1);
    tb.render(|ui| {
        ui.text("\u{05E9}\u{05DC}\u{05D5}\u{05DD}");
    });
    tb.assert_line(0, "\u{05DD}\u{05D5}\u{05DC}\u{05E9}");
}

/// Mixed Latin + Hebrew: the Latin segment stays LTR, the Hebrew reverses.
/// "abc אבג" → "abc גבא" (unicode-bidi reference vector).
#[cfg(feature = "bidi")]
#[test]
fn mixed_ltr_rtl_renders_per_uax9() {
    let mut tb = TestBackend::new(20, 1);
    tb.render(|ui| {
        ui.text("abc \u{05D0}\u{05D1}\u{05D2}");
    });
    tb.assert_line(0, "abc \u{05D2}\u{05D1}\u{05D0}");
}

/// European numbers inside an RTL run stay LTR. "שלום 42 עולם" keeps "42"
/// readable while the Hebrew words reverse.
#[cfg(feature = "bidi")]
#[test]
fn numbers_inside_rtl_stay_ltr() {
    let mut tb = TestBackend::new(20, 1);
    tb.render(|ui| {
        // logical: שלום 42 עולם
        ui.text("\u{05E9}\u{05DC}\u{05D5}\u{05DD} 42 \u{05E2}\u{05D5}\u{05DC}\u{05DD}");
    });
    // visual: םלוע 42 םולש
    tb.assert_line(
        0,
        "\u{05DD}\u{05DC}\u{05D5}\u{05E2} 42 \u{05DD}\u{05D5}\u{05DC}\u{05E9}",
    );
}
