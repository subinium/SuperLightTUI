//! Async run loop (`run_async*`), message channel and render-loop handle.

use super::*;

use std::io;
use std::time::Duration;

use crate::clock::Instant;
use crate::{Color, ColorDepth, Context, Event, Terminal};

/// Error returned when an asynchronous run loop is joined.
#[cfg(all(feature = "crossterm", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "crossterm", feature = "async"))))]
#[derive(Debug)]
pub enum AsyncRunError {
    /// The render loop returned an I/O error.
    Io(io::Error),
    /// Tokio reported task cancellation or a panic from the render loop.
    Join(tokio::task::JoinError),
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl std::fmt::Display for AsyncRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "async render loop failed: {err}"),
            Self::Join(err) => write!(f, "async render task failed: {err}"),
        }
    }
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl std::error::Error for AsyncRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Join(err) => Some(err),
        }
    }
}

#[cfg(all(feature = "crossterm", feature = "async"))]
#[derive(Default)]
pub(super) struct AsyncWake {
    generation: std::sync::atomic::AtomicU64,
    notify: std::sync::Arc<tokio::sync::Notify>,
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl AsyncWake {
    fn notify(&self) {
        self.generation
            .fetch_add(1, std::sync::atomic::Ordering::Release);
        self.notify.notify_one();
    }

    pub(super) fn generation(&self) -> u64 {
        self.generation.load(std::sync::atomic::Ordering::Acquire)
    }
}

/// Bounded async message sender that wakes its associated render loop.
#[cfg(all(feature = "crossterm", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "crossterm", feature = "async"))))]
pub struct AsyncSender<M> {
    pub(super) inner: tokio::sync::mpsc::Sender<M>,
    pub(super) wake: std::sync::Arc<AsyncWake>,
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl<M> Clone for AsyncSender<M> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            wake: std::sync::Arc::clone(&self.wake),
        }
    }
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl<M> Drop for AsyncSender<M> {
    fn drop(&mut self) {
        self.wake.notify();
    }
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl<M> AsyncSender<M> {
    /// Send one message, waiting for bounded-channel capacity.
    pub async fn send(&self, message: M) -> Result<(), tokio::sync::mpsc::error::SendError<M>> {
        self.inner.send(message).await?;
        self.wake.notify();
        Ok(())
    }

    /// Try to send one message without waiting for capacity.
    pub fn try_send(&self, message: M) -> Result<(), tokio::sync::mpsc::error::TrySendError<M>> {
        self.inner.try_send(message)?;
        self.wake.notify();
        Ok(())
    }

    /// Send one message from synchronous code, blocking for capacity.
    pub fn blocking_send(&self, message: M) -> Result<(), tokio::sync::mpsc::error::SendError<M>> {
        self.inner.blocking_send(message)?;
        self.wake.notify();
        Ok(())
    }

    /// Returns `true` when the render-loop receiver has closed.
    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    /// Wait until the render-loop receiver closes.
    pub async fn closed(&self) {
        self.inner.closed().await;
    }

    /// Return the channel's remaining capacity.
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Return the channel's configured maximum capacity.
    pub fn max_capacity(&self) -> usize {
        self.inner.max_capacity()
    }
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl<M> std::ops::Deref for AsyncSender<M> {
    type Target = tokio::sync::mpsc::Sender<M>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Owned lifetime handle for an asynchronous SLT render loop.
///
/// The handle dereferences to [`AsyncSender`], preserving the common
/// `handle.send(message).await` call shape. Dropping it requests cancellation;
/// call [`join`](Self::join) to observe normal completion, I/O failures, or a
/// render-task panic.
#[cfg(all(feature = "crossterm", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "crossterm", feature = "async"))))]
#[must_use = "dropping the handle cancels the render loop; join it to observe completion"]
pub struct AsyncRunHandle<M> {
    pub(super) sender: Option<AsyncSender<M>>,
    pub(super) join: Option<tokio::task::JoinHandle<io::Result<()>>>,
    pub(super) cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub(super) wake: std::sync::Arc<AsyncWake>,
    pub(super) cancel_on_drop: bool,
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl<M> AsyncRunHandle<M> {
    /// Clone the wake-aware sender while retaining ownership of the run loop.
    pub fn sender(&self) -> AsyncSender<M> {
        self.sender
            .as_ref()
            .expect("sender is available until join starts")
            .clone()
    }

    /// Request cooperative cancellation of the render loop.
    pub fn cancel(&self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Release);
        self.wake.notify();
    }

    /// Returns `true` when the Tokio render task has completed.
    pub fn is_finished(&self) -> bool {
        self.join
            .as_ref()
            .is_none_or(tokio::task::JoinHandle::is_finished)
    }

    /// Wait for render-loop completion and preserve both I/O and join errors.
    ///
    /// Joining drops this handle's sender first. Sender clones must also be
    /// dropped, or the UI must quit, before a disconnect-driven loop can end.
    pub async fn join(mut self) -> Result<(), AsyncRunError> {
        self.sender.take();
        let join = self
            .join
            .take()
            .expect("join handle is consumed exactly once");
        join.await
            .map_err(AsyncRunError::Join)?
            .map_err(AsyncRunError::Io)
    }

    /// Request cancellation and wait for deterministic teardown.
    pub async fn cancel_and_join(mut self) -> Result<(), AsyncRunError> {
        self.cancel();
        self.sender.take();
        let join = self
            .join
            .take()
            .expect("join handle is consumed exactly once");
        join.await
            .map_err(AsyncRunError::Join)?
            .map_err(AsyncRunError::Io)
    }

    /// Detach the render task and return only a wake-aware sender.
    ///
    /// This compatibility path intentionally makes completion errors
    /// unobservable. Prefer retaining and joining the owned handle.
    pub fn detach(mut self) -> AsyncSender<M> {
        self.cancel_on_drop = false;
        self.join.take();
        self.sender
            .take()
            .expect("sender is available until detach")
    }
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl<M> std::ops::Deref for AsyncRunHandle<M> {
    type Target = AsyncSender<M>;

    fn deref(&self) -> &Self::Target {
        self.sender
            .as_ref()
            .expect("sender is available until join starts")
    }
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl<M> Drop for AsyncRunHandle<M> {
    fn drop(&mut self) {
        if self.cancel_on_drop {
            self.cancel
                .store(true, std::sync::atomic::Ordering::Release);
            self.wake.notify();
        }
    }
}

#[cfg(all(feature = "crossterm", feature = "async"))]
impl<M: Send + 'static> std::future::IntoFuture for AsyncRunHandle<M> {
    type Output = Result<(), AsyncRunError>;
    type IntoFuture =
        std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + 'static>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.join())
    }
}

/// Run the TUI loop asynchronously with default configuration.
///
/// Requires the `async` feature. Spawns the render loop in a blocking thread
/// and returns an owned [`AsyncRunHandle`]. The handle sends messages directly,
/// can be cloned into an [`AsyncSender`], supports cooperative cancellation,
/// and exposes render-loop I/O or panic failures when joined.
///
/// Returns [`io::ErrorKind::NotConnected`] when stdin or stdout is not a
/// terminal. Use [`frame`](crate::frame) with a custom backend for headless rendering.
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "async")]
/// # async fn example() -> std::io::Result<()> {
/// let run = slt::run_async::<String>(|ui, messages| {
///     for msg in messages.drain(..) {
///         ui.text(msg);
///     }
/// })?;
/// run.send("hello from async".to_string()).await.ok();
/// run.cancel_and_join().await.map_err(std::io::Error::other)?;
/// # Ok(())
/// # }
/// ```
#[cfg(all(feature = "crossterm", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "crossterm", feature = "async"))))]
pub fn run_async<M: Send + 'static>(
    f: impl FnMut(&mut Context, &mut Vec<M>) + Send + 'static,
) -> io::Result<AsyncRunHandle<M>> {
    run_async_with(RunConfig::default(), f)
}

