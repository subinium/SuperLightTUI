//! Issue #201 regression tests for the run-loop F12 / Shift+F12
//! keybinding handler. Exercises [`process_run_loop_event`] directly
//! so we don't need a real crossterm event source.
use super::*;

use std::io;
use std::time::Duration;

use crate::{Buffer, Event, KeyCode, Rect, event, terminal};

static PANIC_HOOK_TEST_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(feature = "async")]
#[test]
fn zero_geometry_retained_events_do_not_schedule_immediate_frames() {
    assert!(!async_work_pending((0, 24), true, true));
    assert!(!async_work_pending((80, 0), true, true));
    assert!(async_work_pending((80, 24), true, false));
    assert!(async_work_pending((80, 24), false, true));
    assert!(!async_work_pending((80, 24), false, false));
    assert_eq!(
        async_wait_remaining(
            Duration::from_secs(1),
            Duration::ZERO,
            None,
            Duration::ZERO,
            false
        ),
        Duration::from_secs(1),
    );
    assert_eq!(
        async_wait_remaining(
            Duration::from_secs(1),
            Duration::ZERO,
            None,
            Duration::ZERO,
            true
        ),
        Duration::ZERO,
    );
}

fn key(modifiers: event::KeyModifiers) -> Event {
    Event::Key(event::KeyEvent {
        code: KeyCode::F(12),
        kind: event::KeyEventKind::Press,
        modifiers,
    })
}

fn char_key(ch: char, modifiers: event::KeyModifiers) -> Event {
    Event::Key(event::KeyEvent {
        code: KeyCode::Char(ch),
        kind: event::KeyEventKind::Press,
        modifiers,
    })
}

#[test]
fn terminal_text_sanitizer_replaces_control_bytes() {
    assert_eq!(
        sanitize_terminal_text("safe\x1b]52;c;AAAA\x07text\u{9b}tail"),
        "safe?]52;c;AAAA?text?tail"
    );
}

#[test]
fn fps_pacing_accounts_for_polling_and_rendering_together() {
    let remaining = fps_sleep_duration(Some(60), Duration::from_millis(16))
        .expect("a sub-frame remainder remains");
    assert!(remaining < Duration::from_millis(1));
    assert_eq!(
        fps_sleep_duration(Some(60), Duration::from_millis(20)),
        None,
        "a slow whole-loop iteration must not sleep again"
    );
    assert_eq!(fps_sleep_duration(None, Duration::ZERO), None);
}

#[test]
fn terminal_endpoint_validation_reports_each_non_tty_case() {
    assert!(validate_terminal_endpoints(true, true).is_ok());
    for endpoints in [(false, true), (true, false), (false, false)] {
        let err = validate_terminal_endpoints(endpoints.0, endpoints.1).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotConnected);
    }
}

#[test]
fn panic_hook_tracks_nested_session_ownership() {
    let _serial = PANIC_HOOK_TEST_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let outer = install_panic_hook();
    let inner = install_panic_hook();
    {
        let state = PANIC_HOOK_STATE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(state.active_sessions, 2);
        assert!(state.installed);
    }
    drop(inner);
    assert_eq!(
        PANIC_HOOK_STATE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .active_sessions,
        1
    );
    drop(outer);
    let state = PANIC_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(state.active_sessions, 0);
    assert!(!state.installed);
    assert!(state.previous.is_none());
    drop(state);

    let caught = std::panic::catch_unwind(|| {
        with_session_panic_hook(|| panic!("session panic"));
    });
    assert!(caught.is_err());
    let state = PANIC_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(state.active_sessions, 0);
    assert!(!state.installed);
    assert!(state.previous.is_none());
}

#[test]
fn static_lines_are_sanitized_before_scrollback_write() {
    let lines = vec!["ok\x1b[31mred\x07".to_string()];
    let mut out = Vec::new();
    write_static_lines_to(&mut out, &lines).unwrap();
    assert_eq!(out, b"ok?[31mred?\r\n");
}

#[test]
fn quit_frame_keeps_final_static_log_available_for_drain() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 2));
    let mut state = FrameState::default();
    let result = run_frame_kernel(
        &mut buffer,
        &mut state,
        &RunConfig::default(),
        (20, 2),
        Vec::new(),
        false,
        &mut |ui| {
            ui.static_log("final line");
            ui.quit();
        },
    );

    assert!(result.should_quit);
    assert_eq!(drain_static_log(&mut state), vec!["final line"]);
}

