//! v0.20.0 gauge / line_gauge demo — block-fill and line-style progress bars.
//!
//! Demonstrates: #224 (gauge / line_gauge builder API, color-tiered fills,
//! custom characters, `f64` ratios).
//!
//! Run: `cargo run --example v020_gauge`
//!
//! Keys:
//!   Ctrl-Q / Esc — quit
//!
//! Builder API (post v0.20.0 consistency pass):
//!   ui.gauge(0.6).label("60%").width(24)
//!   ui.line_gauge(0.6).label("60%").width(24).filled('━')
//!
//! Color tiers are automatic: success below 50%, warning 50–80%, error >= 80%.

use slt::{Border, Color, Context, KeyCode, KeyModifiers, RunConfig};

fn main() -> std::io::Result<()> {
    let mut state = DemoState::default();

    slt::run_with(RunConfig::default().mouse(true), |ui: &mut Context| {
        if ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        state.tick = state.tick.wrapping_add(1);
        render(ui, &mut state);
    })
}

/// Animated demo state — one frame counter drives all three live gauges.
#[derive(Default)]
pub struct DemoState {
    /// Frame counter; converted to seconds for the sin/cos animations.
    pub tick: u64,
}

/// Render one frame. Stable signature for snapshot tests.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    let sp = ui.spacing();
    let t = state.tick as f64 / 60.0;

    // Live values: CPU oscillates 60–90%, memory 40–80%, disk steady at 25%.
    let cpu = (t.sin().abs() * 0.3 + 0.6).clamp(0.0, 1.0);
    let memory = ((t * 0.5).cos().abs() * 0.4 + 0.4).clamp(0.0, 1.0);
    let disk: f64 = 0.25;

    let _ = ui
        .bordered(Border::Rounded)
        .title("v0.20.0 #224 — gauge / line_gauge")
        .p(sp.xs())
        .gap(sp.xs())
        .col(|ui| {
            ui.text("Block-style gauge with inline label (color-tiered):")
                .fg(Color::Cyan);

            metric_row(ui, "CPU  ", cpu);
            metric_row(ui, "MEM  ", memory);
            metric_row(ui, "DISK ", disk);

            ui.text("");
            ui.text("Single-line gauge with custom characters:")
                .fg(Color::Cyan);

            let _ = ui.row_gap(sp.sm(), |ui| {
                ui.text("Default  ");
                ui.line_gauge(0.6).label("60%").width(24);
            });
            let _ = ui.row_gap(sp.sm(), |ui| {
                ui.text("Hash/dot ");
                ui.line_gauge(0.45)
                    .filled('#')
                    .empty('.')
                    .width(24)
                    .label("45%");
            });
            let _ = ui.row_gap(sp.sm(), |ui| {
                ui.text("Block    ");
                ui.line_gauge(0.85)
                    .filled('█')
                    .empty('▒')
                    .width(24)
                    .label("85%");
            });

            ui.text("Ctrl-Q / Esc quits.").dim();
        });
}

/// Render a single labelled `gauge` row with an auto-formatted percentage.
fn metric_row(ui: &mut Context, label: &str, value: f64) {
    let sp = ui.spacing();
    let _ = ui.row_gap(sp.sm(), |ui| {
        ui.text(label);
        ui.gauge(value)
            .label(&format!("{:.0}%", value * 100.0))
            .width(24);
    });
}
