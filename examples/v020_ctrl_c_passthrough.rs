//! v0.20.0 Ctrl+C passthrough demo — opt out of SLT's default Ctrl+C
//! interception so the closure can implement its own quit policy.
//!
//! Demonstrates: #238.
//!
//! Run: `cargo run --example v020_ctrl_c_passthrough`
//!
//! ## What this demo shows
//!
//! `RunConfig::default().handle_ctrl_c(false)` flips off SLT's default
//! "exit on Ctrl+C" behavior. With it off, Ctrl+C arrives at the frame
//! closure as a plain `Event::Key { code: Char('c'), modifiers: CONTROL }`,
//! and the closure decides what to do with it (count strikes, prompt to
//! save, defer, ignore — whatever your app needs).
//!
//! The runtime side of that contract is unconditional: with
//! `handle_ctrl_c(false)`, every Ctrl+C the terminal emits *will* be passed
//! through. Whether the terminal *emits* a Ctrl+C in the first place is a
//! separate, terminal-emulator-level concern.
//!
//! ## macOS / Ghostty / iTerm2 gotcha
//!
//! On macOS, most terminals (Ghostty, iTerm2, Terminal.app) bind Ctrl+C to
//! the system Copy command in their default keybindings. Pressing Ctrl+C
//! puts the current selection on the clipboard; the keystroke never reaches
//! the foreground program. That makes a "press Ctrl+C three times" demo
//! literally untestable on a stock macOS install — the keystroke is
//! intercepted before it can flow through SLT's `handle_ctrl_c(false)`
//! path.
//!
//! Two ways to still observe the passthrough behavior on macOS:
//!
//! 1. **Press Ctrl+G** — by convention not bound to any clipboard command,
//!    so it reaches the closure as a plain `Char('g')` + CONTROL key event.
//!    This demo treats Ctrl+G the same way it treats a real Ctrl+C: as a
//!    "the closure got a keypress with the CONTROL modifier" signal.
//!
//! 2. **Click "Send Ctrl+C"** — the demo synthesizes the same state change
//!    a real `Ctrl+C` would trigger. The runtime path differs (button click
//!    instead of `is_ctrl_c` matching), but the application-visible state
//!    transition (strike counter advances) is the one your closure would
//!    have run on a real Ctrl+C.
//!
//! On Linux/Windows terminals where Ctrl+C is *not* rebound, real Ctrl+C
//! presses also work; the same handler fires. The point of this demo is
//! the API contract — `handle_ctrl_c(false)` — not any one input source.
//!
//! Keys:
//!   Ctrl-C        — counted as a strike when the terminal lets it through
//!   Ctrl-G        — same handler (macOS-friendly alternative)
//!   Click button  — synthesize a strike
//!   q / Esc / Ctrl-Q — quit immediately
//!
//! Layout:
//!   ┌────────────── main view ─────────────┐
//!   │ Ctrl+C passthrough demo (issue #238) │
//!   │ Ctrl+C / Ctrl+G observed: N / 3      │
//!   │ [ Send Ctrl+C ]                      │
//!   │ Press Ctrl+C / Ctrl+G three times…   │
//!   └──────────────────────────────────────┘

use slt::{Color, Context, KeyCode, KeyModifiers, RunConfig, Style};

/// Strike count required before this demo confirms quit. Matches Vim/IPython
/// "interrupt three times to leave" muscle memory.
const QUIT_STRIKES: u32 = 3;

/// Snapshot fixture strike count. Matches the saved snapshot under
/// `tests/snapshots/v020_lib_demos__v020_ctrl_c_passthrough.snap`.
const SNAPSHOT_COUNT: u32 = 1;

/// Persistent strike counter for the passthrough demo. Survives across
/// frames so a real Ctrl+C/Ctrl+G keypress (or a button click) advances
/// the counter the same way every time the demo is rendered.
#[derive(Default)]
pub struct DemoState {
    pub ctrl_c_count: u32,
}

