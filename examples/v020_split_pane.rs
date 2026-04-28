//! v0.20.0 #223 — split_pane / vsplit_pane.
//!
//! Demo: a horizontal split with a draggable handle. Tab to focus the handle,
//! Left/Right to grow/shrink the left pane. Mouse drag also works. Press 'v'
//! to toggle to a vertical split.

use slt::{Border, Color, Context, KeyCode, SplitPaneState};

fn main() -> std::io::Result<()> {
    let mut split = SplitPaneState::new(0.4);
    let mut vertical = false;

    slt::run_with(slt::RunConfig::default().mouse(true), |ui: &mut Context| {
        if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        if ui.key('v') {
            vertical = !vertical;
        }

        let _ = ui
            .bordered(Border::Rounded)
            .title(if vertical {
                "v0.20.0 #223 — vsplit_pane (vertical)"
            } else {
                "v0.20.0 #223 — split_pane (horizontal)"
            })
            .p(1)
            .gap(1)
            .grow(1)
            .col(|ui| {
                ui.text(
                    "Tab focus handle, ←/→ adjusts ratio. 'v' toggles orientation. Ctrl+Q quits.",
                )
                .fg(Color::Cyan);

                if vertical {
                    let r = ui.vsplit_pane(
                        &mut split,
                        |ui| {
                            ui.text("TOP PANE").bold();
                            ui.text("Drag the handle below or arrow-key it.");
                        },
                        |ui| {
                            ui.text("BOTTOM PANE").bold();
                            ui.text("Status: ratio updates live.");
                        },
                    );
                    ui.text(format!(
                        "ratio = {:.2}  drag_active = {}",
                        r.ratio, r.drag_active
                    ))
                    .dim();
                } else {
                    let r = ui.split_pane(
                        &mut split,
                        |ui| {
                            ui.text("LEFT PANE").bold();
                            ui.text("Drag the handle right of this pane.");
                        },
                        |ui| {
                            ui.text("RIGHT PANE").bold();
                            ui.text("Or use ←/→ when handle is focused.");
                        },
                    );
                    ui.text(format!(
                        "ratio = {:.2}  drag_active = {}",
                        r.ratio, r.drag_active
                    ))
                    .dim();
                }
            });
    })
}
