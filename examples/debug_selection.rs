use slt::{Border, Color, Context, RunConfig};

fn main() -> std::io::Result<()> {
    slt::run_with(
        RunConfig {
            mouse: true,
            ..Default::default()
        },
        |ui: &mut Context| {
            if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(slt::KeyCode::Esc) {
                ui.quit();
            }

            ui.text("Drag in left panel only. q = quit").dim();

            let _ = ui.row(|ui| {
                let _ = ui.bordered(Border::Rounded).pad(1).grow(1).col(|ui| {
                    ui.text("Left 1: Hello").fg(Color::Cyan);
                    ui.text("Left 2: World").fg(Color::Cyan);
                    ui.text("Left 3: Rust").fg(Color::Cyan);
                    ui.text("Left 4: SLT").fg(Color::Cyan);
                    ui.text("Left 5: Done").fg(Color::Cyan);
                });
                let _ = ui.bordered(Border::Rounded).pad(1).grow(1).col(|ui| {
                    ui.text("Right 1: Alpha").fg(Color::Magenta);
                    ui.text("Right 2: Beta").fg(Color::Magenta);
                    ui.text("Right 3: Gamma").fg(Color::Magenta);
                    ui.text("Right 4: Delta").fg(Color::Magenta);
                    ui.text("Right 5: Omega").fg(Color::Magenta);
                });
            });
        },
    )
}
