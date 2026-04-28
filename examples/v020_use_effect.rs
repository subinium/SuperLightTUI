//! v0.20.0 demo: `use_effect` for dependency-tracked side effects.
//!
//! - The `&()` effect at the top runs **once** on the first frame and seeds
//!   the activity log.
//! - The `&count` effect runs every time the counter changes, appending a
//!   log line.
//! - The `&log_visible` effect logs visibility transitions (panel show/hide).
//!
//! Run: `cargo run --example v020_use_effect`

use slt::{Border, Color, Context, KeyCode};
use std::cell::RefCell;
use std::rc::Rc;

fn main() -> std::io::Result<()> {
    // Effects need somewhere to write. We share a Vec<String> via Rc<RefCell<_>>;
    // each clone is captured by the closures below.
    let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let mut count: i32 = 0;
    let mut log_visible = true;

    slt::run(move |ui: &mut Context| {
        if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
            return;
        }
        if ui.key('k') || ui.key_code(KeyCode::Up) {
            count += 1;
        }
        if ui.key('j') || ui.key_code(KeyCode::Down) {
            count -= 1;
        }
        if ui.key_code(KeyCode::Char(' ')) {
            log_visible = !log_visible;
        }

        // Run-once setup effect: deps `&()` never change, so the closure
        // fires exactly once across the lifetime of the run loop.
        let log_setup = log.clone();
        ui.use_effect(
            move |_| {
                log_setup
                    .borrow_mut()
                    .push("[setup] run-once effect fired".to_string());
            },
            &(),
        );

        // Counter-change effect: fires whenever `count` differs from the
        // previously stored value. PartialEq + Clone is required.
        let log_count = log.clone();
        ui.use_effect(
            move |c| {
                log_count
                    .borrow_mut()
                    .push(format!("[count] changed → {c}"));
            },
            &count,
        );

        // Visibility-change effect.
        let log_vis = log.clone();
        ui.use_effect(
            move |v| {
                let state = if *v { "shown" } else { "hidden" };
                log_vis.borrow_mut().push(format!("[panel] log {state}"));
            },
            &log_visible,
        );

        let _ = ui
            .bordered(Border::Rounded)
            .title("use_effect — dep-tracked side effects")
            .p(1)
            .gap(1)
            .col(|ui| {
                ui.text("k/Up = count++  j/Down = count--  Space = toggle log  Ctrl+Q = quit")
                    .dim();
                ui.text(format!("count = {count}")).bold().fg(Color::Cyan);
                ui.text(format!(
                    "log panel: {}",
                    if log_visible { "visible" } else { "hidden" }
                ))
                .fg(if log_visible {
                    Color::Green
                } else {
                    Color::Red
                });
                if log_visible {
                    let _ = ui
                        .bordered(Border::Single)
                        .title("Effect log")
                        .p(1)
                        .col(|ui| {
                            // Show last ~10 entries.
                            let entries = log.borrow();
                            let start = entries.len().saturating_sub(10);
                            for entry in &entries[start..] {
                                ui.text(entry.clone()).dim();
                            }
                            if entries.is_empty() {
                                ui.text("(no events yet)").dim();
                            }
                        });
                }
            });
    })
}
