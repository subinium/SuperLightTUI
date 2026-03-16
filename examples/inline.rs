use slt::{Color, Context, KeyCode};

fn main() -> std::io::Result<()> {
    let mut count: i32 = 0;

    slt::run_inline(4, |ui: &mut Context| {
        if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(slt::KeyCode::Esc) {
            ui.quit();
        }
        if ui.key('j') || ui.key_code(KeyCode::Down) {
            count -= 1;
        }
        if ui.key('k') || ui.key_code(KeyCode::Up) {
            count += 1;
        }

        ui.text(format!("Inline count: {count}"))
            .bold()
            .fg(Color::Cyan);
        ui.text("k/Up = +1  j/Down = -1");
        ui.text("Ctrl+Q = quit").dim();
    })
}
