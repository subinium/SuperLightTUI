//! v0.20.0 demo: `register_focusable_named` + `focus_by_name`.
//!
//! Three named text inputs ("name", "email", "city") plus a button row that
//! focuses each input by name. `Tab` / `Shift+Tab` cycle through the inputs
//! positionally; clicking a "Focus" button uses `focus_by_name(...)` to
//! jump directly without caring about render order.
//!
//! Run: `cargo run --example v020_named_focus`

use slt::widgets::TextInputState;
use slt::{Border, Color, Context, KeyCode};

fn main() -> std::io::Result<()> {
    let mut name = TextInputState::default();
    let mut email = TextInputState::default();
    let mut city = TextInputState::default();

    slt::run(move |ui: &mut Context| {
        if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
            return;
        }

        let _ = ui
            .bordered(Border::Rounded)
            .title("register_focusable_named + focus_by_name")
            .p(1)
            .gap(1)
            .col(|ui| {
                ui.text("Tab/Shift+Tab = cycle  Click [Focus X] = jump by name  Ctrl+Q = quit")
                    .dim();
                let current = ui.focused_name().map(|s| s.to_string());
                ui.text(format!(
                    "focused_name: {}",
                    current.as_deref().unwrap_or("(none)")
                ))
                .fg(Color::Cyan);

                // Three named inputs. They can be rendered in any order — the
                // names don't depend on the registration position.
                let _ = ui.col(|ui| {
                    ui.text("Name:");
                    let _ = ui.register_focusable_named("name");
                    let _ = ui.text_input(&mut name);
                    ui.text("Email:");
                    let _ = ui.register_focusable_named("email");
                    let _ = ui.text_input(&mut email);
                    ui.text("City:");
                    let _ = ui.register_focusable_named("city");
                    let _ = ui.text_input(&mut city);
                });

                // Three focus buttons. Each one targets a name. `focus_by_name`
                // resolves against the previous frame's map → next frame the
                // requested input has focus.
                let _ = ui.row_gap(1, |ui| {
                    if ui.button("Focus name").clicked {
                        let _ = ui.focus_by_name("name");
                    }
                    if ui.button("Focus email").clicked {
                        let _ = ui.focus_by_name("email");
                    }
                    if ui.button("Focus city").clicked {
                        let _ = ui.focus_by_name("city");
                    }
                });
            });
    })
}
