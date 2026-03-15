use slt::{Border, Color, Context};

fn main() -> std::io::Result<()> {
    slt::run(|ui: &mut Context| {
        if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(slt::KeyCode::Esc) {
            ui.quit();
        }

        ui.bordered(Border::Rounded)
            .pad(1)
            .title("SLT")
            .col(|ui: &mut Context| {
                ui.text("Hello, World!").bold().fg(Color::Cyan);
                ui.text("Press 'q' to quit").dim();
            });
    })
}