/// Run the TUI loop asynchronously with custom configuration.
///
/// Requires the `async` feature. Like [`run_async`], but accepts a
/// [`RunConfig`] to control tick rate, mouse support, and theming.
///
/// Returns an owned [`AsyncRunHandle`] for sending, cancellation, and joining.
#[cfg(all(feature = "crossterm", feature = "async"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "crossterm", feature = "async"))))]
pub fn run_async_with<M: Send + 'static>(
    config: RunConfig,
    f: impl FnMut(&mut Context, &mut Vec<M>) + Send + 'static,
) -> io::Result<AsyncRunHandle<M>> {
    ensure_interactive_terminal()?;
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let handle =
        tokio::runtime::Handle::try_current().map_err(|err| io::Error::other(err.to_string()))?;
    let wake = std::sync::Arc::new(AsyncWake::default());
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    // Issue #234: clone the runtime handle into the render loop so
    // `Context::spawn` has a runtime to launch tasks onto. The render loop runs
    // on `spawn_blocking` (no ambient runtime), so the handle must be passed
    // explicitly rather than recovered via `Handle::try_current()` inside.
    let loop_handle = handle.clone();
    let loop_wake = std::sync::Arc::clone(&wake);
    let loop_cancel = std::sync::Arc::clone(&cancel);
    let join = handle
        .spawn_blocking(move || run_async_loop(config, f, rx, loop_handle, loop_wake, loop_cancel));

    Ok(AsyncRunHandle {
        sender: Some(AsyncSender {
            inner: tx,
            wake: std::sync::Arc::clone(&wake),
        }),
        join: Some(join),
        cancel,
        wake,
        cancel_on_drop: true,
    })
}

