//! v0.20.0 Ctrl+C passthrough demo — opt out of SLT's default Ctrl+C
//! interception so the closure can implement its own quit policy.
//!
//! Demonstrates: #238.
//!
//! Run: `cargo run --example v020_ctrl_c_passthrough`
//!
//! Keys:
//!   Ctrl-C        — observed as a normal key event; quits after 3 strikes
//!   q             — quit immediately
//!   Ctrl-Q / Esc  — quit
//!
//! Layout:
//!   ┌────────────── main view ─────────────┐
//!   │ Ctrl+C passthrough demo (issue #238) │
//!   │ Ctrl+C presses observed: N / 3       │
//!   │ Press Ctrl+C three times to quit…    │
//!   └──────────────────────────────────────┘

use slt::{Color, Context, KeyCode, KeyModifiers, RunConfig, Style};

/// Strike count required before this demo confirms quit. Matches Vim/IPython
/// "interrupt three times to leave" muscle memory.
const QUIT_STRIKES: u32 = 3;

/// Shared body. The count is the only varying input — keeping the visible
/// text identical between snapshot and live loop avoids documentation drift.
fn body(ui: &mut Context, ctrl_c_count: u32) {
    let _ = ui.col(|ui| {
        ui.styled(
            "Ctrl+C passthrough demo (issue #238)",
            Style::new().bold().fg(Color::Cyan),
        );
        ui.text("");
        ui.styled(
            format!("Ctrl+C presses observed: {ctrl_c_count} / {QUIT_STRIKES}"),
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

/// One-frame deterministic render entry point used by snapshot tests
/// (`tests/v020_lib_demos.rs`). Pins the strike count at one so the
/// snapshot shows the mid-quit state instead of a fresh-counter zero.
pub fn render(ui: &mut Context) {
    body(ui, 1);
}

fn main() -> std::io::Result<()> {
    let mut ctrl_c_count: u32 = 0;

    // Opt out of the default ctrl-c-quits behaviour so the loop can decide
    // when (and after how many strikes) to exit.
    let config = RunConfig::default().handle_ctrl_c(false);

    slt::run_with(config, |ui: &mut Context| {
        if ui.key_mod('c', KeyModifiers::CONTROL) {
            ctrl_c_count = ctrl_c_count.saturating_add(1);
            if ctrl_c_count >= QUIT_STRIKES {
                ui.quit();
            }
        }
        if ui.key('q') || ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }

        body(ui, ctrl_c_count);
    })?;

    Ok(())
}
