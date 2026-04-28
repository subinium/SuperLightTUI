//! v0.20.0 use_effect demo — dependency-tracked side effects.
//!
//! Demonstrates: #216
//!
//! Three effects with three different dependency shapes:
//! - `&()` runs **once** on first frame (run-once setup).
//! - `&count` runs on every counter change (PartialEq + Clone deps).
//! - `&log_visible` runs on each visibility transition.
//!
//! All three append to a shared `Vec<String>` so the effect log is
//! visible in the lower panel. The log itself is rendered last so it
//! reflects the writes from the current frame.
//!
//! Run: `cargo run --example v020_use_effect`
//!
//! Keys:
//!   k / Up         — count++
//!   j / Down       — count--
//!   Space          — toggle effect-log panel
//!   Ctrl-Q / Esc   — quit
//!
//! Layout:
//!   ┌── use_effect: dep-tracked side effects ──┐
//!   │ help line                                  │
//!   │ count = 3                                  │
//!   │ log panel: visible                         │
//!   │ ┌── Effect log ──┐                         │
//!   │ │ [setup] …      │                         │
//!   │ │ [count] → 3    │                         │
//!   │ └────────────────┘                         │
//!   └────────────────────────────────────────────┘

use slt::{Border, Color, Context, KeyCode, KeyModifiers, RunConfig};
use std::cell::RefCell;
use std::rc::Rc;

/// Tail length of the effect log shown in the bottom panel.
const LOG_TAIL_LEN: usize = 10;

/// Shared state for [`render`]. The log is `Rc<RefCell<…>>` because every
/// effect closure captures its own clone — `use_effect`'s callback runs
/// during the render closure, where `&mut state` is already borrowed.
pub struct DemoState {
    pub count: i32,
    pub log_visible: bool,
    pub log: Rc<RefCell<Vec<String>>>,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            count: 0,
            log_visible: true,
            log: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut state = DemoState::default();
    slt::run_with(RunConfig::default().mouse(true), move |ui: &mut Context| {
        render(ui, &mut state)
    })
}

/// Render one frame of the use_effect demo.
///
/// Public so snapshot tests can drive frames sequentially and inspect the
/// shared log without depending on a real terminal backend.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    if ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
        ui.quit();
        return;
    }
    handle_input(ui, state);

    // Run-once setup. `&()` never changes, so the closure fires exactly
    // once across the lifetime of the run loop.
    let log_setup = state.log.clone();
    ui.use_effect(
        move |_| {
            log_setup
                .borrow_mut()
                .push("[setup] run-once effect fired".to_string());
        },
        &(),
    );

    // Counter-change effect. PartialEq + Clone on the dep (`i32`) is what
    // lets `use_effect` detect "did it change since last frame".
    let log_count = state.log.clone();
    ui.use_effect(
        move |c| {
            log_count
                .borrow_mut()
                .push(format!("[count] changed → {c}"));
        },
        &state.count,
    );

    // Visibility-change effect. The dep is a `bool` cloned by value.
    let log_vis = state.log.clone();
    ui.use_effect(
        move |v| {
            let label = if *v { "shown" } else { "hidden" };
            log_vis.borrow_mut().push(format!("[panel] log {label}"));
        },
        &state.log_visible,
    );

    let pad = ui.spacing().xs();
    let gap = ui.spacing().xs();
    let count = state.count;
    let visible = state.log_visible;
    let log = state.log.clone();

    let _ = ui
        .bordered(Border::Rounded)
        .title("use_effect: dep-tracked side effects")
        .p(pad)
        .gap(gap)
        .col(|ui| {
            ui.text("k/Up = count++   j/Down = count--   Space = toggle log   Ctrl+Q = quit")
                .dim();
            ui.text(format!("count = {count}")).bold().fg(Color::Cyan);

            // Visibility status mirrors the bool the effect is watching.
            let status = if visible { "visible" } else { "hidden" };
            let status_color = if visible { Color::Green } else { Color::Red };
            ui.text(format!("log panel: {status}")).fg(status_color);

            if visible {
                let _ = ui
                    .bordered(Border::Single)
                    .title("Effect log")
                    .p(pad)
                    .col(|ui| render_log_tail(ui, &log));
            }
        });
}

/// Apply key bindings to mutate `state` BEFORE effects run, so an effect
/// reading `state.count` sees the post-input value on the same frame.
fn handle_input(ui: &mut Context, state: &mut DemoState) {
    if ui.key('k') || ui.key_code(KeyCode::Up) {
        state.count += 1;
    }
    if ui.key('j') || ui.key_code(KeyCode::Down) {
        state.count -= 1;
    }
    if ui.key_code(KeyCode::Char(' ')) {
        state.log_visible = !state.log_visible;
    }
}

/// Render the last [`LOG_TAIL_LEN`] entries of the effect log. Placed last
/// in the frame so writes from this frame's effects are already in `log`.
fn render_log_tail(ui: &mut Context, log: &Rc<RefCell<Vec<String>>>) {
    let entries = log.borrow();
    if entries.is_empty() {
        ui.text("(no events yet)").dim();
        return;
    }
    let start = entries.len().saturating_sub(LOG_TAIL_LEN);
    for entry in &entries[start..] {
        ui.text(entry.clone()).dim();
    }
}
