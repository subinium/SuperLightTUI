//! v0.20.0 keymap-help demo — auto-generated help overlay from per-widget
//! key bindings.
//!
//! Demonstrates: #236.
//!
//! Run: `cargo run --example v020_keymap_help`
//!
//! Keys:
//!   k / Up        — increment counter
//!   j / Down      — decrement counter
//!   r             — reset counter to zero
//!   ?             — toggle the auto-help overlay
//!   Ctrl-Q / Esc  — quit
//!
//! Layout:
//!   ┌──────── main view ────────┐
//!   │ Keymap publishing demo    │       ┌── help overlay (on '?') ──┐
//!   │ Counter: N                │       │ global: ?, q              │
//!   │ Press ? for help…         │       │ counter: k/Up, j/Down, r  │
//!   └───────────────────────────┘       └───────────────────────────┘

use slt::{Color, Context, KeyCode, RunConfig, Style, WidgetKeyHelp};

/// Counter widget bindings — published every frame the widget renders, so
/// the help overlay only lists keys that are actually live.
struct CounterWidget;
impl WidgetKeyHelp for CounterWidget {
    fn key_help(&self) -> &'static [(&'static str, &'static str)] {
        const HELP: &[(&str, &str)] = &[
            ("k / Up", "increment"),
            ("j / Down", "decrement"),
            ("r", "reset to zero"),
        ];
        HELP
    }
}

/// Always-visible global bindings (quit, help toggle).
struct GlobalKeys;
impl WidgetKeyHelp for GlobalKeys {
    fn key_help(&self) -> &'static [(&'static str, &'static str)] {
        const HELP: &[(&str, &str)] = &[("?", "toggle this help overlay"), ("q", "quit")];
        HELP
    }
}

/// Snapshot fixture counter value. Matches the saved snapshot under
/// `tests/snapshots/v020_lib_demos__v020_keymap_help.snap`.
const SNAPSHOT_COUNT: i32 = 3;

/// Shared body. Every focusable widget publishes its bindings BEFORE the
/// overlay call — the overlay reads the per-frame keymap registry so order
/// matters for deterministic snapshots.
fn body(ui: &mut Context, count: i32, help_open: bool) {
    ui.publish_keymap("global", GlobalKeys.key_help());
    ui.publish_keymap("counter", CounterWidget.key_help());

    let _ = ui.col(|ui| {
        ui.styled(
            "Keymap publishing demo",
            Style::new().bold().fg(Color::Cyan),
        );
        ui.text("");
        ui.styled(format!("Counter: {count}"), Style::new().bold());
        ui.text("");
        ui.styled("Press ? to view the auto-help overlay", Style::new().dim());
        ui.styled("Press q to quit", Style::new().dim());
    });

    ui.keymap_help_overlay(help_open);
}

/// One-frame deterministic render entry point used by snapshot tests
/// (`tests/v020_lib_demos.rs`). Pins the help overlay open so the snapshot
/// covers both the main view and the auto-generated overlay.
pub fn render(ui: &mut Context) {
    body(ui, SNAPSHOT_COUNT, true);
}

fn main() -> std::io::Result<()> {
    let mut count: i32 = 0;
    let mut help_open = false;

    slt::run_with(RunConfig::default().mouse(true), |ui: &mut Context| {
        if ui.key('q') {
            ui.quit();
        }
        if ui.key('?') {
            help_open = !help_open;
        }
        if ui.key('k') || ui.key_code(KeyCode::Up) {
            count = count.saturating_add(1);
        }
        if ui.key('j') || ui.key_code(KeyCode::Down) {
            count = count.saturating_sub(1);
        }
        if ui.key('r') {
            count = 0;
        }

        body(ui, count, help_open);
    })?;

    Ok(())
}
