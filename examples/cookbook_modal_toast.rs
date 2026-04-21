//! Cookbook: confirmation modal with toast feedback.
//!
//! Demonstrates:
//! - opening a modal from a button click
//! - rendering Yes / No buttons inside the modal
//! - pushing toasts via `ToastState::success` / `::info`
//! - `ui.toast(&mut toasts)` auto-expires old messages
//! - Ctrl+Q or Esc to quit
//!
//! Toast duration is expressed in ticks (default tick rate ~60fps), so the
//! messages dismiss in roughly half a second once pushed.

use slt::{Border, ButtonVariant, Color, Context, KeyCode, KeyModifiers, ToastState};

fn main() -> std::io::Result<()> {
    let mut show_modal = false;
    let mut items_left: u32 = 3;
    let mut toasts = ToastState::new();

    slt::run(|ui: &mut Context| {
        if ui.key_mod('q', KeyModifiers::CONTROL) {
            ui.quit();
        }
        if ui.key_code(KeyCode::Esc) && !show_modal {
            ui.quit();
        }

        let tick = ui.tick();

        let _ = ui
            .bordered(Border::Rounded)
            .title("Cookbook — Modal + Toast")
            .pad(2)
            .gap(1)
            .grow(1)
            .col(|ui| {
                ui.text("Destructive actions need a confirmation modal.")
                    .dim();

                let _ = ui.row_gap(2, |ui| {
                    ui.text(format!("Items remaining: {items_left}"))
                        .bold()
                        .fg(Color::Cyan);
                    ui.spacer();
                    if ui.button_with("Delete item", ButtonVariant::Danger).clicked
                        && items_left > 0
                    {
                        show_modal = true;
                    }
                });

                ui.text("").dim();
                ui.text("Tab to cycle focus. Ctrl+Q to quit.").dim();
            });

        if show_modal {
            let _ = ui.modal(|ui| {
                let _ = ui
                    .bordered(Border::Rounded)
                    .title("Confirm")
                    .pad(2)
                    .gap(1)
                    .col(|ui| {
                        ui.text("Delete this item? This cannot be undone.").bold();
                        let _ = ui.row_gap(2, |ui| {
                            if ui.button_with("Yes", ButtonVariant::Danger).clicked {
                                items_left = items_left.saturating_sub(1);
                                toasts.success("Deleted", tick);
                                show_modal = false;
                            }
                            if ui.button_with("No", ButtonVariant::Outline).clicked {
                                toasts.info("Cancelled", tick);
                                show_modal = false;
                            }
                        });
                        ui.text("Esc to dismiss.").dim();
                    });
            });
            // `key_code` ignores events while a modal is active — use the
            // `raw_*` variants for global/modal-aware shortcuts.
            if ui.raw_key_code(KeyCode::Esc) {
                show_modal = false;
            }
        }

        ui.toast(&mut toasts);
    })
}
