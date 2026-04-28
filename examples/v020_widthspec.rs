//! v0.20.0 WidthSpec demo — five constraint variants stacked side-by-side.
//!
//! Demonstrates: #237 (unified WidthSpec / HeightSpec enum, with helpers
//! for Fixed / Pct / Ratio / MinMax / Auto).
//!
//! Run: `cargo run --example v020_widthspec`
//!
//! Keys:
//!   q / Ctrl-C / Esc — quit
//!
//! Layout (80x12 minimum):
//!
//! ```text
//! +- WidthSpec showcase --------------------------------------------+
//! | Each row demonstrates a WidthSpec variant.                      |
//! | Fixed(20)            +- WidthSpec::Fixed(20)  -+                |
//! | Pct(50)              +- WidthSpec::Pct(50)  -----------------+  |
//! | Ratio(1, 3)          +- WidthSpec::Ratio(1, 3)  -+              |
//! | MinMax { 10..=30 }   +- WidthSpec::MinMax { 10..=30 }  --+      |
//! | Auto (content)       +- WidthSpec::Auto -+                      |
//! +-----------------------------------------------------------------+
//! ```

use slt::{Border, Color, Constraints, Context, KeyCode, KeyModifiers, RunConfig};

// Label column width. Pinned so every variant's left-hand label aligns
// regardless of how its right-hand container resolves.
const LABEL_W: usize = 20;

// MinMax bounds for the demo's MinMax variant. Lifted into constants so
// the doc-comment layout, the label, and the constraint stay in sync.
const MINMAX_LO: u32 = 10;
const MINMAX_HI: u32 = 30;

fn main() -> std::io::Result<()> {
    slt::run_with(RunConfig::default().mouse(true), |ui: &mut Context| {
        if ui.key('q') || ui.key_mod('c', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }

        render(ui);
    })
}

/// Render one full WidthSpec showcase frame.
///
/// Public so the snapshot test in `tests/v020_widthspec_demo.rs` (and any
/// future visual regression coverage) can pin a deterministic frame.
pub fn render(ui: &mut Context) {
    let theme = ui.theme();
    let pad = theme.spacing.xs();

    let _ = ui
        .bordered(Border::Rounded)
        .title("WidthSpec showcase")
        .p(pad)
        .col(|ui| {
            ui.text("Each row demonstrates a WidthSpec variant.")
                .fg(Color::Cyan)
                .bold();
            ui.text("(Press q or Ctrl+C to quit)").dim();

            // Fixed(20) — exact column count.
            row(ui, "Fixed(20)", |ui| {
                let _ = ui
                    .bordered(Border::Single)
                    .constraints(Constraints::default().w(20))
                    .col(|ui| {
                        ui.text("WidthSpec::Fixed(20)").fg(Color::Yellow);
                    });
            });

            // Pct(50) — half of the parent width.
            row(ui, "Pct(50)", |ui| {
                let _ = ui
                    .bordered(Border::Single)
                    .constraints(Constraints::default().w_pct(50))
                    .col(|ui| {
                        ui.text("WidthSpec::Pct(50)").fg(Color::Green);
                    });
            });

            // Ratio(1, 3) — exact 1/3 of the parent width.
            row(ui, "Ratio(1, 3)", |ui| {
                let _ = ui
                    .bordered(Border::Single)
                    .constraints(Constraints::default().w_ratio(1, 3))
                    .col(|ui| {
                        ui.text("WidthSpec::Ratio(1, 3)").fg(Color::Magenta);
                    });
            });

            // MinMax { 10..=30 } — clamps to the inclusive range.
            row(ui, "MinMax { 10..=30 }", |ui| {
                let _ = ui
                    .bordered(Border::Single)
                    .constraints(Constraints::default().w_minmax(MINMAX_LO, MINMAX_HI))
                    .col(|ui| {
                        ui.text("WidthSpec::MinMax { 10..=30 }").fg(Color::Blue);
                    });
            });

            // Auto — sized to fit content (the default when no width spec
            // is supplied).
            row(ui, "Auto (content)", |ui| {
                let _ = ui.bordered(Border::Single).col(|ui| {
                    ui.text("WidthSpec::Auto").fg(Color::White);
                });
            });
        });
}

// One labeled row. The label column is pinned so every variant's content
// box starts at the same x — readers can compare resolved widths visually
// without measuring against a moving baseline.
fn row<F: FnOnce(&mut Context)>(ui: &mut Context, label: &str, content: F) {
    let _ = ui.row_gap(1, |ui| {
        ui.text(format!("{label:<width$}", width = LABEL_W)).bold();
        content(ui);
    });
}
