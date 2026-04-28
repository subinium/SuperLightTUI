//! v0.20.0 demo: `RunConfig::handle_ctrl_c(false)` — Ctrl+C delivered as
//! a regular key event (issue #238).
//!
//! Run with: `cargo run --example v020_ctrl_c_passthrough`
//!
//! By default SLT intercepts Ctrl+C and exits the loop cleanly. Setting
//! `handle_ctrl_c(false)` opts out — Ctrl+C reaches the frame closure as a
//! normal [`Event::Key`] with `KeyModifiers::CONTROL`. Press it three
//! times to confirm the quit; press `q` to quit immediately.

use slt::{Color, Context, KeyModifiers, RunConfig, Style};

/// One-frame render entry point used by snapshot tests
/// (`tests/v020_lib_demos.rs`). Mirrors a mid-state where one Ctrl+C has
/// already been observed.
pub fn render(ui: &mut Context) {
    let ctrl_c_count: u32 = 1;

    let _ = ui.col(|ui| {
        ui.styled(
            "Ctrl+C passthrough demo (issue #238)",
            Style::new().bold().fg(Color::Cyan),
        );
        ui.text("");
        ui.styled(
            format!("Ctrl+C presses observed: {ctrl_c_count} / 3"),
            Style::new().bold(),
        );
        ui.text("");
        ui.styled(
            "Press Ctrl+C three times to quit, or 'q' to quit immediately.",
            Style::new().dim(),
        );
        ui.styled(
            "(With handle_ctrl_c(false), Ctrl+C arrives as a normal key event.)",
            Style::new().dim(),
        );
    });
}

fn main() -> std::io::Result<()> {
    let mut ctrl_c_count: u32 = 0;

    let config = RunConfig::default().handle_ctrl_c(false);

    slt::run_with(config, |ui: &mut Context| {
        if ui.key_mod('c', KeyModifiers::CONTROL) {
            ctrl_c_count += 1;
            if ctrl_c_count >= 3 {
                // Three strikes — quit gracefully.
                ui.quit();
            }
        }
        if ui.key('q') {
            ui.quit();
        }

        let _ = ui.col(|ui| {
            ui.styled(
                "Ctrl+C passthrough demo (issue #238)",
                Style::new().bold().fg(Color::Cyan),
            );
            ui.text("");
            ui.styled(
                format!("Ctrl+C presses observed: {ctrl_c_count} / 3"),
                Style::new().bold(),
            );
            ui.text("");
            ui.styled(
                "Press Ctrl+C three times to quit, or 'q' to quit immediately.",
                Style::new().dim(),
            );
            ui.styled(
                "(With handle_ctrl_c(false), Ctrl+C arrives as a normal key event.)",
                Style::new().dim(),
            );
        });
    })?;

    Ok(())
}
