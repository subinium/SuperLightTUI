//! Issue #469: accessible linear output mode through a real PTY.
//!
//! Spawns this test binary as a PTY child (driven by
//! `tests/support/accessible_pty.py`) and answers its prompts the way a
//! screen-reader user types them. The harness asserts the transcript is
//! plain lines (no escape sequence at all: no alternate screen, no cursor
//! movement, no colors), that answers are read from stdin, and that the
//! masked answer never reaches the output stream.
//!
//! Gated behind the dev-only `pty-test` feature: `cargo test --features pty-test`.
#![cfg(all(unix, feature = "pty-test"))]

use slt::{RunConfig, SelectState, TextInputState};

#[test]
fn accessible_pty_child() {
    let Ok(case) = std::env::var("SLT_ACCESSIBLE_TEST_CASE") else {
        return;
    };
    println!("SLT_BEGIN");
    match case.as_str() {
        "flow" => {
            let mut username = TextInputState::new();
            let mut password = TextInputState::password();
            let mut role = SelectState::new(vec!["Viewer", "Editor", "Admin"]);
            let mut create = false;
            let mut answered = false;
            slt::run_with(RunConfig::default().accessible(true), |ui| {
                ui.text("Create an account");
                ui.text("Username:");
                let _ = ui.text_input(&mut username);
                let _ = ui.a11y_label("Password").text_input(&mut password);
                let _ = ui.a11y_label("Role").select(&mut role);
                if ui.confirm("Create account?", &mut create).clicked {
                    answered = true;
                    ui.quit();
                }
            })
            .unwrap();
            println!(
                "SLT_RESULT user={} password_len={} role={} create={} answered={}",
                username.value,
                password.value.len(),
                role.selected_item().unwrap_or_default(),
                create,
                answered
            );
        }
        "env_default" => {
            let mut name = TextInputState::new();
            slt::run(|ui| {
                if ui.a11y_label("Name").text_input(&mut name).submitted {
                    ui.quit();
                }
            })
            .unwrap();
            println!("SLT_RESULT name={}", name.value);
        }
        "explicit_off" => {
            slt::run_with(
                RunConfig::default().accessible(false).handle_suspend(false),
                |ui| {
                    ui.text("VISUAL");
                    if ui.tick() >= 1 {
                        ui.quit();
                    }
                },
            )
            .unwrap();
            println!("SLT_RESULT visual=true");
        }
        other => panic!("unknown case {other}"),
    }
}

fn verify(case: &str) {
    let output = std::process::Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/accessible_pty.py"
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
fn accessible_form_is_line_oriented_and_reads_stdin() {
    verify("flow");
}

#[test]
fn slt_accessible_env_enables_the_default_config() {
    verify("env_default");
}

#[test]
fn explicit_accessible_false_keeps_the_visual_renderer() {
    verify("explicit_off");
}