#[cfg(all(feature = "crossterm", feature = "async"))]
pub(super) fn drain_async_messages<M>(
    rx: &mut tokio::sync::mpsc::Receiver<M>,
    messages: &mut Vec<M>,
    budget: usize,
) -> bool {
    let mut disconnected = false;
    for _ in 0..budget.max(1) {
        match rx.try_recv() {
            Ok(message) => messages.push(message),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                disconnected = true;
                break;
            }
        }
    }
    disconnected || (rx.is_closed() && rx.is_empty())
}

#[cfg(all(feature = "crossterm", feature = "async"))]
fn run_async_loop<M: Send + 'static>(
    config: RunConfig,
    f: impl FnMut(&mut Context, &mut Vec<M>) + Send,
    rx: tokio::sync::mpsc::Receiver<M>,
    runtime: tokio::runtime::Handle,
    wake: std::sync::Arc<AsyncWake>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> io::Result<()> {
    with_session_panic_hook(move || run_async_loop_inner(config, f, rx, runtime, wake, cancel))
}

#[cfg(all(feature = "crossterm", feature = "async"))]
fn run_async_loop_inner<M: Send + 'static>(
    config: RunConfig,
    mut f: impl FnMut(&mut Context, &mut Vec<M>) + Send,
    mut rx: tokio::sync::mpsc::Receiver<M>,
    runtime: tokio::runtime::Handle,
    wake: std::sync::Arc<AsyncWake>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> io::Result<()> {
    let color_depth = config.color_depth.unwrap_or_else(ColorDepth::detect);
    let mut term = Terminal::new(
        config.mouse,
        config.kitty_keyboard,
        config.report_all_keys,
        color_depth,
    )?;
    term.apply_run_config(&config);
    if config.theme.bg != Color::Reset {
        term.theme_bg = Some(config.theme.bg);
    }
    // Issue #263: install the unix Ctrl+Z / `fg` suspend handler for the loop.
    #[cfg(unix)]
    let _suspend_guard = if config.handle_suspend {
        Some(install_suspend_handler(term.session_snapshot())?)
    } else {
        None
    };
    let mut events: Vec<Event> = Vec::new();
    let mut messages: Vec<M> = Vec::new();
    let mut state = FrameState::default();
    // Issue #234: inject the ambient runtime so `Context::spawn` works inside
    // the frame closure. Set once before the loop; round-tripped through
    // `Context` from here on (see `run_frame_kernel`).
    state.async_tasks.set_runtime(runtime.clone());
    state
        .async_tasks
        .set_waker(std::sync::Arc::clone(&wake.notify));
    let mut input_disconnected = false;

    'app: loop {
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            break;
        }
        let frame_start = Instant::now();
        // Issue #263: after a SIGCONT resume, repaint the whole frame.
        #[cfg(unix)]
        drain_resume_redraw(&mut || term.handle_resize())?;
        let observed_wake = wake.generation();
        let (w, h) = term.size();
        if w > 0 && h > 0 {
            messages.clear();
            input_disconnected |=
                drain_async_messages(&mut rx, &mut messages, config.async_message_budget);
            if input_disconnected
                && messages.is_empty()
                && events.is_empty()
                && state.pending_input.is_empty()
            {
                break;
            }
            let mut render = |ctx: &mut Context| {
                f(ctx, &mut messages);
            };
            if !run_frame(
                &mut term,
                &mut state,
                &config,
                std::mem::take(&mut events),
                true,
                &mut render,
            )? {
                break;
            }
            // Issue #233: full-screen async mode has no scrollback channel — warn
            // and drop any pending static_log lines.
            discard_static_log(&mut state, "run_async()");
            events = std::mem::take(&mut state.events_buf);
            if input_disconnected && state.pending_input.is_empty() {
                break;
            }
        } else if rx.is_closed() && rx.is_empty() {
            break;
        }

        #[cfg(unix)]
        let suspend_snapshot = term.session_snapshot();
        #[cfg(unix)]
        let mut on_suspend = || suspend_current_session(suspend_snapshot);
        #[cfg(not(unix))]
        let mut on_suspend = || Ok(());

        let wait_start = Instant::now();
        let mut notified = false;
        loop {
            if cancel.load(std::sync::atomic::Ordering::Acquire) {
                break 'app;
            }
            // A normal channel close drains only already-received UI input.
            // Keep polling terminal controls, but do not extend the drain with
            // fresh typing after the sender has disconnected.
            input_disconnected |= rx.is_closed() && rx.is_empty();
            notified |= wake.generation() != observed_wake
                || runtime.block_on(async {
                    tokio::time::timeout(Duration::ZERO, wake.notify.notified())
                        .await
                        .is_ok()
                });
            let pending = async_work_pending(
                term.size(),
                !events.is_empty() || !state.pending_input.is_empty(),
                notified || !rx.is_empty() || rx.is_closed(),
            );
            let remaining = async_wait_remaining(
                config.tick_rate,
                wait_start.elapsed(),
                config.max_fps,
                frame_start.elapsed(),
                pending,
            );
            let wait = remaining.min(Duration::from_millis(4));
            let wait = if term.size().0 == 0 || term.size().1 == 0 {
                wait.max(Duration::from_millis(4))
            } else {
                wait
            };
            if !poll_events(
                &mut events,
                &mut state,
                !input_disconnected,
                wait,
                &mut || term.handle_resize(),
                &config,
                &mut on_suspend,
            )? {
                break 'app;
            }
            let pending = async_work_pending(
                term.size(),
                !events.is_empty() || !state.pending_input.is_empty(),
                notified || !rx.is_empty() || rx.is_closed(),
            );
            if async_wait_remaining(
                config.tick_rate,
                wait_start.elapsed(),
                config.max_fps,
                frame_start.elapsed(),
                pending,
            )
            .is_zero()
            {
                break;
            }
        }
    }

    Ok(())
}

#[cfg(all(feature = "crossterm", feature = "async"))]
pub(super) fn async_work_pending(size: (u32, u32), events: bool, messages: bool) -> bool {
    size.0 > 0 && size.1 > 0 && (events || messages)
}

#[cfg(all(feature = "crossterm", feature = "async"))]
pub(super) fn async_wait_remaining(
    tick_rate: Duration,
    idle_elapsed: Duration,
    max_fps: Option<u32>,
    loop_elapsed: Duration,
    pending: bool,
) -> Duration {
    let cap = fps_sleep_duration(max_fps, loop_elapsed).unwrap_or(Duration::ZERO);
    if pending {
        cap
    } else {
        cap.max(tick_rate.saturating_sub(idle_elapsed))
    }
}
