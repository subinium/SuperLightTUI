use slt::{Border, ButtonVariant, Color, Context};

fn main() -> std::io::Result<()> {
    let mut panic_count: u32 = 0;

    slt::run(|ui: &mut Context| {
        if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(slt::KeyCode::Esc) {
            ui.quit();
        }

        ui.bordered(Border::Rounded)
            .title("Error Boundary Demo")
            .pad(1)
            .gap(1)
            .col(|ui| {
                ui.text("Trigger panic inside error boundary.").bold();
                ui.text("Press button, Enter, or key 'p'. Ctrl+Q/Esc to quit.")
                    .dim();
                ui.text(format!("Recovered panics: {panic_count}"))
                    .fg(Color::Cyan);

                let trigger_panic =
                    ui.button_with("Panic in boundary", ButtonVariant::Danger) || ui.key('p');

                ui.error_boundary_with(
                    |ui| {
                        if trigger_panic {
                            panic!("demo panic from error boundary");
                        }
                        ui.text("No panic this frame").fg(Color::Green);
                    },
                    |ui, _msg| {
                        panic_count = panic_count.saturating_add(1);
                        ui.text("Recovered from panic").bold().fg(Color::Yellow);
                    },
                );
            });
    })
}
