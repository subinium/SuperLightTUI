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
    if case.starts_with("pending_") {
        let status = std::path::PathBuf::from(std::env::var("SLT_RUNTIME_FOCUS_STATUS").unwrap());
        let closed = status.with_extension("closed");
        let navigated = Arc::new(AtomicBool::new(false));
        let edited = Arc::new(AtomicUsize::new(0));
        let (seen, edits) = (navigated.clone(), edited.clone());
        let start = Instant::now();
        let draw = move |ui: &mut slt::Context| {
            let fields = ui.use_state(|| {
                std::rc::Rc::new(std::cell::RefCell::new([
                    slt::TextInputState::new(),
                    slt::TextInputState::new(),
                ]))
            });
            let fields = std::rc::Rc::clone(fields.get(ui));
            let mut fields = fields.borrow_mut();
            if ui.raw_key_code(slt::KeyCode::Tab) {
                seen.store(true, Ordering::Release);
            }
            for field in fields.iter_mut() {
                let _ = ui.text_input(field);
            }
            edits.store(
                fields.iter().map(|field| field.value.len()).sum(),
                Ordering::Release,
            );
            let temporary = status.with_extension("pending");
            std::fs::write(
                &temporary,
                serde_json::to_vec(&serde_json::json!({"tick": ui.tick()})).unwrap(),
            )
            .unwrap();
            std::fs::rename(&temporary, &status).unwrap();
            if start.elapsed() > Duration::from_secs(5) {
                ui.quit();
            }
        };
        let config = RunConfig::default().max_fps(1).handle_suspend(false);
        if case == "pending_ctrl_c" {
            slt::run_with(config, draw).unwrap();
        } else {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let run = slt::run_async_with::<u8>(config, move |ui, _| draw(ui)).unwrap();
                if case == "pending_disconnect" {
                    tokio::time::timeout(Duration::from_secs(4), async {
                        while !navigated.load(Ordering::Acquire) {
                            tokio::time::sleep(Duration::from_millis(1)).await;
                        }
                    })
                    .await
                    .unwrap();
                    let (joined, ()) = tokio::join!(biased; run.join(), async {
                        tokio::time::sleep(Duration::from_millis(20)).await;
                        std::fs::write(closed, b"closed").unwrap();
                    });
                    joined.unwrap();
                    assert_eq!(
                        edited.load(Ordering::Acquire),
                        1,
                        "disconnect discarded the received character"
                    );
                } else {
                    tokio::time::sleep(Duration::from_millis(3300)).await;
                    run.cancel_and_join().await.unwrap();
                    assert!(navigated.load(Ordering::Acquire));
                    assert!(edited.load(Ordering::Acquire) > 0);
                }
            });
        }
        println!("SLT_RESULT pending=true");
        return;
    }
    if case.starts_with("focus_scroll") {
        let status = std::path::PathBuf::from(std::env::var("SLT_RUNTIME_FOCUS_STATUS").unwrap());
        let mut form = (0..10).fold(slt::FormState::new(), |state, index| {
            let mut field = slt::FormField::new(format!("Label {index:02}"));
            field.input.value = format!("FIELD_{index:02}");
            state.field(field)
        });
        let mut scroll = slt::ScrollState::new();
        let started = Instant::now();
        let mut focused = 0;
        slt::run_with(RunConfig::default().handle_suspend(false), |ui| {
            let mut visible = false;
            let _ = ui.modal(|ui| {
                let _ = ui.scrollable(&mut scroll).w(28).h(6).col(|ui| {
                    for (index, field) in form.fields.iter_mut().enumerate() {
                        let response = ui.form_field_response(field);
                        if response.focused {
                            focused = index;
                            visible = !response.input.rect.is_empty();
                        }
                    }
                });
            });
            let data = serde_json::json!({"tick": ui.tick(), "focused": focused, "visible": visible,
                "offset": scroll.offset, "last": form.fields[9].input.value, "first": form.fields[0].input.value});
            let temporary = status.with_extension("pending");
            std::fs::write(&temporary, serde_json::to_vec(&data).unwrap()).unwrap();
            std::fs::rename(&temporary, &status).unwrap();
            if ui.raw_key_code(slt::KeyCode::Esc) || started.elapsed() > Duration::from_secs(8) { ui.quit(); }
        }).unwrap();
        assert_eq!(focused, 9);
        assert_eq!(form.fields[9].input.value, "ZFIELD_09");
        assert_eq!(form.fields[0].input.value, "FIELD_00");
        println!("SLT_RESULT focus_scroll=true");
        return;
    }
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

#[test]
fn native_tab_reveals_modal_fields_before_editing() {
    verify("focus_scroll");
    verify("focus_scroll_burst");
}

#[test]
fn queued_navigation_keeps_native_ctrl_c_responsive() {
    verify("pending_ctrl_c");
}

#[test]
fn queued_navigation_respects_async_fps_wait() {
    verify("pending_cpu");
}

#[test]
fn async_disconnect_drains_received_navigation_and_text() {
    verify("pending_disconnect");
}
