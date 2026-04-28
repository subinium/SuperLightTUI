//! Density preset side-by-side (#227).
//!
//! Three columns render the SAME widgets under three different `Theme`
//! spacing presets (compact / comfortable / spacious). Padding, gaps, and
//! margins flow from `theme.spacing` so the widgets visibly differ in
//! density without any per-widget tweaks.
//!
//! Run: `cargo run --example v020_spacing_scale`
//! Quit: Ctrl+Q or Esc.

use slt::{Border, Context, KeyCode, KeyModifiers, Theme};

fn main() -> std::io::Result<()> {
    slt::run(|ui: &mut Context| {
        if ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }

        let _ = ui
            .bordered(Border::Rounded)
            .title("SLT v0.20 — Density presets")
            .p(1)
            .grow(1)
            .col(|ui| {
                ui.text("Same widgets, three Theme presets — note the widening padding.")
                    .dim();
                ui.text("compact = base 1, comfortable = base 2, spacious = base 3.")
                    .dim();

                let _ = ui.row_gap(2, |ui| {
                    panel(ui, "compact", Theme::compact());
                    panel(ui, "comfortable", Theme::comfortable());
                    panel(ui, "spacious", Theme::spacious());
                });
            });
    })
}

fn panel(ui: &mut Context, label: &str, theme: Theme) {
    let _ = ui
        .container()
        .theme(theme)
        .border(Border::Rounded)
        .title(label)
        .grow(1)
        .col(|ui| {
            let _ = ui.help(&[("Tab", "next"), ("Enter", "ok"), ("Esc", "cancel")]);
            ui.text("");
            let _ = ui.button("Click me");
            ui.text("");
            let _ = ui.code_block("fn main() {\n    println!(\"hi\");\n}");
        });
}
