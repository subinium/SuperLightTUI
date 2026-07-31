//! In-frame async task API (`Context::spawn` / `Context::poll`), feature-gated
//! behind `async`. See issue #234.
//!
//! BubbleTea's `tea.Cmd` is the inspiration. SLT's existing
//! [`run_async`](crate::run_async) entry point uses an external `mpsc` channel,
//! which works but requires the caller to hold a `Sender` outside the closure
//! and wire messages manually. The in-frame API closes this ergonomics gap for
//! the common case: "click button -> spawn fetch -> show result next frame".
//!
//! This module defines [`TaskHandle`] (the opaque, `#[must_use]` handle
//! returned by `Context::spawn`) and [`AsyncTasks`] (the per-session registry
//! round-tripped through [`Context`] exactly like the scheduler timer table).

use std::any::Any;
use std::marker::PhantomData;
use std::sync::mpsc::{Receiver, Sender};

/// A completed task result delivered from a spawned future back to the
/// [`AsyncTasks`] registry: `(task id, boxed result)`.
type ResultMsg = (u64, Box<dyn Any + Send>);

/// Opaque handle returned by [`Context::spawn`](crate::Context::spawn).
///
/// Store the handle and pass it to [`Context::poll`](crate::Context::poll) on
/// subsequent frames to retrieve the task's result. **Dropping the handle
/// cancels the in-flight task** (via [`tokio::task::JoinHandle::abort`]), so
/// keep it alive for as long as you care about the result.
///
/// The type parameter `T` ties the handle to the future's output type so
/// `poll` can downcast safely. Two handles never collide: each carries a
/// unique `id`, so even two `TaskHandle<String>` live simultaneously route
/// their results to the correct caller.
///
/// Requires the `async` feature.
#[must_use = "dropping a TaskHandle cancels the spawned task; store it to poll the result"]
pub struct TaskHandle<T> {
    pub(crate) id: u64,
    /// Sends this handle's `id` to the registry on drop so the task is
    /// cancelled. `None` only for handles that were already disarmed (never
    /// happens in normal use; kept for forward flexibility).
    cancel: Option<Sender<u64>>,
    _marker: PhantomData<fn() -> T>,
}

impl<T> TaskHandle<T> {
    pub(crate) fn new(id: u64, cancel: Sender<u64>) -> Self {
        Self {
            id,
            cancel: Some(cancel),
            _marker: PhantomData,
        }
    }

    /// The stable task id this handle refers to. Crate-internal: callers match
    /// handles by identity, not by reading the raw id.
    pub(crate) fn id(&self) -> u64 {
        self.id
    }
}

impl<T> std::fmt::Debug for TaskHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskHandle").field("id", &self.id).finish()
    }
}

impl<T> Drop for TaskHandle<T> {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            // Best-effort: if the registry's receiver is already gone (session
            // ended) the send fails and there is nothing left to cancel.
            let _ = cancel.send(self.id);
        }
    }
}

/// Per-session async task registry, round-tripped through [`Context`] each
/// frame (moved out at frame start, moved back at frame end) exactly like the
/// scheduler timer table.
///
/// Holds the ambient Tokio runtime handle (injected by
/// [`run_async`](crate::run_async) / [`run_async_with`](crate::run_async_with)),
/// the live `JoinHandle`s for cancellation, and completed results keyed by id.
///
/// `Default` produces an inert registry with no runtime — `spawn` on it panics,
/// matching the documented contract that `spawn` requires an active runtime.
pub(crate) struct AsyncTasks {
    /// Ambient Tokio runtime handle. `None` outside `run_async*` (TestBackend,
    /// the sync `run` loop) — `spawn` panics in that case.
    runtime: Option<tokio::runtime::Handle>,
    /// Monotonic id allocator. Never reused within a session so a stale handle
    /// can never collide with a freshly-spawned task.
    next_id: u64,
    /// Live abort handles, keyed by task id. Removed when the result arrives or
    /// the task is cancelled.
    joins: std::collections::HashMap<u64, tokio::task::JoinHandle<()>>,
    /// Completed results awaiting a `poll`, keyed by task id. A result is
    /// inserted exactly once and removed by the first matching `poll`.
    results: std::collections::HashMap<u64, Box<dyn Any + Send>>,
    /// Sender cloned into every spawned future; the future sends its boxed
    /// result here on completion. `None` until the first `spawn` lazily wires
    /// the channel.
    result_tx: Option<Sender<ResultMsg>>,
    /// Receiver drained at the top of `spawn`/`poll` to move completed results
    /// into `results`. Paired with `result_tx`.
    result_rx: Option<Receiver<ResultMsg>>,
    /// Sender handed to each [`TaskHandle`]; the handle sends its id here on
    /// drop to request cancellation. Drained alongside the result channel.
    cancel_tx: Sender<u64>,
    /// Receiver for handle-drop cancellation requests.
    cancel_rx: Receiver<u64>,
}