/// Shared body. The count is the only varying input — keeping the visible
/// text identical between snapshot and live loop avoids documentation drift.
///
/// Returns `true` when the embedded button was clicked this frame, so
/// `render` can fold the click into the same strike counter that real
/// Ctrl+C / Ctrl+G presses advance.
fn body(ui: &mut Context, ctrl_c_count: u32) -> bool {
    let mut button_clicked = false;
    let _ = ui.col(|ui| {
        ui.styled(
            "Ctrl+C passthrough demo (issue #238)",
            Style::new().bold().fg(Color::Cyan),
        );
        ui.text("");
        ui.styled(
            format!("Ctrl+C / Ctrl+G observed: {ctrl_c_count} / {QUIT_STRIKES}"),
            Style::new().bold(),
        );
        ui.text("");
        // Banner: macOS users will hit this every time, so put it front and
        // center rather than burying it in the demo header.
        ui.styled(
            "Note: macOS terminals bind Ctrl+C to Copy by default.",
            Style::new().fg(Color::Yellow),
        );
        ui.styled(
            "Use Ctrl+G to test pass-through, or click the button below.",
            Style::new().fg(Color::Yellow),
        );
        ui.text("");
        if ui.button("Send Ctrl+C").clicked {
            button_clicked = true;
        }
        ui.text("");
        ui.styled(
            "(With handle_ctrl_c(false), real Ctrl+C arrives as a normal key event.)",
            Style::new().dim(),
        );
        ui.styled("Quit: q, Esc, or Ctrl-Q.", Style::new().dim());
    });
    button_clicked
}

/// Per-frame entry point. Folds Ctrl+C / Ctrl+G keypresses and the
/// "Send Ctrl+C" button click into the same strike counter so embedding
/// surfaces (the v0.20 tour) react to user input the same way the
/// standalone binary does.
///
/// Caller owns [`DemoState`] so the strike count survives across frames.
/// Reaching `QUIT_STRIKES` does NOT auto-quit here — quit policy is the
/// caller's responsibility (the standalone `main` opts in below; the tour
/// keeps running).
pub fn render(ui: &mut Context, state: &mut DemoState) {
    let mut strike = false;
    if ui.key_mod('c', KeyModifiers::CONTROL) {
        strike = true;
    }
    if ui.key_mod('g', KeyModifiers::CONTROL) {
        strike = true;
    }

    // Render the body first; it returns whether the embedded button was
    // clicked this frame.
    if body(ui, state.ctrl_c_count) {
        strike = true;
    }

    if strike {
        state.ctrl_c_count = state.ctrl_c_count.saturating_add(1);
    }
}

/// One-frame deterministic render entry point used by snapshot tests
/// (`tests/v020_lib_demos.rs`). Pins the strike count at one so the
/// snapshot shows the mid-quit state instead of a fresh-counter zero.
///
/// NEVER call this from a live loop or from another demo — strikes and
/// clicks are silently dropped because state never persists. Live
/// embeddings should call [`render`] with their own `&mut DemoState`.
pub fn render_snapshot(ui: &mut Context) {
    let _ = body(ui, SNAPSHOT_COUNT);
}

fn main() -> std::io::Result<()> {
    let mut state = DemoState::default();

    // Opt out of the default ctrl-c-quits behaviour so the loop can decide
    // when (and after how many strikes) to exit. Mouse on so the
    // "Send Ctrl+C" button can be clicked.
    let config = RunConfig::default().handle_ctrl_c(false).mouse(true);

    slt::run_with(config, move |ui: &mut Context| {
        render(ui, &mut state);

        if state.ctrl_c_count >= QUIT_STRIKES {
            ui.quit();
        }

        // Quit — Ctrl-Q is the portable alternative to Ctrl-C on macOS.
        if ui.key('q') || ui.key_code(KeyCode::Esc) || ui.key_mod('q', KeyModifiers::CONTROL) {
            ui.quit();
        }
    })?;

    Ok(())
}
