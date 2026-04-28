//! v0.20.0 #213 — breadcrumb collapsed to BreadcrumbResponse.
//!
//! Demo: navigate via Tab to focus a segment, press Enter to "click" it.
//! `BreadcrumbResponse` derefs to `Response` so `.hovered`, `.rect`,
//! `.focused` work directly. `clicked_segment` carries the tapped index.

use slt::{Border, Color, Context, KeyCode};

fn main() -> std::io::Result<()> {
    let segments = ["Home", "Projects", "SuperLightTUI", "v0.20.0"];
    let mut current = segments.len() - 1;

    slt::run_with(slt::RunConfig::default().mouse(true), |ui: &mut Context| {
        if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }

        let _ = ui
            .bordered(Border::Rounded)
            .title("v0.20.0 #213 — BreadcrumbResponse")
            .p(1)
            .gap(1)
            .col(|ui| {
                ui.text("Tab/Shift-Tab to focus a segment, Enter to navigate.")
                    .fg(Color::Cyan);

                let visible: Vec<&str> = segments.iter().take(current + 1).copied().collect();
                let r = ui.breadcrumb(&visible);
                if let Some(i) = r.clicked_segment {
                    current = i;
                }
                if r.hovered {
                    ui.text("  (whole bar hovered)").fg(Color::Yellow);
                }
                ui.text(format!("Current segment: {}", segments[current]))
                    .dim();

                ui.text("Ctrl+Q = quit").dim();
            });
    })
}
