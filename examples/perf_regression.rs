use std::time::Instant;

use slt::widgets::{TextInputState, TextareaState};
use slt::TestBackend;

fn main() {
    let mut tb = TestBackend::new(120, 40);
    let mut input = TextInputState::with_placeholder("Search docs, commands, and diagnostics...");
    input.value = "superlighttui immediate mode rendering benchmark".into();
    input.cursor = input.value.chars().count() / 2;

    let mut textarea = TextareaState::new();
    textarea.lines = (0..120)
        .map(|i| {
            format!(
                "Line {i:03} — wrapped content for cache validation and repeated layout measurement in perf sanity demo"
            )
        })
        .collect();
    textarea.cursor_row = 32;
    textarea.cursor_col = 18;

    let long_text = "SuperLightTUI focuses on immediate-mode ergonomics while still needing the hot path to stay lean under repeated wrapped layout work. ".repeat(12);

    let start = Instant::now();
    for frame in 0..200 {
        input.cursor = (frame as usize) % input.value.chars().count().max(1);
        textarea.scroll_offset = frame % 20;
        tb.render(|ui| {
            let _ = ui
                .bordered(slt::Border::Rounded)
                .title("Perf sanity")
                .pad(1)
                .grow(1)
                .col(|ui| {
                    ui.text("Input cursor path").bold();
                    let _ = ui.text_input(&mut input);
                    ui.separator();
                    ui.text("Wrapped text path").bold();
                    ui.text(long_text.clone()).wrap();
                    ui.separator();
                    ui.text("Textarea cursor path").bold();
                    let _ = ui.textarea(&mut textarea, 8);
                });
        });
    }
    let elapsed = start.elapsed();

    println!("Rendered 200 frames in {:?}", elapsed);
    println!("Final frame excerpt:");
    println!("{}", tb.to_string_trimmed());
}
