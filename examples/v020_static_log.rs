//! v0.20.0 demo: `ui.static_log(...)` — append-only scrollback above an
//! inline TUI (issue #233).
//!
//! Run with: `cargo run --example v020_static_log`
//!
//! The dynamic inline area shows a counter and live status. Each tick, the
//! demo also commits a line to terminal scrollback via
//! [`Context::static_log`]. Lines never re-render — they accumulate in the
//! terminal's history exactly like `println!` from a normal CLI tool.
//!
//! Key bindings:
//! - `Space` / Enter — bump the counter
//! - `q` — quit
//! - `Ctrl+C` — quit (default)

use slt::{Color, Context, KeyCode, StaticOutput, Style};

/// One-frame render entry point used by snapshot tests
/// (`tests/v020_lib_demos.rs`). Mirrors what `main` would render when the
/// counter is at 5 and a static-log line was just queued.
pub fn render(ui: &mut Context) {
    // Sample state — non-interactive, deterministic for snapshots.
    let count: u32 = 5;
    ui.static_log(format!("[tick] counter reached {count}"));

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

        // Only log on every 5th increment so the demo doesn't spam scrollback.
        if count != last_logged && count % 5 == 0 {
            ui.static_log(format!("[tick] counter reached {count}"));
            last_logged = count;
        }

        // Dynamic inline area.
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
    })?;
    Ok(())
}
