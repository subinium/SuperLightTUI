//! Tests for the in-frame async task API (`Context::spawn` / `Context::poll`),
//! issue #234. Gated behind the `async` feature.
//!
//! `spawn` launches onto the ambient Tokio runtime; the result is delivered
//! back through an internal channel and surfaced by `poll` on a later frame.
//! Tests run under `#[tokio::test]` and inject the runtime handle into the
//! `TestBackend` via the test-only `set_async_runtime` helper (mirroring what
//! `run_async_loop` does once before its loop).
#![allow(clippy::unwrap_used)]

use super::AsyncTasks;
use crate::test_utils::TestBackend;
use std::time::Duration;

/// Wait until `cond` is satisfied by re-rendering frames, yielding to the
/// runtime between attempts so spawned tasks make progress. Avoids flakiness
/// from a single fixed sleep.
async fn render_until(
    tb: &mut TestBackend,
    mut frame: impl FnMut(&mut crate::Context),
    mut cond: impl FnMut() -> bool,
) {
    for _ in 0..200 {
        tb.render(&mut frame);
        if cond() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("condition not satisfied within timeout");
}

#[tokio::test]
async fn spawn_then_poll_returns_result_once() {
    let mut tb = TestBackend::new(40, 3);
    tb.set_async_runtime(tokio::runtime::Handle::current());

    // Frame 1: spawn a task, store its handle.
    let mut handle = None;
    tb.render(|ui| {
        handle = Some(ui.spawn(async { 7u32 }));
    });
    let handle = handle.expect("spawn returns a handle");

    // Poll across frames until the result arrives.
    // `Cell` so the frame closure and the `cond` closure can both hold a
    // shared reference (a plain `let mut` would be borrowed mutably by one and
    // immutably by the other in the same `render_until` call).
    let got: std::cell::Cell<Option<u32>> = std::cell::Cell::new(None);
    render_until(
        &mut tb,
        |ui| {
            if got.get().is_none() {
                got.set(ui.poll(&handle));
            }
        },
        || got.get().is_some(),
    )
    .await;
    assert_eq!(got.get(), Some(7));

    // A subsequent poll yields `None` — the result is taken exactly once.
    let mut second: Option<u32> = Some(0);
    tb.render(|ui| {
        second = ui.poll(&handle);
    });
    assert_eq!(second, None, "result must only be delivered once");
}

#[tokio::test]
async fn poll_returns_none_while_pending() {
    let mut tb = TestBackend::new(40, 3);
    tb.set_async_runtime(tokio::runtime::Handle::current());

    let mut handle = None;
    tb.render(|ui| {
        // Sleep long enough that it is still pending on the next frame.
        handle = Some(ui.spawn(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
            1u8
        }));
    });
    let handle = handle.expect("spawn returns a handle");

    let mut got: Option<u8> = Some(0);
    tb.render(|ui| {
        got = ui.poll(&handle);
    });
    assert_eq!(got, None, "pending task must poll to None");
}

#[tokio::test]
async fn two_handles_same_type_do_not_cross_results() {
    let mut tb = TestBackend::new(40, 3);
    tb.set_async_runtime(tokio::runtime::Handle::current());

    let mut h1 = None;
    let mut h2 = None;
    tb.render(|ui| {
        h1 = Some(ui.spawn(async { "first".to_string() }));
        h2 = Some(ui.spawn(async { "second".to_string() }));
    });
    let h1 = h1.unwrap();
    let h2 = h2.unwrap();

    // `RefCell` (values are `String`, not `Copy`) so both the frame closure and
    // the `cond` closure share access without a mutable/immutable borrow clash.
    let r1: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
    let r2: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
    render_until(
        &mut tb,
        |ui| {
            if r1.borrow().is_none() {
                *r1.borrow_mut() = ui.poll(&h1);
            }
            if r2.borrow().is_none() {
                *r2.borrow_mut() = ui.poll(&h2);
            }
        },
        || r1.borrow().is_some() && r2.borrow().is_some(),
    )
    .await;

    assert_eq!(r1.borrow().as_deref(), Some("first"));
    assert_eq!(r2.borrow().as_deref(), Some("second"));
}

#[tokio::test]
async fn dropping_handle_cancels_task() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let mut tb = TestBackend::new(40, 3);
    tb.set_async_runtime(tokio::runtime::Handle::current());

    let completed = Arc::new(AtomicBool::new(false));
    let completed_in_task = Arc::clone(&completed);

    // Spawn a task and immediately drop its handle (never store it).
    tb.render(|ui| {
        let handle = ui.spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            completed_in_task.store(true, Ordering::SeqCst);
        });
        drop(handle);
    });

    // The drop enqueued a cancellation; the next frame drains it and aborts.
    tb.render(|_ui| {});

    // Give the aborted task more than its sleep duration to (not) complete.
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert!(
        !completed.load(Ordering::SeqCst),
        "dropping the handle must cancel the in-flight task before it completes"
    );
}

#[tokio::test]
#[should_panic(expected = "requires an active Tokio runtime")]
async fn spawn_without_runtime_panics() {
    // No `set_async_runtime` call -> the registry has no runtime handle.
    let mut tb = TestBackend::new(40, 3);
    tb.render(|ui| {
        let _ = ui.spawn(async { 1u32 });
    });
}

#[test]
fn default_registry_has_no_runtime_and_polls_none() {
    // Pure unit test on the registry itself — no runtime, no spawn.
    let mut tasks = AsyncTasks::default();
    // Polling an id that was never spawned yields None.
    assert_eq!(tasks.poll::<u32>(999), None);
}
