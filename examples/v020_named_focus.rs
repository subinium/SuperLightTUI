//! v0.20.0 named-focus demo — register_focusable_named + focus_by_name.
//!
//! Demonstrates: #217
//!
//! Three text inputs ("name", "email", "city") and three "Focus X"
//! buttons. Tab / Shift-Tab cycle through inputs positionally; clicking a
//! button calls `focus_by_name(...)` to jump directly without caring
//! about render order. The current focus name is echoed at the top —
//! it stays in sync because `focused_name()` reads the resolved name from
//! the previous frame's name map.
//!
//! Run: `cargo run --example v020_named_focus`
//!
//! Keys:
//!   Tab            — focus next input (positional)
//!   Shift-Tab      — focus previous input (positional)
//!   Click [Focus N]— jump focus to that named input
//!   Ctrl-Q / Esc   — quit
//!
//! Layout:
//!   ┌── register_focusable_named + focus_by_name ──┐
//!   │ help line                                     │
//!   │ focused_name: city                            │
//!   │ Name:  [_________]                            │
//!   │ Email: [_________]                            │
//!   │ City:  [_________]                            │
//!   │ [ Focus name ] [ Focus email ] [ Focus city ] │
//!   └───────────────────────────────────────────────┘

use slt::widgets::TextInputState;
use slt::{Border, Color, Context, KeyCode, KeyModifiers};

/// Persistent inputs across frames. Each `TextInputState` carries its own
/// cursor and selection, so we can't substitute a bare `String` here.
#[derive(Default)]
pub struct DemoState {
    pub name: TextInputState,
    pub email: TextInputState,
    pub city: TextInputState,
}

fn main() -> std::io::Result<()> {
    let mut state = DemoState::default();
    slt::run(move |ui: &mut Context| render(ui, &mut state))
}

/// Render one frame of the named-focus demo.
///
/// Public so snapshot tests can pin a specific focus state without
/// reimplementing the input layout.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    if ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
        ui.quit();
        return;
    }

    let pad = ui.spacing().xs();
    let gap = ui.spacing().xs();
    let row_gap = ui.spacing().xs();

    let _ = ui
        .bordered(Border::Rounded)
        .title("register_focusable_named + focus_by_name")
        .p(pad)
        .gap(gap)
        .col(|ui| {
            ui.text("Tab/Shift+Tab = cycle   Click [Focus X] = jump by name   Ctrl+Q = quit")
                .dim();

            // `focused_name` resolves against the previous frame's map, so
            // it lags one frame on the very first focus_by_name call —
            // acceptable because the next frame already shows the new name.
            let current = ui.focused_name().map(|s| s.to_string());
            let label = current.as_deref().unwrap_or("(none)");
            ui.text(format!("focused_name: {label}")).fg(Color::Cyan);

            render_input_rows(ui, state);
            render_focus_buttons(ui, row_gap);
        });
}

/// Render the three named inputs. The order here is positional Tab order;
/// names are independent of visual position and persist across re-orders.
fn render_input_rows(ui: &mut Context, state: &mut DemoState) {
    let _ = ui.col(|ui| {
        ui.text("Name:");
        let _ = ui.register_focusable_named("name");
        let _ = ui.text_input(&mut state.name);

        ui.text("Email:");
        let _ = ui.register_focusable_named("email");
        let _ = ui.text_input(&mut state.email);

        ui.text("City:");
        let _ = ui.register_focusable_named("city");
        let _ = ui.text_input(&mut state.city);
    });
}

/// Render three focus buttons. Each button targets a name; clicking it
/// asks the focus system to jump on the next frame.
fn render_focus_buttons(ui: &mut Context, gap: u32) {
    let _ = ui.row_gap(gap, |ui| {
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
}
