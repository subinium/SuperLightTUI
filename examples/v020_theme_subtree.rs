//! Per-subtree theme override (#226).
//!
//! Three side-by-side panels render the same widgets under three different
//! themes — proving `ContainerBuilder::theme()` correctly scopes the theme
//! change to its own subtree.
//!
//! Run: `cargo run --example v020_theme_subtree`
//! Quit: Ctrl+Q or Esc.

use slt::{Border, Context, KeyCode, KeyModifiers, Theme};

fn main() -> std::io::Result<()> {
    slt::run(|ui: &mut Context| {
        if ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }

        let _ = ui
            .bordered(Border::Rounded)
            .title("SLT v0.20 — Per-subtree theme override")
            .p(1)
            .grow(1)
            .col(|ui| {
                ui.text("Each panel below uses a different Theme via container().theme(...).")
                    .dim();
                ui.text("Outer scope keeps its parent theme — nothing leaks across panels.")
                    .dim();

                let _ = ui.row_gap(2, |ui| {
                    panel(ui, "Dark (default)", Theme::dark());
                    panel(ui, "Light", Theme::light());
                    panel(ui, "Dracula", Theme::dracula());
                    panel(ui, "Nord", Theme::nord());
                });
            });
    })
}

fn panel(ui: &mut Context, label: &str, theme: Theme) {
    let _ = ui
        .container()
        .theme(theme)
        .border(Border::Rounded)
        .p(1)
        .grow(1)
        .col(|ui| {
            // All widgets inside this closure resolve colors against `theme`.
            ui.text(label).bold();
            ui.text("body text").dim();
            let _ = ui.button("Press me");
            let _ = ui.alert("info banner", slt::widgets::AlertLevel::Info);
            let _ = ui.alert("warning", slt::widgets::AlertLevel::Warning);
            let _ = ui.code_block("let x = 1;");
        });
}
