//! v0.20.0 keymap-help demo — auto-generated help overlay from per-widget
//! key bindings.
//!
//! Demonstrates: #236.
//!
//! Run: `cargo run --example v020_keymap_help`
//!
//! Keys:
//!   k / Up         — increment counter
//!   j / Down       — decrement counter
//!   r              — reset counter to zero
//!   ?              — toggle the auto-help overlay
//!   Esc            — close the overlay (when open)
//!   q / Ctrl-Q     — quit
//!
//! Note: while the help overlay is open it acts as a modal, so the
//! regular `ui.key('?')` / `ui.key_code(KeyCode::Esc)` checks are blocked
//! by the modal guard. The toggle uses `raw_key_mod` / `raw_key_code` so
//! '?' and Esc keep working while the overlay is up.
//!
//! Layout:
//!   ┌──────── main view ────────┐
//!   │ Keymap publishing demo    │       ┌── help overlay (on '?') ──┐
//!   │ Counter: N                │       │ global: ?, q              │
//!   │ Press ? for help…         │       │ counter: k/Up, j/Down, r  │
//!   └───────────────────────────┘       └───────────────────────────┘

use slt::{Color, Context, KeyCode, KeyModifiers, RunConfig, Style, WidgetKeyHelp};

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

/// Persistent state for the keymap-help demo.
///
/// Counter increments survive across frames; `help_open` controls whether
/// the auto-generated overlay is rendered.
#[derive(Default)]
pub struct DemoState {
    pub count: i32,
    pub help_open: bool,
}

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

/// Per-frame entry point. Handles k/j/r counter updates and the `?`/Esc
/// overlay toggle, then renders the body. Caller owns [`DemoState`] so
/// counter and overlay state persist across frames — this is the path
/// the tour uses.
///
/// `?` and Esc go through `raw_key_*` because once the overlay is open it
/// counts as a modal and the regular `key()` checks are blocked by the
/// modal guard.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    // Toggle help overlay. Use `raw_key_mod` so '?' keeps toggling
    // even after the overlay opens — the regular `key('?')` is
    // blocked by the overlay's modal guard.
    if ui.raw_key_mod('?', KeyModifiers::NONE) {
        state.help_open = !state.help_open;
    }
    // Close overlay on Esc as well (also via `raw_*` to bypass the
    // modal guard).
    if state.help_open && ui.raw_key_code(KeyCode::Esc) {
        state.help_open = false;
    }
    if ui.key('k') || ui.key_code(KeyCode::Up) {
        state.count = state.count.saturating_add(1);
    }
    if ui.key('j') || ui.key_code(KeyCode::Down) {
        state.count = state.count.saturating_sub(1);
    }
    if ui.key('r') {
        state.count = 0;
    }

    body(ui, state.count, state.help_open);
}

/// One-frame deterministic render entry point used by snapshot tests
/// (`tests/v020_lib_demos.rs`). Pins the help overlay open so the snapshot
/// covers both the main view and the auto-generated overlay.
///
/// NEVER call this from a live loop or from another demo — clicks and
/// counter mutations are silently dropped because state never persists.
/// Live embeddings should call [`render`] with their own `&mut DemoState`.
pub fn render_snapshot(ui: &mut Context) {
    body(ui, SNAPSHOT_COUNT, true);
}

fn main() -> std::io::Result<()> {
    let mut state = DemoState::default();

    slt::run_with(RunConfig::default().mouse(true), move |ui: &mut Context| {
        // Quit. Ctrl-Q is the portable alternative to Ctrl-C, which is
        // intercepted as Copy on macOS terminals. Esc is gated on
        // `!help_open` so the overlay's Esc-to-dismiss takes precedence.
        if ui.key('q') || ui.key_mod('q', KeyModifiers::CONTROL) {
            ui.quit();
        }
        if !state.help_open && ui.key_code(KeyCode::Esc) {
            ui.quit();
        }

        render(ui, &mut state);
    })?;

    Ok(())
}
