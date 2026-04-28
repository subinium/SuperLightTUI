//! v0.20.0 modal focus-trap demo — Tab cycles inside the modal and never
//! escapes to background widgets (WCAG 2.1 SC 2.4.3).
//!
//! Demonstrates: #225.
//!
//! Run: `cargo run --example v020_modal_trap`
//!
//! Keys:
//!   Tab / Shift-Tab — cycle focus (only inside modal once it is open)
//!   Enter           — activate the focused button
//!   Esc             — close the modal (or quit, when no modal is open)
//!   Ctrl-Q          — quit
//!
//! Layout:
//!   ┌── main view ────────────────────────┐
//!   │ [bg btn] [bg btn] [bg btn]          │      ┌── modal ──┐
//!   │ [Open modal]                        │      │ Confirm   │
//!   │ Last answer: …                      │      │ [Yes][No] │
//!   └─────────────────────────────────────┘      └───────────┘

use slt::{context::ModalOptions, Border, ButtonVariant, Context, KeyCode, KeyModifiers};

/// Mutable demo state. Bundling these into a struct keeps `main()` minimal
/// and lets `render` synthesise a deterministic snapshot frame without
/// duplicating field-by-field defaults.
pub struct State {
    pub show_modal: bool,
    pub answered: Option<bool>,
}

impl State {
    pub fn new() -> Self {
        Self {
            show_modal: false,
            answered: None,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared body. Padding/gap come from `theme.spacing` — when the embedding
/// app overrides the theme, this demo's density follows automatically.
fn body(ui: &mut Context, state: &mut State) {
    let sp = ui.spacing();
    let _ = ui
        .bordered(Border::Rounded)
        .title("SLT v0.20 — Modal focus trap")
        .p(sp.sm())
        .gap(sp.xs())
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

            // Three throwaway background buttons. They exist so the user can
            // see that focus genuinely escapes the modal scope when no trap
            // is active — and is held captive once the modal opens.
            let _ = ui.row_gap(sp.sm(), |ui| {
                let _ = ui.button("First bg button");
                let _ = ui.button("Second bg button");
                let _ = ui.button("Third bg button");
            });

            ui.text("");
            if ui.button_with("Open modal", ButtonVariant::Primary).clicked {
                state.show_modal = true;
                state.answered = None;
            }
            if let Some(a) = state.answered {
                let label = if a { "Yes" } else { "No" };
                ui.text(format!("Last answer: {label}")).dim();
            }
        });

    if state.show_modal {
        // tab_trap = true is the load-bearing line: focus cannot leave the
        // modal even if a stray click hits a background widget rect.
        let _ = ui.modal_with(ModalOptions { tab_trap: true }, |ui| {
            let _ = ui
                .bordered(Border::Rounded)
                .title("Confirm")
                .p(sp.sm())
                .gap(sp.xs())
                .col(|ui| {
                    ui.text("Press Tab — focus stays inside the modal.").bold();
                    let _ = ui.row_gap(sp.sm(), |ui| {
                        if ui.button_with("Yes", ButtonVariant::Primary).clicked {
                            state.answered = Some(true);
                            state.show_modal = false;
                        }
                        if ui.button_with("No", ButtonVariant::Outline).clicked {
                            state.answered = Some(false);
                            state.show_modal = false;
                        }
                    });
                    ui.text("Esc to dismiss.").dim();
                });
        });
    }
}

/// One-frame deterministic render entry point used by snapshot tests.
/// Renders the modal-open state because that is the interesting case
/// — the closed-modal frame is just the bordered base view.
pub fn render(ui: &mut Context) {
    let mut state = State {
        show_modal: true,
        answered: None,
    };
    body(ui, &mut state);
}

fn main() -> std::io::Result<()> {
    let mut state = State::new();

    slt::run(|ui: &mut Context| {
        if ui.key_mod('q', KeyModifiers::CONTROL) {
            ui.quit();
        }
        // Esc closes the modal first; only quits when no modal is open.
        // raw_key_code below sees the same press unfiltered, so we guard
        // with `!show_modal` here to avoid a double-handle.
        if ui.key_code(KeyCode::Esc) && !state.show_modal {
            ui.quit();
        }

        body(ui, &mut state);

        // Modal-scoped Esc-to-dismiss. raw_key_code bypasses focus filtering
        // so Esc still works even when a modal button has focus.
        if state.show_modal && ui.raw_key_code(KeyCode::Esc) {
            state.show_modal = false;
        }
    })
}