#[test]
fn ctrl_z_suspend_key_is_detected_separately_from_ctrl_c() {
    assert!(is_ctrl_z(&char_key('z', event::KeyModifiers::CONTROL)));
    assert!(!is_ctrl_z(&char_key('z', event::KeyModifiers::NONE)));
    assert!(is_ctrl_c(&char_key('c', event::KeyModifiers::CONTROL)));
    assert!(!is_ctrl_c(&char_key('z', event::KeyModifiers::CONTROL)));
}

#[test]
fn plain_f12_toggles_debug_mode() {
    let mut state = FrameState::default();
    let mut has_resize = false;
    assert!(!state.diagnostics.debug_mode);
    process_run_loop_event(&key(event::KeyModifiers::NONE), &mut state, &mut has_resize);
    assert!(state.diagnostics.debug_mode);
    process_run_loop_event(&key(event::KeyModifiers::NONE), &mut state, &mut has_resize);
    assert!(!state.diagnostics.debug_mode);
}

#[test]
fn shift_f12_cycles_debug_layer_without_toggling_overlay() {
    let mut state = FrameState::default();
    let mut has_resize = false;
    // Default layer is `All`; debug overlay starts off.
    assert_eq!(state.diagnostics.debug_layer, DebugLayer::All);
    assert!(!state.diagnostics.debug_mode);

    process_run_loop_event(
        &key(event::KeyModifiers::SHIFT),
        &mut state,
        &mut has_resize,
    );
    assert_eq!(state.diagnostics.debug_layer, DebugLayer::TopMost);
    // Cycling does not flip the on/off state.
    assert!(!state.diagnostics.debug_mode);

    process_run_loop_event(
        &key(event::KeyModifiers::SHIFT),
        &mut state,
        &mut has_resize,
    );
    assert_eq!(state.diagnostics.debug_layer, DebugLayer::BaseOnly);

    process_run_loop_event(
        &key(event::KeyModifiers::SHIFT),
        &mut state,
        &mut has_resize,
    );
    assert_eq!(state.diagnostics.debug_layer, DebugLayer::All);
}

#[test]
fn shift_f12_does_not_also_toggle_overlay() {
    // Regression for the modifier disambiguation: pre-fix, the F12
    // arm matched `..` modifiers so Shift+F12 would both cycle the
    // layer AND toggle the overlay on the same press.
    let mut state = FrameState::default();
    let mut has_resize = false;
    let before = state.diagnostics.debug_mode;
    process_run_loop_event(
        &key(event::KeyModifiers::SHIFT),
        &mut state,
        &mut has_resize,
    );
    assert_eq!(
        state.diagnostics.debug_mode, before,
        "Shift+F12 must not flip the on/off toggle"
    );
}

#[test]
fn plain_f12_does_not_cycle_layer() {
    // Symmetric guard: pressing plain F12 must not change the active
    // layer, only the on/off flag.
    let mut state = FrameState::default();
    let mut has_resize = false;
    let before = state.diagnostics.debug_layer;
    process_run_loop_event(&key(event::KeyModifiers::NONE), &mut state, &mut has_resize);
    assert_eq!(state.diagnostics.debug_layer, before);
}

#[cfg(feature = "async")]
#[test]
fn async_message_drain_reports_disconnect_after_sender_drop() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    tx.try_send(1u8).expect("channel has capacity");
    tx.try_send(2u8).expect("channel has capacity");
    drop(tx);

    let mut messages = Vec::new();
    let disconnected = drain_async_messages(&mut rx, &mut messages, 4);

    assert!(disconnected);
    assert_eq!(messages, vec![1, 2]);
}

#[cfg(feature = "async")]
#[test]
fn async_message_drain_respects_per_frame_budget_and_fifo_order() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    for message in 0u8..6 {
        tx.try_send(message).expect("channel has capacity");
    }

    let mut first = Vec::new();
    assert!(!drain_async_messages(&mut rx, &mut first, 2));
    assert_eq!(first, vec![0, 1]);

    let mut second = Vec::new();
    assert!(!drain_async_messages(&mut rx, &mut second, 3));
    assert_eq!(second, vec![2, 3, 4]);

    drop(tx);
    let mut final_batch = Vec::new();
    assert!(drain_async_messages(&mut rx, &mut final_batch, 3));
    assert_eq!(final_batch, vec![5]);
}

