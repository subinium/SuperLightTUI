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
