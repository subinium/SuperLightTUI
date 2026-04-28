//! v0.20.0 #224 — gauge / line_gauge.
//!
//! Demo: animated gauges showing CPU/Memory/Disk values, plus three line
//! gauges with custom characters and inline labels. The `gauge` color
//! tier is automatic: green < 50%, yellow 50–80%, red >= 80%.
//!
//! Builder API (v0.20.0 consistency pass):
//!
//! ```text
//! ui.gauge(0.6).label("60%").width(24)
//! ui.line_gauge(0.6).label("60%").width(24).filled('━')
//! ```

use slt::{Border, Color, Context, KeyCode};

fn main() -> std::io::Result<()> {
    let mut tick: u64 = 0;

    slt::run_with(slt::RunConfig::default().mouse(true), |ui: &mut Context| {
        if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        tick += 1;
        let t = tick as f64 / 60.0;
        let cpu = (t.sin().abs() * 0.3 + 0.6).clamp(0.0, 1.0); // 60–90%
        let memory = (t * 0.5).cos().abs() * 0.4 + 0.4; // 40–80%
        let disk: f64 = 0.25; // steady

        let _ = ui
            .bordered(Border::Rounded)
            .title("v0.20.0 #224 — gauge / line_gauge")
            .p(1)
            .gap(1)
            .col(|ui| {
                ui.text("Block-style gauge with inline label (color-tiered):")
                    .fg(Color::Cyan);

                let _ = ui.row_gap(2, |ui| {
                    ui.text("CPU   ");
                    ui.gauge(cpu)
                        .label(&format!("{:.0}%", cpu * 100.0))
                        .width(24);
                });
                let _ = ui.row_gap(2, |ui| {
                    ui.text("MEM   ");
                    ui.gauge(memory)
                        .label(&format!("{:.0}%", memory * 100.0))
                        .width(24);
                });
                let _ = ui.row_gap(2, |ui| {
                    ui.text("DISK  ");
                    ui.gauge(disk)
                        .label(&format!("{:.0}%", disk * 100.0))
                        .width(24);
                });

                ui.text("");
                ui.text("Single-line gauge with custom characters:")
                    .fg(Color::Cyan);
                let _ = ui.row_gap(2, |ui| {
                    ui.text("Default  ");
                    ui.line_gauge(0.6).label("60%").width(24);
                });
                let _ = ui.row_gap(2, |ui| {
                    ui.text("Hash/dot ");
                    ui.line_gauge(0.45)
                        .filled('#')
                        .empty('.')
                        .width(24)
                        .label("45%");
                });
                let _ = ui.row_gap(2, |ui| {
                    ui.text("Block    ");
                    ui.line_gauge(0.85)
                        .filled('█')
                        .empty('▒')
                        .width(24)
                        .label("85%");
                });

                ui.text("Ctrl+Q = quit").dim();
            });
    })
}
