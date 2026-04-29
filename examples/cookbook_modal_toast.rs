//! Cookbook: confirmation modal with toast feedback.
//!
//! Archetype: **Overlay-first** (full-canvas + a centered modal that
//! claims overlay z-order while open). Toasts also draw in overlay
//! space.
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
//!
//! §2 (Demo Guide): exposes `pub fn render(ui, &mut DemoState)` so a
//! composing demo can preserve `show_modal`, the items counter, and the
//! pending toasts across tab switches. The earlier stateless form ate
//! every modal click because state reset every frame — see Demo Guide
//! §2 for the canonical fix.
//!
//! §4 modal-aware: Esc-to-dismiss inside the modal uses
//! `raw_key_code(Esc)` so the global Esc-to-quit doesn't fire when a
//! modal is open. The standalone `main()` gates `ui.key_code(Esc)` on
//! `!state.show_modal` for the same reason.

use slt::{Border, ButtonVariant, Color, Context, KeyCode, KeyModifiers, ToastState};

/// Persistent demo state. `show_modal` and `items_left` survive across
/// frames; `toasts` retains pending messages until they expire.
pub struct DemoState {
    pub show_modal: bool,
    pub items_left: u32,
    pub toasts: ToastState,
}

impl DemoState {
    pub fn new() -> Self {
        Self {
            show_modal: false,
            items_left: 3,
            toasts: ToastState::new(),
        }
    }
}

impl Default for DemoState {
    fn default() -> Self {
        Self::new()
    }
}

/// Render one frame of the modal-toast demo. Caller owns the modal
/// open/closed flag so a click on Yes/No resolves instead of being
/// thrown away on the next frame.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    let tick = ui.tick();

    let _ = ui
        .bordered(Border::Rounded)
        .title("Cookbook: Modal + Toast")
        .p(2)
        .gap(1)
        .grow(1)
        .col(|ui| {
            ui.text("Destructive actions need a confirmation modal.")
                .dim();

            let _ = ui.container().gap(2).row(|ui| {
                ui.text(format!("Items remaining: {}", state.items_left))
                    .bold()
                    .fg(Color::Cyan);
                ui.spacer();
                if ui.button_with("Delete item", ButtonVariant::Danger).clicked
                    && state.items_left > 0
                {
                    state.show_modal = true;
                }
            });

            ui.text("").dim();
            ui.text("Tab to cycle focus. Ctrl+Q to quit.").dim();
        });

    if state.show_modal {
        let _ = ui.modal(|ui| {
            let _ = ui
                .bordered(Border::Rounded)
                .title("Confirm")
                .p(2)
                .gap(1)
                .col(|ui| {
                    ui.text("Delete this item? This cannot be undone.").bold();
                    let _ = ui.container().gap(2).row(|ui| {
                        if ui.button_with("Yes", ButtonVariant::Danger).clicked {
                            state.items_left = state.items_left.saturating_sub(1);
                            state.toasts.success("Deleted", tick);
                            state.show_modal = false;
                        }
                        if ui.button_with("No", ButtonVariant::Outline).clicked {
                            state.toasts.info("Cancelled", tick);
                            state.show_modal = false;
                        }
                    });
                    ui.text("Esc to dismiss.").dim();
                });
        });
        // `key_code` ignores events while a modal is active — use the
        // `raw_*` variants for global/modal-aware shortcuts.
        if ui.raw_key_code(KeyCode::Esc) {
            state.show_modal = false;
        }
    }

    ui.toast(&mut state.toasts);
}

fn main() -> std::io::Result<()> {
    let mut state = DemoState::new();
    slt::run(move |ui: &mut Context| {
        if ui.key_mod('q', KeyModifiers::CONTROL) {
            ui.quit();
        }
        // Esc-to-quit is gated on `!show_modal` so the modal's own
        // Esc-to-dismiss path (inside `render`) wins when the modal is
        // open.
        if ui.key_code(KeyCode::Esc) && !state.show_modal {
            ui.quit();
        }
        render(ui, &mut state);
    })
}
