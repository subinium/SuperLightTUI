//! Terminal capability negotiation through a real PTY (issues #447, #448,
//! #449, #468).
//!
//! Spawns this test binary as a PTY child under `tests/support/terminal_caps_pty.py`,
//! which plays the terminal: it answers (or ignores) the startup probe and
//! asserts on the exact bytes the child wrote.

#![cfg(all(unix, feature = "crossterm"))]

use slt::{Color, CursorShape, Progress, RunConfig};
use std::time::Duration;

/// Frames rendered before the child quits.
const FRAMES: u64 = 5;

#[test]
fn terminal_caps_pty_child() {
    let Ok(case) = std::env::var("SLT_CAPS_TEST_CASE") else {
        return;
    };
    let mut config = RunConfig::default().tick_rate(Duration::from_millis(5));
    match case.as_str() {
        "kitty_reply" | "kitty_silent" => config = config.kitty_keyboard(true),
        "width_probe" => config = config.width_probe(true),
        "clipboard" => {
            config =
                config.clipboard_passthrough(std::env::var_os("SLT_CAPS_PASSTHROUGH").is_some());
        }
        _ => {}
    }
    // Manual multiplexer checks keep the app alive for a while.
    let linger = Duration::from_millis(
        std::env::var("SLT_CAPS_LINGER_MS")
            .ok()
            .and_then(|ms| ms.parse().ok())
            .unwrap_or(0),
    );
    let started = std::time::Instant::now();
    let mut frame = 0u64;
    slt::run_with(config, |ui| {
        frame += 1;
        ui.text("CAPS").fg(Color::Rgb(1, 2, 3));
        if matches!(case.as_str(), "meta" | "meta_dumb") {
            ui.set_title("caps: idle");
            ui.set_progress(Some(Progress::Normal(0.5)));
            ui.set_cursor_shape(CursorShape::SteadyBar);
        }
        if case == "clipboard" && frame == 1 {
            ui.copy_to_clipboard("SLT-OSC52-PROBE");
        }
        if case == "clipboard" && frame == 2 {
            println!("SLT_CAPS_CLIPBOARD {:?}", ui.clipboard_status());
        }
        if frame >= FRAMES && started.elapsed() >= linger {
            ui.quit();
        }
    })
    .unwrap();
    println!("SLT_CAPS_RESULT frames={frame}");
}

fn verify(case: &str) {
    let output = std::process::Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/terminal_caps_pty.py"
        ))
        .arg(std::env::current_exe().unwrap())
        .arg(case)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn issue_448_push_follows_kitty_keyboard_reply() {
    verify("kitty_reply");
    verify("kitty_silent");
}

#[test]
fn issue_447_probe_confirmed_truecolor_drives_depth() {
    verify("truecolor");
    verify("no_truecolor");
}

#[test]
fn issue_468_metadata_emitted_once_and_restored() {
    verify("meta");
    verify("meta_dumb");
}

#[test]
fn issue_446_clipboard_reaches_the_pty_and_reports_sent() {
    verify("clipboard");
}

#[test]
fn issue_449_width_probe_measures_and_erases() {
    verify("width_probe");
}
