use slt::{Border, Color, Context, KeyCode};

fn main() -> std::io::Result<()> {
    let mut count: i32 = 0;
    slt::run(|ui: &mut Context| {
        if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        if ui.key('k') || ui.key_code(KeyCode::Up) {
            count += 1;
        }
        if ui.key('j') || ui.key_code(KeyCode::Down) {
            count -= 1;
        }
        ui.bordered(Border::Single)
            .title("Counter")
            .pad(1)
            .gap(1)
            .col(|ui| {
                ui.text("SLT Counter").bold().fg(Color::Cyan);
                ui.row_gap(2, |ui| {
                    ui.text("Count:");
                    let color = if count >= 0 { Color::Green } else { Color::Red };
                    ui.text(format!("{count}")).bold().fg(color);
                });
                ui.text("k/Up = +1  j/Down = -1  Ctrl+Q = quit").dim();
            });
    })
}