#[test]
fn async_message_budget_normalizes_zero_to_one() {
    assert_eq!(
        RunConfig::default()
            .async_message_budget(0)
            .async_message_budget,
        1
    );
}

#[cfg(feature = "async")]
fn test_async_handle(
    task: impl FnOnce(std::sync::Arc<std::sync::atomic::AtomicBool>) -> io::Result<()> + Send + 'static,
) -> AsyncRunHandle<()> {
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    let wake = std::sync::Arc::new(AsyncWake::default());
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let task_cancel = std::sync::Arc::clone(&cancel);
    let join = tokio::task::spawn_blocking(move || task(task_cancel));
    AsyncRunHandle {
        sender: Some(AsyncSender {
            inner: tx,
            wake: std::sync::Arc::clone(&wake),
        }),
        join: Some(join),
        cancel,
        wake,
        cancel_on_drop: true,
    }
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_run_handle_observes_normal_and_io_completion() {
    test_async_handle(|_| Ok(())).await.unwrap();

    let error = test_async_handle(|_| {
        Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "deterministic failure",
        ))
    })
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AsyncRunError::Io(ref err) if err.kind() == io::ErrorKind::BrokenPipe
    ));
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_run_handle_observes_panics_and_cancels_cooperatively() {
    let panic_error = test_async_handle(|_| panic!("render panic"))
        .await
        .unwrap_err();
    assert!(matches!(
        panic_error,
        AsyncRunError::Join(ref err) if err.is_panic()
    ));

    let handle = test_async_handle(|cancel| {
        while !cancel.load(std::sync::atomic::Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    });
    handle.cancel_and_join().await.unwrap();
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_sender_notifies_on_send_and_last_clone_drop() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(2);
    let wake = std::sync::Arc::new(AsyncWake::default());
    let sender = AsyncSender {
        inner: tx,
        wake: std::sync::Arc::clone(&wake),
    };

    let before_send = wake.generation();
    sender.send(7u8).await.unwrap();
    assert_ne!(wake.generation(), before_send);
    assert_eq!(rx.recv().await, Some(7));

    let clone = sender.clone();
    let before_drop = wake.generation();
    drop(clone);
    assert_ne!(wake.generation(), before_drop);
}

// ── Issue #268: Ctrl+F12 devtools inspector toggle ───────────────────

#[test]
fn ctrl_f12_toggles_inspector_independently() {
    let mut state = FrameState::default();
    let mut has_resize = false;
    assert!(!state.diagnostics.inspector_mode);

    // Ctrl+F12 flips the inspector without touching debug overlay state.
    process_run_loop_event(
        &key(event::KeyModifiers::CONTROL),
        &mut state,
        &mut has_resize,
    );
    assert!(state.diagnostics.inspector_mode);
    assert!(
        !state.diagnostics.debug_mode,
        "Ctrl+F12 must not toggle the F12 outline overlay"
    );
    assert_eq!(
        state.diagnostics.debug_layer,
        DebugLayer::All,
        "Ctrl+F12 must not cycle the debug layer"
    );

    // A second Ctrl+F12 toggles it back off.
    process_run_loop_event(
        &key(event::KeyModifiers::CONTROL),
        &mut state,
        &mut has_resize,
    );
    assert!(!state.diagnostics.inspector_mode);
}

#[test]
fn plain_and_shift_f12_do_not_touch_inspector() {
    let mut state = FrameState::default();
    let mut has_resize = false;
    // Plain F12 (overlay toggle) leaves the inspector alone.
    process_run_loop_event(&key(event::KeyModifiers::NONE), &mut state, &mut has_resize);
    assert!(state.diagnostics.debug_mode);
    assert!(!state.diagnostics.inspector_mode);
    // Shift+F12 (layer cycle) also leaves the inspector alone.
    process_run_loop_event(
        &key(event::KeyModifiers::SHIFT),
        &mut state,
        &mut has_resize,
    );
    assert!(!state.diagnostics.inspector_mode);
}

// ── Issue #263: RunConfig::handle_suspend ────────────────────────────

#[test]
fn handle_suspend_defaults_to_true() {
    assert!(RunConfig::default().handle_suspend);
}

#[test]
fn handle_suspend_builder_opts_out() {
    let cfg = RunConfig::default().handle_suspend(false);
    assert!(!cfg.handle_suspend);
}

#[test]
fn handle_suspend_builder_is_independent_of_ctrl_c() {
    // Toggling suspend must not perturb the unrelated Ctrl+C toggle.
    let cfg = RunConfig::default()
        .handle_ctrl_c(false)
        .handle_suspend(false);
    assert!(!cfg.handle_ctrl_c);
    assert!(!cfg.handle_suspend);

    let cfg = RunConfig::default().handle_suspend(true);
    assert!(cfg.handle_suspend);
    assert!(cfg.handle_ctrl_c, "Ctrl+C default preserved");
}

// ── v0.21.1: resize debounce / coalesce ─────────────────────────────

fn resize(w: u32, h: u32) -> Event {
    Event::Resize(w, h)
}

#[test]
fn resize_batch_coalesces_to_single_invocation() {
    // Three resize events in one poll batch must collapse to exactly one
    // `on_resize` call (the helper that drives the single end-of-batch
    // call in `poll_events`). The final size is irrelevant to the count —
    // `handle_resize` re-reads `terminal::size()` — but we feed distinct
    // sizes to mirror a real drag burst.
    let batch = vec![resize(80, 24), resize(100, 30), resize(120, 40)];
    assert_eq!(
        resize_invocations_for_batch(&batch),
        1,
        "a burst of resizes must coalesce to one on_resize"
    );
}

#[test]
fn resize_batch_without_resize_invokes_zero_times() {
    // A batch with no resize event must not trigger `on_resize` at all.
    let batch = vec![key(event::KeyModifiers::NONE)];
    assert_eq!(resize_invocations_for_batch(&batch), 0);
    // Empty batch is likewise a no-op.
    assert_eq!(resize_invocations_for_batch(&[]), 0);
}

#[test]
fn resize_coalesce_uses_final_size_via_has_resize_flag() {
    // The single deferred `on_resize` is gated on `has_resize`, which
    // `process_run_loop_event` sets to `true` for any resize in the batch.
    // Feeding three resizes leaves the flag set once (idempotent), and the
    // coalescing helper agrees — this is exactly the `debug_assert_eq!`
    // invariant `poll_events` checks before its single `on_resize` call.
    let mut state = FrameState::default();
    let mut has_resize = false;
    let batch = vec![resize(80, 24), resize(100, 30), resize(120, 40)];
    for ev in &batch {
        process_run_loop_event(ev, &mut state, &mut has_resize);
    }
    assert!(has_resize, "any resize in the batch must set has_resize");
    assert_eq!(
        usize::from(has_resize),
        resize_invocations_for_batch(&batch)
    );
}

/// End-to-end test of the real signal-delivery wiring: install the
/// handler, deliver a real `SIGCONT` through signal-hook's registry +
/// background thread, then drop the guard and confirm it closes the
/// registration and joins the thread without hanging or panicking.
///
/// `SIGCONT`'s default disposition is "continue", so it is safe to raise on
/// the running test process — unlike `SIGTSTP`, which would stop the test
/// runner. The suspend (`SIGTSTP`) sequence itself is covered hermetically
/// by the `write_suspend_sequence` unit tests in `terminal`.
#[cfg(unix)]
#[test]
fn suspend_handler_installs_delivers_and_tears_down() {
    // In constrained sandboxes signal registration can fail; if so the
    // wiring under test cannot be exercised, so skip rather than flake.
    let Ok(guard) = install_suspend_handler(terminal::test_session_snapshot()) else {
        return;
    };

    // Deliver a real SIGCONT; the background thread must drain it. With no
    // prior SIGTSTP the handler's `has_terminal` guard makes this a no-op
    // re-enter (idempotency), which is exactly what we want to verify does
    // not corrupt state or crash the thread.
    let _ = signal_hook::low_level::raise(signal_hook::consts::SIGCONT);
    std::thread::sleep(Duration::from_millis(50));

    // Dropping the guard closes the registration and joins the thread.
    // If `Handle::close` failed to wake `Signals::forever`, this hangs and
    // the test times out — a real regression signal.
    drop(guard);
}
