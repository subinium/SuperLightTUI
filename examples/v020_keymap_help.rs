//! v0.20.0 demo: widget keymap publishing + auto help overlay (issue #236).
//!
//! Run with: `cargo run --example v020_keymap_help`
//!
//! Each focusable widget publishes its [`WidgetKeyHelp`] bindings via
//! [`Context::publish_keymap`]. Press `?` to render an automatic
//! [`Context::keymap_help_overlay`] listing every binding registered this
//! frame.

use slt::keymap::WidgetKeyHelp;
use slt::{Color, Context, KeyCode, Style};

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

struct GlobalKeys;
impl WidgetKeyHelp for GlobalKeys {
    fn key_help(&self) -> &'static [(&'static str, &'static str)] {
        const HELP: &[(&str, &str)] = &[("?", "toggle this help overlay"), ("q", "quit")];
        HELP
    }
}

/// One-frame render entry point used by snapshot tests
/// (`tests/v020_lib_demos.rs`). Renders the keymap help overlay open.
pub fn render(ui: &mut Context) {
    let count: i32 = 3;
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

    ui.keymap_help_overlay(true);
}

fn main() -> std::io::Result<()> {
    let mut count: i32 = 0;
    let mut help_open = false;

    slt::run(|ui: &mut Context| {
        // Publish bindings for each "widget" rendered this frame. The
        // overlay below picks them up automatically.
        ui.publish_keymap("global", GlobalKeys.key_help());
        ui.publish_keymap("counter", CounterWidget.key_help());

        // Global key handling.
        if ui.key('q') {
            ui.quit();
        }
        if ui.key('?') {
            help_open = !help_open;
        }

        // Counter widget key handling — declared keys match the published help.
        if ui.key('k') || ui.key_code(KeyCode::Up) {
            count = count.saturating_add(1);
        }
        if ui.key('j') || ui.key_code(KeyCode::Down) {
            count = count.saturating_sub(1);
        }
        if ui.key('r') {
            count = 0;
        }

        // Main UI.
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

        // Auto-rendered help overlay.
        ui.keymap_help_overlay(help_open);
    })?;

    Ok(())
}
