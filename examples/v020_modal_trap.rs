//! Modal focus trap (#225).
//!
//! Demonstrates `ModalOptions::tab_trap` — Tab cycles within the modal
//! and never escapes to background widgets, complying with WCAG 2.1
//! SC 2.4.3 (Focus Order).
//!
//! Run: `cargo run --example v020_modal_trap`
//! Open the modal with the "Open modal" button, then Tab through the
//! Yes/No buttons. Focus stays inside the modal regardless of how many
//! Tab presses you fire. Esc dismisses the modal. Ctrl+Q to quit.

use slt::{context::ModalOptions, Border, ButtonVariant, Context, KeyCode, KeyModifiers};

fn main() -> std::io::Result<()> {
    let mut show_modal = false;
    let mut answered: Option<bool> = None;

    slt::run(|ui: &mut Context| {
        if ui.key_mod('q', KeyModifiers::CONTROL) {
            ui.quit();
        }
        if ui.key_code(KeyCode::Esc) && !show_modal {
            ui.quit();
        }

        let _ = ui
            .bordered(Border::Rounded)
            .title("SLT v0.20 — Modal focus trap")
            .p(2)
            .gap(1)
            .grow(1)
            .col(|ui| {
                ui.text("Tab cycles through the BACKGROUND focusables right now.")
                    .dim();
                ui.text("Open the modal — Tab will then cycle ONLY between Yes/No.")
                    .dim();
                ui.text("Click outside or press a Tab while focused on a background")
                    .dim();
                ui.text("widget: focus is yanked back inside the modal.")
                    .dim();

                let _ = ui.row_gap(2, |ui| {
                    let _ = ui.button("First bg button");
                    let _ = ui.button("Second bg button");
                    let _ = ui.button("Third bg button");
                });

                ui.text("");
                if ui.button_with("Open modal", ButtonVariant::Primary).clicked {
                    show_modal = true;
                    answered = None;
                }
                if let Some(a) = answered {
                    let label = if a { "Yes" } else { "No" };
                    ui.text(format!("Last answer: {label}")).dim();
                }
            });

        if show_modal {
            // tab_trap: true — focus cannot escape the modal even if a stray
            // click hits a background widget rect.
            let _ = ui.modal_with(ModalOptions { tab_trap: true }, |ui| {
                let _ = ui
                    .bordered(Border::Rounded)
                    .title("Confirm")
                    .p(2)
                    .gap(1)
                    .col(|ui| {
                        ui.text("Press Tab — focus stays inside the modal.").bold();
                        let _ = ui.row_gap(2, |ui| {
                            if ui.button_with("Yes", ButtonVariant::Primary).clicked {
                                answered = Some(true);
                                show_modal = false;
                            }
                            if ui.button_with("No", ButtonVariant::Outline).clicked {
                                answered = Some(false);
                                show_modal = false;
                            }
                        });
                        ui.text("Esc to dismiss.").dim();
                    });
            });
            if ui.raw_key_code(KeyCode::Esc) {
                show_modal = false;
            }
        }
    })
}
