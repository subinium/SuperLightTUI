//! v0.20.0 named-focus demo — register_focusable_named + focus_by_name.
//!
//! Demonstrates: #217
//!
//! Three text inputs ("name", "email", "city") and three "Focus X"
//! buttons. Tab / Shift-Tab cycle through inputs positionally; clicking a
//! button — or pressing the matching number key — calls
//! `focus_by_name(...)` to jump directly without caring about render
//! order. Clicking a row also routes through `focus_by_name` so the
//! mouse-click experience matches a normal form. The current focus name
//! is echoed at the top — it stays in sync because `focused_name()`
//! reads the resolved name from the previous frame's name map.
//!
//! Run: `cargo run --example v020_tour`
//!
//! Keys:
//!   Tab            — focus next input (positional)
//!   Shift-Tab      — focus previous input (positional)
//!   1 / 2 / 3      — jump focus to name / email / city
//!   Click row      — jump focus to that input by name
//!   Click [Focus N]— jump focus to that named input
//!   Type           — text flows into the focused input
//!   q / Esc / Ctrl-Q — quit (Esc and Ctrl-Q always quit; plain `q`
//!                       quits only when no input has focus, otherwise
//!                       it types into the focused input)
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
use slt::{Border, Color, Context, KeyCode, KeyModifiers, RunConfig};

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
    slt::run_with(RunConfig::default().mouse(true), move |ui: &mut Context| {
        render(ui, &mut state)
    })
}

/// Render one frame of the named-focus demo.
///
/// Public so snapshot tests can pin a specific focus state without
/// reimplementing the input layout.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    // Esc / Ctrl-Q always quit — they never collide with text-input
    // typing, so they can fire before the body renders.
    if ui.key_code(KeyCode::Esc) || ui.key_mod('q', KeyModifiers::CONTROL) {
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
        .grow(1)
        .col(|ui| {
            ui.text("Tab/Shift+Tab cycle   1/2/3 or click row jumps by name   Esc / Ctrl+Q quit")
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

    // Numeric shortcuts and plain `q` are checked AFTER the body so a
    // focused `text_input` consumes typed digits and 'q' first. This
    // means: if no input is focused, `1`/`2`/`3` jump focus and `q`
    // quits; if an input has focus, the keys flow into it as text. Use
    // Esc or Ctrl-Q to quit unconditionally.
    if ui.key('1') {
        let _ = ui.focus_by_name("name");
    }
    if ui.key('2') {
        let _ = ui.focus_by_name("email");
    }
    if ui.key('3') {
        let _ = ui.focus_by_name("city");
    }
    if ui.key('q') {
        ui.quit();
    }
}

/// Render the three named inputs. The order here is positional Tab order;
/// names are independent of visual position and persist across re-orders.
///
/// Each row is wrapped in a clickable container. Clicking anywhere in the
/// row routes through `focus_by_name` so mouse focus matches keyboard
/// focus — text_input itself doesn't claim focus on click.
fn render_input_rows(ui: &mut Context, state: &mut DemoState) {
    let _ = ui.col(|ui| {
        let row_gap = ui.spacing().xs();

        // The wrapping `container().fill().col()` has its own hit area, so
        // we read its `clicked` from inside the row closure (via the
        // returned Response) rather than the row's. The outer
        // `row_gap(...)` Response also captures clicks on the bare
        // "Name:"/"Email:"/"City:" label cells outside the input box,
        // routing them through the same `focus_by_name` call.
        let name_row = ui.container().gap(row_gap).row(|ui| {
            ui.text("Name: ");
            let _ = ui.register_focusable_named("name");
            let r = ui.container().fill().col(|ui| {
                let _ = ui.text_input(&mut state.name);
            });
            if r.clicked {
                let _ = ui.focus_by_name("name");
            }
        });
        if name_row.clicked {
            let _ = ui.focus_by_name("name");
        }

        let email_row = ui.container().gap(row_gap).row(|ui| {
            ui.text("Email:");
            let _ = ui.register_focusable_named("email");
            let r = ui.container().fill().col(|ui| {
                let _ = ui.text_input(&mut state.email);
            });
            if r.clicked {
                let _ = ui.focus_by_name("email");
            }
        });
        if email_row.clicked {
            let _ = ui.focus_by_name("email");
        }

        let city_row = ui.container().gap(row_gap).row(|ui| {
            ui.text("City: ");
            let _ = ui.register_focusable_named("city");
            let r = ui.container().fill().col(|ui| {
                let _ = ui.text_input(&mut state.city);
            });
            if r.clicked {
                let _ = ui.focus_by_name("city");
            }
        });
        if city_row.clicked {
            let _ = ui.focus_by_name("city");
        }
    });
}

/// Render three focus buttons. Each button targets a name; clicking it
/// asks the focus system to jump on the next frame.
fn render_focus_buttons(ui: &mut Context, gap: u32) {
    let _ = ui.container().gap(gap).row(|ui| {
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
