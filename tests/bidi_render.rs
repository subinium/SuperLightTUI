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

#[cfg(feature = "bidi")]
#[test]
fn nko_and_adlam_take_the_uax9_path() {
    let mut nko = TestBackend::new(10, 1);
    nko.render(|ui| {
        ui.text("\u{07CA}\u{07CB}\u{07CC}");
    });
    nko.assert_line(0, "\u{07CC}\u{07CB}\u{07CA}");

    let mut adlam = TestBackend::new(10, 1);
    adlam.render(|ui| {
        ui.text("\u{1E900}\u{1E901}\u{1E902}");
    });
    adlam.assert_line(0, "\u{1E902}\u{1E901}\u{1E900}");

    let mut samaritan = TestBackend::new(10, 1);
    samaritan.render(|ui| {
        ui.text("\u{0800}\u{0801}\u{0802}");
    });
    samaritan.assert_line(0, "\u{0802}\u{0801}\u{0800}");

    let mut mandaic = TestBackend::new(10, 1);
    mandaic.render(|ui| {
        ui.text("\u{0840}\u{0841}\u{0842}");
    });
    mandaic.assert_line(0, "\u{0842}\u{0841}\u{0840}");
}

#[cfg(feature = "bidi")]
#[test]
fn arabic_renders_in_visual_order() {
    let mut tb = TestBackend::new(10, 1);
    tb.render(|ui| {
        ui.text("\u{0633}\u{0644}\u{0627}\u{0645}");
    });
    tb.assert_line(0, "\u{0645}\u{0627}\u{0644}\u{0633}");
}

#[cfg(feature = "bidi")]
#[test]
fn combining_mark_stays_attached_to_its_rtl_base() {
    let mut tb = TestBackend::new(10, 1);
    tb.render(|ui| {
        ui.text("\u{05D0}\u{05B7}\u{05D1}");
    });
    tb.assert_line(0, "\u{05D1}\u{05D0}\u{05B7}");
    assert_eq!(tb.buffer().get(1, 0).symbol, "\u{05D0}\u{05B7}");
}

#[cfg(feature = "bidi")]
#[test]
fn styled_rtl_segments_are_remapped_as_one_logical_line() {
    let mut tb = TestBackend::new(20, 1);
    tb.render(|ui| {
        ui.line_wrap(|ui| {
            ui.styled("abc ", Style::new().fg(Color::Red));
            ui.styled("\u{05D0}\u{05D1}", Style::new().fg(Color::Green));
            ui.styled("\u{05D2}", Style::new().fg(Color::Blue));
        });
    });

    tb.assert_line(0, "abc \u{05D2}\u{05D1}\u{05D0}");
    let buffer = tb.buffer();
    assert_eq!(buffer.get(4, 0).style.fg, Some(Color::Blue));
    assert_eq!(buffer.get(5, 0).style.fg, Some(Color::Green));
    assert_eq!(buffer.get(6, 0).style.fg, Some(Color::Green));
}