impl Default for AsyncTasks {
    fn default() -> Self {
        let (cancel_tx, cancel_rx) = std::sync::mpsc::channel();
        Self {
            runtime: None,
            next_id: 0,
            joins: std::collections::HashMap::new(),
            results: std::collections::HashMap::new(),
            result_tx: None,
            result_rx: None,
            cancel_tx,
            cancel_rx,
        }
    }
}

impl Drop for AsyncTasks {
    fn drop(&mut self) {
        for (_, join) in self.joins.drain() {
            join.abort();
        }
        self.results.clear();
    }
}

impl AsyncTasks {
    /// Inject the ambient Tokio runtime handle. Called once by `run_async*`
    /// before the first frame so `spawn` has a runtime to launch onto.
    pub(crate) fn set_runtime(&mut self, handle: tokio::runtime::Handle) {
        self.runtime = Some(handle);
    }

    /// Spawn `fut` onto the ambient runtime, returning a [`TaskHandle`] keyed by
    /// a fresh id. Panics if no runtime was injected.
    pub(crate) fn spawn<T: Send + 'static>(
        &mut self,
        fut: impl std::future::Future<Output = T> + Send + 'static,
    ) -> TaskHandle<T> {
        let runtime = self.runtime.clone().unwrap_or_else(|| {
            panic!(
                "Context::spawn requires an active Tokio runtime; call it inside \
                 run_async() / run_async_with()"
            )
        });

        // Lazily wire the result channel on first spawn so the inert
        // `Default` registry carries no allocation.
        if self.result_tx.is_none() {
            let (tx, rx) = std::sync::mpsc::channel();
            self.result_tx = Some(tx);
            self.result_rx = Some(rx);
        }
        let result_tx = self
            .result_tx
            .clone()
            .expect("result_tx wired immediately above");

        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        let join = runtime.spawn(async move {
            let out = fut.await;
            // Best-effort delivery: if the registry (and its receiver) is gone
            // the result is simply dropped.
            let _ = result_tx.send((id, Box::new(out) as Box<dyn Any + Send>));
        });
        self.joins.insert(id, join);

        TaskHandle::new(id, self.cancel_tx.clone())
    }

    /// Drain the result and cancellation channels: move completed results into
    /// `results`, and abort any tasks whose handle was dropped. Called at the
    /// top of `spawn` and `poll` so the registry stays current without a
    /// dedicated per-frame tick.
    fn drain(&mut self) {
        if let Some(rx) = self.result_rx.as_ref() {
            while let Ok((id, value)) = rx.try_recv() {
                // The task finished on its own; its JoinHandle no longer needs
                // tracking for cancellation.
                self.joins.remove(&id);
                self.results.insert(id, value);
            }
        }
        while let Ok(id) = self.cancel_rx.try_recv() {
            self.cancel(id);
        }
    }

    /// Per-frame pump: move in completed results and process handle-drop
    /// cancellations. Called once per frame from the frame kernel's registry
    /// round-trip so a frame that calls neither `spawn` nor `poll` still honours
    /// a [`TaskHandle`] dropped on the previous frame (otherwise the abort would
    /// only fire on the next frame that happens to spawn or poll).
    pub(crate) fn maintain(&mut self) {
        self.drain();
    }

    /// Take the result for `id` if it has arrived. Returns `Some(T)` exactly
    /// once, then `None`.
    pub(crate) fn poll<T: 'static>(&mut self, id: u64) -> Option<T> {
        self.drain();
        let boxed = self.results.remove(&id)?;
        match boxed.downcast::<T>() {
            Ok(value) => Some(*value),
            Err(boxed) => {
                // Type mismatch should be impossible: the id-typed handle pins
                // the result type. Re-insert defensively rather than lose the
                // result, and report `None` to this (mistyped) caller.
                self.results.insert(id, boxed);
                None
            }
        }
    }

    /// Cancel the task with `id`: abort the future and drop any pending result.
    fn cancel(&mut self, id: u64) {
        if let Some(join) = self.joins.remove(&id) {
            join.abort();
        }
        self.results.remove(&id);
    }
}
