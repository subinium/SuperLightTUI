#![cfg(all(unix, feature = "async"))]

use slt::{RunConfig, TaskHandle, TaskOutcome};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

#[test]
fn runtime_pty_child() {
    let Ok(case) = std::env::var("SLT_RUNTIME_TEST_CASE") else {
        return;
    };
    if case == "boundary" || case.starts_with("inline") {
        let start = Instant::now();
        let mut first = true;
        let mut clicks = 0;
        slt::run_inline_with(
            3,
            RunConfig::default()
                .mouse(true)
                .kitty_keyboard(true)
                .handle_suspend(false),
            |ui| {
                if first && case == "boundary" {
                    first = false;
                    ui.error_boundary_with(
                        |_| panic!("recoverable test panic"),
                        |ui, _| {
                            ui.text("RECOVERED");
                        },
                    );
                } else if ui.button("ACT").clicked {
                    clicks += 1;
                }
                if start.elapsed() >= Duration::from_millis(500) {
                    ui.quit();
                }
            },
        )
        .unwrap();
        println!("SLT_RESULT clicks={clicks}");
        return;
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let frames = Arc::new(AtomicUsize::new(0));
        let delivered = Arc::new(AtomicUsize::new(0));
        let outcome = Arc::new(AtomicBool::new(false));
        let (frame_count, received, panicked) =
            (frames.clone(), delivered.clone(), outcome.clone());
        let background = case == "background";
        let mut task: Option<TaskHandle<()>> = None;
        let config = RunConfig::default()
            .mouse(true)
            .handle_suspend(false)
            .tick_rate(if case == "idle" {
                Duration::from_secs(1)
            } else {
                Duration::from_millis(16)
            });
        let run = slt::run_async_with::<u8>(config, move |ui, messages| {
            frame_count.fetch_add(1, Ordering::Relaxed);
            received.fetch_add(messages.len(), Ordering::Relaxed);
            if background {
                if task.is_none() {
                    task = Some(ui.spawn(async {
                        panic!("supervised test panic");
                    }));
                }
                if let Some(TaskOutcome::Panicked(_)) = ui.poll_outcome(task.as_ref().unwrap()) {
                    panicked.store(true, Ordering::Relaxed);
                }
            }
            ui.text(if panicked.load(Ordering::Relaxed) {
                "PANIC_OUTCOME"
            } else {
                "READY"
            });
        })
        .unwrap();
        if case.starts_with("zero") {
            run.send(42).await.unwrap();
        }
        if case == "zero_closed" {
            run.join().await.unwrap();
        } else {
            tokio::time::sleep(Duration::from_millis(350)).await;
            run.cancel_and_join().await.unwrap();
        }
        println!(
            "SLT_RESULT frames={} delivered={} panic={}",
            frames.load(Ordering::Relaxed),
            delivered.load(Ordering::Relaxed),
            outcome.load(Ordering::Relaxed)
        );
    });
}

fn verify(case: &str) {
    let output = std::process::Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/runtime_pty.py"
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
fn caught_panic_keeps_inline_modes() {
    verify("boundary");
}
#[test]
fn supervised_panic_keeps_fullscreen_modes() {
    verify("background");
}
#[test]
fn inline_mouse_uses_physical_viewport() {
    verify("inline_inside");
    verify("inline_outside");
}
#[test]
fn zero_size_preserves_async_messages() {
    verify("zero");
    verify("zero_closed");
}
#[test]
fn async_idle_respects_tick_and_cancellation() {
    verify("idle");
}
