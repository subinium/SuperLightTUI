use slt::{Border, Color, Context, KeyCode, RunConfig};

fn main() -> std::io::Result<()> {
    let mut input = slt::TextInputState::with_placeholder(
        "Type to move the cursor and exercise IME/cursor placement...",
    );
    input.value = "superlighttui immediate mode rendering benchmark".into();
    input.cursor = input.value.chars().count() / 2;

    let mut textarea = slt::TextareaState::new();
    textarea.lines = (0..160)
        .map(|i| {
            format!(
                "Line {i:03} - wrapped content for interactive perf validation across repeated layout and cursor tracking paths"
            )
        })
        .collect();
    textarea.cursor_row = 24;
    textarea.cursor_col = 18;

    let long_text = "SuperLightTUI focuses on immediate-mode ergonomics while still needing the hot path to stay lean under repeated wrapped layout work. ".repeat(6);

    slt::run_with(
        RunConfig::default().mouse(true).kitty_keyboard(true),
        |ui: &mut Context| {
            if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
                ui.quit();
            }

            let theme = *ui.theme();
            let textarea_rows = ui.height().saturating_sub(18).clamp(5, 8);

            let _ = ui
                .bordered(Border::Rounded)
                .title("Interactive Perf Demo")
                .pad(1)
                .grow(1)
                .col(|ui| {
                    ui.line(|ui| {
                        ui.text("Exercise cursor and wrap hot paths")
                            .bold()
                            .fg(theme.primary);
                        ui.spacer();
                        ui.text("Esc/Ctrl+Q quit").dim();
                    });

                    ui.text("Text input cursor path").bold().fg(Color::Cyan);
                    let _ = ui.text_input(&mut input);

                    ui.separator();
                    ui.text("Wrapped text path").bold().fg(Color::Yellow);
                    ui.text(long_text.clone()).wrap();

                    ui.separator();
                    ui.text("Textarea cursor path").bold().fg(Color::Green);
                    let _ = ui.textarea(&mut textarea, textarea_rows);
                });
        },
    )
}
