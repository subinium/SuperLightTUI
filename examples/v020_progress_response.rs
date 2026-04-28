//! v0.20.0 #212 — spinner / progress now return Response.
//!
//! Demo: hover the progress bar to show a tooltip, hover the spinner to
//! show the status text. Both interactions are now possible because
//! `ui.spinner()` and `ui.progress()` return `Response` (was `&mut Self`
//! prior to v0.20.0).

use slt::widgets::SpinnerState;
use slt::{Border, Color, Context, KeyCode};

fn main() -> std::io::Result<()> {
    let spinner = SpinnerState::dots();
    let mut ratio: f64 = 0.0;
    let mut step: f64 = 0.01;

    slt::run_with(slt::RunConfig::default().mouse(true), |ui: &mut Context| {
        if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        ratio += step;
        if ratio >= 1.0 {
            ratio = 1.0;
            step = -step;
        } else if ratio <= 0.0 {
            ratio = 0.0;
            step = -step;
        }

        let _ = ui
            .bordered(Border::Rounded)
            .title("v0.20.0 #212 — Response from progress / spinner")
            .p(1)
            .gap(1)
            .col(|ui| {
                ui.text("Hover the spinner or the progress bar.")
                    .fg(Color::Cyan);

                let _ = ui.row(|ui| {
                    let s = ui.spinner(&spinner);
                    ui.text(" Loading...").dim();
                    if s.hovered {
                        ui.text("  (hovered!)").fg(Color::Yellow);
                    }
                });

                let pr = ui.progress(ratio);
                ui.text(format!("ratio = {:.0}%", ratio * 100.0)).dim();
                if pr.hovered {
                    ui.text("  Progress hovered — click would trigger scrubber")
                        .fg(Color::Yellow);
                }

                ui.text("Ctrl+Q = quit").dim();
            });
    })
}
