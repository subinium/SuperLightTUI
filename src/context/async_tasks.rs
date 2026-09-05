//! In-frame async task API (`Context::spawn` / `Context::poll`), feature-gated
//! behind `async`. See issues #234, #334, and #343.

use std::any::Any;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};

/// Terminal outcome of an in-frame task.
///
/// `Pending` is represented by `None` from the polling API; every value of
/// this enum is terminal and delivered at most once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskOutcome<T> {
    /// The future returned normally.
    Completed(T),
    /// The task was explicitly cancelled.
    Cancelled,
    /// The future panicked. The payload is normalized into a message.
    Panicked(String),
}

enum ErasedTaskOutcome {
    Completed(Box<dyn Any + Send>),
    Cancelled,
    Panicked(String),
}

type ResultMsg = (u64, ErasedTaskOutcome);

#[derive(Clone, Copy)]
struct CancelMsg {
    id: u64,
    retain_outcome: bool,
}

struct TaskJoin {
    worker_abort: tokio::task::AbortHandle,
    supervisor: tokio::task::JoinHandle<()>,
    discard_outcome: bool,
}

/// Opaque handle returned by [`Context::spawn`](crate::Context::spawn).
///
/// Store the handle and pass it to [`Context::poll`](crate::Context::poll) on
/// subsequent frames. Dropping it cancels the task and discards its outcome.
#[must_use = "dropping a TaskHandle cancels the spawned task; store it to poll the result"]
pub struct TaskHandle<T> {
    pub(crate) id: u64,
    cancel: Option<Sender<CancelMsg>>,
    wake: Option<Arc<tokio::sync::Notify>>,
    cancellation_requested: bool,
    _marker: PhantomData<fn() -> T>,
}

impl<T> TaskHandle<T> {
    fn new(id: u64, cancel: Sender<CancelMsg>, wake: Option<Arc<tokio::sync::Notify>>) -> Self {
        Self {
            id,
            cancel: Some(cancel),
            wake,
            cancellation_requested: false,
            _marker: PhantomData,
        }
    }

    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    /// Request cancellation while keeping the terminal outcome observable.
    ///
    /// This method is idempotent. Continue polling the handle to observe
    /// [`TaskOutcome::Cancelled`] once the runtime acknowledges the abort.
    pub fn cancel(&mut self) {
        if self.cancellation_requested {
            return;
        }
        self.cancellation_requested = true;
        if let Some(cancel) = self.cancel.as_ref() {
            let _ = cancel.send(CancelMsg {
                id: self.id,
                retain_outcome: true,
            });
            if let Some(wake) = self.wake.as_ref() {
                wake.notify_one();
            }
        }
    }
}

impl<T> std::fmt::Debug for TaskHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskHandle")
            .field("id", &self.id)
            .field("cancellation_requested", &self.cancellation_requested)
            .finish()
    }
}

impl<T> Drop for TaskHandle<T> {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(CancelMsg {
                id: self.id,
                retain_outcome: false,
            });
            if let Some(wake) = self.wake.as_ref() {
                wake.notify_one();
            }
        }
    }
}

/// Per-session async task registry, round-tripped through [`Context`] each
/// frame. Worker tasks are supervised so all normal, cancelled, and panicked
/// exits produce a terminal message and release their join entry.
#[derive(Default)]
pub(crate) struct AsyncTasks {
    runtime: Option<tokio::runtime::Handle>,
    next_id: u64,
    joins: std::collections::HashMap<u64, TaskJoin>,
    results: std::collections::HashMap<u64, ErasedTaskOutcome>,
    result_tx: Option<Sender<ResultMsg>>,
    result_rx: Option<Receiver<ResultMsg>>,
    cancel_tx: Option<Sender<CancelMsg>>,
    cancel_rx: Option<Receiver<CancelMsg>>,
    /// Coalescing wake primitive for the owning render dispatcher. `Notify`
    /// stores at most one permit, so completion bursts cannot grow a queue.
    wake: Option<Arc<tokio::sync::Notify>>,
}

impl Drop for AsyncTasks {
    fn drop(&mut self) {
        for (_, join) in self.joins.drain() {
            join.worker_abort.abort();
            join.supervisor.abort();
        }
        self.results.clear();
    }
}

impl AsyncTasks {
    pub(crate) fn set_runtime(&mut self, handle: tokio::runtime::Handle) {
        self.runtime = Some(handle);
    }

    /// Install the coalescing wake primitive used by the owning render loop.
    /// Task completion and handle cancellation each issue one notification.
    pub(crate) fn set_waker(&mut self, wake: Arc<tokio::sync::Notify>) {
        self.wake = Some(wake);
    }

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

        if self.result_tx.is_none() {
            let (tx, rx) = std::sync::mpsc::channel();
            self.result_tx = Some(tx);
            self.result_rx = Some(rx);
        }
        let result_tx = self
            .result_tx
            .clone()
            .expect("result channel initialized immediately above");

        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("in-frame async task id space exhausted");

        if self.cancel_tx.is_none() {
            let (tx, rx) = std::sync::mpsc::channel();
            self.cancel_tx = Some(tx);
            self.cancel_rx = Some(rx);
        }
        // A task may migrate threads between polls. Scope panic handling to
        // each poll rather than keeping thread-local state across an await.
        let worker = runtime.spawn(async move {
            let mut future = std::pin::pin!(fut);
            std::future::poll_fn(|cx| crate::with_recoverable_panics(|| future.as_mut().poll(cx)))
                .await
        });
        let worker_abort = worker.abort_handle();
        let completion_wake = self.wake.clone();
        let supervisor = runtime.spawn(async move {
            let outcome = match worker.await {
                Ok(value) => ErasedTaskOutcome::Completed(Box::new(value)),
                Err(error) if error.is_cancelled() => ErasedTaskOutcome::Cancelled,
                Err(error) => ErasedTaskOutcome::Panicked(join_panic_message(error)),
            };
            let _ = result_tx.send((id, outcome));
            if let Some(wake) = completion_wake {
                wake.notify_one();
            }
        });
        self.joins.insert(
            id,
            TaskJoin {
                worker_abort,
                supervisor,
                discard_outcome: false,
            },
        );

