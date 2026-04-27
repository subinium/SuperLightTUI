use super::*;
use crate::{EventBuilder, TestBackend};
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
fn big_text_renders_half_block_grid() {
    let mut backend = TestBackend::new(16, 4);
    backend.render(|ui| {
        let _ = ui.big_text("A");
    });

    let output = backend.to_string();
    assert!(
        output.contains('▀') || output.contains('▄') || output.contains('█'),
        "output should contain half-block glyphs: {output:?}"
    );
}

#[test]
fn timer_display_formats_minutes_seconds_centis() {
    let mut backend = TestBackend::new(20, 4);
    backend.render(|ui| {
        ui.timer_display(Duration::from_secs(83) + Duration::from_millis(450));
    });

    backend.assert_contains("01:23.45");
}

#[test]
fn image_single_rawdraw_renders_halfblock_cells() {
    // 40x20 image — each pixel pair is a distinct color so we can verify fg/bg
    let width: u32 = 40;
    let height: u32 = 20;
    let red = Color::Rgb(255, 0, 0);
    let blue = Color::Rgb(0, 0, 255);
    let pixels: Vec<(Color, Color)> = vec![(red, blue); (width * height) as usize];
    let img = HalfBlockImage {
        width,
        height,
        pixels,
    };

    let mut backend = TestBackend::new(width, height);
    backend.render(|ui| {
        let _ = ui.image(&img);
    });

    let buf = backend.buffer();

    // Top-left cell
    let tl = buf.get(0, 0);
    assert_eq!(tl.symbol.as_str(), "▀", "top-left char should be ▀");
    assert_eq!(tl.style.fg, Some(red), "top-left fg should be red");
    assert_eq!(tl.style.bg, Some(blue), "top-left bg should be blue");

    // Bottom-right cell
    let br = buf.get(width - 1, height - 1);
    assert_eq!(br.symbol.as_str(), "▀", "bottom-right char should be ▀");
    assert_eq!(br.style.fg, Some(red), "bottom-right fg should be red");
    assert_eq!(br.style.bg, Some(blue), "bottom-right bg should be blue");
}

#[test]
fn image_empty_does_not_panic() {
    let img = HalfBlockImage {
        width: 0,
        height: 0,
        pixels: vec![],
    };
    let mut backend = TestBackend::new(10, 4);
    backend.render(|ui| {
        let _ = ui.image(&img);
    });
    // No assertion needed — must not panic
}

// confirm() hit-test regression tests (issue #175)
// Layout: "ok? [Yes] [No]" at x=0
//   q_width=3, space=1 → yes_start=4, yes_end=9, no_start=10, no_end=14

#[test]
fn confirm_click_yes_registers_yes() {
    let mut backend = TestBackend::new(40, 4);
    let mut answer = false;
    let events = EventBuilder::new().click(4, 0).build(); // yes_start
    let mut clicked = false;
    backend.run_with_events(events, |ui| {
        let r = ui.confirm("ok?", &mut answer);
        clicked = r.clicked;
    });
    assert!(clicked, "click at yes_start should register");
    assert!(answer, "answer should be true");
}

#[test]
fn confirm_click_yes_right_boundary_does_not_hit_no() {
    let mut backend = TestBackend::new(40, 4);
    let mut answer = false;
    // yes_end=9, no_start=10: clicking at col 9 (between Yes and No) should not register
    let events = EventBuilder::new().click(9, 0).build();
    let mut clicked = false;
    backend.run_with_events(events, |ui| {
        let r = ui.confirm("ok?", &mut answer);
        clicked = r.clicked;
    });
    assert!(
        !clicked,
        "click between [Yes] and [No] (col 9) should not register"
    );
}

#[test]
fn confirm_click_no_registers_no() {
    let mut backend = TestBackend::new(40, 4);
    let mut answer = true;
    let events = EventBuilder::new().click(10, 0).build(); // no_start
    let mut clicked = false;
    backend.run_with_events(events, |ui| {
        let r = ui.confirm("ok?", &mut answer);
        clicked = r.clicked;
    });
    assert!(clicked, "click at no_start should register");
    assert!(!answer, "answer should be false");
}

#[test]
fn confirm_click_beyond_no_does_not_register() {
    let mut backend = TestBackend::new(40, 4);
    let mut answer = true;
    let events = EventBuilder::new().click(20, 0).build(); // well past no_end=14
    let mut clicked = false;
    backend.run_with_events(events, |ui| {
        let r = ui.confirm("ok?", &mut answer);
        clicked = r.clicked;
    });
    assert!(!clicked, "click past no_end should not register");
    // answer unchanged
    assert!(answer, "answer should remain unchanged");
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

// streaming_markdown should not allocate a new String on `code_block_lang`
// when the value is unchanged across frames (issue #187).
#[test]
fn streaming_markdown_skips_state_writes_when_unchanged() {
    use crate::widgets::StreamingMarkdownState;

    let mut state = StreamingMarkdownState::new();
    state.start();
    state.push("hello world\n");
    state.streaming = false;

    // First render: not in code block; lang stays empty.
    let mut tb = TestBackend::new(40, 4);
    tb.render(|ui| {
        let _ = ui.streaming_markdown(&mut state);
    });
    assert!(!state.in_code_block);
    assert!(state.code_block_lang.is_empty());

    // Re-render with no content change — values should remain identical.
    tb.render(|ui| {
        let _ = ui.streaming_markdown(&mut state);
    });
    assert!(!state.in_code_block);
    assert!(state.code_block_lang.is_empty());
}
