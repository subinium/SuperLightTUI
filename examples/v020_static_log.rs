//! v0.20.0 static-log demo — append-only scrollback above an inline TUI.
//!
//! Demonstrates: #233.
//!
//! Run: `cargo run --example v020_static_log`
//!
//! Keys:
//!   Space / Enter — bump the counter
//!   Ctrl-Q / Esc  — quit
//!
//! Layout:
//!   ┌──────────────────────────────────┐
//!   │ scrollback (println-like, frozen)│
//!   │   [tick] counter reached 5       │
//!   │   [tick] counter reached 10      │
//!   ├──────────────────────────────────┤  ← inline frame redraws here
//!   │ Counter: N                       │
//!   │ Space/Enter: bump counter | q…   │
//!   └──────────────────────────────────┘

use slt::{Color, Context, KeyCode, StaticOutput, Style};

/// Counter increment that triggers a scrollback log entry. Five matches the
/// snapshot fixture below — keeping it as a constant avoids a magic number
/// drifting between `render` and `main` if either is later edited.
const LOG_EVERY: u32 = 5;

/// Shared inline-area body. Used by both `render` (one-frame snapshot) and
/// `main` (live loop) so the visible UI stays identical across both paths.
fn inline_body(ui: &mut Context, count: u32) {
    let _ = ui.col(|ui| {
        ui.styled(
            format!("Counter: {count}"),
            Style::new().bold().fg(Color::Cyan),
        );
        ui.text("Space/Enter: bump counter | q: quit");
        ui.styled(
            "Lines logged to scrollback every 5 ticks via ui.static_log()",
            Style::new().dim(),
        );
    });
}

/// One-frame deterministic render entry point used by snapshot tests
/// (`tests/v020_lib_demos.rs`). Mirrors the state right after the fifth
/// counter bump, when the next scrollback line was just queued.
pub fn render(ui: &mut Context) {
    let count: u32 = LOG_EVERY;
    // Queue a scrollback line so the snapshot also exercises the
    // static_log code path, not just the inline body.
    ui.static_log(format!("[tick] counter reached {count}"));
    inline_body(ui, count);
}

fn main() -> std::io::Result<()> {
    let mut output = StaticOutput::new();
    output.println("[demo] starting v020_static_log — try pressing Space");

    let mut count: u32 = 0;
    let mut last_logged: u32 = 0;

    slt::run_static(&mut output, 4, |ui: &mut Context| {
        if ui.key('q') || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        if ui.key(' ') || ui.key_code(KeyCode::Enter) {
            count = count.saturating_add(1);
        }

        // Throttle so a held key cannot flood scrollback faster than the
        // user can read it.
        if count != last_logged && count % LOG_EVERY == 0 {
            ui.static_log(format!("[tick] counter reached {count}"));
            last_logged = count;
        }

        inline_body(ui, count);
    })?;
    Ok(())
}
