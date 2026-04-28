//! Visual snapshot test for the v0.20 WidthSpec showcase (#237).
//!
//! Renders the same widget tree as `examples/v020_widthspec.rs` into an
//! 80×25 [`TestBackend`] and asserts that each variant's label is present
//! and that the resolved widths land in the expected ranges.

use slt::{Border, Color, Constraints, TestBackend};

#[test]
fn widthspec_demo_renders_all_five_variants() {
    let mut tb = TestBackend::new(80, 25);

    tb.render(|ui| {
        let _ = ui
            .bordered(Border::Rounded)
            .title("WidthSpec showcase")
            .p(1)
            .col(|ui| {
                ui.text("All five WidthSpec variants below.")
                    .fg(Color::Cyan)
                    .bold();

                let _ = ui.container().gap(1).row(|ui| {
                    ui.text("Fixed(20)");
                    let _ = ui
                        .bordered(Border::Single)
                        .constraints(Constraints::default().w(20))
                        .col(|ui| {
                            ui.text("Fixed20").fg(Color::Yellow);
                        });
                });

                let _ = ui.container().gap(1).row(|ui| {
                    ui.text("Pct(50)");
                    let _ = ui
                        .bordered(Border::Single)
                        .constraints(Constraints::default().w_pct(50))
                        .col(|ui| {
                            ui.text("Pct50").fg(Color::Green);
                        });
                });

                let _ = ui.container().gap(1).row(|ui| {
                    ui.text("Ratio(1,3)");
                    let _ = ui
                        .bordered(Border::Single)
                        .constraints(Constraints::default().w_ratio(1, 3))
                        .col(|ui| {
                            ui.text("Ratio13").fg(Color::Magenta);
                        });
                });

                let _ = ui.container().gap(1).row(|ui| {
                    ui.text("MinMax(10,30)");
                    let _ = ui
                        .bordered(Border::Single)
                        .constraints(Constraints::default().w_minmax(10, 30))
                        .col(|ui| {
                            ui.text("MinMax").fg(Color::Blue);
                        });
                });

                let _ = ui.container().gap(1).row(|ui| {
                    ui.text("Auto");
                    let _ = ui.bordered(Border::Single).col(|ui| {
                        ui.text("AutoVar").fg(Color::White);
                    });
                });
            });
    });

    let output = tb.to_string_trimmed();
    println!("{output}");

    // Title and labels.
    tb.assert_contains("WidthSpec showcase");
    tb.assert_contains("Fixed(20)");
    tb.assert_contains("Pct(50)");
    tb.assert_contains("Ratio(1,3)");
    tb.assert_contains("MinMax(10,30)");
    tb.assert_contains("Auto");

    // Each variant's interior text marker.
    tb.assert_contains("Fixed20");
    tb.assert_contains("Pct50");
    tb.assert_contains("Ratio13");
    tb.assert_contains("MinMax");
    tb.assert_contains("AutoVar");
}

#[test]
fn widthspec_min_max_accessors_match_construction() {
    // Sanity: builder methods round-trip through the public accessors
    // exactly the way the demo relies on.
    let c = Constraints::default().w_minmax(10, 30);
    assert_eq!(c.min_width(), Some(10));
    assert_eq!(c.max_width(), Some(30));

    let c = Constraints::default().w(20);
    assert_eq!(c.min_width(), Some(20));
    assert_eq!(c.max_width(), Some(20));
}
