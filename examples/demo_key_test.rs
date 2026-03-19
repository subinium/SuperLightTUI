use slt::*;
use std::time::Duration;

fn main() -> std::io::Result<()> {
    let kitty = std::env::args().any(|a| a == "--kitty");
    let mut log: Vec<String> = Vec::new();
    let mut input = TextInputState::with_placeholder("Type here...");
    let mut mode = 0u8;
    let modes = ["JENNIE", "LISA", "ROSÉ", "JISOO"];

    log.push(format!("kitty_keyboard: {kitty}"));

    let config = RunConfig::default()
        .tick_rate(Duration::from_millis(33))
        .mouse(true)
        .kitty_keyboard(kitty)
        .max_fps(30);

    slt::run_with(config, |ui| {
        if ui.key_code(KeyCode::BackTab) {
            log.push("BackTab".into());
            mode = (mode + 1) % 4;
        }
        if ui.key_code(KeyCode::Tab) {
            log.push("Tab".into());
        }
        if ui.key_mod('t', KeyModifiers::CONTROL) {
            log.push("Ctrl+T".into());
        }
        if ui.key_mod('c', KeyModifiers::CONTROL) {
            log.push("Ctrl+C".into());
            ui.quit();
        }
        if ui.key_code(KeyCode::Esc) {
            log.push("Esc".into());
        }
        if ui.key_code(KeyCode::F(1)) {
            log.push("F1".into());
        }
        if ui.key_code(KeyCode::F(2)) {
            log.push("F2".into());
            mode = (mode + 1) % 4;
        }

        let _ = ui.col(|ui| {
            let _ = ui.container().h(1).px(1).row(|ui| {
                let resp = ui.container().row(|ui| {
                    ui.text(format!(" {} ", modes[mode as usize]))
                        .bold()
                        .fg(Color::Cyan);
                });
                if resp.clicked {
                    log.push("Click -> mode".into());
                    mode = (mode + 1) % 4;
                }
                ui.text(" . ").dim();
                ui.text(format!("kitty={kitty}")).fg(Color::Yellow);
                ui.spacer();
                ui.text("SLT 0.15 Key Test").dim();
            });
            ui.separator();

            let _ = ui.container().grow(1).p(1).col(|ui| {
                ui.text("Events:").bold();
                ui.text("");
                for line in log.iter().rev().take(20) {
                    ui.text(line).fg(Color::Green);
                }
            });
            ui.separator();

            let _ = ui.container().px(1).pb(1).col(|ui| {
                let _ = ui.text_input(&mut input);
                ui.text("Shift+Tab/F2=mode | Esc | Ctrl+T | Ctrl+C=quit | Click [MODE]")
                    .dim();
            });
        });
    })
}