        TaskHandle::new(
            id,
            self.cancel_tx
                .as_ref()
                .expect("cancel channel initialized")
                .clone(),
            self.wake.clone(),
        )
    }

    fn drain(&mut self) {
        if let Some(rx) = self.result_rx.as_ref() {
            while let Ok((id, outcome)) = rx.try_recv() {
                let discard = self
                    .joins
                    .remove(&id)
                    .is_some_and(|join| join.discard_outcome);
                if !discard {
                    self.results.insert(id, outcome);
                }
            }
        }
        while let Some(cancel) = self.cancel_rx.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.cancel(cancel);
        }
    }

    pub(crate) fn maintain(&mut self) {
        self.drain();
    }

    pub(crate) fn poll<T: 'static>(&mut self, id: u64) -> Option<T> {
        match self.poll_outcome(id)? {
            TaskOutcome::Completed(value) => Some(value),
            TaskOutcome::Cancelled | TaskOutcome::Panicked(_) => None,
        }
    }

    pub(crate) fn poll_outcome<T: 'static>(&mut self, id: u64) -> Option<TaskOutcome<T>> {
        self.drain();
        let outcome = self.results.remove(&id)?;
        match outcome {
            ErasedTaskOutcome::Completed(value) => match value.downcast::<T>() {
                Ok(value) => Some(TaskOutcome::Completed(*value)),
                Err(value) => {
                    self.results.insert(id, ErasedTaskOutcome::Completed(value));
                    None
                }
            },
            ErasedTaskOutcome::Cancelled => Some(TaskOutcome::Cancelled),
            ErasedTaskOutcome::Panicked(message) => Some(TaskOutcome::Panicked(message)),
        }
    }

    fn cancel(&mut self, cancel: CancelMsg) {
        if !cancel.retain_outcome {
            self.results.remove(&cancel.id);
        }
        if let Some(join) = self.joins.get_mut(&cancel.id) {
            join.discard_outcome |= !cancel.retain_outcome;
            join.worker_abort.abort();
        }
    }
}

fn join_panic_message(error: tokio::task::JoinError) -> String {
    debug_assert!(error.is_panic());
    let payload = error.into_panic();
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "task panicked with a non-string payload".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    async fn wait_for_outcome<T: 'static>(
        tasks: &mut AsyncTasks,
        handle: &TaskHandle<T>,
    ) -> TaskOutcome<T> {
        for _ in 0..200 {
            if let Some(outcome) = tasks.poll_outcome(handle.id()) {
                return outcome;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        panic!("task outcome was not delivered within timeout");
    }

    #[tokio::test]
    async fn completed_cancelled_and_panicked_tasks_are_reaped() {
        let mut tasks = AsyncTasks::default();
        tasks.set_runtime(tokio::runtime::Handle::current());

        let completed = tasks.spawn(async { 7u32 });
        assert_eq!(
            wait_for_outcome(&mut tasks, &completed).await,
            TaskOutcome::Completed(7)
        );
        assert!(!tasks.joins.contains_key(&completed.id()));

        let mut cancelled = tasks.spawn(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
            9u32
        });
        cancelled.cancel();
        assert_eq!(
            wait_for_outcome(&mut tasks, &cancelled).await,
            TaskOutcome::Cancelled
        );
        assert!(!tasks.joins.contains_key(&cancelled.id()));

        let panicked = tasks.spawn(async {
            panic!("task exploded");
            #[allow(unreachable_code)]
            11u32
        });
        let outcome = wait_for_outcome(&mut tasks, &panicked).await;
        assert!(matches!(
            outcome,
            TaskOutcome::Panicked(message) if message.contains("task exploded")
        ));
        assert!(!tasks.joins.contains_key(&panicked.id()));
    }

    #[tokio::test]
    async fn completion_and_cancellation_coalesce_wake_notifications() {
        let mut tasks = AsyncTasks::default();
        tasks.set_runtime(tokio::runtime::Handle::current());
        let wake = Arc::new(tokio::sync::Notify::new());
        tasks.set_waker(Arc::clone(&wake));

        let handle = tasks.spawn(async { 1u8 });
        tokio::time::timeout(Duration::from_secs(1), wake.notified())
            .await
            .expect("task completion should wake the dispatcher");
        assert_eq!(
            wait_for_outcome(&mut tasks, &handle).await,
            TaskOutcome::Completed(1)
        );

        let mut cancelled = tasks.spawn(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
        });
        cancelled.cancel();
        tokio::time::timeout(Duration::from_secs(1), wake.notified())
            .await
            .expect("task cancellation should wake the dispatcher");
    }

    #[tokio::test]
    async fn thousands_of_task_ids_leave_no_join_entries() {
        let mut tasks = AsyncTasks::default();
        tasks.set_runtime(tokio::runtime::Handle::current());
        let handles: Vec<_> = (0..2_000u32)
            .map(|value| tasks.spawn(async move { value }))
            .collect();

        for handle in &handles {
            assert!(matches!(
                wait_for_outcome(&mut tasks, handle).await,
                TaskOutcome::Completed(_)
            ));
        }
        assert!(tasks.joins.is_empty());
        assert!(tasks.results.is_empty());
    }
}
