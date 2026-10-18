use super::*;
use crate::TestBackend;

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

#[test]
fn kitty_image_headless_uses_halfblock_fallback_without_placement() {
    let rgba = [255u8, 0, 0, 255].repeat(4);
    let mut backend = TestBackend::new(4, 2);
    backend.render(|ui| {
        let _ = ui.kitty_image(&rgba, 2, 2, 4, 2);
    });

    assert!(
        backend.buffer().kitty_placements.is_empty(),
        "headless Kitty fallback must not enqueue graphics placements"
    );
    backend.assert_contains("▀");
}

#[test]
fn kitty_image_fit_headless_uses_halfblock_fallback_without_placement() {
    let rgba = [0u8, 128, 255, 255].repeat(4);
    let mut backend = TestBackend::new(4, 2);
    backend.render(|ui| {
        let _ = ui.kitty_image_fit(&rgba, 2, 2, 4);
    });

    assert!(
        backend.buffer().kitty_placements.is_empty(),
        "headless Kitty fit fallback must not enqueue graphics placements"
    );
    backend.assert_contains("▀");
}

#[test]
fn sixel_image_headless_uses_placeholder_without_sprixel() {
    let rgba = [255u8, 0, 0, 255].repeat(4);
    let mut backend = TestBackend::new(20, 2);
    backend.render(|ui| {
        let _ = ui.sixel_image(&rgba, 2, 2, 20, 2);
    });

    assert!(
        backend.buffer().sprixels.is_empty(),
        "headless Sixel fallback must not enqueue sprixels"
    );
    backend.assert_contains("[sixel unsupported]");
}

#[test]
fn iterm_image_fit_headless_uses_placeholder_without_sprixel() {
    let png = [0x89u8, b'P', b'N', b'G'];
    let mut backend = TestBackend::new(20, 10);
    backend.render(|ui| {
        let _ = ui.iterm_image_fit(&png, 20);
    });

    assert!(
        backend.buffer().sprixels.is_empty(),
        "headless iTerm fallback must not enqueue sprixels"
    );
    backend.assert_contains("[iterm2 unsupported]");
}
