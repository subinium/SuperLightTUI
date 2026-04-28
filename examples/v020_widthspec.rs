//! v0.20 `WidthSpec` showcase (#237).
//!
//! Stacks five rows side-by-side, each illustrating one variant of the
//! unified `WidthSpec` enum. Run with:
//!
//! ```sh
//! cargo run --example v020_widthspec
//! ```
//!
//! Press `q` or `Ctrl+C` to quit.

use slt::{Border, Color, Constraints, Context, KeyCode, KeyModifiers};

fn main() -> std::io::Result<()> {
    slt::run(|ui: &mut Context| {
        if ui.key('q') || ui.key_mod('c', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }

        let _ = ui
            .bordered(Border::Rounded)
            .title("WidthSpec showcase")
            .p(1)
            .col(|ui| {
                ui.text("Each row demonstrates a WidthSpec variant.")
                    .fg(Color::Cyan)
                    .bold();
                ui.text("(Press q or Ctrl+C to quit)").dim();

                // Row 1: Fixed(20)
                row(ui, "Fixed(20)", |ui| {
                    let _ = ui
                        .bordered(Border::Single)
                        .constraints(Constraints::default().w(20))
                        .col(|ui| {
                            ui.text("WidthSpec::Fixed(20)").fg(Color::Yellow);
                        });
                });

                // Row 2: Pct(50)
                row(ui, "Pct(50)", |ui| {
                    let _ = ui
                        .bordered(Border::Single)
                        .constraints(Constraints::default().w_pct(50))
                        .col(|ui| {
                            ui.text("WidthSpec::Pct(50)").fg(Color::Green);
                        });
                });

                // Row 3: Ratio(1, 3)
                row(ui, "Ratio(1, 3)", |ui| {
                    let _ = ui
                        .bordered(Border::Single)
                        .constraints(Constraints::default().w_ratio(1, 3))
                        .col(|ui| {
                            ui.text("WidthSpec::Ratio(1, 3)").fg(Color::Magenta);
                        });
                });

                // Row 4: MinMax { min: 10, max: 30 }
                row(ui, "MinMax { 10..=30 }", |ui| {
                    let _ = ui
                        .bordered(Border::Single)
                        .constraints(Constraints::default().w_minmax(10, 30))
                        .col(|ui| {
                            ui.text("WidthSpec::MinMax { 10..=30 }").fg(Color::Blue);
                        });
                });

                // Row 5: Auto (content-sized)
                row(ui, "Auto (content)", |ui| {
                    let _ = ui.bordered(Border::Single).col(|ui| {
                        ui.text("WidthSpec::Auto").fg(Color::White);
                    });
                });
            });
    })
}

fn row<F: FnOnce(&mut Context)>(ui: &mut Context, label: &str, content: F) {
    let _ = ui.row_gap(1, |ui| {
        ui.text(format!("{label:<20}")).bold();
        content(ui);
    });
}
