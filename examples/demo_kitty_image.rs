use slt::{Border, Color, Context, ScrollState};

fn main() -> std::io::Result<()> {
    let mut scroll = ScrollState::default();

    // Generate 10 test images — enough to guarantee scrolling
    let images: Vec<(String, Vec<u8>, u32, u32)> = vec![
        gradient_image("Red-Blue", 120, 60, (255, 60, 60), (60, 60, 255)),
        gradient_image("Green-Yellow", 120, 60, (60, 255, 60), (255, 255, 60)),
        checkerboard_image("Checkerboard", 120, 60, 12),
        gradient_image("Cyan-Magenta", 120, 60, (60, 255, 255), (255, 60, 255)),
        stripe_image("Rainbow", 120, 60),
        gradient_image("White-Black", 120, 60, (240, 240, 240), (20, 20, 20)),
        gradient_image("Orange-Purple", 120, 60, (255, 140, 0), (128, 0, 128)),
        checkerboard_image("Fine Grid", 120, 60, 6),
        gradient_image("Teal-Rose", 120, 60, (0, 128, 128), (255, 100, 130)),
        stripe_image("Rainbow 2", 120, 60),
    ];

    slt::run(|ui: &mut Context| {
        if ui.key('q') {
            ui.quit();
        }

        // Manual keyboard scroll
        if ui.key('j') || ui.key_code(slt::KeyCode::Down) {
            scroll.offset = scroll.offset.saturating_add(2);
        }
        if ui.key('k') || ui.key_code(slt::KeyCode::Up) {
            scroll.offset = scroll.offset.saturating_sub(2);
        }

        let _ = ui
            .bordered(Border::Rounded)
            .title("Kitty Image Gallery")
            .grow(1)
            .col(|ui| {
                let _ = ui.row(|ui| {
                    ui.text("j/k scroll | q quit").dim();
                    ui.spacer();
                    ui.text(format!("offset: {}", scroll.offset)).dim();
                });

                let _ = ui.scrollable(&mut scroll).grow(1).col(|ui| {
                    for (i, (label, rgba, w, h)) in images.iter().enumerate() {
                        ui.text(format!("{}. {}", i + 1, label))
                            .bold()
                            .fg(Color::Yellow);
                        let _ = ui.kitty_image(rgba, *w, *h, 30, 8);
                        if i < images.len() - 1 {
                            ui.separator();
                        }
                    }
                    ui.text("");
                    ui.text("--- End of gallery ---").dim();
                });
            });
    })
}

fn gradient_image(
    label: &str,
    width: u32,
    height: u32,
    from: (u8, u8, u8),
    to: (u8, u8, u8),
) -> (String, Vec<u8>, u32, u32) {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let t = y as f64 / height.max(1) as f64;
        let r = lerp(from.0, to.0, t);
        let g = lerp(from.1, to.1, t);
        let b = lerp(from.2, to.2, t);
        for _x in 0..width {
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    (label.to_string(), rgba, width, height)
}

fn checkerboard_image(
    label: &str,
    width: u32,
    height: u32,
    cell_size: u32,
) -> (String, Vec<u8>, u32, u32) {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let is_dark = ((x / cell_size) + (y / cell_size)) % 2 == 0;
            let v = if is_dark { 40u8 } else { 200u8 };
            rgba.extend_from_slice(&[v, v, v, 255]);
        }
    }
    (label.to_string(), rgba, width, height)
}

fn stripe_image(label: &str, width: u32, height: u32) -> (String, Vec<u8>, u32, u32) {
    let colors: [(u8, u8, u8); 7] = [
        (255, 0, 0),
        (255, 127, 0),
        (255, 255, 0),
        (0, 255, 0),
        (0, 0, 255),
        (75, 0, 130),
        (148, 0, 211),
    ];
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    let stripe_h = height / colors.len() as u32;
    for y in 0..height {
        let idx = ((y / stripe_h.max(1)) as usize).min(colors.len() - 1);
        let (r, g, b) = colors[idx];
        for _x in 0..width {
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    (label.to_string(), rgba, width, height)
}

fn lerp(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t).clamp(0.0, 255.0) as u8
}
